use bstr::BString;
use bstring_serde::BStringSerde;
use tracing::debug;

use crate::blob_id::BlobId;
use crate::location::{LocationMapping, Location};
use crate::matcher::BlobMatch;
use crate::snippet::Snippet;

// -------------------------------------------------------------------------------------------------
// Match
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, serde::Serialize)]
pub struct Match {
    /// The blob this match comes from
    pub blob_id: BlobId,

    /// The location of the entire matching content
    pub location: Location,

    /// The capture group number, indexed from 1
    pub capture_group_index: u8,

    /// The capture group
    #[serde(with="BStringSerde")]
    pub match_content: BString,

    /// A snippet of the match and surrounding context
    pub snippet: Snippet,

    /// The rule that produced this match
    pub rule_name: String,

    // FIXME: add pattern
    // FIXME: add pattern shasum
}

impl Match {
    #[inline]
    pub fn convert<'a>(
        loc_mapping: &'a LocationMapping,
        blob_match: &'a BlobMatch<'a>,
        snippet_context_bytes: usize,
    ) -> impl Iterator<Item=Self> + 'a {
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

        debug_assert!(blob_match.captures.len() > 1, "blob {}: no capture groups for rule {}", blob_match.blob.id, blob_match.rule.id);

        blob_match
            .captures
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(move |(group_index, group)| {
                let group = match group {
                    Some(group) => group,
                    None => {
                        debug!("blob {}: empty match group at index {group_index}: {} {}", blob_match.blob.id, blob_match.rule.id, blob_match.rule.name);
                        return None;
                    }
                };
                Some(Match {
                    blob_id: blob_match.blob.id,
                    rule_name: blob_match.rule.name.clone(),
                    snippet: Snippet {
                        matching: BString::from(blob_match.matching_input),
                        before: BString::from(before_snippet),
                        after: BString::from(after_snippet),
                    },
                    location: Location {
                        offset_span,
                        source_span: source_span.clone(),
                    },
                    match_content: BString::from(group.as_bytes()),
                    capture_group_index: group_index
                        .try_into()
                        .expect("group index should fit in u8"),
                })
            })
    }
}
