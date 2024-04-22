use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Status;
use crate::blob_id::BlobId;
use crate::match_type::Groups;

// TODO: include source location information in annotations?

// -------------------------------------------------------------------------------------------------
// MatchAnnotation
// -------------------------------------------------------------------------------------------------
/// Represents an user-assigned annotation on a match: a status and/or a comment
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct MatchAnnotation {
    /// The content-based finding identifier
    pub finding_id: String,

    /// The name of the rule that detected the match
    pub rule_name: String,

    /// The textual identifier of the rule that detected the match
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

    /// The capture groups of the match
    pub groups: Groups,

    /// The assigned status
    pub status: Option<Status>,

    /// The assigned comment
    pub comment: Option<String>,
}

impl MatchAnnotation {
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

// -------------------------------------------------------------------------------------------------
// FindingAnnotation
// -------------------------------------------------------------------------------------------------
/// Represents an user-assigned annotation on a finding: a comment
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FindingAnnotation {
    /// The content-based finding identifier
    pub finding_id: String,

    /// The name of the rule that detected the finding
    pub rule_name: String,

    /// The textual identifier of the rule that detected the finding
    pub rule_text_id: String,

    /// The structural identifier of the rule that detected the finding
    pub rule_structural_id: String,

    /// The capture groups of the finding
    pub groups: Groups,

    /// The assigned comment
    pub comment: String,
}

impl FindingAnnotation {
    pub fn validate(&self) -> Result<()> {
        todo!();
    }
}

// -------------------------------------------------------------------------------------------------
// Annotations
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Annotations {
    pub match_annotations: Vec<MatchAnnotation>,
    pub finding_annotations: Vec<FindingAnnotation>,
}

impl Annotations {
    pub fn validate(&self) -> Result<()> {
        self.match_annotations
            .iter()
            .try_for_each(|a| a.validate())?;
        self.finding_annotations
            .iter()
            .try_for_each(|a| a.validate())?;
        Ok(())
    }
}
