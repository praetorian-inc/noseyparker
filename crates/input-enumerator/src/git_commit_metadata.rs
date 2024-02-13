use bstr::BString;
use gix::date::Time;
use gix::ObjectId;

use bstring_serde::BStringLossyUtf8;

/*
// FIXME: figure out how to do this without allocating
fn serialize_object_id<S: serde::Serializer>(object_id: &ObjectId, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&object_id.to_hex().to_string())
}

*/

mod text_time {
    use super::*;

    pub fn serialize<S: serde::Serializer>(time: &Time, serializer: S) -> Result<S::Ok, S::Error> {
        // XXX any way to do this without allocating?
        serializer.serialize_str(&time.to_bstring().to_string())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Time, D::Error> {
        struct Vis;
        impl serde::de::Visitor<'_> for Vis {
            type Value = Time;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                gix::date::parse(v, None).map_err(|e| serde::de::Error::custom(e))
            }
        }
        d.deserialize_str(Vis)
    }
}

mod hex_object_id {
    use super::*;

    pub fn serialize<S: serde::Serializer>(v: &ObjectId, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&v.to_hex())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<ObjectId, D::Error> {
        struct Vis;
        impl serde::de::Visitor<'_> for Vis {
            type Value = ObjectId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                ObjectId::from_hex(v.as_bytes()).map_err(|e| serde::de::Error::custom(e))
            }
        }
        d.deserialize_str(Vis)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct CommitMetadata {
    #[serde(with="hex_object_id")]
    pub commit_id: ObjectId,

    #[serde(with = "BStringLossyUtf8")]
    pub committer_name: BString,

    #[serde(with = "BStringLossyUtf8")]
    pub committer_email: BString,

    #[serde(with = "text_time")]
    pub committer_timestamp: Time,

    #[serde(with = "BStringLossyUtf8")]
    pub author_name: BString,

    #[serde(with = "BStringLossyUtf8")]
    pub author_email: BString,

    #[serde(with = "text_time")]
    pub author_timestamp: Time,

    #[serde(with = "BStringLossyUtf8")]
    pub message: BString,
}
