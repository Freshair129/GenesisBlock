//! Storage-bound HQL2 and typed-IR set semantics against independent P7.
#[path = "support/hql2_reference.rs"]
mod reference;

use genesis_block_native::{
    query::hql2::{value::QueryValueV2 as V, QueryOutcomeV2},
    uee_v2::{HqlLanguageVersionV2, QueryIrV2, QueryRequestV2},
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

fn parameter(values: &[Option<i64>]) -> Value {
    let values: Vec<Value> = values
        .iter()
        .map(|value| {
            value
                .map(|value| json!(value.to_string()))
                .unwrap_or(Value::Null)
        })
        .collect();
    json!({"type":"List<Nullable<I64>>","value":values})
}

fn request(request_id: String, left: &[Option<i64>], right: &[Option<i64>]) -> QueryRequestV2 {
    let ir: QueryIrV2 = serde_json::from_value(json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"l","op":"Values","inputs":[],"config":{"param":"left","as":"x"}},
            {"id":"r","op":"Values","inputs":[],"config":{"param":"right","as":"x"}},
            {"id":"u","op":"UnionAll","inputs":["l","r"],"config":{}},
            {"id":"s","op":"Sort","inputs":["u"],"config":{"keys":[{
                "expression":{"field":{"alias":"x","path":[]}},
                "direction":"asc","nulls":"last"
            }]}}
        ],
        "root":"s",
        "parameter_types":{
            "left":"List<Nullable<I64>>",
            "right":"List<Nullable<I64>>"
        }
    }))
    .unwrap();

    QueryRequestV2 {
        contract_version: "genesis.api.v2".into(),
        request_id,
        namespace: "default".into(),
        hql: None,
        ir: Some(ir),
        language_version: None,
        params: BTreeMap::from([
            ("left".into(), parameter(left)),
            ("right".into(), parameter(right)),
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

fn bags_up_to_two() -> Vec<Vec<Option<i64>>> {
    let alphabet = [None, Some(-1), Some(2)];
    let mut bags = vec![Vec::new()];
    for len in 1..=2 {
        for seed in 0..3usize.pow(len as u32) {
            let mut digits = seed;
            let bag = (0..len)
                .map(|_| {
                    let value = alphabet[digits % alphabet.len()];
                    digits /= alphabet.len();
                    value
                })
                .collect();
            bags.push(bag);
        }
    }
    bags
}

fn p7_rows(values: &[Option<i64>]) -> Vec<BTreeMap<String, reference::Value>> {
    values
        .iter()
        .map(|value| {
            BTreeMap::from([(
                "x".into(),
                value
                    .map(reference::Value::I64)
                    .unwrap_or(reference::Value::Null),
            )])
        })
        .collect()
}

fn expected_rows(left: &[Option<i64>], right: &[Option<i64>]) -> Vec<BTreeMap<String, V>> {
    use reference::{Expr, Plan, SortKey};

    reference::execute(&Plan::Sort(
        Box::new(Plan::UnionAll(
            Box::new(Plan::Values(p7_rows(left))),
            Box::new(Plan::Values(p7_rows(right))),
        )),
        vec![SortKey {
            expr: Expr::Field("x".into()),
            descending: false,
            nulls_first: false,
        }],
    ))
    .unwrap()
    .iter()
    .map(|row| {
        BTreeMap::from([(
            "x".into(),
            match row.get("x").unwrap() {
                reference::Value::Null => V::Null,
                reference::Value::I64(value) => V::I64(*value),
                other => panic!("unexpected P7 set value: {other:?}"),
            },
        )])
    })
    .collect()
}

#[test]
fn storage_hql_and_ir_union_all_match_p7_for_nullable_bag_pairs() {
    let directory = TempDir::new().unwrap();
    let storage = open(directory.path());
    let bags = bags_up_to_two();

    for (case, (left, right)) in bags
        .iter()
        .flat_map(|left| bags.iter().map(move |right| (left, right)))
        .enumerate()
    {
        let expected = expected_rows(left, right);
        let mut hql = request(format!("p7-union-{case}-hql"), left, right);
        hql.ir = None;
        hql.hql = Some(
            "UNION ALL { VALUES $left AS x |> RETURN x }{ VALUES $right AS x |> RETURN x } |> ORDER BY x ASC NULLS LAST |> RETURN x".into(),
        );
        hql.language_version = Some(HqlLanguageVersionV2::HqlV2);
        let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql).unwrap() else {
            panic!("HQL UNION ALL should return rows for case {case}");
        };

        let ir = request(format!("p7-union-{case}-ir"), left, right);
        let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir).unwrap() else {
            panic!("typed-IR UnionAll should return rows for case {case}");
        };

        assert_eq!(
            hql_result.rows, expected,
            "HQL differs from P7 at case {case}"
        );
        assert_eq!(
            ir_result.rows, expected,
            "IR differs from P7 at case {case}"
        );
        assert_eq!(
            hql_result.rows, ir_result.rows,
            "HQL/IR differ at case {case}"
        );
    }
}
