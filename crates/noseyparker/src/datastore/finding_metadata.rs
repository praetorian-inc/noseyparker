use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Statuses;
use crate::match_type::Groups;

// -------------------------------------------------------------------------------------------------
// FindingMetadata
// -------------------------------------------------------------------------------------------------
/// Metadata for a group of matches that have identical rule name and match content.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FindingMetadata {
    /// The content-based finding identifier for this group of matches
    pub finding_id: String,

    /// The name of the rule that detected each match
    pub rule_name: String,

    /// The textual identifier of the rule that detected each match
    pub rule_text_id: String,

    /// The structural identifier of the rule that detected each match
    pub rule_structural_id: String,

    /// The matched content of all the matches in the group
    pub groups: Groups,

    /// The number of matches in the group
    pub num_matches: usize,

    /// The number of matches in the group that are considered redundant
    pub num_redundant_matches: usize,

    /// The unique statuses assigned to matches in the group
    pub statuses: Statuses,

    /// A comment assigned to this finding
    pub comment: Option<String>,

    /// The mean score in this group of matches
    pub mean_score: Option<f64>,
}
