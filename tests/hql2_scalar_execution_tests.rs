//! Production scalar kernels, independently expected results; no storage opens.
#![allow(dead_code)]
#[path = "../src/query/hql2/bind.rs"]
mod bind;
#[path = "../src/query/hql2/catalog.rs"]
mod catalog;
#[path = "../src/query/hql2/error.rs"]
mod error;
#[path = "../src/query/hql2/exec.rs"]
mod exec;
#[path = "../src/query/hql2/plan.rs"]
mod plan;
#[path = "support/hql2_reference.rs"]
mod reference;
#[path = "../src/query/hql2/result.rs"]
mod result;
#[path = "../src/query/hql2/source.rs"]
mod source;
#[path = "../src/uee_v2.rs"]
mod uee_v2;
#[path = "../src/query/hql2/value.rs"]
mod value;
#[path = "../src/query/hql2/wire.rs"]
mod wire;

use serde_json::{json, Value};
use std::collections::BTreeMap;
use value::QueryValueV2 as V;

fn field(name: &str) -> Value {
    json!({"field":{"alias":name,"path":[]}})
}
fn lit(v: &str) -> Value {
    json!({"type":"I64","literal":v})
}
fn binary(op: &str, a: Value, b: Value) -> Value {
    json!({"binary":op,"left":a,"right":b})
}
fn node(id: &str, op: &str, inputs: &[&str], config: Value) -> Value {
    json!({"id":id,"op":op,"inputs":inputs,"config":config})
}
fn source(id: &str, param: &str, alias: &str) -> Value {
    node(id, "Values", &[], json!({"param":param,"as":alias}))
}
fn params(ty: &str, values: Value) -> BTreeMap<String, Value> {
    BTreeMap::from([(
        "xs".into(),
        json!({"type":format!("List<{ty}>"),"value":values}),
    )])
}
fn prepare(
    nodes: Vec<Value>,
    root: &str,
    p: &BTreeMap<String, Value>,
) -> Result<plan::PhysicalPlanV2, error::QueryErrorV2> {
    let declarations: BTreeMap<_, _> = p
        .iter()
        .map(|(k, v)| (k.clone(), v["type"].clone()))
        .collect();
    let ir=serde_json::from_value(json!({"contract_version":"query-ir.v2","nodes":nodes,"root":root,"parameter_types":declarations})).unwrap();
    let logical = wire::decode_ir_v2(ir)?;
    let parameters = bind::BoundParametersV2::decode(p, None)?;
    let marker = ();
    let catalog = catalog::AuthorizedCatalogV2::with_tables(
        result::CatalogStampV2 {
            observed_frontier: 0,
            policy_revision: 0,
            schema_fingerprint: "test".into(),
        },
        &marker,
        std::collections::BTreeSet::new(),
    );
    plan::plan_v2(bind::bind_v2(logical, &parameters, &catalog)?)
}
fn run(
    nodes: Vec<Value>,
    root: &str,
    p: BTreeMap<String, Value>,
) -> Result<Vec<Vec<V>>, error::QueryErrorV2> {
    let plan = prepare(nodes, root, &p)?;
    Ok(exec::execute_v2(&plan, &mut exec::ExecutionBudgetV2::new(&None)?)?.rows)
}
fn project(expr: Value) -> Value {
    node(
        "p",
        "Project",
        &["s"],
        json!({"fields":[{"expression":expr,"as":"out"}]}),
    )
}

#[test]
fn values_preserve_bag_and_typed_nullable_elements() {
    assert_eq!(
        run(
            vec![source("s", "xs", "x")],
            "s",
            params("Nullable<I64>", json!(["2", null, "2"]))
        )
        .unwrap(),
        vec![vec![V::I64(2)], vec![V::Null], vec![V::I64(2)]]
    );
}
#[test]
fn closed_parameters_and_declaration_agreement() {
    for invalid in [
        json!({"type":"I64","value":1}),
        json!({"type":"I64","value":"01"}),
        json!({"type":"I64","value":"1","extra":true}),
        json!({"value":"1"}),
    ] {
        assert!(
            bind::BoundParametersV2::decode(&BTreeMap::from([("x".into(), invalid)]), None)
                .is_err()
        );
    }
}
#[test]
fn filter_project_distinct_sort_offset_take_composition() {
    let nodes = vec![
        source("s", "xs", "x"),
        node(
            "f",
            "Filter",
            &["s"],
            json!({"predicate":binary("gt",field("x"),lit("1"))}),
        ),
        node(
            "p",
            "Project",
            &["f"],
            json!({"fields":[{"expression":binary("mul",field("x"),lit("2")),"as":"y"}]}),
        ),
        node("d", "Distinct", &["p"], json!({})),
        node(
            "o",
            "Sort",
            &["d"],
            json!({"keys":[{"expression":field("y"),"direction":"desc","nulls":"last"}]}),
        ),
        node("z", "Offset", &["o"], json!({"count":1})),
        node("t", "Take", &["z"], json!({"count":1})),
    ];
    assert_eq!(
        run(nodes, "t", params("I64", json!(["1", "3", "2", "3"]))).unwrap(),
        vec![vec![V::I64(4)]]
    );
}
#[test]
fn empty_input_still_checks_scope_and_types() {
    for expr in [
        field("missing"),
        binary("add", field("x"), json!({"type":"Utf8","literal":"wrong"})),
        json!({"call":"unknown","args":[]}),
    ] {
        assert_eq!(
            run(
                vec![source("s", "xs", "x"), project(expr)],
                "p",
                params("I64", json!([]))
            )
            .unwrap_err()
            .code,
            "BIND_ERROR"
        );
    }
    assert!(run(
        vec![
            source("s", "xs", "x"),
            node("f", "Filter", &["s"], json!({"predicate":field("x")}))
        ],
        "f",
        params("I64", json!([]))
    )
    .is_err());
}
#[test]
fn eager_boolean_evaluation_does_not_hide_division_error() {
    let expr = binary(
        "and",
        json!({"type":"Bool","literal":false}),
        binary("eq", binary("div", lit("1"), lit("0")), lit("0")),
    );
    let e = run(
        vec![source("s", "xs", "x"), project(expr)],
        "p",
        params("I64", json!(["0"])),
    )
    .unwrap_err();
    assert_eq!(e.stage, "execute");
    assert_eq!(e.detail.unwrap()["reason"], "division_by_zero");
}
#[test]
fn checked_numerics_and_unicode_functions() {
    for expr in [
        binary("add", lit("9223372036854775807"), lit("1")),
        binary("div", lit("-9223372036854775808"), lit("-1")),
    ] {
        assert!(run(
            vec![source("s", "xs", "x"), project(expr)],
            "p",
            params("I64", json!(["1"]))
        )
        .is_err());
    }
    assert_eq!(
        run(
            vec![
                source("s", "xs", "x"),
                project(json!({"call":"length","args":[field("x")]}))
            ],
            "p",
            params("Utf8", json!(["ก😀"]))
        )
        .unwrap(),
        vec![vec![V::I64(2)]]
    );
}

#[test]
fn checked_remainder_uses_truncating_quotient_and_propagates_null() {
    use reference::{Expr as ReferenceExpr, Plan as ReferencePlan, Value as ReferenceValue};
    let input = ReferencePlan::Values(
        [Some(-7), None, Some(7)]
            .into_iter()
            .map(|value| {
                BTreeMap::from([(
                    "x".into(),
                    value
                        .map(ReferenceValue::I64)
                        .unwrap_or(ReferenceValue::Null),
                )])
            })
            .collect(),
    );
    let expected = reference::execute(&ReferencePlan::Project(
        Box::new(input),
        vec![(
            "remainder".into(),
            ReferenceExpr::Rem(
                Box::new(ReferenceExpr::Field("x".into())),
                Box::new(ReferenceExpr::Literal(ReferenceValue::I64(4))),
            ),
        )],
    ))
    .unwrap();
    let native = run(
        vec![
            source("s", "xs", "x"),
            project(binary("rem", field("x"), lit("4"))),
        ],
        "p",
        params("Nullable<I64>", json!(["-7", null, "7"])),
    )
    .unwrap();
    assert_eq!(
        native,
        expected
            .iter()
            .map(|row| {
                vec![match &row["remainder"] {
                    ReferenceValue::Null => V::Null,
                    ReferenceValue::I64(value) => V::I64(*value),
                    _ => panic!("unexpected reference value"),
                }]
            })
            .collect::<Vec<_>>()
    );

    assert_eq!(
        run(
            vec![
                source("s", "xs", "x"),
                project(binary(
                    "rem",
                    field("x"),
                    json!({"type":"F64Finite","literal":2.0}),
                )),
            ],
            "p",
            params("F64Finite", json!([-4.5, 4.5])),
        )
        .unwrap(),
        vec![vec![V::F64(-0.5)], vec![V::F64(0.5)]]
    );

    let error = run(
        vec![
            source("s", "xs", "x"),
            project(binary("rem", field("x"), lit("0"))),
        ],
        "p",
        params("I64", json!(["7"])),
    )
    .unwrap_err();
    assert_eq!(error.detail.unwrap()["reason"], "division_by_zero");

    for zero in [json!(0.0), json!(-0.0)] {
        let error = run(
            vec![
                source("s", "xs", "x"),
                project(binary(
                    "rem",
                    field("x"),
                    json!({"type":"F64Finite","literal":zero}),
                )),
            ],
            "p",
            params("F64Finite", json!([7.0])),
        )
        .unwrap_err();
        assert_eq!(error.detail.unwrap()["reason"], "division_by_zero");
    }

    let error = run(
        vec![
            source("s", "xs", "x"),
            project(binary("rem", field("x"), lit("-1"))),
        ],
        "p",
        params("I64", json!([i64::MIN.to_string()])),
    )
    .unwrap_err();
    assert_eq!(error.detail.unwrap()["reason"], "integer_overflow");

    let error = run(
        vec![
            source("s", "xs", "x"),
            project(binary(
                "rem",
                lit("7"),
                json!({"type":"Utf8","literal":"2"}),
            )),
        ],
        "p",
        params("I64", json!(["1"])),
    )
    .unwrap_err();
    assert_eq!(error.detail.unwrap()["reason"], "numeric_type");
}
#[test]
fn three_valued_boolean_truth_tables() {
    for (op, other, expected) in [
        ("and", false, V::Bool(false)),
        ("and", true, V::Null),
        ("or", true, V::Bool(true)),
        ("or", false, V::Null),
    ] {
        let e = binary(op, field("x"), json!({"type":"Bool","literal":other}));
        assert_eq!(
            run(
                vec![source("s", "xs", "x"), project(e)],
                "p",
                params("Nullable<Bool>", json!([null]))
            )
            .unwrap(),
            vec![vec![expected]]
        );
    }
}
#[test]
fn null_sort_order_independent_of_direction_and_list_order_rejected() {
    let sort = node(
        "t",
        "Sort",
        &["s"],
        json!({"keys":[{"expression":field("x"),"direction":"desc","nulls":"first"}]}),
    );
    assert_eq!(
        run(
            vec![source("s", "xs", "x"), sort.clone()],
            "t",
            params("Nullable<I64>", json!(["1", null, "2"]))
        )
        .unwrap(),
        vec![vec![V::Null], vec![V::I64(2)], vec![V::I64(1)]]
    );
    assert!(run(
        vec![source("s", "xs", "x"), sort],
        "t",
        params("List<I64>", json!([]))
    )
    .is_err());
}
#[test]
fn all_join_kinds_preserve_multiplicity_and_null_keys() {
    let mut p = params("Nullable<I64>", json!(["1", "1", null, "2"]));
    p.insert(
        "ys".into(),
        json!({"type":"List<Nullable<I64>>","value":["1","1",null]}),
    );
    for (kind, n) in [("inner", 4), ("left", 6), ("semi", 2), ("anti", 2)] {
        let nodes = vec![
            source("l", "xs", "x"),
            source("r", "ys", "y"),
            node(
                "j",
                "Join",
                &["l", "r"],
                json!({"kind":kind,"condition":binary("eq",field("x"),field("y"))}),
            ),
        ];
        let rows = run(nodes, "j", p.clone()).unwrap();
        assert_eq!(rows.len(), n, "{kind}");
        assert_eq!(
            rows[0].len(),
            if kind == "semi" || kind == "anti" {
                1
            } else {
                2
            }
        );
    }
}
#[test]
fn union_requires_equal_scope_and_types_even_empty() {
    let mut p = params("I64", json!([]));
    p.insert("ys".into(), json!({"type":"List<Utf8>","value":[]}));
    let nodes = vec![
        source("l", "xs", "x"),
        source("r", "ys", "x"),
        node("u", "UnionAll", &["l", "r"], json!({})),
    ];
    assert!(run(nodes.clone(), "u", p.clone()).is_err());
    p.insert("ys".into(), json!({"type":"List<I64>","value":["5","5"]}));
    assert_eq!(
        run(nodes, "u", p).unwrap(),
        vec![vec![V::I64(5)], vec![V::I64(5)]]
    );
}
fn aggregate(group: Vec<Value>, calls: Vec<(&str, Vec<Value>)>) -> Value {
    node(
        "a",
        "Aggregate",
        &["s"],
        json!({"group_by":group,"aggregates":calls.into_iter().enumerate().map(|(i,(name,args))|json!({"expression":{"call":name,"args":args},"as":format!("v{i}")})).collect::<Vec<_>>()}),
    )
}
#[test]
fn empty_global_aggregates_and_grouped_empty() {
    let a = aggregate(
        vec![],
        vec![
            ("count_all", vec![]),
            ("count", vec![field("x")]),
            ("sum", vec![field("x")]),
            ("avg", vec![field("x")]),
            ("min", vec![field("x")]),
            ("max", vec![field("x")]),
            ("collect", vec![field("x")]),
        ],
    );
    assert_eq!(
        run(
            vec![source("s", "xs", "x"), a],
            "a",
            params("I64", json!([]))
        )
        .unwrap(),
        vec![vec![
            V::I64(0),
            V::I64(0),
            V::Null,
            V::Null,
            V::Null,
            V::Null,
            V::List(vec![])
        ]]
    );
    let a = aggregate(
        vec![json!({"expression":field("x"),"as":"g"})],
        vec![("count_all", vec![])],
    );
    assert!(run(
        vec![source("s", "xs", "x"), a],
        "a",
        params("I64", json!([]))
    )
    .unwrap()
    .is_empty());
}
#[test]
fn aggregate_validates_every_arity_and_argument_on_empty() {
    for (call, args) in [
        ("count", vec![]),
        ("count_all", vec![field("x")]),
        ("sum", vec![field("x"), field("x")]),
        ("collect", vec![field("missing")]),
        ("min", vec![field("x")]),
    ] {
        assert!(run(
            vec![
                source("s", "xs", "x"),
                aggregate(vec![], vec![(call, args)])
            ],
            "a",
            params("List<I64>", json!([]))
        )
        .is_err());
    }
}
#[test]
fn aggregates_ignore_null_except_collect_and_group_null_together() {
    let a = aggregate(
        vec![],
        vec![
            ("count", vec![field("x")]),
            ("sum", vec![field("x")]),
            ("avg", vec![field("x")]),
            ("collect", vec![field("x")]),
        ],
    );
    assert_eq!(
        run(
            vec![source("s", "xs", "x"), a],
            "a",
            params("Nullable<I64>", json!(["1", null, "3"]))
        )
        .unwrap(),
        vec![vec![
            V::I64(2),
            V::I64(4),
            V::F64(2.0),
            V::List(vec![V::I64(1), V::Null, V::I64(3)])
        ]]
    );
}
#[test]
fn change_scan_binds_a_typed_change_event_source() {
    let plan = prepare(
        vec![node(
            "s",
            "ChangeScan",
            &[],
            json!({"after_seq":"0","as":"c"}),
        )],
        "s",
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(plan.root(), "s");
    assert_eq!(plan.columns()[0].data_type, "ChangeEvent");
}
#[test]
fn explain_has_no_actuals_execution_has_measured_counters() {
    let plan = prepare(
        vec![source("s", "xs", "x")],
        "s",
        &params("I64", json!(["1", "2"])),
    )
    .unwrap();
    assert_eq!(plan.root(), "s");
    assert_eq!(plan.columns()[0].data_type, "I64");
    assert!(plan.explain_nodes().iter().all(|n| n.actual.is_none()));
    let output =
        exec::execute_v2(&plan, &mut exec::ExecutionBudgetV2::new(&None).unwrap()).unwrap();
    let actual = serde_json::to_value(output.nodes[0].actual.as_ref().unwrap()).unwrap();
    assert_eq!(actual["rows_out"]["measurement"]["status"], "measured");
    assert_eq!(actual["rows_out"]["measurement"]["value"], 2);
}
#[test]
fn budgets_error_instead_of_successful_partial_output() {
    let plan = prepare(
        vec![source("s", "xs", "x")],
        "s",
        &params("I64", json!(["1", "2"])),
    )
    .unwrap();
    for options in [
        json!({"max_result_rows":1}),
        json!({"max_memory_bytes":1}),
        json!({"max_result_bytes":1}),
    ] {
        let mut budget =
            exec::ExecutionBudgetV2::new(&Some(serde_json::from_value(options).unwrap())).unwrap();
        assert_eq!(
            exec::execute_v2(&plan, &mut budget).unwrap_err().code,
            "QUERY_BUDGET_EXCEEDED"
        );
    }
    assert!(exec::ExecutionBudgetV2::new(&Some(
        serde_json::from_value(json!({"max_elapsed_ms":5001})).unwrap()
    ))
    .is_err());
    assert!(exec::ExecutionBudgetV2::new(&Some(
        serde_json::from_value(json!({"max_spill_bytes":1})).unwrap()
    ))
    .is_err());
}

#[test]
fn declarations_hql_inference_and_ir_mismatch() {
    let p = params("I64", json!(["1"]));
    let params = bind::BoundParametersV2::decode(&p, None).unwrap();
    let marker = ();
    let cat = catalog::AuthorizedCatalogV2::with_tables(
        result::CatalogStampV2 {
            observed_frontier: 0,
            policy_revision: 0,
            schema_fingerprint: "test".into(),
        },
        &marker,
        std::collections::BTreeSet::new(),
    );
    let ir:uee_v2::QueryIrV2=serde_json::from_value(json!({"contract_version":"query-ir.v2","root":"s","nodes":[source("s","xs","x")],"parameter_types":{"xs":"List<Utf8>"}})).unwrap();
    let mut logical = wire::decode_ir_v2(ir).unwrap();
    assert!(bind::bind_v2(logical.clone(), &params, &cat).is_err());
    logical.parameter_types.clear();
    assert!(bind::bind_v2(logical.clone(), &params, &cat).is_err());
    logical.from_hql = true;
    assert!(bind::bind_v2(logical, &params, &cat).is_ok());
}
#[test]
fn replaced_scopes_duplicate_aliases_and_semi_right_are_rejected() {
    let p = params("I64", json!([]));
    let first = node(
        "p",
        "Project",
        &["s"],
        json!({"fields":[{"expression":field("x"),"as":"y"}]}),
    );
    let second = node(
        "q",
        "Project",
        &["p"],
        json!({"fields":[{"expression":field("x"),"as":"z"}]}),
    );
    assert!(run(vec![source("s", "xs", "x"), first, second], "q", p.clone()).is_err());
    let duplicate = node(
        "p",
        "Project",
        &["s"],
        json!({"fields":[{"expression":field("x"),"as":"y"},{"expression":field("x"),"as":"y"}]}),
    );
    assert!(run(vec![source("s", "xs", "x"), duplicate], "p", p.clone()).is_err());
    for kind in ["semi", "anti"] {
        let mut p = p.clone();
        p.insert("ys".into(), json!({"type":"List<I64>","value":[]}));
        let join = node(
            "j",
            "Join",
            &["s", "r"],
            json!({"kind":kind,"condition":binary("eq",field("x"),field("y"))}),
        );
        let project = node(
            "p",
            "Project",
            &["j"],
            json!({"fields":[{"expression":field("y"),"as":"z"}]}),
        );
        assert!(run(
            vec![
                source("s", "xs", "x"),
                source("r", "ys", "y"),
                join,
                project
            ],
            "p",
            p
        )
        .is_err());
    }
}
#[test]
fn binder_defends_internal_hql_graph_invariants() {
    let p = bind::BoundParametersV2::decode(&params("I64", json!([])), None).unwrap();
    let marker = ();
    let cat = catalog::AuthorizedCatalogV2::with_tables(
        result::CatalogStampV2 {
            observed_frontier: 0,
            policy_revision: 0,
            schema_fingerprint: "test".into(),
        },
        &marker,
        std::collections::BTreeSet::new(),
    );
    let s = wire::LogicalNodeV2 {
        id: "s".into(),
        op: uee_v2::QueryOpV2::Values,
        inputs: vec![],
        config: wire::Config::Values {
            param: "xs".into(),
            alias: "x".into(),
        },
    };
    let base = wire::LogicalRequestV2 {
        nodes: vec![s.clone()],
        root: "s".into(),
        parameter_types: BTreeMap::new(),
        from_hql: true,
    };
    let mut bad = base.clone();
    bad.nodes.push(s.clone());
    assert!(bind::bind_v2(bad, &p, &cat).is_err());
    let mut bad = base.clone();
    bad.nodes[0].inputs.push("s".into());
    assert!(bind::bind_v2(bad, &p, &cat).is_err());
    let mut chain = base;
    for i in 1..128 {
        chain.nodes.push(wire::LogicalNodeV2 {
            id: format!("n{i}"),
            op: uee_v2::QueryOpV2::Take,
            inputs: vec![if i == 1 {
                "s".into()
            } else {
                format!("n{}", i - 1)
            }],
            config: wire::Config::Take { count: 0 },
        });
    }
    chain.root = "n127".into();
    assert!(bind::bind_v2(chain.clone(), &p, &cat).is_ok());
    chain.nodes.push(wire::LogicalNodeV2 {
        id: "n128".into(),
        op: uee_v2::QueryOpV2::Take,
        inputs: vec!["n127".into()],
        config: wire::Config::Take { count: 0 },
    });
    chain.root = "n128".into();
    assert!(bind::bind_v2(chain, &p, &cat).is_err());
}
#[test]
fn differential_scalar_bags_order_and_aggregates_against_p7() {
    use reference::{Aggregate as A, Expr as E, Plan as P, SortKey, Value as R};
    for seed in 0..81 {
        let mut n = seed;
        let mut values = Vec::new();
        for _ in 0..4 {
            values.push(match n % 3 {
                0 => None,
                1 => Some(-1),
                _ => Some(2),
            });
            n /= 3;
        }
        let rp = P::Values(
            values
                .iter()
                .map(|v| BTreeMap::from([("x".into(), v.map(R::I64).unwrap_or(R::Null))]))
                .collect(),
        );
        let p = params(
            "Nullable<I64>",
            Value::Array(
                values
                    .iter()
                    .map(|v| v.map(|v| json!(v.to_string())).unwrap_or(Value::Null))
                    .collect(),
            ),
        );
        let native = run(
            vec![
                source("s", "xs", "x"),
                node("d", "Distinct", &["s"], json!({})),
                node(
                    "t",
                    "Sort",
                    &["d"],
                    json!({"keys":[{"expression":field("x"),"direction":"asc","nulls":"last"}]}),
                ),
            ],
            "t",
            p.clone(),
        )
        .unwrap();
        let expected = reference::execute(&P::Sort(
            Box::new(P::Distinct(Box::new(rp.clone()))),
            vec![SortKey {
                expr: E::Field("x".into()),
                descending: false,
                nulls_first: false,
            }],
        ))
        .unwrap();
        let convert = |r: &R| match r {
            R::Null => V::Null,
            R::I64(v) => V::I64(*v),
            _ => panic!("unexpected reference value"),
        };
        assert_eq!(
            native,
            expected
                .iter()
                .map(|r| vec![convert(&r["x"])])
                .collect::<Vec<_>>(),
            "seed{seed}"
        );
        let native = run(
            vec![
                source("s", "xs", "x"),
                aggregate(
                    vec![],
                    vec![("sum", vec![field("x")]), ("count", vec![field("x")])],
                ),
            ],
            "a",
            p,
        )
        .unwrap();
        let expected = reference::execute(&P::Aggregate {
            input: Box::new(rp),
            keys: vec![],
            aggregates: vec![
                ("sum".into(), A::Sum(E::Field("x".into()))),
                ("count".into(), A::Count(E::Field("x".into()))),
            ],
        })
        .unwrap();
        assert_eq!(
            native,
            expected
                .iter()
                .map(|r| vec![convert(&r["sum"]), convert(&r["count"])])
                .collect::<Vec<_>>()
        );
    }
}
#[test]
fn float_invalid_results_fail_and_scalar_string_predicates_work() {
    let overflow = binary(
        "mul",
        json!({"type":"F64Finite","literal":1e308}),
        json!({"type":"F64Finite","literal":1e308}),
    );
    assert_eq!(
        run(
            vec![source("s", "xs", "x"), project(overflow)],
            "p",
            params("I64", json!(["0"]))
        )
        .unwrap_err()
        .detail
        .unwrap()["reason"],
        "nonfinite_result"
    );
    let e = binary(
        "startswith",
        json!({"call":"lower","args":[field("x")]}),
        json!({"type":"Utf8","literal":"ä"}),
    );
    assert_eq!(
        run(
            vec![source("s", "xs", "x"), project(e)],
            "p",
            params("Utf8", json!(["Äther"]))
        )
        .unwrap(),
        vec![vec![V::Bool(true)]]
    );
}
#[test]
fn deadline_starts_at_budget_creation_before_execution() {
    let budget = exec::ExecutionBudgetV2::new(&Some(
        serde_json::from_value(json!({"max_elapsed_ms":1})).unwrap(),
    ))
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(3));
    assert_eq!(budget.check().unwrap_err().code, "QUERY_BUDGET_EXCEEDED");
}

#[test]
fn budget_includes_time_waiting_for_catalog_guard() {
    let started = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(3));
    let mut budget = exec::ExecutionBudgetV2::new(&Some(
        serde_json::from_value(json!({"max_elapsed_ms":1})).unwrap(),
    ))
    .unwrap();
    budget.account_elapsed_since(started);
    assert_eq!(budget.check().unwrap_err().code, "QUERY_BUDGET_EXCEEDED");
}

#[test]
fn collect_obeys_row_budget_even_when_result_has_one_row() {
    let plan = prepare(
        vec![
            source("s", "xs", "x"),
            aggregate(vec![], vec![("collect", vec![field("x")])]),
        ],
        "a",
        &params("I64", json!(["1", "2"])),
    )
    .unwrap();
    let mut budget = exec::ExecutionBudgetV2::new(&Some(
        serde_json::from_value(json!({"max_result_rows":1})).unwrap(),
    ))
    .unwrap();
    assert_eq!(
        exec::execute_v2(&plan, &mut budget).unwrap_err().code,
        "QUERY_BUDGET_EXCEEDED"
    );
}
#[test]
fn list_comparison_is_not_an_implicitly_registered_scalar_comparator() {
    assert_eq!(
        run(
            vec![
                source("s", "xs", "x"),
                project(binary("eq", field("x"), field("x")))
            ],
            "p",
            params("List<I64>", json!([]))
        )
        .unwrap_err()
        .code,
        "CAPABILITY_UNSUPPORTED"
    );
}

#[test]
fn average_preserves_checked_numeric_error_behavior_of_reference() {
    use reference::{Aggregate as A, Expr as E, Plan as P, Value as R};
    let rows = vec![BTreeMap::from([("x".into(), R::I64(i64::MAX))]); 2];
    assert_eq!(
        reference::execute(&P::Aggregate {
            input: Box::new(P::Values(rows)),
            keys: vec![],
            aggregates: vec![("avg".into(), A::Avg(E::Field("x".into())))]
        })
        .unwrap_err(),
        "INTEGER_OVERFLOW"
    );
    let e = run(
        vec![
            source("s", "xs", "x"),
            aggregate(vec![], vec![("avg", vec![field("x")])]),
        ],
        "a",
        params("I64", json!([i64::MAX.to_string(), i64::MAX.to_string()])),
    )
    .unwrap_err();
    assert_eq!(e.detail.unwrap()["reason"], "integer_overflow");
}

#[test]
fn deeply_nested_scalar_expression_works_on_windows_sized_stack() {
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let mut expression = json!({"type":"Bool","literal":true});
            for _ in 0..99 {
                expression = json!({"unary":"not","arg":expression});
            }
            assert_eq!(
                run(
                    vec![source("s", "xs", "x"), project(expression)],
                    "p",
                    params("I64", json!(["0"]))
                )
                .unwrap(),
                vec![vec![V::Bool(false)]]
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
