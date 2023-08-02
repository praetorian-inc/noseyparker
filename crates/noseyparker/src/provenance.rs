use bstr::BString;
use serde::Serialize;
use std::path::{Path, PathBuf};

// -------------------------------------------------------------------------------------------------
// Provenance
// -------------------------------------------------------------------------------------------------
/// `Provenance` indicates where a particular blob or match was found when scanning.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Provenance {
    File(FileProvenance),
    GitRepo(GitRepoProvenance),
}

impl Provenance {
    /// Create a `Provenance` entry for a plain file.
    pub fn from_file(fname: &Path) -> Provenance {
        Provenance::File(FileProvenance {
            path: fname.to_owned(),
        })
    }

    /// Create a `Provenance` entry for a blob found within a Git repo's history, without any extra
    /// commit provenance.
    ///
    /// See also `from_git_repo_and_commit`.
    pub fn from_git_repo(repo_path: &Path) -> Provenance {
        Provenance::GitRepo(GitRepoProvenance {
            repo_path: repo_path.to_owned(),
            commit_provenance: None,
        })
    }

    /// Create a `Provenance` entry for a blob found within a Git repo's history, with commit
    /// provenance.
    ///
    /// See also `from_git_repo`.
    pub fn from_git_repo_and_commit(
        repo_path: &Path,
        commit_kind: CommitKind,
        commit_id: gix::ObjectId,
        blob_path: BString,
    ) -> Provenance {
        let commit_provenance = Some(CommitProvenance {
            commit_kind,
            commit_id,
            blob_path,
        });
        Provenance::GitRepo(GitRepoProvenance {
            repo_path: repo_path.to_owned(),
            commit_provenance,
        })
    }
}

impl std::fmt::Display for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provenance::File(e) => write!(f, "file {}", e.path.display()),
            Provenance::GitRepo(e) => match &e.commit_provenance {
                Some(md) => write!(
                    f,
                    "git repo {}: {} in commit {} as {}",
                    e.repo_path.display(),
                    md.commit_kind,
                    md.commit_id,
                    md.blob_path,
                ),
                None => write!(f, "git repo {}", e.repo_path.display()),
            },
        }
    }
}

/// Indicates that a blob was seen at a particular file path
#[derive(Debug, Clone, Serialize)]
pub struct FileProvenance {
    pub path: PathBuf,
}

/// Indicates that a blob was seen in a Git repo, optionally with particular commit provenance info
#[derive(Debug, Clone, Serialize)]
pub struct GitRepoProvenance {
    pub repo_path: PathBuf,
    pub commit_provenance: Option<CommitProvenance>,
}

/// What is the kind of this commit metadata?
#[derive(Debug, Copy, Clone, Serialize)]
pub enum CommitKind {
    /// The first commit in which a blob was seen
    FirstSeen,

    /// The last commit in which a blob was seen
    LastSeen,
}

impl std::fmt::Display for CommitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::FirstSeen => "first seen",
            Self::LastSeen => "last seen",
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitProvenance {
    pub commit_kind: CommitKind,
    pub commit_id: gix::ObjectId,
    pub blob_path: BString,
}
