use bstr::BString;
use gix::ObjectId;
use gix::date::Time;

use bstring_serde::BStringSerde;


// FIXME: figure out how to do this without allocating
fn serialize_object_id<S: serde::Serializer>(object_id: &ObjectId, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&object_id.to_hex().to_string())
}

// FIXME: figure out how to do this without allocating
fn serialize_time<S: serde::Serializer>(time: &Time, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&time.to_bstring().to_string())
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq, Hash)]
pub struct CommitMetadata {
    #[serde(serialize_with="serialize_object_id")]
    pub commit_id: ObjectId,

    #[serde(with="BStringSerde")]
    pub committer_name: BString,

    #[serde(with="BStringSerde")]
    pub committer_email: BString,

    #[serde(serialize_with="serialize_time")]
    pub committer_timestamp: Time,

    #[serde(with="BStringSerde")]
    pub author_name: BString,

    #[serde(with="BStringSerde")]
    pub author_email: BString,

    #[serde(serialize_with="serialize_time")]
    pub author_timestamp: Time,

    #[serde(with="BStringSerde")]
    pub message: BString,
}
