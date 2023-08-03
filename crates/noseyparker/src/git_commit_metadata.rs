use bstr::BString;
use gix::ObjectId;
use gix::date::Time;

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq, Hash)]
pub struct CommitMetadata {
    pub commit_id: ObjectId,

    pub committer_name: BString,
    pub committer_email: BString,
    pub committer_timestamp: Time,

    pub author_name: BString,
    pub author_email: BString,
    pub author_timestamp: Time,

    pub message: BString,
}
