//! Storage-bound HQL2 and typed-IR aggregate parity against independent P7.
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

fn request(request_id: String, values: &[Option<i64>]) -> QueryRequestV2 {
    let parameter_type = "List<Nullable<I64>>";
    let param_values: Vec<Value> = values
        .iter()
        .map(|value| {
            value
                .map(|value| json!(value.to_string()))
                .unwrap_or(Value::Null)
        })
        .collect();
    let field = json!({"field":{"alias":"x","path":[]}});
    let aggregates = [
        ("count_all", "n_all", Vec::new()),
        ("count", "n", vec![field.clone()]),
        ("sum", "sum_x", vec![field.clone()]),
        ("avg", "avg_x", vec![field.clone()]),
        ("min", "min_x", vec![field.clone()]),
        ("max", "max_x", vec![field.clone()]),
        ("collect", "items", vec![field]),
    ]
    .into_iter()
    .map(|(call, alias, args)| json!({"expression":{"call":call,"args":args},"as":alias}))
    .collect::<Vec<_>>();
    let ir: QueryIrV2 = serde_json::from_value(json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"v","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}},
            {"id":"a","op":"Aggregate","inputs":["v"],"config":{"group_by":[],"aggregates":aggregates}}
        ],
        "root":"a",
        "parameter_types":{"xs":parameter_type}
    }))
    .unwrap();

    QueryRequestV2 {
        contract_version: "genesis.api.v2".into(),
        request_id,
        namespace: "default".into(),
        hql: None,
        ir: Some(ir),
        language_version: None,
        params: BTreeMap::from([(
            "xs".into(),
            json!({"type":parameter_type,"value":param_values}),
        )]),
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

fn bags_up_to_four() -> Vec<Vec<Option<i64>>> {
    let mut bags = vec![Vec::new()];
    for len in 1..=4 {
        for seed in 0..3usize.pow(len as u32) {
            let mut digits = seed;
            let bag = (0..len)
                .map(|_| {
                    let value = match digits % 3 {
                        0 => None,
                        1 => Some(-1),
                        _ => Some(2),
                    };
                    digits /= 3;
                    value
                })
                .collect();
            bags.push(bag);
        }
    }
    bags
}

fn from_p7(value: &reference::Value) -> V {
    match value {
        reference::Value::Null => V::Null,
        reference::Value::I64(value) => V::I64(*value),
        reference::Value::F64(value) => V::F64(*value),
        reference::Value::List(values) => V::List(values.iter().map(from_p7).collect()),
        other => panic!("unexpected P7 aggregate value: {other:?}"),
    }
}

fn expected_rows(values: &[Option<i64>]) -> Vec<BTreeMap<String, V>> {
    use reference::{Aggregate as A, Expr as E, Plan as P, Value as R};

    let input = P::Values(
        values
            .iter()
            .map(|value| BTreeMap::from([("x".into(), value.map(R::I64).unwrap_or(R::Null))]))
            .collect(),
    );
    reference::execute(&P::Aggregate {
        input: Box::new(input),
        keys: vec![],
        aggregates: vec![
            ("n_all".into(), A::CountAll),
            ("n".into(), A::Count(E::Field("x".into()))),
            ("sum_x".into(), A::Sum(E::Field("x".into()))),
            ("avg_x".into(), A::Avg(E::Field("x".into()))),
            ("min_x".into(), A::Min(E::Field("x".into()))),
            ("max_x".into(), A::Max(E::Field("x".into()))),
            ("items".into(), A::Collect(E::Field("x".into()), 4)),
        ],
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
fn storage_hql_and_ir_aggregates_match_independent_p7_for_nullable_bags() {
    let directory = TempDir::new().unwrap();
    let storage = open(directory.path());

    for (case, values) in bags_up_to_four().iter().enumerate() {
        let expected = expected_rows(values);
        let mut hql = request(format!("p7-aggregate-{case}-hql"), values);
        hql.ir = None;
        hql.hql = Some(
            "VALUES $xs AS x |> AGG count(*) AS n_all, count(x) AS n, sum(x) AS sum_x, avg(x) AS avg_x, min(x) AS min_x, max(x) AS max_x, collect(x) AS items |> RETURN n_all, n, sum_x, avg_x, min_x, max_x, items"
                .into(),
        );
        hql.language_version = Some(HqlLanguageVersionV2::HqlV2);
        let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql).unwrap() else {
            panic!("HQL aggregate should return rows for case {case}");
        };

        let ir = request(format!("p7-aggregate-{case}-ir"), values);
        let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir).unwrap() else {
            panic!("IR aggregate should return rows for case {case}");
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
