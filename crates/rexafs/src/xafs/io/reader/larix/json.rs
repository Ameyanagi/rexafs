//! Inert JSON values with duplicate-key validation and Python nonfinite tokens.
use super::super::ReadError;
use super::error;
use serde_json::Value;

/// Python permits these nonfinite JSON tokens. Quote only whole tokens outside
/// strings for inspection; the exact original text is retained in the document.
pub(super) fn parse(source: &str, context: &str) -> Result<Value, ReadError> {
    let bytes = source.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let (mut quoted, mut escaped, mut i) = (false, false, 0);
    while i < bytes.len() {
        let b = bytes[i];
        if quoted {
            out.push(b);
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
            i += 1;
        } else if b == b'"' {
            quoted = true;
            out.push(b);
            i += 1;
        } else {
            let boundary = |c: u8| c.is_ascii_whitespace() || b",:[]{}".contains(&c);
            let token = ["-Infinity", "Infinity", "NaN"].into_iter().find(|s| {
                bytes[i..].starts_with(s.as_bytes())
                    && (i == 0 || boundary(bytes[i - 1]))
                    && (i + s.len() == bytes.len() || boundary(bytes[i + s.len()]))
            });
            if let Some(token) = token {
                out.push(b'"');
                out.extend(token.as_bytes());
                out.push(b'"');
                i += token.len();
            } else {
                out.push(b);
                i += 1;
            }
        }
    }
    serde_json::from_slice::<UniqueValue>(&out)
        .map(|value| value.0)
        .map_err(|e| error(context, format!("invalid JSON: {e}")))
}

// Reject duplicate object keys at every level instead of silently replacing
// an earlier detector array, unit declaration or shape. Journal entries are
// lists and may legitimately repeat their own key field across entries.
struct UniqueValue(Value);

impl<'de> serde::Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON with unique object keys")
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueValue(value)) = seq.next_element()? {
                    values.push(value);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, UniqueValue(value))) =
                    map.next_entry::<String, UniqueValue>()?
                {
                    if values.insert(key.clone(), value).is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate object key {key:?}"
                        )));
                    }
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
