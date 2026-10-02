//! Storage-bound HQL2 and typed-IR scalar parity against the independent P7 interpreter.
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

fn request(request_id: String, values: Vec<Value>) -> QueryRequestV2 {
    let parameter_type = "List<Nullable<I64>>";
    let ir: QueryIrV2 = serde_json::from_value(json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"v","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}},
            {"id":"d","op":"Distinct","inputs":["v"],"config":{}},
            {"id":"s","op":"Sort","inputs":["d"],"config":{"keys":[{
                "expression":{"field":{"alias":"x","path":[]}},
                "direction":"asc","nulls":"last"
            }]}}
        ],
        "root":"s",
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
        params: BTreeMap::from([("xs".into(), json!({"type":parameter_type,"value":values}))]),
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

fn expected_rows(values: &[Option<i64>]) -> Vec<BTreeMap<String, V>> {
    use reference::{Expr, Plan, SortKey, Value as R};

    let input = Plan::Values(
        values
            .iter()
            .map(|value| BTreeMap::from([("x".into(), value.map(R::I64).unwrap_or(R::Null))]))
            .collect(),
    );
    reference::execute(&Plan::Sort(
        Box::new(Plan::Distinct(Box::new(input))),
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
                R::Null => V::Null,
                R::I64(value) => V::I64(*value),
                other => panic!("unexpected P7 value: {other:?}"),
            },
        )])
    })
    .collect()
}

#[test]
fn storage_hql_and_ir_match_independent_p7_for_81_nullable_bags() {
    let directory = TempDir::new().unwrap();
    let storage = open(directory.path());

    for seed in 0..81 {
        let mut digits = seed;
        let values: Vec<Option<i64>> = (0..4)
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
        let expected = expected_rows(&values);
        let params: Vec<Value> = values
            .iter()
            .map(|value| {
                value
                    .map(|value| json!(value.to_string()))
                    .unwrap_or(Value::Null)
            })
            .collect();
        let request_id = format!("p7-scalar-{seed}");

        let mut hql = request(format!("{request_id}-hql"), params.clone());
        hql.ir = None;
        hql.hql =
            Some("VALUES $xs AS x |> DISTINCT |> ORDER BY x ASC NULLS LAST |> RETURN x".into());
        hql.language_version = Some(HqlLanguageVersionV2::HqlV2);
        let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql).unwrap() else {
            panic!("HQL query should return rows for seed {seed}");
        };

        let ir = request(format!("{request_id}-ir"), params);
        let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir).unwrap() else {
            panic!("IR query should return rows for seed {seed}");
        };

        assert_eq!(
            hql_result.rows, expected,
            "HQL differs from P7 at seed {seed}"
        );
        assert_eq!(
            ir_result.rows, expected,
            "IR differs from P7 at seed {seed}"
        );
        assert_eq!(
            hql_result.rows, ir_result.rows,
            "HQL/IR differ at seed {seed}"
        );
    }
}
