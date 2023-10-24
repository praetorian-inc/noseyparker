use bstr::BString;
use bstring_serde::BStringSerde;
use input_enumerator::git_commit_metadata::CommitMetadata;
use serde::Serialize;
use std::path::PathBuf;


// -------------------------------------------------------------------------------------------------
// Provenance
// -------------------------------------------------------------------------------------------------
/// `Provenance` indicates where a particular blob or match was found when scanning.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[allow(clippy::large_enum_variant)]
pub enum Provenance {
    File(FileProvenance),
    GitRepo(GitRepoProvenance),
}

impl Provenance {
    /// Create a `Provenance` entry for a plain file.
    pub fn from_file(path: PathBuf) -> Provenance {
        Provenance::File(FileProvenance {
            path,
        })
    }

    /// Create a `Provenance` entry for a blob found within a Git repo's history, without any extra
    /// commit provenance.
    ///
    /// See also `from_git_repo_and_commit`.
    pub fn from_git_repo(repo_path: PathBuf) -> Provenance {
        Provenance::GitRepo(GitRepoProvenance {
            repo_path,
            commit_provenance: None,
        })
    }

    /// Create a `Provenance` entry for a blob found within a Git repo's history, with commit
    /// provenance.
    ///
    /// See also `from_git_repo`.
    pub fn from_git_repo_and_commit_metadata(
        repo_path: PathBuf,
        commit_kind: CommitKind,
        commit_metadata: CommitMetadata,
        blob_path: BString,
    ) -> Provenance {
        let commit_provenance = Some(CommitProvenance {
            commit_kind,
            commit_metadata,
            blob_path,
        });
        Provenance::GitRepo(GitRepoProvenance {
            repo_path,
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
                    md.commit_metadata.commit_id,
                    md.blob_path,
                ),
                None => write!(f, "git repo {}", e.repo_path.display()),
            },
        }
    }
}

/// Indicates that a blob was seen at a particular file path
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct FileProvenance {
    pub path: PathBuf,
}

/// Indicates that a blob was seen in a Git repo, optionally with particular commit provenance info
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct GitRepoProvenance {
    pub repo_path: PathBuf,
    pub commit_provenance: Option<CommitProvenance>,
}

/// What is the kind of this commit metadata?
#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CommitKind {
    /// The first commit in which a blob was seen
    FirstSeen,

    /// The last commit in which a blob was seen
    LastSeen,
}

impl rusqlite::types::ToSql for CommitKind {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            CommitKind::FirstSeen => Ok("first_seen".into()),
            CommitKind::LastSeen => Ok("last_seen".into()),
        }
    }
}

impl rusqlite::types::FromSql for CommitKind {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "first_seen" => Ok(CommitKind::FirstSeen),
            "last_seen" => Ok(CommitKind::LastSeen),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

impl std::fmt::Display for CommitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::FirstSeen => "first seen",
            Self::LastSeen => "last seen",
        })
    }
}

/// What is the kind of a provenance object?
#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind {
    GitRepo,
    GitCommit,
    File,
}

impl rusqlite::types::ToSql for ProvenanceKind {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            ProvenanceKind::GitRepo => Ok("git_repo".into()),
            ProvenanceKind::GitCommit => Ok("git_commit".into()),
            ProvenanceKind::File => Ok("file".into()),
        }
    }
}

impl rusqlite::types::FromSql for ProvenanceKind {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "git_repo" => Ok(ProvenanceKind::GitRepo),
            "git_commit" => Ok(ProvenanceKind::GitCommit),
            "file" => Ok(ProvenanceKind::File),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

/// How was a particular Git commit encountered?
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct CommitProvenance {
    pub commit_kind: CommitKind,

    pub commit_metadata: CommitMetadata,

    #[serde(with = "BStringSerde")]
    pub blob_path: BString,
}
