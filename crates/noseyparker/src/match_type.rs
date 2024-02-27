use bstr::BString;
use bstring_serde::BStringBase64;
use noseyparker_digest::Sha1;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::io::Write;
use tracing::debug;

use crate::blob_id::BlobId;
use crate::location::{Location, LocationMapping, OffsetSpan};
use crate::matcher::BlobMatch;
use crate::snippet::Snippet;

// -------------------------------------------------------------------------------------------------
// Group
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Group(#[serde(with = "BStringBase64")] pub BString);

impl Group {
    pub fn new(m: regex::bytes::Match<'_>) -> Self {
        Self(BString::from(m.as_bytes()))
    }
}

// -------------------------------------------------------------------------------------------------
// Groups
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Groups(pub SmallVec<[Group; 1]>);

// -------------------------------------------------------------------------------------------------
// sql
// -------------------------------------------------------------------------------------------------
mod sql {
    use super::*;

    use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
    use rusqlite::Error::ToSqlConversionFailure;

    impl ToSql for Groups {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            match serde_json::to_string(self) {
                Err(e) => Err(ToSqlConversionFailure(e.into())),
                Ok(s) => Ok(s.into()),
            }
        }
    }

    impl FromSql for Groups {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            match value {
                ValueRef::Text(s) => {
                    serde_json::from_slice(s).map_err(|e| FromSqlError::Other(e.into()))
                }
                ValueRef::Blob(b) => {
                    serde_json::from_slice(b).map_err(|e| FromSqlError::Other(e.into()))
                }
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Match
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Match {
    /// The blob this match comes from
    pub blob_id: BlobId,

    /// The location of the entire matching content
    pub location: Location,

    /// The capture groups
    pub groups: Groups,

    /// A snippet of the match and surrounding context
    pub snippet: Snippet,

    /// The unique content-based identifier of this match
    pub structural_id: String,

    /// The rule that produced this match
    pub rule_structural_id: String,

    /// The text identifier of the rule that produced this match
    pub rule_text_id: String,

    /// The name of the rule that produced this match
    pub rule_name: String,
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

        let rule_structural_id = blob_match.rule.structural_id().to_owned();
        let structural_id =
            Self::compute_structural_id(&rule_structural_id, &blob_match.blob.id, offset_span);

        Match {
            blob_id: blob_match.blob.id,
            rule_structural_id,
            rule_name: blob_match.rule.name().to_owned(),
            rule_text_id: blob_match.rule.id().to_owned(),
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
            structural_id,
        }
    }

    /// Returns a content-based unique identifier of the match.
    fn compute_structural_id(
        rule_structural_id: &str,
        blob_id: &BlobId,
        span: OffsetSpan,
    ) -> String {
        let mut h = Sha1::new();
        write!(
            &mut h,
            "{}\0{}\0{}\0{}",
            rule_structural_id,
            blob_id.hex(),
            span.start,
            span.end,
        )
        .expect("should be able to compute structural id");

        h.hexdigest()
    }

    pub fn finding_id(&self) -> String {
        let mut h = Sha1::new();
        write!(&mut h, "{}\0", self.rule_structural_id).expect("should be able to write to memory");
        serde_json::to_writer(&mut h, &self.groups)
            .expect("should be able to serialize groups as JSON");
        h.hexdigest()
    }
}
