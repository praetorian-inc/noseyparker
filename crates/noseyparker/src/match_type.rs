use bstr::BString;
use bstring_serde::BStringBase64;
use noseyparker_digest::Sha1;
use smallvec::SmallVec;
use std::io::Write;
use tracing::debug;

use crate::blob_id::BlobId;
use crate::location::{Location, LocationMapping};
use crate::matcher::BlobMatch;
use crate::snippet::Snippet;

// -------------------------------------------------------------------------------------------------
// Group
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Group(#[serde(with = "BStringBase64")] BString);

impl Group {
    pub fn new(m: regex::bytes::Match<'_>) -> Self {
        Self(BString::from(m.as_bytes()))
    }
}

// -------------------------------------------------------------------------------------------------
// Groups
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Groups(SmallVec<[Group; 1]>);

// -------------------------------------------------------------------------------------------------
// Match
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, serde::Serialize)]
pub struct Match {
    /// The blob this match comes from
    pub blob_id: BlobId,

    /// The location of the entire matching content
    pub location: Location,

    /// The capture groups
    pub groups: Groups,

    /// A snippet of the match and surrounding context
    pub snippet: Snippet,

    /// The rule that produced this match
    pub rule_structural_id: String,
}

impl Match {
    #[inline]
    pub fn convert<'a>(
        loc_mapping: &'a LocationMapping,
        blob_match: &'a BlobMatch<'a>,
        snippet_context_bytes: usize,
    ) -> Self {
        let offset_span = blob_match.matching_input_offset_span;

        // FIXME: have the snippets start from a line break in the input when feasible, and include an ellipsis otherwise to indicate truncation
        let before_snippet = {
            let start = offset_span.start.saturating_sub(snippet_context_bytes);
            let end = offset_span.start;
            &blob_match.blob.bytes[start..end]
        };

        let after_snippet = {
            let start = offset_span.end;
            let end = offset_span
                .end
                .saturating_add(snippet_context_bytes)
                .min(blob_match.blob.len());
            &blob_match.blob.bytes[start..end]
        };
        let source_span = loc_mapping.get_source_span(&offset_span);

        debug_assert!(
            blob_match.captures.len() > 1,
            "blob {}: no capture groups for rule {}",
            blob_match.blob.id,
            blob_match.rule.id()
        );

        let groups = blob_match
            .captures
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(move |(group_index, group)| {
                let group = match group {
                    Some(group) => group,
                    None => {
                        debug!(
                            "blob {}: empty match group at index {group_index}: {} {}",
                            blob_match.blob.id,
                            blob_match.rule.id(),
                            blob_match.rule.name()
                        );
                        return None;
                    }
                };
                Some(Group::new(group))
            })
            .collect();

        Match {
            blob_id: blob_match.blob.id,
            rule_structural_id: blob_match.rule.structural_id().to_owned(),
            snippet: Snippet {
                matching: BString::from(blob_match.matching_input),
                before: BString::from(before_snippet),
                after: BString::from(after_snippet),
            },
            location: Location {
                offset_span,
                source_span: source_span.clone(),
            },
            groups: Groups(groups),
        }
    }

    /// Returns the content-based unique identifier of the match.
    /// Such an identifier is defined as
    ///
    ///     sha1_hex(rule structural identifier + '\0' + hex blob id + '\0' + decimal start byte + '\0' + decimal end byte)
    pub fn structural_id(&self) -> String {
        let mut h = Sha1::new();
        write!(
            &mut h,
            "{}\0{}\0{}\0{}",
            self.rule_structural_id,
            self.blob_id.hex(),
            self.location.offset_span.start,
            self.location.offset_span.end,
        )
        .expect("should be able to compute structural id");

        h.hexdigest()
    }

    pub fn finding_id(&self) -> String {
        let mut h = Sha1::new();
        write!(&mut h, "{}\0", self.rule_structural_id)
            .expect("should be able to compute finding id");
        serde_json::to_writer(&mut h, &self.groups)
            .expect("should be able to serialize groups as JSON");
        h.hexdigest()
    }
}
