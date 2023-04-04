use crate::blob_id::BlobId;
use crate::location::{LocationMapping, Location};
use crate::matcher::BlobMatch;
use crate::provenance::Provenance;
use crate::snippet::Snippet;
use crate::utils::BStringSerde;

use bstr::BString;
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------------------------------------
// Match
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Deserialize, Serialize)]
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

    /// Where did this blob come from?
    pub provenance: Provenance,
}

impl Match {
    #[inline]
    pub fn new(
        loc_mapping: &LocationMapping,
        blob_match: BlobMatch<'_, '_>,
        provenance: &Provenance,
        snippet_context_bytes: usize,
    ) -> Vec<Self> {
        let offsets = &blob_match.matching_input_offset_span;

        // FIXME: have the snippets start from a line break in the input when feasible, and include an ellipsis otherwise to indicate truncation
        let start = offsets.start.saturating_sub(snippet_context_bytes);
        let end = offsets.start;
        let before_snippet = &blob_match.blob.bytes[start..end];

        let start = offsets.end;
        let end = offsets
            .end
            .saturating_add(snippet_context_bytes)
            .min(blob_match.blob.len());
        let after_snippet = &blob_match.blob.bytes[start..end];
        let blob_id = &blob_match.blob.id;
        let rule_name = &blob_match.rule.name;
        let matching_input = blob_match.matching_input;
        let source_span = loc_mapping.get_source_span(offsets);

        blob_match
            .captures
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(group_index, group)| {
                let group = group?; // XXX should we warn on empty match groups?
                Some(Match {
                    blob_id: *blob_id,
                    rule_name: rule_name.clone(),
                    snippet: Snippet {
                        matching: BString::from(matching_input),
                        before: BString::from(before_snippet),
                        after: BString::from(after_snippet),
                    },
                    location: Location {
                        offset_span: offsets.clone(),
                        source_span: source_span.clone(),
                    },
                    match_content: BString::from(group.as_bytes()),
                    capture_group_index: group_index
                        .try_into()
                        .expect("group index should fit in u8"),
                    provenance: provenance.clone(),
                })
            })
            .collect()
    }
}
