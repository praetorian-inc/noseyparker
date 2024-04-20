use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Status;
use crate::blob_id::BlobId;
use crate::match_type::Groups;

// -------------------------------------------------------------------------------------------------
// Annotation
// -------------------------------------------------------------------------------------------------
/// Represents an user-assigned annotation: a status and/or a comment
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Annotation {
    /// The content-based finding identifier for this group of matches
    pub finding_id: String,

    /// The name of the rule that detected each match
    pub rule_name: String,

    /// The textual identifier of the rule that detected each match
    pub rule_text_id: String,

    /// The structural identifier of the rule that detected the match
    pub rule_structural_id: String,

    /// The structural identifier of the match the annotations are associated with
    pub match_id: String,

    /// The blob where the match occurs
    pub blob_id: BlobId,

    /// The start byte where the match occurs
    pub start_byte: usize,

    /// The end byte where the match occurs
    pub end_byte: usize,

    /// The matched content of all the matches in the group
    pub groups: Groups,

    /// The assigned status
    pub status: Option<Status>,

    /// The assigned comment
    pub comment: Option<String>,
}

impl Annotation {
    pub fn validate(&self) -> Result<()> {
        // TODO: check that the given finding ID matches the computed one
        // TODO: check that the given match ID matches the computed one
        // TODO: check that start_byte < end_byte
        // TODO: check that at least one of status and comment are given
        // TODO: check that groups is nonempty
        // TODO: check that rule_structural_id has the correct format (40-character hex string)

        todo!();
    }
}
