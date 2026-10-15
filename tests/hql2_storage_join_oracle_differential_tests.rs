//! Storage-bound typed-IR joins against the independent P7 bag interpreter.
#[path = "support/hql2_reference.rs"]
mod reference;

use genesis_block_native::{
    query::hql2::{value::QueryValueV2 as V, QueryOutcomeV2},
    uee_v2::{QueryIrV2, QueryRequestV2},
    *,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
use tempfile::TempDir;

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn access() -> AccessContext {
    AccessContext {
        principal: "reader".into(),
        namespace: "default".into(),
    }
}

fn values(values: &[Option<i64>]) -> Value {
    Value::Array(
        values
            .iter()
            .map(|value| {
                value
                    .map(|value| json!(value.to_string()))
                    .unwrap_or(Value::Null)
            })
            .collect(),
    )
}

fn request(
    request_id: &str,
    kind: &str,
    left: &[Option<i64>],
    right: &[Option<i64>],
) -> QueryRequestV2 {
    let parameter_type = "List<Nullable<I64>>";
    let field = |alias| json!({"field":{"alias":alias,"path":[]}});
    let ir: QueryIrV2 = serde_json::from_value(json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"left","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}},
            {"id":"right","op":"Values","inputs":[],"config":{"param":"ys","as":"y"}},
            {"id":"join","op":"Join","inputs":["left","right"],"config":{
                "kind":kind,
                "condition":{"binary":"eq","left":field("x"),"right":field("y")}
            }}
        ],
        "root":"join",
        "parameter_types":{"xs":parameter_type,"ys":parameter_type}
    }))
    .unwrap();
    QueryRequestV2 {
        contract_version: "genesis.api.v2".into(),
        request_id: request_id.into(),
        namespace: "default".into(),
        hql: None,
        ir: Some(ir),
        language_version: None,
        params: BTreeMap::from([
            (
                "xs".into(),
                json!({"type":parameter_type,"value":values(left)}),
            ),
            (
                "ys".into(),
                json!({"type":parameter_type,"value":values(right)}),
            ),
        ]),
        temporal: None,
        index_policy: None,
        required_frontier: None,
        transaction_id: None,
        budget: None,
        allow_partial: None,
        explain: None,
        format: None,
    }
}

fn from_p7(value: &reference::Value) -> V {
    match value {
        reference::Value::Null => V::Null,
        reference::Value::I64(value) => V::I64(*value),
        other => panic!("unexpected P7 join value: {other:?}"),
    }
}

fn expected_rows(
    left: &[Option<i64>],
    right: &[Option<i64>],
    kind: reference::JoinKind,
) -> Vec<BTreeMap<String, V>> {
    use reference::{Expr as E, Plan as P, Value as R};

    let values = |alias: &str, rows: &[Option<i64>]| {
        P::Values(
            rows.iter()
                .map(|value| BTreeMap::from([(alias.into(), value.map(R::I64).unwrap_or(R::Null))]))
                .collect(),
        )
    };
    reference::execute(&P::Join {
        left: Box::new(values("x", left)),
        right: Box::new(values("y", right)),
        on: E::Eq(
            Box::new(E::Field("x".into())),
            Box::new(E::Field("y".into())),
        ),
        kind,
        right_fields: vec!["y".into()],
    })
    .unwrap()
    .iter()
    .map(|row| {
        row.iter()
            .map(|(key, value)| (key.clone(), from_p7(value)))
            .collect()
    })
    .collect()
}

#[test]
fn storage_typed_ir_join_kinds_match_independent_p7_for_duplicate_and_null_keys() {
    let directory = TempDir::new().unwrap();
    let storage = open(directory.path());
    let left = [Some(1), Some(1), None, Some(2)];
    let right = [Some(1), Some(1), None, Some(99)];

    for (kind, oracle_kind, expected_count) in [
        ("inner", reference::JoinKind::Inner, 4),
        ("left", reference::JoinKind::Left, 6),
        ("semi", reference::JoinKind::Semi, 2),
        ("anti", reference::JoinKind::Anti, 2),
    ] {
        let expected = expected_rows(&left, &right, oracle_kind);
        let QueryOutcomeV2::Rows(actual) = storage
            .query_v2(
                access(),
                request(&format!("p7-join-{kind}"), kind, &left, &right),
            )
            .unwrap()
        else {
            panic!("typed-IR join should return rows for {kind}");
        };

        assert_eq!(actual.rows, expected, "{kind} differs from P7");
        assert_eq!(actual.rows.len(), expected_count, "{kind} row count");
    }
}
