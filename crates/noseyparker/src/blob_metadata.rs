use crate::blob_id::BlobId;

/// Metadata about a blob
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct BlobMetadata {
    /// The blob ID this metadata applies to
    pub id: BlobId,

    /// The length in bytes of the blob
    pub num_bytes: usize,

    /// The guessed multimedia type of the blob
    pub mime_essence: Option<String>,

    /// The guessed charset of the blob
    pub charset: Option<String>,
}

impl BlobMetadata {
    /// Get the length of the blob in bytes.
    #[inline]
    pub fn num_bytes(&self) -> usize {
        self.num_bytes
    }

    #[inline]
    pub fn mime_essence(&self) -> Option<&str> {
        self.mime_essence.as_deref()
    }

    #[inline]
    pub fn charset(&self) -> Option<&str> {
        self.charset.as_deref()
    }
}
