use anyhow::Result;
use std::sync::Mutex;
use tracing::error;

use crate::blob::Blob;
use crate::blob_id_set::BlobIdSet;
use crate::location::OffsetSpan;
use crate::matcher_stats::MatcherStats;
use crate::provenance::Provenance;
use crate::rules::Rule;
use crate::rules_database::RulesDatabase;

// -------------------------------------------------------------------------------------------------
// RawMatch
// -------------------------------------------------------------------------------------------------
/// A raw match, as recorded by a callback to Hyperscan.
///
/// When matching with Hyperscan, we simply collect all matches into a preallocated `Vec`,
/// and then go through them all after scanning is complete.
/// This type represents a raw match from a Hyperscan.
#[derive(PartialEq, Eq, Debug)]
struct RawMatch {
    rule_id: u32,
    start_idx: u64,
    end_idx: u64,
}

// -------------------------------------------------------------------------------------------------
// BlobMatch
// -------------------------------------------------------------------------------------------------
/// A `BlobMatch` is the result type from `Matcher::scan_blob`.
///
/// It is mostly made up of references and small data.
/// For a representation that is more friendly for human consumption, see `Match`.
pub struct BlobMatch<'r, 'b> {
    /// The rule that was matched
    pub rule: &'r Rule,

    /// The blob that was matched
    pub blob: &'b Blob,

    /// The matching input in `blob.input`
    pub matching_input: &'b [u8],

    /// The location of the matching input in `blob.input`
    pub matching_input_offset_span: OffsetSpan,

    /// The capture groups from the match
    pub captures: regex::bytes::Captures<'b>,
}

// -------------------------------------------------------------------------------------------------
// Matcher
// -------------------------------------------------------------------------------------------------
/// A `Matcher` is able to scan inputs for matches from rules in a `RulesDatabase`.
///
/// If doing multi-threaded scanning, use a separate `Matcher` for each thread.
pub struct Matcher<'a> {
    /// A scratch buffer for Hyperscan
    hs_scratch: hyperscan::Scratch,

    /// A scratch vector for raw matches from Hyperscan, to minimize allocation
    raw_matches_scratch: Vec<RawMatch>,

    /// The rules database used for matching
    rules_db: &'a RulesDatabase,

    /// Local statistics for this `Matcher`
    local_stats: MatcherStats,

    /// Global statistics, updated with the local statsistics when this `Matcher` is dropped
    global_stats: Option<&'a Mutex<MatcherStats>>,

    /// The set of blobs that have been seen
    seen_blobs: &'a BlobIdSet,
}

/// This `Drop` implementation updates the `global_stats` with the local stats
impl<'a> Drop for Matcher<'a> {
    fn drop(&mut self) {
        if let Some(global_stats) = self.global_stats {
            let mut global_stats = global_stats.lock().unwrap();
            global_stats.update(&self.local_stats);
        }
    }
}

impl<'a> Matcher<'a> {
    /// Create a new `Matcher` from the given `RulesDatabase`.
    ///
    /// If `global_stats` is provided, it will be updated with the local stats from this `Matcher`
    /// when it is dropped.
    pub fn new(
        rules_db: &'a RulesDatabase,
        seen_blobs: &'a BlobIdSet,
        global_stats: Option<&'a Mutex<MatcherStats>>,
    ) -> Result<Self> {
        Ok(Matcher {
            hs_scratch: rules_db.hsdb.alloc_scratch()?,
            raw_matches_scratch: Vec::with_capacity(16384),
            rules_db,
            local_stats: MatcherStats::default(),
            global_stats,
            seen_blobs,
        })
    }

    #[inline]
    fn scan_bytes_raw(&mut self, input: &[u8]) -> Result<()> {
        self.raw_matches_scratch.clear();
        let input_len: u64 = input.len().try_into().unwrap();
        self.rules_db
            .hsdb
            .scan(input, &self.hs_scratch, |id: u32, from: u64, to: u64, _flags: u32| {
                // let start_idx = if from == hyperscan_sys::HS_OFFSET_PAST_HORIZON { 0 } else { from };
                //
                // NOTE: `from` is only going to be meaningful here if we start compiling rules
                // with the HS_SOM_LEFTMOST flag. But it doesn't seem to hurt to use the 0-value
                // provided when that flag is not used.
                let start_idx = from.min(input_len);
                self.raw_matches_scratch.push(RawMatch {
                    rule_id: id,
                    start_idx,
                    end_idx: to,
                });
                hyperscan::Matching::Continue
            })?;
        Ok(())
    }

    /// Scan a blob.
    ///
    /// `provenance` is used only for diagnostic purposes if something goes wrong.
    // #[inline]
    pub fn scan_blob<'b>(
        &mut self,
        blob: &'b Blob,
        provenance: &Provenance,
    ) -> Result<Vec<BlobMatch<'a, 'b>>> {
        // -----------------------------------------------------------------------------------------
        // Update local stats
        // -----------------------------------------------------------------------------------------
        self.local_stats.blobs_seen += 1;
        let nbytes = blob.bytes.len() as u64;
        self.local_stats.bytes_seen += nbytes;

        if !self.seen_blobs.insert(blob.id) {
            // debug!("Blob {} already seen; skipping", &blob.id);
            return Ok(Vec::new());
        }

        self.local_stats.blobs_scanned += 1;
        self.local_stats.bytes_scanned += nbytes;

        // -----------------------------------------------------------------------------------------
        // Actually scan the content
        // -----------------------------------------------------------------------------------------
        self.scan_bytes_raw(&blob.bytes)?;

        if self.raw_matches_scratch.is_empty() {
            // No matches! We can exit early and save work.
            return Ok(Vec::new());
        }

        // -----------------------------------------------------------------------------------------
        // Perform second-stage regex matching to get groups and precise start locations
        //
        // Also deduplicate overlapping matches with the same rule
        // -----------------------------------------------------------------------------------------

        self.raw_matches_scratch.sort_by_key(|m| {
            debug_assert!(m.start_idx <= m.end_idx);
            (m.rule_id, m.end_idx, m.end_idx - m.start_idx)
        });

        let rules = &self.rules_db.rules.rules;
        let anchored_regexes = &self.rules_db.anchored_regexes;
        // (rule id, regex captures) from most recently emitted match
        let mut previous: Option<(usize, OffsetSpan)> = None;
        // note that we walk _backwards_ over the raw matches: this allows us to detect and
        // suppress overlapping matches in a single pass
        let matches = self.raw_matches_scratch.iter().rev()
            .filter_map(|&RawMatch{ rule_id, start_idx, end_idx }| {
                let rule_id = rule_id as usize;
                let start_idx = start_idx as usize;
                let end_idx = end_idx as usize;
                let rule = &rules[rule_id];
                let re = &anchored_regexes[rule_id];
                // second-stage regex match
                let captures = match re.captures(&blob.bytes[start_idx..end_idx]) {
                    None => {
                        // static ONCE: std::sync::Once = std::sync::Once::new();
                        // ONCE.call_once(|| {
                            let cxt = String::from_utf8_lossy(
                                &blob.bytes[end_idx.saturating_sub(400)..end_idx]
                            );
                            error!("\
                                Regex failed to match where hyperscan did; something is probably odd about the rule:\n\
                                Blob: {}\n\
                                Provenance: {:?}\n\
                                Offsets: [{}..{}]\n\
                                Rule id: {}\n\
                                Rule name: {:?}:\n\
                                Regex: {:?}:\n\
                                Snippet: {:?}",
                                &blob.id,
                                provenance,
                                start_idx,
                                end_idx,
                                rule_id,
                                rule.name,
                                re,
                                cxt,
                            );
                        // });
                        return None;
                    }
                    Some(cs) => { cs }
                };

                let matching_input = captures.get(0).expect("regex captures should have group for entire match");
                let matching_input_offset_span = OffsetSpan::from_range(matching_input.range());

                // deduplicate overlaps
                let suppress = match &previous {
                    None => false,
                    Some((prev_rule_id, prev_loc)) => {
                        *prev_rule_id == rule_id && prev_loc.fully_contains(&matching_input_offset_span)
                    }
                };
                if suppress {
                    return None;
                }

                // Not a duplicate! Turn the RawMatch into a BlobMatch
                let m = BlobMatch {
                    rule,
                    blob,
                    matching_input: matching_input.as_bytes(),
                    matching_input_offset_span: matching_input_offset_span.clone(),
                    captures,
                };
                previous = Some((rule_id, matching_input_offset_span));
                Some(m)
            });
        Ok(matches.collect())
    }
}

// -------------------------------------------------------------------------------------------------
// test
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod test {
    use super::*;

    use crate::rules::Rules;

    use pretty_assertions::assert_eq;

    #[test]
    pub fn test_simple() -> Result<()> {
        let rules = vec![Rule {
            name: "test".to_string(),
            pattern: "test".to_string(),
            examples: vec![],
            negative_examples: vec![],
            references: vec![],
        }];
        let rules = Rules { rules };
        let rules_db = RulesDatabase::from_rules(rules)?;
        let input = "some test data for hyperscan";
        let seen_blobs = BlobIdSet::new();
        let mut matcher = Matcher::new(&rules_db, &seen_blobs, None)?;
        matcher.scan_bytes_raw(input.as_bytes())?;
        assert_eq!(
            matcher.raw_matches_scratch,
            vec![RawMatch {
                rule_id: 0,
                start_idx: 0,
                end_idx: 9
            },]
        );
        Ok(())
    }
}
