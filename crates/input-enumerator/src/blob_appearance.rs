use bstr::{BString, ByteSlice};
use gix::ObjectId;
use smallvec::SmallVec;
use std::path::Path;

/// Where was a particular blob seen?
#[derive(Clone, Debug, serde::Serialize)]
pub struct BlobAppearance {
    /// The commit ID
    pub commit_oid: ObjectId,

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
pub type BlobAppearanceSet = SmallVec<[BlobAppearance; 1]>;
