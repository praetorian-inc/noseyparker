use bstr::BString;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A custom `serde` codec for `bstr::BString` that uses lossy UTF-8 encoding.
#[derive(Deserialize, Serialize)]
#[serde(remote = "BString")]
pub struct BStringLossyUtf8(
    #[serde(
        getter = "bstring_as_vec",
        serialize_with = "serialize_bytes_string_lossy",
        deserialize_with = "deserialize_bytes_string"
    )]
    pub Vec<u8>,
);

#[inline]
fn bstring_as_vec(b: &BString) -> &Vec<u8> {
    b
}

impl From<BStringLossyUtf8> for BString {
    fn from(b: BStringLossyUtf8) -> BString {
        BString::new(b.0)
    }
}

fn serialize_bytes_string_lossy<S: serde::Serializer>(
    bytes: &[u8],
    s: S,
) -> Result<S::Ok, S::Error> {
    s.serialize_str(&String::from_utf8_lossy(bytes))
}

fn deserialize_bytes_string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    struct Vis;
    impl serde::de::Visitor<'_> for Vis {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(v.into())
        }
    }
    d.deserialize_str(Vis)
}

/// Use plain `string` as the JSON schema for `BStringLossyUtf8`.
impl JsonSchema for BStringLossyUtf8 {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        <String as JsonSchema>::schema_id()
    }

    fn schema_name() -> String {
        <String as JsonSchema>::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(gen)
    }
}

/// A custom `serde` codec for `bstr::BString` that uses standard base64.
#[derive(Deserialize, Serialize)]
#[serde(remote = "BString")]
pub struct BStringBase64(
    #[serde(
        getter = "bstring_as_vec",
        serialize_with = "serialize_bytes_string_base64",
        deserialize_with = "deserialize_bytes_string_base64"
    )]
    pub Vec<u8>,
);

impl From<BStringBase64> for BString {
    fn from(b: BStringBase64) -> BString {
        BString::new(b.0)
    }
}

fn serialize_bytes_string_base64<S: serde::Serializer>(
    bytes: &[u8],
    s: S,
) -> Result<S::Ok, S::Error> {
    use base64::prelude::*;
    s.collect_str(&base64::display::Base64Display::new(bytes, &BASE64_STANDARD))
}

fn deserialize_bytes_string_base64<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<u8>, D::Error> {
    struct Vis;
    impl serde::de::Visitor<'_> for Vis {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a base64 string")
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            use base64::prelude::*;
            BASE64_STANDARD.decode(v).map_err(serde::de::Error::custom)
        }
    }
    d.deserialize_str(Vis)
}

impl JsonSchema for BStringBase64 {
    fn schema_name() -> String {
        "BStringBase64".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let s = String::json_schema(gen);
        let mut o = s.into_object();
        o.string().pattern = Some("[a-zA-Z0-9/+]*={0,2}".into());
        let md = o.metadata();
        md.description = Some("A standard base64-encoded bytestring".into());
        schemars::schema::Schema::Object(o)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_roundtrip_base64_json_1(input: Vec<u8>) {
            #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
            struct Test(#[serde(with="BStringBase64")] BString);

            let v0: Test = Test(input.into());
            let v1: String = serde_json::to_string(&v0).expect("should be able to serialize");
            let v2: Test = serde_json::from_str(&v1).expect("should be able to deserialize");
            prop_assert_eq!(v0, v2);
        }

        #[test]
        fn test_roundtrip_lossyutf8_json_1(input: String) {
            #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
            struct Test(#[serde(with="BStringLossyUtf8")] BString);

            let v0: Test = Test(input.into());
            let v1: String = serde_json::to_string(&v0).expect("should be able to serialize");
            let v2: Test = serde_json::from_str(&v1).expect("should be able to deserialize");
            prop_assert_eq!(v0, v2);
        }

        #[test]
        fn test_roundtrip_lossyutf8_json_2(input: Vec<u8>) {
            #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
            struct Test(#[serde(with="BStringLossyUtf8")] BString);

            // Here we need to do the serialize -> deserialize cycle twice, since the original
            // input may contain bytes that are lossily encoded. A simple round-trip test isn't
            // actually correct with this codec.
            let v0: Test = Test(input.into());
            let v1: String = serde_json::to_string(&v0).expect("should be able to serialize");
            let v2: Test = serde_json::from_str(&v1).expect("should be able to deserialize");

            let v3: String = serde_json::to_string(&v2).expect("should be able to deserialize");
            let v4: Test = serde_json::from_str(&v3).expect("should be able to deserialize");
            prop_assert_eq!(v1, v3);
            prop_assert_eq!(v2, v4);
        }
    }
}
