use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// -------------------------------------------------------------------------------------------------
// Provenance
// -------------------------------------------------------------------------------------------------
/// `Provenance` indicates where a particular blob or match was found when scanning.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Provenance {
    FromFile(PathBuf),
    FromGitRepo(PathBuf),
}
