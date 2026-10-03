use serde_json::Value;
use std::{borrow::Cow, io};

const PREFIX: &[u8; 7] = b"HQL2RK1";
const MAX_ENCODED_KEY_BYTES: usize = 16 * 1024;

pub enum KeyComponentV1<'a> {
    Null,
    Text(&'a str),
    Integer(i64),
    Real(f64),
    Boolean(bool),
    Json(&'a Value),
    Blob(Vec<u8>),
    Timestamp(&'a str),
    EntityId(&'a str),
}

struct LimitedWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl io::Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "encoded key exceeds the size limit",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn component_bytes<'a>(
    component: &'a KeyComponentV1<'a>,
    json_limit: usize,
) -> Result<(u8, Cow<'a, [u8]>), &'static str> {
    Ok(match component {
        KeyComponentV1::Null => (0, Cow::Borrowed(&[])),
        KeyComponentV1::Text(value) => (1, Cow::Borrowed(value.as_bytes())),
        KeyComponentV1::Integer(value) => (2, Cow::Owned(value.to_be_bytes().to_vec())),
        KeyComponentV1::Real(value) if value.is_finite() => {
            let canonical = if *value == 0.0 { 0.0 } else { *value };
            (3, Cow::Owned(canonical.to_be_bytes().to_vec()))
        }
        KeyComponentV1::Real(_) => return Err("key_codec_nonfinite_real"),
        KeyComponentV1::Boolean(value) => (4, Cow::Owned(vec![u8::from(*value)])),
        KeyComponentV1::Json(value) => {
            let mut writer = LimitedWriter {
                bytes: Vec::with_capacity(json_limit.min(1024)),
                limit: json_limit,
            };
            serde_json::to_writer(&mut writer, value).map_err(|_| "key_codec_value_too_large")?;
            (5, Cow::Owned(writer.bytes))
        }
        KeyComponentV1::Blob(value) => (6, Cow::Owned(value.clone())),
        KeyComponentV1::Timestamp(value) => (7, Cow::Borrowed(value.as_bytes())),
        KeyComponentV1::EntityId(value) => (8, Cow::Borrowed(value.as_bytes())),
    })
}

/// Encode an ordered typed primary-key tuple using the approved HQL2 KeyCodec v1.
pub fn encode_key_v1(components: &[KeyComponentV1<'_>]) -> Result<Vec<u8>, &'static str> {
    if components.is_empty() || components.len() > u8::MAX as usize {
        return Err("key_codec_component_count");
    }

    let mut encoded = Vec::with_capacity(8);
    encoded.extend_from_slice(PREFIX);
    encoded.push(components.len() as u8);

    for component in components {
        let remaining = MAX_ENCODED_KEY_BYTES.saturating_sub(encoded.len());
        if remaining < 5 {
            return Err("key_codec_value_too_large");
        }
        let (tag, bytes) = component_bytes(component, remaining - 5)?;
        let length = u32::try_from(bytes.len()).map_err(|_| "key_codec_value_too_large")?;
        let next_len = encoded
            .len()
            .checked_add(5)
            .and_then(|len| len.checked_add(bytes.len()))
            .ok_or("key_codec_value_too_large")?;
        if next_len > MAX_ENCODED_KEY_BYTES {
            return Err("key_codec_value_too_large");
        }
        encoded.push(tag);
        encoded.extend_from_slice(&length.to_be_bytes());
        encoded.extend_from_slice(&bytes);
    }

    Ok(encoded)
}
