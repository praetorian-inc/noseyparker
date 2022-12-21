use bstr::BString;
use serde::{Deserialize, Serialize};

const SIZEOF_PREFIXES: [(usize, &str); 5] = [
    (1024 * 1024 * 1024 * 1024 * 1024, "PiB"),
    (1024 * 1024 * 1024 * 1024, "TiB"),
    (1024 * 1024 * 1024, "GiB"),
    (1024 * 1024, "MiB"),
    (1024, "KiB"),
];

pub fn sizeof_fmt(bytes: usize) -> String {
    let (d, unit) = SIZEOF_PREFIXES
        .iter()
        .find(|(v, _t)| bytes >= *v)
        .unwrap_or(&(1, "B"));
    let v = bytes as f64 / *d as f64;
    format!("{:.2} {}", v, unit)
}

const DURATION_PREFIXES: [(f64, &str); 4] = [
    ((60 * 60 * 24 * 7) as f64, "weeks"),
    ((60 * 60 * 24) as f64, "days"),
    ((60 * 60) as f64, "hours"),
    (60_f64, "minutes"),
];

pub fn duration_fmt(secs: f64) -> String {
    let (d, unit) = DURATION_PREFIXES
        .iter()
        .find(|(v, _t)| secs >= *v)
        .unwrap_or(&(1.0, "seconds"));
    let v = secs / *d;
    format!("{:.2} {}", v, unit)
}


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
