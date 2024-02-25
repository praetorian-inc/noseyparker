use bstr::BString;
use bstring_serde::BStringLossyUtf8;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
// use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use crate::bstring_escape::Escaped;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct Snippet {
    /// A snippet of the input immediately prior to `content`
    #[serde(with = "BStringLossyUtf8")]
    pub before: BString,

    /// The matching input
    #[serde(with = "BStringLossyUtf8")]
    pub matching: BString,

    /// A snippet of the input immediately after `content`
    #[serde(with = "BStringLossyUtf8")]
    pub after: BString,
}

impl Display for Snippet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            Escaped(&self.before),
            Escaped(&self.matching),
            Escaped(&self.after)
        )
    }
}
