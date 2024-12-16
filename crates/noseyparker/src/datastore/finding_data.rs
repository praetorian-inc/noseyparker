use crate::blob_metadata::BlobMetadata;
use crate::match_type::Match;
use crate::provenance_set::ProvenanceSet;

use super::MatchIdInt;
use super::Status;

// -------------------------------------------------------------------------------------------------
// FindingData
// -------------------------------------------------------------------------------------------------
/// A set of match data entries
pub type FindingData = Vec<FindingDataEntry>;

// -------------------------------------------------------------------------------------------------
// FindingDataEntry
// -------------------------------------------------------------------------------------------------
/// Data for a single `Match`
#[derive(Debug)]
pub struct FindingDataEntry {
    pub provenance: ProvenanceSet,
    pub blob_metadata: BlobMetadata,
    pub match_id: MatchIdInt,
    pub match_val: Match,
    pub match_comment: Option<String>,
    pub match_score: Option<f64>,
    pub match_status: Option<Status>,
    pub redundant_to: Vec<String>,
}
