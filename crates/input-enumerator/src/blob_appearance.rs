use crate::git_commit_metadata::CommitMetadata;
use bstr::{BString, ByteSlice};
use smallvec::SmallVec;
use std::path::Path;
use std::sync::Arc;

/// Where was a particular blob seen?
#[derive(Clone, Debug, serde::Serialize)]
pub struct BlobAppearance {
    pub commit_metadata: Arc<CommitMetadata>,

    /// The path given to the blob
    pub path: BString,
}

impl BlobAppearance {
    #[inline]
    pub fn path(&self) -> Result<&Path, bstr::Utf8Error> {
        self.path.to_path()
    }
}

/// A set of `BlobAppearance` entries
pub type BlobAppearanceSet = SmallVec<[BlobAppearance; 2]>;
