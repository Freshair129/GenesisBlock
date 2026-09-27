//! Independent P7 exact interpreter acceptance; this target imports no engine evaluator.
#[path = "support/hql2_reference.rs"]
mod reference;
use reference::*;
use std::collections::BTreeMap;

fn row(values: &[(&str, Value)]) -> Row {
    values
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}
fn field(name: &str) -> Expr {
    Expr::Field(name.into())
}
fn lit(v: Value) -> Expr {
    Expr::Literal(v)
}
fn eq(a: Expr, b: Expr) -> Expr {
    Expr::Eq(Box::new(a), Box::new(b))
}
fn ints(rows: &[Row], key: &str) -> Vec<i64> {
    rows.iter()
        .map(|r| match r.get(key) {
            Some(Value::I64(n)) => *n,
            v => panic!("{v:?}"),
        })
        .collect()
}
fn sample() -> Vec<Row> {
    vec![
        row(&[
            ("id", Value::I64(1)),
            ("lang", Value::Text("en".into())),
            ("v", Value::Vector(vec![0.1, 0.0])),
        ]),
        row(&[
            ("id", Value::I64(2)),
            ("lang", Value::Text("th".into())),
            ("v", Value::Vector(vec![0.2, 0.0])),
        ]),
        row(&[
            ("id", Value::I64(3)),
            ("lang", Value::Text("th".into())),
            ("v", Value::Vector(vec![0.3, 0.0])),
        ]),
    ]
}
fn thai() -> Expr {
    eq(field("lang"), lit(Value::Text("th".into())))
}
fn knn(input: Plan, k: usize) -> Plan {
    Plan::Knn {
        input: Box::new(input),
        vector: "v".into(),
        query: vec![0.0, 0.0],
        metric: Metric::L2Squared,
        k,
        tie: vec!["id".into()],
        distance: "distance".into(),
    }
}

#[test]
fn p7_filter_and_topk_have_distinct_expected_answers() {
    let post = Plan::Filter(Box::new(knn(Plan::Values(sample()), 2)), thai());
    let pre = knn(Plan::Filter(Box::new(Plan::Values(sample())), thai()), 2);
    assert_eq!(ints(&execute(&post).unwrap(), "id"), vec![2]);
    assert_eq!(ints(&execute(&pre).unwrap(), "id"), vec![2, 3]);
}
#[test]
fn p7_knn_preserves_bag_rows_until_explicit_distinct() {
    let mut rows = sample();
    rows.push(rows[0].clone());
    assert_eq!(
        ints(&execute(&knn(Plan::Values(rows.clone()), 2)).unwrap(), "id"),
        vec![1, 1]
    );
    assert_eq!(
        ints(
            &execute(&knn(Plan::Distinct(Box::new(Plan::Values(rows))), 2)).unwrap(),
            "id"
        ),
        vec![1, 2]
    );
}
#[test]
fn p7_all_three_valued_truth_table_cells() {
    let values = [Value::Bool(false), Value::Bool(true), Value::Null];
    let and = [
        [Value::Bool(false), Value::Bool(false), Value::Bool(false)],
        [Value::Bool(false), Value::Bool(true), Value::Null],
        [Value::Bool(false), Value::Null, Value::Null],
    ];
    let or = [
        [Value::Bool(false), Value::Bool(true), Value::Null],
        [Value::Bool(true), Value::Bool(true), Value::Bool(true)],
        [Value::Null, Value::Bool(true), Value::Null],
    ];
    for a in 0..3 {
        for b in 0..3 {
            assert_eq!(
                eval(
                    &Expr::And(
                        Box::new(lit(values[a].clone())),
                        Box::new(lit(values[b].clone()))
                    ),
                    &BTreeMap::new()
                )
                .unwrap(),
                and[a][b]
            );
            assert_eq!(
                eval(
                    &Expr::Or(
                        Box::new(lit(values[a].clone())),
                        Box::new(lit(values[b].clone()))
                    ),
                    &BTreeMap::new()
                )
                .unwrap(),
                or[a][b]
            );
        }
    }
    assert_eq!(
        eval(&Expr::Not(Box::new(lit(Value::Null))), &Row::new()).unwrap(),
        Value::Null
    );
}
#[test]
fn p7_filter_retains_true_only() {
    let rows = vec![
        row(&[("p", Value::Null)]),
        row(&[("p", Value::Bool(false))]),
        row(&[("p", Value::Bool(true))]),
    ];
    assert_eq!(
        execute(&Plan::Filter(Box::new(Plan::Values(rows)), field("p"))).unwrap(),
        vec![row(&[("p", Value::Bool(true))])]
    );
}
#[test]
fn p7_checked_arithmetic_and_no_implicit_casts() {
    assert_eq!(
        eval(
            &Expr::Add(
                Box::new(lit(Value::I64(i64::MAX))),
                Box::new(lit(Value::I64(1)))
            ),
            &Row::new()
        ),
        Err("INTEGER_OVERFLOW")
    );
    assert_eq!(
        eval(
            &Expr::Div(Box::new(lit(Value::I64(3))), Box::new(lit(Value::I64(0)))),
            &Row::new()
        ),
        Err("DIVISION_BY_ZERO")
    );
    assert_eq!(
        eval(
            &eq(lit(Value::Text("1".into())), lit(Value::I64(1))),
            &Row::new()
        ),
        Err("TYPE_MISMATCH")
    );
    assert_eq!(
        eval(&eq(lit(Value::Null), lit(Value::I64(1))), &Row::new()).unwrap(),
        Value::Null
    );
}
#[test]
fn p7_fields_and_optional_properties_are_distinct() {
    assert_eq!(eval(&field("missing"), &Row::new()), Err("FIELD_UNKNOWN"));
    assert_eq!(
        eval(&Expr::Property("missing".into()), &Row::new()).unwrap(),
        Value::Null
    );
    assert_eq!(
        eval(
            &Expr::HasProperty("missing".into()),
            &row(&[("missing", Value::Null)])
        )
        .unwrap(),
        Value::Bool(true)
    );
}
#[test]
fn p7_projection_replaces_scope_and_rejects_duplicate_outputs() {
    let projected = Plan::Project(
        Box::new(Plan::Values(sample())),
        vec![("x".into(), field("id"))],
    );
    assert_eq!(
        execute(&projected).unwrap()[0],
        row(&[("x", Value::I64(1))])
    );
    let duplicate = Plan::Project(
        Box::new(Plan::Values(vec![])),
        vec![
            ("x".into(), lit(Value::I64(1))),
            ("x".into(), lit(Value::I64(2))),
        ],
    );
    assert_eq!(execute(&duplicate), Err("DUPLICATE_OUTPUT"));
}
#[test]
fn p7_joins_preserve_duplicates_and_do_not_match_null_keys() {
    let left = vec![
        row(&[("l.k", Value::I64(1))]),
        row(&[("l.k", Value::Null)]),
        row(&[("l.k", Value::I64(2))]),
    ];
    let right = vec![
        row(&[("r.k", Value::I64(1))]),
        row(&[("r.k", Value::I64(1))]),
        row(&[("r.k", Value::Null)]),
    ];
    let join = |kind| Plan::Join {
        left: Box::new(Plan::Values(left.clone())),
        right: Box::new(Plan::Values(right.clone())),
        on: eq(field("l.k"), field("r.k")),
        kind,
        right_fields: vec!["r.k".into()],
    };
    assert_eq!(execute(&join(JoinKind::Inner)).unwrap().len(), 2);
    let outer = execute(&join(JoinKind::Left)).unwrap();
    assert_eq!(outer.len(), 4);
    assert_eq!(outer[2].get("r.k"), Some(&Value::Null));
    assert_eq!(
        execute(&join(JoinKind::Semi)).unwrap(),
        vec![left[0].clone()]
    );
    assert_eq!(execute(&join(JoinKind::Anti)).unwrap(), left[1..].to_vec());
}
#[test]
fn p7_empty_right_outer_join_produces_one_null_extended_row() {
    let p = Plan::Join {
        left: Box::new(Plan::Values(vec![row(&[("id", Value::I64(1))])])),
        right: Box::new(Plan::Values(vec![])),
        on: lit(Value::Bool(true)),
        kind: JoinKind::Left,
        right_fields: vec!["r.x".into()],
    };
    assert_eq!(
        execute(&p).unwrap(),
        vec![row(&[("id", Value::I64(1)), ("r.x", Value::Null)])]
    );
}
#[test]
fn p7_empty_global_aggregate_and_grouped_input_differ() {
    let aggs = vec![
        ("n".into(), Aggregate::CountAll),
        ("sum".into(), Aggregate::Sum(field("x"))),
        ("avg".into(), Aggregate::Avg(field("x"))),
        ("min".into(), Aggregate::Min(field("x"))),
        ("max".into(), Aggregate::Max(field("x"))),
    ];
    let global = Plan::Aggregate {
        input: Box::new(Plan::Values(vec![])),
        keys: vec![],
        aggregates: aggs.clone(),
    };
    assert_eq!(
        execute(&global).unwrap(),
        vec![row(&[
            ("n", Value::I64(0)),
            ("sum", Value::Null),
            ("avg", Value::Null),
            ("min", Value::Null),
            ("max", Value::Null)
        ])]
    );
    let grouped = Plan::Aggregate {
        input: Box::new(Plan::Values(vec![])),
        keys: vec![("g".into(), lit(Value::I64(1)))],
        aggregates: aggs,
    };
    assert!(execute(&grouped).unwrap().is_empty());
}
#[test]
fn p7_aggregates_skip_nulls_without_losing_duplicate_rows() {
    let rows = vec![
        row(&[("x", Value::I64(2))]),
        row(&[("x", Value::I64(2))]),
        row(&[("x", Value::Null)]),
    ];
    let p = Plan::Aggregate {
        input: Box::new(Plan::Values(rows)),
        keys: vec![],
        aggregates: vec![
            ("n".into(), Aggregate::CountAll),
            ("present".into(), Aggregate::Count(field("x"))),
            ("s".into(), Aggregate::Sum(field("x"))),
            ("a".into(), Aggregate::Avg(field("x"))),
        ],
    };
    assert_eq!(
        execute(&p).unwrap(),
        vec![row(&[
            ("n", Value::I64(3)),
            ("present", Value::I64(2)),
            ("s", Value::I64(4)),
            ("a", Value::F64(2.0))
        ])]
    );
}
#[test]
fn p7_sort_null_placement_independent_of_direction() {
    let rows = vec![
        row(&[("x", Value::Null)]),
        row(&[("x", Value::I64(1))]),
        row(&[("x", Value::I64(2))]),
    ];
    let sort = Plan::Sort(
        Box::new(Plan::Values(rows)),
        vec![SortKey {
            expr: field("x"),
            descending: true,
            nulls_first: false,
        }],
    );
    let result = execute(&sort).unwrap();
    assert_eq!(
        result.iter().map(|r| r["x"].clone()).collect::<Vec<_>>(),
        vec![Value::I64(2), Value::I64(1), Value::Null]
    );
}
#[test]
fn p7_take_zero_and_offset_preserve_pipeline_order() {
    assert!(execute(&Plan::Take(Box::new(Plan::Values(sample())), 0))
        .unwrap()
        .is_empty());
    assert_eq!(
        ints(
            &execute(&Plan::Take(
                Box::new(Plan::Offset(Box::new(Plan::Values(sample())), 1)),
                1
            ))
            .unwrap(),
            "id"
        ),
        vec![2]
    );
}
#[test]
fn p7_vector_validation_happens_even_for_empty_input() {
    let p = Plan::Knn {
        input: Box::new(Plan::Values(vec![])),
        vector: "v".into(),
        query: vec![0.0, 0.0],
        metric: Metric::Cosine,
        k: 0,
        tie: vec!["id".into()],
        distance: "distance".into(),
    };
    assert_eq!(execute(&p), Err("ZERO_COSINE_NORM"));
    assert_eq!(
        distance(&[f64::NAN], &[0.0], Metric::L2Squared),
        Err("NONFINITE_VECTOR")
    );
    assert_eq!(
        distance(&[1.0], &[1.0, 2.0], Metric::L2Squared),
        Err("DIMENSION_MISMATCH")
    );
    assert_eq!(
        distance(&[1.0, 2.0], &[3.0, 4.0], Metric::NegDot).unwrap(),
        -11.0
    );
}
#[test]
fn p7_union_all_and_distinct_treat_null_as_duplicate_value() {
    let one = Plan::Values(vec![row(&[("x", Value::Null)])]);
    let union = Plan::UnionAll(Box::new(one.clone()), Box::new(one));
    assert_eq!(execute(&union).unwrap().len(), 2);
    assert_eq!(execute(&Plan::Distinct(Box::new(union))).unwrap().len(), 1);
}
#[test]
fn p7_stable_ties_do_not_depend_on_input_permutation() {
    let mut rows = sample();
    for r in &mut rows {
        r.insert("v".into(), Value::Vector(vec![1.0, 0.0]));
    }
    let expected = vec![1, 2, 3];
    for permutation in [[0, 1, 2], [2, 0, 1], [1, 2, 0], [2, 1, 0]] {
        assert_eq!(
            ints(
                &execute(&knn(
                    Plan::Values(permutation.into_iter().map(|i| rows[i].clone()).collect()),
                    3
                ))
                .unwrap(),
                "id"
            ),
            expected
        );
    }
}

#[test]
fn p7_reference_rejects_nonfinite_fixture_values() {
    assert_eq!(
        execute(&Plan::Values(vec![row(&[("x", Value::F64(f64::NAN))])])),
        Err("NONFINITE_VALUE")
    );
}

#[test]
fn p7_grouping_preserves_null_group_and_distinct_key_counts() {
    let rows = vec![
        row(&[("k", Value::Null)]),
        row(&[("k", Value::I64(1))]),
        row(&[("k", Value::Null)]),
    ];
    let grouped = Plan::Aggregate {
        input: Box::new(Plan::Values(rows)),
        keys: vec![("group".into(), field("k"))],
        aggregates: vec![("n".into(), Aggregate::CountAll)],
    };
    assert_eq!(
        execute(&grouped).unwrap(),
        vec![
            row(&[("group", Value::Null), ("n", Value::I64(2))]),
            row(&[("group", Value::I64(1)), ("n", Value::I64(1))])
        ]
    );
}

#[test]
fn p7_bounded_collect_errors_instead_of_silent_truncation() {
    let collect = |limit| Plan::Aggregate {
        input: Box::new(Plan::Values(sample())),
        keys: vec![],
        aggregates: vec![("ids".into(), Aggregate::Collect(field("id"), limit))],
    };
    assert_eq!(execute(&collect(2)), Err("RESOURCE_LIMIT_EXCEEDED"));
    assert_eq!(
        execute(&collect(3)).unwrap(),
        vec![row(&[(
            "ids",
            Value::List(vec![Value::I64(1), Value::I64(2), Value::I64(3)])
        )])]
    );
}

#[test]
fn p7_exhaustive_small_bag_join_cardinalities() {
    for l in 0..5 {
        for r in 0..5 {
            let left = Plan::Values((0..l).map(|_| row(&[("l", Value::I64(1))])).collect());
            let right = Plan::Values((0..r).map(|_| row(&[("r", Value::I64(1))])).collect());
            for (kind, expected) in [
                (JoinKind::Inner, l * r),
                (JoinKind::Left, l * r.max(1)),
                (JoinKind::Semi, if r > 0 { l } else { 0 }),
                (JoinKind::Anti, if r == 0 { l } else { 0 }),
            ] {
                let plan = Plan::Join {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    on: eq(field("l"), field("r")),
                    kind,
                    right_fields: vec!["r".into()],
                };
                assert_eq!(
                    execute(&plan).unwrap().len(),
                    expected,
                    "{kind:?} l={l} r={r}"
                );
            }
        }
    }
}

#[test]
fn p7_declared_plan_depth_bound_is_enforced() {
    let mut plan = Plan::Values(vec![]);
    for _ in 0..130 {
        plan = Plan::Take(Box::new(plan), 0);
    }
    assert_eq!(execute(&plan), Err("IR_DEPTH"));
}

fn context_package_fixture() -> rank::ContextPackage {
    rank::ContextPackage {
        rendered_context: "ก้🙂[1]".into(),
        token_count: 6,
        token_budget: 6,
        tokenizer_fingerprint: "custom-fixture-tokenizer".into(),
        diagnostic_envelope_counted: false,
        fragments: vec![rank::Fragment {
            evidence: rank::EvidenceRef {
                row_key: "occurrence-1".into(),
                owner_id: "a".into(),
                revision: "r1".into(),
                source_hash: "fixture-hash".into(),
                start_scalar: 0,
                end_scalar: 3,
            },
            text: "ก้🙂".into(),
            citation: "[1]".into(),
        }],
        omitted_refs: vec![],
        truncated: false,
        truncation_reason: None,
        lineage: rank::Lineage {
            approximate_sources: vec!["ann-fixture".into()],
            scope: rank::Scope::RerankCandidates,
        },
    }
}

#[test]
fn p7_context_value_projects_and_compares_without_domain_ordering() {
    let package = context_package_fixture();
    let value = Value::Context(Box::new(package.clone()));
    let input = Plan::Values(vec![row(&[("package", value.clone())])]);
    assert_eq!(
        execute(&Plan::Project(
            Box::new(input.clone()),
            vec![("returned".into(), field("package"))]
        )),
        Ok(vec![row(&[("returned", value.clone())])])
    );
    assert_eq!(
        eval(&eq(lit(value.clone()), lit(value.clone())), &Row::new()),
        Ok(Value::Bool(true))
    );
    let mut changed = package.clone();
    changed.lineage.approximate_sources.clear();
    assert_eq!(
        eval(
            &eq(lit(value.clone()), lit(Value::Context(Box::new(changed)))),
            &Row::new()
        ),
        Ok(Value::Bool(false))
    );
    let mut changed = package;
    changed.fragments[0].evidence.revision = "r2".into();
    assert_eq!(
        eval(
            &eq(lit(value.clone()), lit(Value::Context(Box::new(changed)))),
            &Row::new()
        ),
        Ok(Value::Bool(false))
    );
    assert_eq!(compare(&value, &value), Err("TYPE_MISMATCH"));
    assert_eq!(
        eval(&eq(lit(value), lit(Value::I64(6))), &Row::new()),
        Err("TYPE_MISMATCH")
    );
    assert_eq!(
        execute(&Plan::Sort(
            Box::new(input.clone()),
            vec![SortKey {
                expr: field("package"),
                descending: false,
                nulls_first: false,
            }]
        )),
        Err("TYPE_MISMATCH")
    );
    for aggregate in [
        Aggregate::Min(field("package")),
        Aggregate::Max(field("package")),
    ] {
        assert_eq!(
            execute(&Plan::Aggregate {
                input: Box::new(input.clone()),
                keys: vec![],
                aggregates: vec![("x".into(), aggregate)],
            }),
            Err("TYPE_MISMATCH")
        );
    }
}

#[test]
fn p7_context_value_validates_budget_recursively_without_guessing_tokenizer() {
    let mut invalid = context_package_fixture();
    invalid.token_count = invalid.token_budget + 1;
    let value = Value::Context(Box::new(invalid));
    for nested in [value.clone(), Value::List(vec![Value::List(vec![value])])] {
        assert_eq!(
            execute(&Plan::Values(vec![row(&[("package", nested.clone())])])),
            Err("INVALID_CONTEXT_PACKAGE")
        );
        assert_eq!(
            execute(&Plan::Project(
                Box::new(Plan::Values(vec![Row::new()])),
                vec![("package".into(), lit(nested))]
            )),
            Err("INVALID_CONTEXT_PACKAGE")
        );
    }
    // The package alone cannot resolve this opaque registry key to a token counter.
    let mut custom = context_package_fixture();
    custom.tokenizer_fingerprint = "custom-nonscalar-tokenizer".into();
    custom.token_count = 1;
    custom.token_budget = 1;
    let value = Value::Context(Box::new(custom));
    assert_eq!(eval(&lit(value.clone()), &Row::new()), Ok(value));
}

fn review_neg_dot(rows: Vec<Row>, query: Vec<f64>) -> Plan {
    Plan::Knn {
        input: Box::new(Plan::Values(rows)),
        vector: "v".into(),
        query,
        metric: Metric::NegDot,
        k: 1,
        tie: vec!["id".into()],
        distance: "distance".into(),
    }
}

#[test]
fn p7_review_neg_dot_cancellation_preserves_distance_and_rank() {
    let query = vec![1.0, 1.0, 1.0];
    let plan = review_neg_dot(
        vec![
            row(&[
                ("id", Value::I64(1)),
                ("v", Value::Vector(vec![1e16, 1.0, -1e16])),
            ]),
            row(&[
                ("id", Value::I64(2)),
                ("v", Value::Vector(vec![0.0, 0.5, 0.0])),
            ]),
        ],
        query.clone(),
    );
    assert_eq!(ints(&execute(&plan).unwrap(), "id"), vec![1]);
    for terms in [[1e16, 1.0, -1e16], [-1e16, 1e16, 1.0], [1.0, -1e16, 1e16]] {
        assert_eq!(distance(&terms, &query, Metric::NegDot), Ok(-1.0));
    }
}

#[test]
fn p7_review_neg_dot_query_validation_does_not_score_self() {
    assert_eq!(execute(&review_neg_dot(vec![], vec![1e200])), Ok(vec![]));
    let plan = review_neg_dot(
        vec![row(&[
            ("id", Value::I64(1)),
            ("v", Value::Vector(vec![0.0])),
        ])],
        vec![1e200],
    );
    assert_eq!(
        execute(&plan),
        Ok(vec![row(&[
            ("id", Value::I64(1)),
            ("v", Value::Vector(vec![0.0])),
            ("distance", Value::F64(0.0)),
        ])])
    );
    assert_eq!(
        execute(&review_neg_dot(vec![], vec![])),
        Err("DIMENSION_MISMATCH")
    );
    assert_eq!(
        execute(&review_neg_dot(vec![], vec![f64::INFINITY])),
        Err("NONFINITE_VECTOR")
    );
    assert_eq!(
        distance(&[1e200], &[1e200], Metric::NegDot),
        Err("NONFINITE_DISTANCE")
    );
}

#[test]
fn p7_review_project_rejects_nested_nonfinite_literals() {
    for invalid in [
        Value::Vector(vec![f64::NAN]),
        Value::List(vec![Value::F64(f64::INFINITY)]),
        Value::List(vec![Value::List(vec![Value::Vector(vec![
            f64::NEG_INFINITY,
        ])])]),
    ] {
        let plan = Plan::Project(
            Box::new(Plan::Values(vec![Row::new()])),
            vec![("x".into(), lit(invalid))],
        );
        assert_eq!(execute(&plan), Err("NONFINITE_VALUE"));
    }
    let mut nested = Value::I64(1);
    for _ in 0..129 {
        nested = Value::List(vec![nested]);
    }
    assert_eq!(eval(&lit(nested), &Row::new()), Err("VALUE_DEPTH"));
}

#[test]
fn p7_review_knn_depth_excludes_internal_sort() {
    let mut plan = knn(Plan::Values(sample()), 3);
    for _ in 0..127 {
        plan = Plan::Take(Box::new(plan), 3);
    }
    assert_eq!(ints(&execute(&plan).unwrap(), "id"), vec![1, 2, 3]);
    assert_eq!(execute(&Plan::Take(Box::new(plan), 3)), Err("IR_DEPTH"));
}

#[test]
fn p7_review_graph_equality_does_not_enable_domain_ordering() {
    use reference::graph::{Binding, EntityRef, Kind, Path};
    let entity = EntityRef {
        namespace: "ns".into(),
        kind: Kind::Node,
        id: "a".into(),
        revision: "r1".into(),
    };
    for binding in [
        Binding::Entity(entity.clone()),
        Binding::Edges(vec![entity.clone()]),
        Binding::Path(Path {
            vertices: vec![entity],
            edges: vec![],
        }),
    ] {
        let value = Value::Graph(binding);
        assert_eq!(
            eval(&eq(lit(value.clone()), lit(value.clone())), &Row::new()),
            Ok(Value::Bool(true))
        );
        assert_eq!(compare(&value, &value), Err("TYPE_MISMATCH"));
        let input = Plan::Values(vec![row(&[("g", value)])]);
        assert_eq!(
            execute(&Plan::Sort(
                Box::new(input.clone()),
                vec![SortKey {
                    expr: field("g"),
                    descending: false,
                    nulls_first: false,
                }]
            )),
            Err("TYPE_MISMATCH")
        );
        for aggregate in [Aggregate::Min(field("g")), Aggregate::Max(field("g"))] {
            assert_eq!(
                execute(&Plan::Aggregate {
                    input: Box::new(input.clone()),
                    keys: vec![],
                    aggregates: vec![("x".into(), aggregate)],
                }),
                Err("TYPE_MISMATCH")
            );
        }
    }
}
#[test]
fn p7_entity_equality_keeps_namespace_kind_and_revision() {
    use reference::graph::{Binding, EntityRef, Kind};
    let a = EntityRef {
        namespace: "ns".into(),
        kind: Kind::Node,
        id: "a".into(),
        revision: "r1".into(),
    };
    let value = |e| Expr::Literal(Value::Graph(Binding::Entity(e)));
    assert_eq!(
        eval(
            &Expr::Eq(Box::new(value(a.clone())), Box::new(value(a.clone()))),
            &Row::new()
        ),
        Ok(Value::Bool(true))
    );
    for b in [
        EntityRef {
            namespace: "elsewhere".into(),
            ..a.clone()
        },
        EntityRef {
            kind: Kind::Row,
            ..a.clone()
        },
        EntityRef {
            revision: "r2".into(),
            ..a.clone()
        },
    ] {
        assert_eq!(
            eval(
                &Expr::Eq(Box::new(value(a.clone())), Box::new(value(b))),
                &Row::new()
            ),
            Ok(Value::Bool(false))
        );
    }
}
