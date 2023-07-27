use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use smallvec::SmallVec;

// FIXME: factor this out
use crate::input_enumerator::BlobSeenIn;

// -------------------------------------------------------------------------------------------------
// Provenance
// -------------------------------------------------------------------------------------------------
/// `Provenance` indicates where a particular blob or match was found when scanning.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all="snake_case", tag="kind")]
pub enum Provenance {
    File {
        path: PathBuf,
    },
    GitRepo {
        repo_path: PathBuf,
        first_seen: SmallVec<[BlobSeenIn; 1]>,
    },
}

impl std::fmt::Display for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provenance::File { path } => write!(f, "file {}", path.display()),
            Provenance::GitRepo { repo_path, .. } => write!(f, "git repo {}", repo_path.display()),
        }
    }
}
