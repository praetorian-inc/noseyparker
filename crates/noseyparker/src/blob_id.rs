use anyhow::Result;
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------------------------------------
// BlobId
// -------------------------------------------------------------------------------------------------
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone, Deserialize, Serialize)]
#[serde(into="String", try_from="&str")]
pub struct BlobId([u8; 20]);

impl std::fmt::Debug for BlobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlobId({})", self.hex())
    }
}

impl BlobId {
    /// Create a new BlobId computed from the given input.
    #[inline]
    pub fn new(input: &[u8]) -> Self {
        use crate::digest::Sha1;
        use std::io::Write;

        // XXX implement a Write instance for `Sha1`, in an attempt to avoid allocations for
        // formatting the input length. Not sure how well this actually avoids allocation.
        struct Sha1Writer(Sha1);

        impl Write for Sha1Writer {
            #[inline]
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.update(buf);
                Ok(buf.len())
            }

            #[inline]
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut writer = Sha1Writer(Sha1::default());
        write!(&mut writer, "blob {}\0", input.len()).unwrap();
        writer.0.update(input);
        BlobId(writer.0.digest())
    }

    #[inline]
    pub fn from_hex(v: &str) -> Result<Self> {
        Ok(BlobId(hex::decode(v)?.as_slice().try_into()?))
    }

    #[inline]
    pub fn hex(&self) -> String {
        hex::encode(self.0)
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<BlobId> for String where {
    #[inline]
    fn from(blob_id: BlobId) -> String {
        blob_id.hex()
    }
}

impl TryFrom<&str> for BlobId where {
    type Error = anyhow::Error;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        BlobId::from_hex(s)
    }
}

impl std::fmt::Display for BlobId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hex())
    }
}

impl<'a> From<&'a gix::ObjectId> for BlobId {
    #[inline]
    fn from(id: &'a gix::ObjectId) -> Self {
        BlobId(
            id.as_bytes()
                .try_into()
                .expect("oid should be a 20-byte value"),
        )
    }
}

impl From<gix::ObjectId> for BlobId {
    #[inline]
    fn from(id: gix::ObjectId) -> Self {
        BlobId(
            id.as_bytes()
                .try_into()
                .expect("oid should be a 20-byte value"),
        )
    }
}

impl<'a> From<&'a BlobId> for gix::ObjectId {
    #[inline]
    fn from(blob_id: &'a BlobId) -> Self {
        gix::hash::ObjectId::from(blob_id.as_bytes())
    }
}

impl From<BlobId> for gix::ObjectId {
    #[inline]
    fn from(blob_id: BlobId) -> Self {
        gix::hash::ObjectId::from(blob_id.as_bytes())
    }
}

// -------------------------------------------------------------------------------------------------
// test
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn simple() {
        assert_eq!(BlobId::new(&vec![0; 0]).hex(), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
        assert_eq!(BlobId::new(&vec![0; 1024]).hex(), "06d7405020018ddf3cacee90fd4af10487da3d20");
    }
}
