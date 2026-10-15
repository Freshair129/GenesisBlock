#![allow(dead_code)]
#[path = "../src/query/hql2/error.rs"]
mod error;
#[path = "../src/uee_v2.rs"]
mod uee_v2;
#[path = "../src/query/hql2/value.rs"]
mod value;

use error::QueryErrorV2;
use serde_json::json;
use value::{QueryTypeV2, QueryValueV2};

#[test]
fn typed_literals_reject_coercion_overflow_and_unknown_types() {
    let ty = QueryTypeV2::parse("I64").unwrap();
    assert_eq!(
        ty.decode(&json!("-9223372036854775808")).unwrap(),
        QueryValueV2::I64(i64::MIN)
    );
    for input in [
        json!(1),
        json!("01"),
        json!("-0"),
        json!("9223372036854775808"),
    ] {
        assert_eq!(ty.decode(&input).unwrap_err().code, "BIND_ERROR");
    }
    assert_eq!(
        QueryTypeV2::parse("Decimal").unwrap_err().code,
        "CAPABILITY_UNSUPPORTED"
    );
}

#[test]
fn nullable_lists_preserve_values_and_declared_types_on_empty_input() {
    let ty = QueryTypeV2::parse("List<Nullable<I64>>").unwrap();
    assert_eq!(ty.name(), "List<Nullable<I64>>");
    assert_eq!(
        ty.decode(&json!(["2", null])).unwrap(),
        QueryValueV2::List(vec![QueryValueV2::I64(2), QueryValueV2::Null])
    );
    assert!(ty.decode(&json!(["2", true])).is_err());
    assert_eq!(ty.decode(&json!([])).unwrap(), QueryValueV2::List(vec![]));
    assert!(QueryTypeV2::parse("Bool")
        .unwrap()
        .decode(&json!(null))
        .is_err());
}

#[test]
fn entity_values_are_tagged_and_valid_record_refs_can_be_supplied_as_parameters() {
    let entity_type = QueryTypeV2::parse("Entity").unwrap();
    let record_ref = json!({
        "database_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "namespace":"default",
        "kind":"annotation",
        "id":"review:1",
        "revision":"00000000-0000-4000-8000-000000000001"
    });
    assert_eq!(
        entity_type.decode(&record_ref).unwrap(),
        QueryValueV2::Entity(crate::uee_v2::RecordRefV2 {
            database_id: "a".repeat(64),
            namespace: "default".into(),
            kind: crate::uee_v2::RecordKindV2::Annotation,
            id: "review:1".into(),
            revision: "00000000-0000-4000-8000-000000000001".into(),
        })
    );

    let error = entity_type
        .decode(&json!({
            "database_id":"db",
            "namespace":"default",
            "kind":"annotation",
            "id":"review:1",
            "revision":"00000000-0000-4000-8000-000000000001"
        }))
        .unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");

    let entity = QueryValueV2::Entity(crate::uee_v2::RecordRefV2 {
        database_id: "a".repeat(64),
        namespace: "default".into(),
        kind: crate::uee_v2::RecordKindV2::Annotation,
        id: "review:1".into(),
        revision: "00000000-0000-4000-8000-000000000001".into(),
    });
    assert_eq!(
        serde_json::to_value(entity).unwrap(),
        json!({
            "type":"Entity",
            "value":{
            "database_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "namespace":"default",
                "kind":"annotation",
                "id":"review:1",
                "revision":"00000000-0000-4000-8000-000000000001"
            }
        })
    );
}

#[test]
fn wire_values_tag_i64_without_precision_loss() {
    assert_eq!(
        serde_json::to_value(QueryValueV2::I64(i64::MAX)).unwrap(),
        json!({"type":"I64","value":"9223372036854775807"})
    );
    assert_eq!(
        serde_json::to_value(QueryValueV2::Utf8("ไทย".into())).unwrap(),
        json!({"type":"Utf8","value":"ไทย"})
    );
}

#[test]
fn errors_use_unicode_scalar_columns_and_safe_reasons() {
    let source = "ไทย\r\n\tก😃?";
    let error = QueryErrorV2::parse(source, source.find('?').unwrap(), "unexpected_token");
    let span = error.span.unwrap();
    assert_eq!((span.line, span.column), (2, 4));
    assert_eq!(error.code, "HQL_PARSE_ERROR");
    assert!(!error.message.contains("ไทย"));
}

#[test]
fn type_depth_is_bounded() {
    let ty = format!("{}I64{}", "List<".repeat(129), ">".repeat(129));
    assert!(QueryTypeV2::parse(&ty).is_err());
}
