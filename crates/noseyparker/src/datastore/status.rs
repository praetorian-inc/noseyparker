use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

// -------------------------------------------------------------------------------------------------
// Status
// -------------------------------------------------------------------------------------------------

/// A status assigned to a match group
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
// FIXME(overhaul): use an integer representation for serialization and db
pub enum Status {
    Accept,
    Reject,
}

// -------------------------------------------------------------------------------------------------
// Statuses
// -------------------------------------------------------------------------------------------------
/// A collection of statuses
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
// FIXME(overhaul): use a bitflag representation here?
pub struct Statuses(pub SmallVec<[Status; 16]>);

// -------------------------------------------------------------------------------------------------
// sql
// -------------------------------------------------------------------------------------------------
mod sql {
    use super::*;

    use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
    use rusqlite::Error::ToSqlConversionFailure;

    impl ToSql for Status {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            match self {
                Status::Accept => Ok("accept".into()),
                Status::Reject => Ok("reject".into()),
            }
        }
    }

    impl FromSql for Status {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            match value.as_str()? {
                "accept" => Ok(Status::Accept),
                "reject" => Ok(Status::Reject),
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }

    impl ToSql for Statuses {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            match serde_json::to_string(self) {
                Err(e) => Err(ToSqlConversionFailure(e.into())),
                Ok(s) => Ok(s.into()),
            }
        }
    }

    impl FromSql for Statuses {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            match value {
                ValueRef::Text(s) => {
                    serde_json::from_slice(s).map_err(|e| FromSqlError::Other(e.into()))
                }
                ValueRef::Blob(b) => {
                    serde_json::from_slice(b).map_err(|e| FromSqlError::Other(e.into()))
                }
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }
}
