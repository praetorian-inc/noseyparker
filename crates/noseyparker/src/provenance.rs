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

impl std::fmt::Display for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provenance::File { path } => write!(f, "file {:?}", path),
            Provenance::GitRepo { path } => write!(f, "git repo {:?}", path),
        }
    }
}
