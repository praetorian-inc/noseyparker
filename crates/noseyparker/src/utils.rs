use bstr::BString;
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------------------------------------
// BStringSerde
// -------------------------------------------------------------------------------------------------
/// Used to explicitly specify a custom `serde` codec for `bstr::BString`
#[derive(Deserialize, Serialize)]
#[serde(remote="BString")]
pub struct BStringSerde (
    #[serde(
        getter = "BStringSerde::get_bstring",
        serialize_with = "serialize_bytes_string_lossy",
        deserialize_with = "deserialize_bytes_string",
    )]
    pub Vec<u8>,
);

impl BStringSerde {
    /// This function only exists to customize the `BStringSerde` serialization.
    /// Maybe that can be re-spelled to avoid having to write this at all.
    #[inline]
    #[allow(dead_code)]
    fn get_bstring(b: &BString) -> &Vec<u8> {
        b
    }
}

impl From<BStringSerde> for BString {
    fn from(b: BStringSerde) -> BString {
        BString::new(b.0)
    }
}

#[inline]
pub fn serialize_bytes_string_lossy<S: serde::Serializer>(
    bytes: &[u8],
    s: S,
) -> Result<S::Ok, S::Error> {
    s.serialize_str(&String::from_utf8_lossy(bytes))
}

#[inline]
pub fn deserialize_bytes_string<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<u8>, D::Error> {
    let s: &str = serde::Deserialize::deserialize(d)?;
    Ok(s.as_bytes().to_vec())
}
