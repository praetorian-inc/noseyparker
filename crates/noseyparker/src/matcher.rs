use anyhow::Result;
use std::sync::Mutex;
use tracing::error;

use noseyparker_rules::Rule;

use crate::blob::Blob;
use crate::blob_id_map::BlobIdMap;
use crate::location::OffsetSpan;
use crate::matcher_stats::MatcherStats;
use crate::provenance_set::ProvenanceSet;
use crate::rules_database::RulesDatabase;

// -------------------------------------------------------------------------------------------------
// RawMatch
// -------------------------------------------------------------------------------------------------
/// A raw match, as recorded by a callback to Vectorscan.
///
/// When matching with Vectorscan, we simply collect all matches into a preallocated `Vec`,
/// and then go through them all after scanning is complete.
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
pub struct BlobMatch<'a> {
    /// The rule that was matched
    pub rule: &'a Rule,

    /// The blob that was matched
    pub blob: &'a Blob,

    /// The matching input in `blob.input`
    pub matching_input: &'a [u8],

    /// The location of the matching input in `blob.input`
    pub matching_input_offset_span: OffsetSpan,

    /// The capture groups from the match
    pub captures: regex::bytes::Captures<'a>,
}

struct UserData {
    /// A scratch vector for raw matches from Vectorscan, to minimize allocation
    raw_matches_scratch: Vec<RawMatch>,

    /// The length of the input being scanned
    input_len: u64,
}

// -------------------------------------------------------------------------------------------------
// Matcher
// -------------------------------------------------------------------------------------------------
/// A `Matcher` is able to scan inputs for matches from rules in a `RulesDatabase`.
///
/// If doing multi-threaded scanning, use a separate `Matcher` for each thread.
pub struct Matcher<'a> {
    /// A scratch buffer for Vectorscan
    vs_scanner: vectorscan::BlockScanner<'a>,

    /// The rules database used for matching
    rules_db: &'a RulesDatabase,

    /// Local statistics for this `Matcher`
    local_stats: MatcherStats,

    /// Global statistics, updated with the local statsistics when this `Matcher` is dropped
    global_stats: Option<&'a Mutex<MatcherStats>>,

    /// The set of blobs that have been seen
    seen_blobs: &'a BlobIdMap<bool>,

    /// Data passed to the Vectorscan callback
    user_data: UserData,
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

pub enum ScanResult<'a> {
    SeenWithMatches,
    SeenSansMatches,
    New(Vec<BlobMatch<'a>>),
}

impl<'a> Matcher<'a> {
    /// Create a new `Matcher` from the given `RulesDatabase`.
    ///
    /// If `global_stats` is provided, it will be updated with the local stats from this `Matcher`
    /// when it is dropped.
    pub fn new(
        rules_db: &'a RulesDatabase,
        seen_blobs: &'a BlobIdMap<bool>,
        global_stats: Option<&'a Mutex<MatcherStats>>,
    ) -> Result<Self> {
        let raw_matches_scratch = Vec::with_capacity(16384);
        let user_data = UserData {
            raw_matches_scratch,
            input_len: 0,
        };
        let vs_scanner = vectorscan::BlockScanner::new(&rules_db.vsdb)?;
        Ok(Matcher {
            vs_scanner,
            rules_db,
            local_stats: MatcherStats::default(),
            global_stats,
            seen_blobs,
            user_data,
        })
    }

    fn scan_bytes_raw(&mut self, input: &[u8]) -> Result<()> {
        self.user_data.raw_matches_scratch.clear();
        self.user_data.input_len = input.len().try_into().unwrap();
        self.vs_scanner
            .scan(input, |rule_id: u32, from: u64, to: u64, _flags: u32| {
                // let start_idx = if from == vectorscan_sys::HS_OFFSET_PAST_HORIZON { 0 } else { from };
                //
                // NOTE: `from` is only going to be meaningful here if we start compiling rules
                // with the HS_SOM_LEFTMOST flag. But it doesn't seem to hurt to use the 0-value
                // provided when that flag is not used.
                let start_idx = from.min(self.user_data.input_len);
                self.user_data.raw_matches_scratch.push(RawMatch {
                    rule_id,
                    start_idx,
                    end_idx: to,
                });
                vectorscan::Scan::Continue
            })?;
        Ok(())
    }

    /// Scan a blob.
    ///
    /// If the blob was already scanned, `None` is returned.
    /// Otherwise, the matches found within the blob are returned.
    ///
    /// NOTE: `provenance` is used only for diagnostic purposes if something goes wrong.
    ///
    /// NOTE: There is a race condition in determining if a blob was already scanned.
    /// There is a chance that when using multiple scanning threads that a blob will be scanned
    /// multiple times.
    ///
    /// However, only a single `ScanResult::New` result will be returned in such a case.
    pub fn scan_blob<'b>(
        &mut self,
        blob: &'b Blob,
        provenance: &ProvenanceSet,
    ) -> Result<ScanResult<'b>>
    where
        'a: 'b,
    {
        // -----------------------------------------------------------------------------------------
        // Update local stats
        // -----------------------------------------------------------------------------------------
        self.local_stats.blobs_seen += 1;
        let nbytes: u64 = blob.bytes.len().try_into().unwrap();
        self.local_stats.bytes_seen += nbytes;

        if let Some(had_matches) = self.seen_blobs.get(&blob.id) {
            // debug!("Blob {} already seen; skipping", &blob.id);
            return Ok(if had_matches {
                ScanResult::SeenWithMatches
            } else {
                ScanResult::SeenSansMatches
            });
        }

        self.local_stats.blobs_scanned += 1;
        self.local_stats.bytes_scanned += nbytes;

        // -----------------------------------------------------------------------------------------
        // Actually scan the content
        // -----------------------------------------------------------------------------------------
        self.scan_bytes_raw(&blob.bytes)?;

        let raw_matches_scratch = &mut self.user_data.raw_matches_scratch;
        if raw_matches_scratch.is_empty() {
            // No matches! We can exit early and save work.
            return Ok(match self.seen_blobs.insert(blob.id, false) {
                None => ScanResult::New(Vec::new()),

                // We raced with another thread, which beat us, but we ended up scanning anyway.
                Some(true) => ScanResult::SeenWithMatches,
                Some(false) => ScanResult::SeenSansMatches,
            });
        }

        // -----------------------------------------------------------------------------------------
        // Update rule raw match stats
        // -----------------------------------------------------------------------------------------
        #[cfg(feature = "rule_profiling")]
        for m in raw_matches_scratch.iter() {
            self.local_stats
                .rule_stats
                .increment_match_count(m.rule_id as usize, 1);
        }

        // -----------------------------------------------------------------------------------------
        // Perform second-stage regex matching to get groups and precise start locations
        //
        // Also deduplicate overlapping matches with the same rule, keeping only the longest match
        // -----------------------------------------------------------------------------------------
        raw_matches_scratch.sort_by_key(|m| {
            debug_assert!(m.start_idx <= m.end_idx);
            (m.rule_id, m.end_idx, m.end_idx - m.start_idx)
        });

        let rules = &self.rules_db.rules;
        let anchored_regexes = &self.rules_db.anchored_regexes;
        // (rule id, regex captures) from most recently emitted match
        let mut previous: Option<(usize, OffsetSpan)> = None;
        // note that we walk _backwards_ over the raw matches: this allows us to detect and
        // suppress overlapping matches in a single pass
        let matches: Vec<_> = raw_matches_scratch.iter().rev()
            .filter_map(|&RawMatch{ rule_id, start_idx, end_idx }| {
                let rule_id: usize = rule_id.try_into().unwrap();

                #[cfg(feature = "rule_profiling")]
                let _rule_profiler = self.local_stats.rule_stats.time_stage2(rule_id);

                let start_idx: usize = start_idx.try_into().unwrap();
                let end_idx: usize = end_idx.try_into().unwrap();
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
                                Regex failed to match where vectorscan did; something is probably odd about the rule:\n\
                                Blob: {}\n\
                                Provenance: {}\n\
                                Offsets: [{start_idx}..{end_idx}]\n\
                                Rule id: {rule_id}\n\
                                Rule name: {:?}:\n\
                                Regex: {re:?}:\n\
                                Snippet: {cxt:?}",
                                &blob.id,
                                provenance.first(),
                                rule.name(),
                            );
                        // });

                        return None;
                    }
                    Some(cs) => { cs }
                };

                let matching_input = captures.get(0).expect("regex captures should have group for entire match");
                let matching_input_offset_span = OffsetSpan::from_range(matching_input.range());

                // deduplicate overlaps
                if let Some((prev_rule_id, prev_loc)) = previous {
                    if prev_rule_id == rule_id && prev_loc.fully_contains(&matching_input_offset_span) {
                        return None
                    }
                }

                // Not a duplicate! Turn the RawMatch into a BlobMatch
                let m = BlobMatch {
                    rule,
                    blob,
                    matching_input: matching_input.as_bytes(),
                    matching_input_offset_span,
                    captures,
                };
                previous = Some((rule_id, matching_input_offset_span));
                Some(m)
            }).collect();
        Ok(match self.seen_blobs.insert(blob.id, !matches.is_empty()) {
            None => ScanResult::New(matches),

            // We raced with another thread, which beat us, but we ended up scanning anyway.
            Some(true) => ScanResult::SeenWithMatches,
            Some(false) => ScanResult::SeenSansMatches,
        })
    }
}

// -------------------------------------------------------------------------------------------------
// test
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod test {
    use super::*;

    use noseyparker_rules::RuleSyntax;

    use pretty_assertions::assert_eq;

    #[test]
    pub fn test_simple() -> Result<()> {
        let rules = vec![Rule::new(RuleSyntax {
            id: "test.1".to_string(),
            name: "test".to_string(),
            pattern: "test".to_string(),
            examples: vec![],
            negative_examples: vec![],
            references: vec![],
        })];
        let rules_db = RulesDatabase::from_rules(rules)?;
        let input = "some test data for vectorscan";
        let seen_blobs = BlobIdMap::new();
        let mut matcher = Matcher::new(&rules_db, &seen_blobs, None)?;
        matcher.scan_bytes_raw(input.as_bytes())?;
        assert_eq!(
            matcher.user_data.raw_matches_scratch,
            vec![RawMatch {
                rule_id: 0,
                start_idx: 0,
                end_idx: 9
            },]
        );
        Ok(())
    }
}
