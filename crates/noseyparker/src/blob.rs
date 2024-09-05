use anyhow::Result;
use std::path::Path;

pub use crate::blob_id::BlobId;

// -------------------------------------------------------------------------------------------------
// Blob
// -------------------------------------------------------------------------------------------------
pub struct Blob {
    pub id: BlobId,
    pub bytes: Vec<u8>,
}

impl Blob {
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let id = BlobId::compute_from_bytes(&bytes);
        Ok(Blob { id, bytes })
    }

    #[inline]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let id = BlobId::compute_from_bytes(&bytes);
        Blob { id, bytes }
    }

    /// Create a new `Blob` with the given ID and content.
    ///
    /// It is not checked that the ID matches that of the provided content.
    #[inline]
    pub fn new(id: BlobId, bytes: Vec<u8>) -> Self {
        Blob { id, bytes }
    }

    /// Get the size of the blob in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Is the blob empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}
