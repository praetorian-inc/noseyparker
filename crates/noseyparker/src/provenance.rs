use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// -------------------------------------------------------------------------------------------------
// Provenance
// -------------------------------------------------------------------------------------------------
/// `Provenance` indicates where a particular blob or match was found when scanning.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all="snake_case", tag="kind")]
pub enum Provenance {
    File {
        path: PathBuf,
    },
    GitRepo {
        path: PathBuf,
    },
}
