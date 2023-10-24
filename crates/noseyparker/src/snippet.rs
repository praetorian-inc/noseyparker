use bstr::BString;
use bstring_serde::BStringSerde;
use serde::{Deserialize, Serialize};
// use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use crate::bstring_escape::Escaped;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Snippet {
    /// A snippet of the input immediately prior to `content`
    #[serde(with = "BStringSerde")]
    pub before: BString,

    /// The matching input
    #[serde(with = "BStringSerde")]
    pub matching: BString,

    /// A snippet of the input immediately after `content`
    #[serde(with = "BStringSerde")]
    pub after: BString,
}

impl Display for Snippet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", Escaped(&self.before), Escaped(&self.matching), Escaped(&self.after))
    }
}
