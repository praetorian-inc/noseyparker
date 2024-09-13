use anyhow::Result;
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------------------------------------
// BlobId
// -------------------------------------------------------------------------------------------------
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone, Serialize)]
#[serde(into = "String")]
pub struct BlobId([u8; 20]);

impl<'de> Deserialize<'de> for BlobId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Vis;
        impl serde::de::Visitor<'_> for Vis {
            type Value = BlobId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                BlobId::from_hex(v).map_err(|e| serde::de::Error::custom(e))
            }
        }
        d.deserialize_str(Vis)
    }
}

impl std::fmt::Debug for BlobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlobId({})", self.hex())
    }
}

impl schemars::JsonSchema for BlobId {
    fn schema_name() -> String {
        "BlobId".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let s = String::json_schema(gen);
        let mut o = s.into_object();
        o.string().pattern = Some("[0-9a-f]{40}".into());
        let md = o.metadata();
        md.description = Some("A hex-encoded blob ID as computed by Git".into());
        schemars::schema::Schema::Object(o)
    }
}

impl BlobId {
    /// Create a new `BlobId` computed from the given input.
    #[inline]
    pub fn compute_from_bytes(input: &[u8]) -> Self {
        use noseyparker_digest::Sha1;
        use std::io::Write;

        let mut h = Sha1::default();
        write!(&mut h, "blob {}\0", input.len()).unwrap();
        h.update(input);
        BlobId(h.digest())
    }

    /// Create new new `BlobId` from a hexadecimal string.
    #[inline]
    pub fn from_hex(v: &str) -> Result<Self> {
        Ok(BlobId(hex::decode(v)?.as_slice().try_into()?))
    }

    /// Render the `BlobId` as a hexadecimal string.
    #[inline]
    pub fn hex(&self) -> String {
        hex::encode(self.0)
    }

    /// View the `BlobId` as bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<BlobId> for String {
    #[inline]
    fn from(blob_id: BlobId) -> String {
        blob_id.hex()
    }
}

impl TryFrom<&str> for BlobId {
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
        gix::hash::ObjectId::try_from(blob_id.as_bytes()).unwrap()
    }
}

impl From<BlobId> for gix::ObjectId {
    #[inline]
    fn from(blob_id: BlobId) -> Self {
        gix::hash::ObjectId::try_from(blob_id.as_bytes()).unwrap()
    }
}

// -------------------------------------------------------------------------------------------------
// sql
// -------------------------------------------------------------------------------------------------
mod sql {
    use super::*;

    use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

    impl ToSql for BlobId {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            Ok(self.hex().into())
        }
    }

    impl FromSql for BlobId {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            Self::from_hex(value.as_str()?).map_err(|e| FromSqlError::Other(e.into()))
        }
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
        assert_eq!(
            BlobId::compute_from_bytes(&vec![0; 0]).hex(),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
        assert_eq!(
            BlobId::compute_from_bytes(&vec![0; 1024]).hex(),
            "06d7405020018ddf3cacee90fd4af10487da3d20"
        );
    }
}
