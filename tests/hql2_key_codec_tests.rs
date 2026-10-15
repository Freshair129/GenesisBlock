#[path = "../src/query/hql2/key_codec.rs"]
mod key_codec;

use key_codec::{encode_key_v1, KeyComponentV1};
use serde_json::json;

fn decode_payloads(encoded: &[u8]) -> (u8, Vec<(u8, Vec<u8>)>) {
    assert_eq!(&encoded[..7], b"HQL2RK1");
    let count = encoded[7];
    let mut cursor = 8;
    let mut parts = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let tag = encoded[cursor];
        let length =
            u32::from_be_bytes(encoded[cursor + 1..cursor + 5].try_into().unwrap()) as usize;
        cursor += 5;
        parts.push((tag, encoded[cursor..cursor + length].to_vec()));
        cursor += length;
    }
    assert_eq!(cursor, encoded.len());
    (count, parts)
}

#[test]
fn key_codec_v1_uses_fixed_tags_typed_bytes_and_order() {
    let json = json!({"b":2,"a":1});
    let encoded = encode_key_v1(&[
        KeyComponentV1::Null,
        KeyComponentV1::Text("a"),
        KeyComponentV1::Integer(-2),
        KeyComponentV1::Real(-0.0),
        KeyComponentV1::Boolean(true),
        KeyComponentV1::Json(&json),
        KeyComponentV1::Blob(vec![0, 255]),
        KeyComponentV1::Timestamp("2026-09-28T00:00:00+07:00"),
        KeyComponentV1::EntityId("node:1"),
    ])
    .unwrap();

    assert_eq!(
        decode_payloads(&encoded),
        (
            9,
            vec![
                (0, vec![]),
                (1, b"a".to_vec()),
                (2, (-2i64).to_be_bytes().to_vec()),
                (3, 0.0f64.to_be_bytes().to_vec()),
                (4, vec![1]),
                (5, br#"{"a":1,"b":2}"#.to_vec()),
                (6, vec![0, 255]),
                (7, b"2026-09-28T00:00:00+07:00".to_vec()),
                (8, b"node:1".to_vec()),
            ]
        )
    );
}

#[test]
fn key_codec_v1_canonicalizes_zero_and_rejects_nonfinite_reals() {
    let positive = encode_key_v1(&[KeyComponentV1::Real(0.0)]).unwrap();
    let negative = encode_key_v1(&[KeyComponentV1::Real(-0.0)]).unwrap();
    assert_eq!(positive, negative);
    assert!(encode_key_v1(&[KeyComponentV1::Real(f64::NAN)]).is_err());
    assert!(encode_key_v1(&[KeyComponentV1::Real(f64::INFINITY)]).is_err());
}

#[test]
fn key_codec_v1_enforces_component_and_encoded_size_bounds() {
    assert!(encode_key_v1(&[]).is_err());
    let too_many = (0..256).map(|_| KeyComponentV1::Null).collect::<Vec<_>>();
    assert!(encode_key_v1(&too_many).is_err());

    let at_limit_text = "x".repeat(16_371);
    let at_limit = encode_key_v1(&[KeyComponentV1::Text(&at_limit_text)]).unwrap();
    assert_eq!(at_limit.len(), 16_384);
    let oversized_text = "x".repeat(16_372);
    assert!(encode_key_v1(&[KeyComponentV1::Text(&oversized_text)]).is_err());
}
