use bstr::BString;
use bstring_serde::BStringLossyUtf8;
use input_enumerator::git_commit_metadata::CommitMetadata;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// -------------------------------------------------------------------------------------------------
// Provenance
// -------------------------------------------------------------------------------------------------
/// `Provenance` indicates where a particular blob or match was found when scanning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
        Provenance::File(FileProvenance { path })
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

    /// Create a `Provenance` entry from an arbitrary JSON value.
    pub fn from_extended(value: serde_json::Value) -> Self {
        Provenance::Extended(ExtendedProvenance(value))
    }

    /// Get the path for the blob from this `Provenance` entry, if one is specified.
    pub fn blob_path(&self) -> Option<&Path> {
        use bstr::ByteSlice;
        match self {
            Self::File(e) => Some(&e.path),
            Self::GitRepo(e) => e
                .first_commit
                .as_ref()
                .and_then(|c| c.blob_path.to_path().ok()),
            Self::Extended(e) => e.path(),
        }
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
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// FileProvenance
// -------------------------------------------------------------------------------------------------
/// Indicates that a blob was seen at a particular file path
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FileProvenance {
    pub path: PathBuf,
}

// -------------------------------------------------------------------------------------------------
// GitRepoProvenance
// -------------------------------------------------------------------------------------------------
/// Indicates that a blob was seen in a Git repo, optionally with particular commit provenance info
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitRepoProvenance {
    pub repo_path: PathBuf,
    pub first_commit: Option<CommitProvenance>,
}

// -------------------------------------------------------------------------------------------------
// CommitProvenance
// -------------------------------------------------------------------------------------------------
/// How was a particular Git commit encountered?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommitProvenance {
    pub commit_metadata: CommitMetadata,

    #[serde(with = "BStringLossyUtf8")]
    pub blob_path: BString,
}

// -------------------------------------------------------------------------------------------------
// ExtendedProvenance
// -------------------------------------------------------------------------------------------------
/// An extended provenance entry.
///
/// This is an arbitrary JSON value.
/// If the value is an object containing certain fields, they will be interpreted specially by
/// Nosey Parker:
///
/// - A `path` field containing a string
//
// - XXX A `url` string field that is a syntactically-valid URL
// - XXX A `time` string field
// - XXX A `display` string field
//
// - XXX A `parent_blob` string field with a hex-encoded blob ID that the associated blob was derived from
// - XXX A `parent_transform` string field identifying the transform method used to derive the associated blob
// - XXX A `parent_start_byte` integer field
// - XXX A `parent_end_byte` integer field
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtendedProvenance(pub serde_json::Value);

impl std::fmt::Display for ExtendedProvenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl ExtendedProvenance {
    pub fn path(&self) -> Option<&Path> {
        let p = self.0.get("path")?.as_str()?;
        Some(Path::new(p))
    }
}

// -------------------------------------------------------------------------------------------------
// sql
// -------------------------------------------------------------------------------------------------
mod sql {
    use super::*;

    use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
    use rusqlite::Error::ToSqlConversionFailure;

    impl ToSql for Provenance {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            match serde_json::to_string(self) {
                Err(e) => Err(ToSqlConversionFailure(e.into())),
                Ok(s) => Ok(s.into()),
            }
        }
    }

    impl FromSql for Provenance {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            let s = value.as_str()?;
            serde_json::from_str(s).map_err(|e| FromSqlError::Other(e.into()))
        }
    }
}
