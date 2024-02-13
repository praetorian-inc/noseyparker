use bstr::BString;
use bstring_serde::BStringLossyUtf8;
use input_enumerator::git_commit_metadata::CommitMetadata;
use serde::Serialize;
use std::path::PathBuf;


// -------------------------------------------------------------------------------------------------
// Provenance
// -------------------------------------------------------------------------------------------------
/// `Provenance` indicates where a particular blob or match was found when scanning.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[allow(clippy::large_enum_variant)]
pub enum Provenance {
    File(FileProvenance),
    GitRepo(GitRepoProvenance),
    Extended(ExtendedProvenance),
}

impl Provenance {
    /// Create a `Provenance` entry for a plain file.
    pub fn from_file(path: PathBuf) -> Self {
        Provenance::File(FileProvenance {
            path,
        })
    }

    /// Create a `Provenance` entry for a blob found within a Git repo's history, without any extra
    /// commit provenance.
    ///
    /// See also `from_git_repo_with_first_commit`.
    pub fn from_git_repo(repo_path: PathBuf) -> Self {
        Provenance::GitRepo(GitRepoProvenance {
            repo_path,
            first_commit: None,
        })
    }

    /// Create a `Provenance` entry for a blob found within a Git repo's history, with commit
    /// provenance.
    ///
    /// See also `from_git_repo`.
    pub fn from_git_repo_with_first_commit(
        repo_path: PathBuf,
        commit_metadata: CommitMetadata,
        blob_path: BString,
    ) -> Self {
        let first_commit = Some(CommitProvenance {
            commit_metadata,
            blob_path,
        });
        Provenance::GitRepo(GitRepoProvenance {
            repo_path,
            first_commit,
        })
    }

    /// Create a `Provenance` entry from a JSON object.
    pub fn from_extended(value: serde_json::Value) -> Self {
        Provenance::Extended(ExtendedProvenance(value))
    }
}

impl std::fmt::Display for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provenance::File(e) => write!(f, "file {}", e.path.display()),
            Provenance::GitRepo(e) => match &e.first_commit {
                Some(md) => write!(
                    f,
                    "git repo {}: first seen in commit {} as {}",
                    e.repo_path.display(),
                    md.commit_metadata.commit_id,
                    md.blob_path,
                ),
                None => write!(f, "git repo {}", e.repo_path.display()),
            },
            Provenance::Extended(e) => {
                write!(f, "extended {}", e)
            },
        }
    }
}

// -------------------------------------------------------------------------------------------------
// FileProvenance
// -------------------------------------------------------------------------------------------------
/// Indicates that a blob was seen at a particular file path
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileProvenance {
    pub path: PathBuf,
}

// -------------------------------------------------------------------------------------------------
// GitRepoProvenance
// -------------------------------------------------------------------------------------------------
/// Indicates that a blob was seen in a Git repo, optionally with particular commit provenance info
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GitRepoProvenance {
    pub repo_path: PathBuf,
    pub first_commit: Option<CommitProvenance>,
}

// -------------------------------------------------------------------------------------------------
// CommitProvenance
// -------------------------------------------------------------------------------------------------
/// How was a particular Git commit encountered?
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommitProvenance {
    pub commit_metadata: CommitMetadata,

    #[serde(with = "BStringLossyUtf8")]
    pub blob_path: BString,
}

// -------------------------------------------------------------------------------------------------
// ExtendedProvenance
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExtendedProvenance(pub serde_json::Value);

impl std::fmt::Display for ExtendedProvenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
