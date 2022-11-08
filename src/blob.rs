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
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let id = BlobId::new(&bytes);
        Ok(Blob { id, bytes })
    }

    /// Create a new `Blob` with the given ID and content.
    ///
    /// It is not checked that the ID matches that of the provided content.
    pub fn new(id: BlobId, bytes: Vec<u8>) -> Self {
        Blob { id, bytes }
    }

    /// Get the size of the blob in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}
