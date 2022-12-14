use anyhow::Result;
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------------------------------------
// BlobId
// -------------------------------------------------------------------------------------------------
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(into="String", try_from="&str")]
pub struct BlobId([u8; 20]);

impl BlobId {
    #[inline]
    pub fn new(input: &[u8]) -> Self {
        use openssl::sha;
        use std::io::Write;

        // XXX implement a Write instance for `sha::Sha1`, in an attempt to avoid allocations for
        // formatting the input length. Not sure how well this actually avoids allocation.
        struct Sha1Writer(sha::Sha1);

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

        let mut writer = Sha1Writer(sha::Sha1::new());
        write!(&mut writer, "blob {}\0", input.len()).unwrap();
        writer.0.update(input);
        BlobId(writer.0.finish())
    }

    #[inline]
    pub fn from_oid(oid: &git2::Oid) -> Self {
        BlobId(
            oid.as_bytes()
                .try_into()
                .expect("oid should be a 20-byte value"),
        )
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
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<BlobId> for String where {
    fn from(blob_id: BlobId) -> String {
        blob_id.hex()
    }
}

impl TryFrom<&str> for BlobId where {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        BlobId::from_hex(s)
    }
}

impl std::fmt::Display for BlobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hex())
    }
}

// -------------------------------------------------------------------------------------------------
// test
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    fn git2_hash_hex(input: &[u8]) -> String {
        hex::encode(
            git2::Oid::hash_object(git2::ObjectType::Blob, &input)
                .unwrap()
                .as_ref(),
        )
    }

    #[test]
    fn sanity_check_git2_reference() {
        assert_eq!(git2_hash_hex(&vec![0; 0]), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
        assert_eq!(git2_hash_hex(&vec![0; 1024]), "06d7405020018ddf3cacee90fd4af10487da3d20");
    }

    #[test]
    fn simple() {
        assert_eq!(BlobId::new(&vec![0; 0]).hex(), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
        assert_eq!(BlobId::new(&vec![0; 1024]).hex(), "06d7405020018ddf3cacee90fd4af10487da3d20");
    }

    proptest! {
        #[test]
        fn matches_git2_hex(input: Vec<u8>) {
            let id1 = BlobId::new(&input).hex();
            let id2 = git2_hash_hex(&input);
            assert_eq!(id1, id2);
        }
    }
}
