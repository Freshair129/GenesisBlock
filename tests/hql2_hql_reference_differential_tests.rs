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

fn storage_request(request_id: &str, values: &[Option<i64>]) -> QueryRequestV2 {
    let values = values
        .iter()
        .map(|value| {
            value
                .map(|value| json!(value.to_string()))
                .unwrap_or(Value::Null)
        })
        .collect();
    request(request_id.to_owned(), values)
}

fn scalar_ir(nodes: Vec<Value>, root: &str) -> QueryIrV2 {
    serde_json::from_value(json!({
        "contract_version":"query-ir.v2",
        "nodes":nodes,
        "root":root,
        "parameter_types":{"xs":"List<Nullable<I64>>"}
    }))
    .unwrap()
}

fn p7_values(values: &[Option<i64>]) -> reference::Plan {
    reference::Plan::Values(
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
            .collect(),
    )
}

fn runtime_rows(rows: &[reference::Row]) -> Vec<BTreeMap<String, V>> {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|(alias, value)| {
                    let value = match value {
                        reference::Value::Null => V::Null,
                        reference::Value::I64(value) => V::I64(*value),
                        other => panic!("unexpected P7 scalar value: {other:?}"),
                    };
                    (alias.clone(), value)
                })
                .collect()
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

#[test]
fn storage_hql_and_ir_scalar_pipelines_match_p7() {
    use reference::{Expr as E, Plan as P, SortKey, Value as R};

    let directory = TempDir::new().unwrap();
    let storage = open(directory.path());
    let projection_cases: [&[Option<i64>]; 3] = [
        &[],
        &[None, Some(-1), Some(2), None, Some(-1)],
        &[Some(2), None, Some(-1)],
    ];

    for (case, values) in projection_cases.into_iter().enumerate() {
        let projected = P::Project(
            Box::new(p7_values(values)),
            vec![
                ("x".into(), E::Field("x".into())),
                (
                    "adjusted".into(),
                    E::Add(
                        Box::new(E::Field("x".into())),
                        Box::new(E::Literal(R::I64(1))),
                    ),
                ),
            ],
        );
        let sorted = P::Sort(
            Box::new(projected),
            vec![SortKey {
                expr: E::Field("adjusted".into()),
                descending: false,
                nulls_first: false,
            }],
        );
        let expected = runtime_rows(
            &reference::execute(&P::Take(Box::new(P::Offset(Box::new(sorted), 1)), 3)).unwrap(),
        );
        let pipeline = vec![
            json!({"id":"v","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}}),
            json!({"id":"p","op":"Project","inputs":["v"],"config":{"fields":[
                {"as":"x","expression":{"field":{"alias":"x","path":[]}}},
                {"as":"adjusted","expression":{"binary":"add","left":{"field":{"alias":"x","path":[]}},"right":{"type":"I64","literal":"1"}}}
            ]}}),
            json!({"id":"s","op":"Sort","inputs":["p"],"config":{"keys":[{
                "expression":{"field":{"alias":"adjusted","path":[]}},"direction":"asc","nulls":"last"
            }]}}),
            json!({"id":"o","op":"Offset","inputs":["s"],"config":{"count":1}}),
            json!({"id":"t","op":"Take","inputs":["o"],"config":{"count":3}}),
        ];
        let mut hql = storage_request(&format!("p7-scalar-pipeline-{case}-hql"), values);
        hql.ir = None;
        hql.hql = Some("VALUES $xs AS x |> PROJECT x AS x, x + 1 AS adjusted |> ORDER BY adjusted ASC NULLS LAST |> SKIP 1 |> TAKE 3 |> RETURN x, adjusted".into());
        hql.language_version = Some(HqlLanguageVersionV2::HqlV2);
        let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql).unwrap() else {
            panic!("HQL scalar pipeline should return rows for case {case}");
        };

        let mut ir = storage_request(&format!("p7-scalar-pipeline-{case}-ir"), values);
        ir.ir = Some(scalar_ir(pipeline, "t"));
        let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir).unwrap() else {
            panic!("typed-IR scalar pipeline should return rows for case {case}");
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

    let filter_cases: [&[Option<i64>]; 3] = [
        &[],
        &[
            None,
            Some(-2),
            Some(-1),
            Some(1),
            Some(2),
            Some(-1),
            Some(2),
            Some(90),
        ],
        &[Some(2), Some(-2), Some(-1)],
    ];
    let equality = |value| {
        E::Eq(
            Box::new(E::Field("x".into())),
            Box::new(E::Literal(R::I64(value))),
        )
    };
    let predicate = E::Or(
        Box::new(E::Or(
            Box::new(E::Or(Box::new(equality(-2)), Box::new(equality(-1)))),
            Box::new(equality(1)),
        )),
        Box::new(equality(2)),
    );
    let json_equality = |value: i64| json!({"binary":"eq","left":{"field":{"alias":"x","path":[]}},"right":{"type":"I64","literal":value.to_string()}});
    let json_predicate = json!({"binary":"or","left":{"binary":"or","left":{"binary":"or","left":json_equality(-2),"right":json_equality(-1)},"right":json_equality(1)},"right":json_equality(2)});

    for (case, values) in filter_cases.into_iter().enumerate() {
        let filtered = P::Filter(Box::new(p7_values(values)), predicate.clone());
        let projected = P::Project(
            Box::new(filtered),
            vec![
                ("x".into(), E::Field("x".into())),
                (
                    "adjusted".into(),
                    E::Add(
                        Box::new(E::Field("x".into())),
                        Box::new(E::Literal(R::I64(10))),
                    ),
                ),
            ],
        );
        let sorted = P::Sort(
            Box::new(P::Distinct(Box::new(projected))),
            vec![SortKey {
                expr: E::Field("adjusted".into()),
                descending: true,
                nulls_first: false,
            }],
        );
        let expected = runtime_rows(&reference::execute(&P::Take(Box::new(sorted), 2)).unwrap());
        let pipeline = vec![
            json!({"id":"v","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}}),
            json!({"id":"f","op":"Filter","inputs":["v"],"config":{"predicate":json_predicate}}),
            json!({"id":"p","op":"Project","inputs":["f"],"config":{"fields":[
                {"as":"x","expression":{"field":{"alias":"x","path":[]}}},
                {"as":"adjusted","expression":{"binary":"add","left":{"field":{"alias":"x","path":[]}},"right":{"type":"I64","literal":"10"}}}
            ]}}),
            json!({"id":"d","op":"Distinct","inputs":["p"],"config":{}}),
            json!({"id":"s","op":"Sort","inputs":["d"],"config":{"keys":[{
                "expression":{"field":{"alias":"adjusted","path":[]}},"direction":"desc","nulls":"last"
            }]}}),
            json!({"id":"t","op":"Take","inputs":["s"],"config":{"count":2}}),
        ];
        let mut hql = storage_request(&format!("p7-filter-pipeline-{case}-hql"), values);
        hql.ir = None;
        hql.hql = Some("VALUES $xs AS x |> FILTER x = -2 OR x = -1 OR x = 1 OR x = 2 |> PROJECT x AS x, x + 10 AS adjusted |> DISTINCT |> ORDER BY adjusted DESC NULLS LAST |> TAKE 2 |> RETURN x, adjusted".into());
        hql.language_version = Some(HqlLanguageVersionV2::HqlV2);
        let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql).unwrap() else {
            panic!("HQL filter pipeline should return rows for case {case}");
        };

        let mut ir = storage_request(&format!("p7-filter-pipeline-{case}-ir"), values);
        ir.ir = Some(scalar_ir(pipeline, "t"));
        let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir).unwrap() else {
            panic!("typed-IR filter pipeline should return rows for case {case}");
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
