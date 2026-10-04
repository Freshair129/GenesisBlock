//! Cross-domain expected answers: no production evaluator is linked into this module.
#[path = "support/hql2_pipeline_reference.rs"]
mod reference;
use reference::*;
use std::collections::{BTreeMap, BTreeSet};

fn entity(kind: graph::Kind, id: &str) -> graph::EntityRef {
    graph::EntityRef {
        namespace: "fixture".into(),
        kind,
        id: id.into(),
        revision: "r1".into(),
    }
}
fn record(kind: graph::Kind, id: &str) -> graph::Revision {
    graph::Revision {
        entity: entity(kind, id),
        transaction: graph::Interval {
            start: 1,
            end: None,
        },
        valid: graph::Interval {
            start: 0,
            end: None,
        },
        retracted: false,
        fields: BTreeMap::new(),
        data: graph::RecordData::Plain,
    }
}
fn environment() -> Environment {
    let mut records = vec![
        record(graph::Kind::Node, "A"),
        record(graph::Kind::Node, "B"),
        record(graph::Kind::Node, "C"),
    ];
    for (id, from, to) in [("AB1", "A", "B"), ("AB2", "A", "B"), ("BC", "B", "C")] {
        let mut r = record(graph::Kind::Edge, id);
        r.data = graph::RecordData::Edge {
            source: entity(graph::Kind::Node, from).identity(),
            target: entity(graph::Kind::Node, to).identity(),
            relation: "LINK".into(),
        };
        records.push(r);
    }
    let mut annotation = record(graph::Kind::Annotation, "note");
    annotation.data = graph::RecordData::Annotation {
        targets: vec![graph::Target {
            binding: graph::TargetBinding::Live(entity(graph::Kind::Node, "B").identity()),
            selector: graph::Selector::Whole,
        }],
    };
    records.push(annotation);
    records.push(record(graph::Kind::Row, "row1"));
    let mut event = record(graph::Kind::Event, "evt");
    event.data = graph::RecordData::Change {
        subject: entity(graph::Kind::Node, "A"),
        operation: graph::ChangeKind::Upsert,
    };
    records.push(event);
    let catalog = graph::Catalog {
        frontier: 10,
        history: [
            graph::Kind::Node,
            graph::Kind::Edge,
            graph::Kind::Row,
            graph::Kind::Annotation,
            graph::Kind::Event,
        ]
        .into_iter()
        .map(|k| {
            (
                k,
                graph::HistoryCapability {
                    horizon: 0,
                    available: true,
                },
            )
        })
        .collect(),
        revisions: records,
    };
    let view = graph::View {
        namespace: "fixture".into(),
        transaction: 10,
        valid_at: 5,
        permissions: graph::Permissions {
            read: catalog
                .revisions
                .iter()
                .map(|r| r.entity.identity())
                .collect(),
            annotation_body: BTreeSet::from([entity(graph::Kind::Annotation, "note").identity()]),
        },
    };
    let rank = rank::Fixture {
        space: rank::Space {
            fingerprint: "vec-v1".into(),
            dimension: 1,
            metric: rank::Metric::L2Squared,
        },
        analyzer_fingerprint: "pretok".into(),
        documents: [("A", 0.1, "red"), ("B", 0.2, "red red"), ("C", 0.3, "blue")]
            .into_iter()
            .map(|(id, x, text)| rank::Document {
                id: id.into(),
                revision: "r1".into(),
                visibility: rank::Visibility {
                    tx_from: 1,
                    tx_to: None,
                    valid_from: 0,
                    valid_to: None,
                    authorized: true,
                },
                original: Some(rank::OriginalVector {
                    fingerprint: "vec-v1".into(),
                    values: vec![x],
                }),
                text: text.into(),
                source_hash: graph::source_hash(text),
                analyzer_fingerprint: "pretok".into(),
                tokens: text.split(' ').map(str::to_owned).collect(),
            })
            .collect(),
    };
    Environment {
        catalog,
        view,
        ranking: rank,
        tokenizers: rank::TokenizerRegistry {
            entries: BTreeMap::from([(
                "scalar-v1".into(),
                rank::FixtureTokenizer::UnicodeScalarV1,
            )]),
        },
        limits: graph::Limits::default(),
    }
}
fn scan() -> Plan {
    Plan::NodeScan("s".into(), graph::Predicate::default())
}
fn vector(top: usize) -> rank::VectorRequest {
    rank::VectorRequest {
        query: rank::OriginalVector {
            fingerprint: "vec-v1".into(),
            values: vec![0.0],
        },
        top,
        source: "dense".into(),
    }
}
fn knn(input: Plan, top: usize) -> Plan {
    Plan::Knn {
        input: Box::new(input),
        owner: "s".into(),
        output: "seed_distance".into(),
        request: vector(top),
    }
}
fn expansion() -> graph::Expand {
    graph::Expand {
        start_alias: "s".into(),
        segments: vec![graph::Segment {
            end_alias: "t".into(),
            edge_alias: Some("e".into()),
            direction: graph::Direction::Out,
            relations: BTreeSet::new(),
            min_hops: 1,
            max_hops: 1,
            node_predicate: graph::Predicate::default(),
            edge_predicate: graph::Predicate::default(),
        }],
        path_alias: None,
        mode: graph::PathMode::Trail,
        optional: false,
        shortest: false,
    }
}
fn field(s: &str) -> Expr {
    Expr::Field(s.into())
}
fn count(input: Plan) -> Plan {
    Plan::Aggregate {
        input: Box::new(input),
        keys: vec![],
        aggregates: vec![("n".into(), Aggregate::CountAll)],
    }
}
fn ref_value(id: &str) -> Value {
    Value::Graph(graph::Binding::Entity(entity(graph::Kind::Node, id)))
}

#[test]
fn knn_expand_annotation_project_aggregate_preserves_parallel_edge_bag() {
    let expanded = Plan::Expand(Box::new(knn(scan(), 1)), expansion());
    let lookup = Plan::AnnotationLookup(
        Box::new(expanded),
        graph::AnnotationLookup {
            target_alias: "t".into(),
            alias: "note".into(),
            predicate: graph::Predicate::default(),
            optional: false,
        },
    );
    let projected = Plan::Project(Box::new(lookup), vec![("target".into(), field("t"))]);
    let out = execute(&environment(), &count(projected.clone())).unwrap();
    assert_eq!(
        out.rows,
        vec![BTreeMap::from([("n".into(), Value::I64(2))])]
    );
    let unique = execute(&environment(), &count(Plan::Distinct(Box::new(projected)))).unwrap();
    assert_eq!(unique.rows[0]["n"], Value::I64(1));
    assert_eq!(
        out.metadata.snapshot,
        rank::Snapshot {
            transaction: 10,
            valid: 5
        }
    );
    assert!(out.metadata.approximate_sources.is_empty());
}

#[test]
fn expanded_seed_score_keeps_its_owner_and_does_not_score_the_neighbor() {
    let out = execute(
        &environment(),
        &Plan::Expand(Box::new(knn(scan(), 1)), expansion()),
    )
    .unwrap();
    assert_eq!(out.rows.len(), 2);
    assert_eq!(
        out.metadata.score_owners["seed_distance"]
            .binding
            .as_deref(),
        Some("s")
    );
    assert_eq!(out.rows[0]["s"], ref_value("A"));
    assert_eq!(out.rows[0]["t"], ref_value("B"));
    assert!(!out.rows[0].contains_key("target_distance"));
}

#[test]
fn cross_domain_filter_topk_stage_order_is_observable() {
    let filter = |p| {
        Plan::Filter(
            Box::new(p),
            Expr::Or(
                Box::new(Expr::Eq(
                    Box::new(field("s")),
                    Box::new(Expr::Literal(ref_value("B"))),
                )),
                Box::new(Expr::Eq(
                    Box::new(field("s")),
                    Box::new(Expr::Literal(ref_value("C"))),
                )),
            ),
        )
    };
    let after = execute(&environment(), &filter(knn(scan(), 2))).unwrap();
    let before = execute(&environment(), &knn(filter(scan()), 2)).unwrap();
    assert_eq!(
        after
            .rows
            .iter()
            .map(|r| r["s"].clone())
            .collect::<Vec<_>>(),
        [ref_value("B")]
    );
    assert_eq!(
        before
            .rows
            .iter()
            .map(|r| r["s"].clone())
            .collect::<Vec<_>>(),
        [ref_value("B"), ref_value("C")]
    );
}

#[test]
fn lexical_rerank_context_is_composed_and_token_budget_includes_citations() {
    let lex = Plan::LexicalMatch {
        input: Box::new(scan()),
        owner: "s".into(),
        output: "lex".into(),
        request: rank::LexicalRequest {
            analyzer_fingerprint: "pretok".into(),
            terms: vec!["red".into()],
            k1: 1.2,
            b: 0.75,
            top: 2,
            source: "lexical".into(),
        },
    };
    let rerank = Plan::Rerank {
        input: Box::new(lex),
        owner: "s".into(),
        output: "distance".into(),
        request: vector(2),
    };
    let out = execute(
        &environment(),
        &Plan::ContextPack {
            input: Box::new(rerank),
            owner: "s".into(),
            output: "package".into(),
            tokenizer: "scalar-v1".into(),
            budget: 6,
        },
    )
    .unwrap();
    let Value::Context(pack) = &out.rows[0]["package"] else {
        panic!("package")
    };
    assert_eq!(pack.rendered_context, "red[1]");
    assert_eq!(pack.token_count, 6);
    assert!(pack.truncated);
    assert_eq!(pack.omitted_refs[0].owner_id, "B");
    assert!(out.metadata.rerank_candidates);
}

#[test]
fn source_families_and_relational_composition_have_checked_answers() {
    let env = environment();
    for (source, n) in [
        (
            Plan::EdgeScan("edge".into(), graph::Predicate::default()),
            3,
        ),
        (Plan::RowScan("row".into(), graph::Predicate::default()), 1),
        (
            Plan::AnnotationScan("a".into(), graph::Predicate::default()),
            1,
        ),
        (
            Plan::HistoryScan {
                kind: graph::Kind::Node,
                alias: "n".into(),
                predicate: graph::Predicate::default(),
                transactions: graph::Interval {
                    start: 1,
                    end: Some(11),
                },
                valid: graph::Interval {
                    start: 0,
                    end: Some(6),
                },
            },
            3,
        ),
        (
            Plan::ChangeScan {
                alias: "ev".into(),
                predicate: graph::Predicate::default(),
                after: 0,
                through: 10,
            },
            1,
        ),
    ] {
        assert_eq!(
            execute(&env, &count(source)).unwrap().rows[0]["n"],
            Value::I64(n)
        );
    }
    let input = Plan::Values(vec![
        BTreeMap::from([("x".into(), Value::I64(2))]),
        BTreeMap::from([("x".into(), Value::I64(1))]),
    ]);
    let p = Plan::Take(
        Box::new(Plan::Offset(
            Box::new(Plan::Sort(
                Box::new(Plan::UnionAll(Box::new(input.clone()), Box::new(input))),
                vec![SortKey {
                    expr: field("x"),
                    descending: false,
                    nulls_first: false,
                }],
            )),
            1,
        )),
        2,
    );
    assert_eq!(
        execute(&env, &p)
            .unwrap()
            .rows
            .iter()
            .map(|r| r["x"].clone())
            .collect::<Vec<_>>(),
        [Value::I64(1), Value::I64(2)]
    );
    let joined = Plan::Join {
        left: Box::new(scan()),
        right: Box::new(Plan::Values(vec![BTreeMap::from([(
            "x".into(),
            Value::Bool(true),
        )])])),
        on: Expr::Literal(Value::Bool(true)),
        kind: JoinKind::Inner,
        right_fields: vec!["x".into()],
    };
    assert_eq!(
        execute(&env, &count(joined)).unwrap().rows[0]["n"],
        Value::I64(3)
    );
    let matched = Plan::Match {
        start: "s".into(),
        predicate: graph::Predicate {
            id: Some("A".into()),
            ..Default::default()
        },
        expansion: expansion(),
    };
    assert_eq!(
        execute(&env, &count(matched)).unwrap().rows[0]["n"],
        Value::I64(2)
    );
}

#[test]
fn graph_acl_blocks_hidden_seed_before_vector_ranking() {
    let mut env = environment();
    env.view
        .permissions
        .read
        .remove(&entity(graph::Kind::Node, "A").identity());
    let out = execute(&env, &knn(scan(), 1)).unwrap();
    assert_eq!(out.rows[0]["s"], ref_value("B"));
}

#[test]
fn lexical_corpus_statistics_do_not_use_graph_hidden_documents() {
    let mut env = environment();
    env.view
        .permissions
        .read
        .remove(&entity(graph::Kind::Node, "A").identity());
    let query = Plan::LexicalMatch {
        input: Box::new(scan()),
        owner: "s".into(),
        output: "score".into(),
        request: rank::LexicalRequest {
            analyzer_fingerprint: "pretok".into(),
            terms: vec!["red".into()],
            k1: 1.2,
            b: 0.75,
            top: 3,
            source: "lexical".into(),
        },
    };
    let result = execute(&env, &query).unwrap();
    // Visible corpus is B(len2,tf2), C(len1,tf0): N=2, df=1, avgdl=1.5.
    let expected = 2.0_f64.ln() * (2.0 * 2.2) / (2.0 + 1.2 * (0.25 + 0.75 * 2.0 / 1.5));
    assert_eq!(result.rows.len(), 1);
    let Value::F64(score) = result.rows[0]["score"] else {
        panic!("score type")
    };
    assert!((score - expected).abs() < 1e-12, "{score} != {expected}");
}

#[test]
fn empty_graph_input_still_rejects_output_scalar_alias_collision() {
    let input = Plan::Project(
        Box::new(scan()),
        vec![
            ("s".into(), field("s")),
            ("t".into(), Expr::Literal(Value::I64(7))),
        ],
    );
    let empty = Plan::Take(Box::new(input), 0);
    assert_eq!(
        execute(&environment(), &Plan::Expand(Box::new(empty), expansion())),
        Err(Error::Schema)
    );
}

#[test]
fn context_package_remains_a_single_typed_binding_for_return_and_count() {
    let context = Plan::ContextPack {
        input: Box::new(scan()),
        owner: "s".into(),
        output: "package".into(),
        tokenizer: "scalar-v1".into(),
        budget: 100,
    };
    assert_eq!(
        execute(&environment(), &count(context.clone()))
            .unwrap()
            .rows[0]["n"],
        Value::I64(1)
    );
    let projected = execute(
        &environment(),
        &Plan::Project(
            Box::new(context),
            vec![("returned".into(), field("package"))],
        ),
    )
    .unwrap();
    assert_eq!(projected.rows.len(), 1);
    assert!(matches!(projected.rows[0]["returned"], Value::Context(_)));
}
#[test]
fn match_shortest_binds_endpoint_pairs_and_chooses_one_parallel_path() {
    let mut x = expansion();
    x.shortest = true;
    x.segments[0].max_hops = 2;
    x.path_alias = Some("p".into());
    let query = Plan::Match {
        start: "s".into(),
        predicate: graph::Predicate {
            id: Some("A".into()),
            ..Default::default()
        },
        expansion: x,
    };
    let result = execute(&environment(), &query).unwrap();
    let paths = result
        .rows
        .iter()
        .map(|r| match &r["p"] {
            Value::Graph(graph::Binding::Path(p)) => {
                p.edges.iter().map(|e| e.id.as_str()).collect::<Vec<_>>()
            }
            _ => panic!("path"),
        })
        .collect::<Vec<_>>();
    assert_eq!(paths, vec![vec!["AB1"], vec!["AB1", "BC"]]);
}

#[test]
fn empty_relations_do_not_hide_unknown_fields_in_filter_project_sort_aggregate() {
    let input = Plan::Take(Box::new(scan()), 0);
    for query in [
        Plan::Filter(Box::new(input.clone()), field("missing")),
        Plan::Project(
            Box::new(input.clone()),
            vec![("x".into(), field("missing"))],
        ),
        Plan::Sort(
            Box::new(input.clone()),
            vec![SortKey {
                expr: field("missing"),
                descending: false,
                nulls_first: false,
            }],
        ),
        Plan::Aggregate {
            input: Box::new(input),
            keys: vec![],
            aggregates: vec![("n".into(), Aggregate::Count(field("missing")))],
        },
    ] {
        assert_eq!(
            execute(&environment(), &query),
            Err(Error::Scalar("FIELD_UNKNOWN"))
        );
    }
}
#[test]
fn graph_work_budget_is_shared_across_the_entire_composed_plan() {
    let mut env = environment();
    env.limits.max_work = 1000;
    assert_eq!(execute(&env, &scan()).unwrap().rows.len(), 3);
    let mut plan = scan();
    for _ in 0..99 {
        plan = Plan::UnionAll(Box::new(plan), Box::new(scan()));
    }
    assert_eq!(
        execute(&env, &plan).err(),
        Some(Error::Graph(graph::Error::BudgetExceeded))
    );
}

#[test]
fn logical_depth_boundary_is_checked_without_recursive_dispatch_frames() {
    let mut query = Plan::Values(vec![BTreeMap::from([("x".into(), Value::I64(1))])]);
    for _ in 0..128 {
        query = Plan::Take(Box::new(query), 1);
    }
    assert_eq!(
        execute(&environment(), &query).unwrap().rows[0]["x"],
        Value::I64(1)
    );
    assert_eq!(
        execute(&environment(), &Plan::Take(Box::new(query), 1)).err(),
        Some(Error::Depth)
    );
}
#[test]
fn project_preserves_score_origin_without_reassigning_it_to_reused_alias() {
    let renamed = Plan::Project(
        Box::new(knn(scan(), 1)),
        vec![
            ("seed".into(), field("s")),
            ("d".into(), field("seed_distance")),
        ],
    );
    let out = execute(&environment(), &renamed).unwrap();
    assert_eq!(
        out.metadata.score_owners["d"].binding.as_deref(),
        Some("seed")
    );
    assert_eq!(out.metadata.score_owners["d"].origin, "s");
    let dropped = Plan::Project(
        Box::new(renamed),
        vec![
            ("s".into(), Expr::Literal(ref_value("B"))),
            ("d".into(), field("d")),
        ],
    );
    let out = execute(&environment(), &dropped).unwrap();
    assert_eq!(out.metadata.score_owners["d"].binding, None);
    assert_eq!(out.metadata.score_owners["d"].origin, "s");
}

#[test]
fn knn_skips_node_without_any_ranking_document_but_rerank_requires_original() {
    let mut env = environment();
    env.ranking.documents.retain(|d| d.id != "A");
    assert_eq!(
        execute(&env, &knn(scan(), 1)).unwrap().rows[0]["s"],
        ref_value("B")
    );
    let rerank = Plan::Rerank {
        input: Box::new(scan()),
        owner: "s".into(),
        output: "d".into(),
        request: vector(3),
    };
    assert_eq!(
        execute(&env, &rerank).err(),
        Some(Error::Ranking(rank::Error::OriginalUnavailable))
    );
}

#[test]
fn projected_entity_literal_is_typed_for_following_knn() {
    let input = Plan::Project(
        Box::new(Plan::Values(vec![BTreeMap::new()])),
        vec![("s".into(), Expr::Literal(ref_value("B")))],
    );
    assert_eq!(
        execute(&environment(), &knn(input, 1)).unwrap().rows[0]["s"],
        ref_value("B")
    );
}

#[test]
fn semi_and_anti_do_not_export_right_score_bindings() {
    let right = knn(scan(), 1);
    for kind in [JoinKind::Semi, JoinKind::Anti] {
        let query = Plan::Join {
            left: Box::new(Plan::Values(vec![BTreeMap::from([(
                "left".into(),
                Value::I64(1),
            )])])),
            right: Box::new(right.clone()),
            on: Expr::Literal(Value::Bool(true)),
            kind,
            right_fields: vec!["s".into(), "seed_distance".into()],
        };
        let out = execute(&environment(), &query).unwrap();
        assert!(out.metadata.score_owners.is_empty());
        assert_eq!(out.columns, BTreeSet::from(["left".into()]));
    }
}

#[test]
fn source_match_rejects_optional_mode_absent_from_its_wire_contract() {
    let mut x = expansion();
    x.shortest = true;
    x.optional = true;
    let query = Plan::Match {
        start: "s".into(),
        predicate: graph::Predicate::default(),
        expansion: x,
    };
    assert_eq!(execute(&environment(), &query).err(), Some(Error::Schema));
}
#[test]
fn empty_input_rejects_non_boolean_predicates_and_operands() {
    let input = Plan::Take(Box::new(scan()), 0);
    for expr in [
        Expr::Literal(Value::I64(1)),
        field("s"),
        Expr::And(
            Box::new(Expr::Literal(Value::Bool(false))),
            Box::new(Expr::Literal(Value::Text("wrong".into()))),
        ),
    ] {
        assert_eq!(
            execute(&environment(), &Plan::Filter(Box::new(input.clone()), expr)).err(),
            Some(Error::Scalar("TYPE_MISMATCH"))
        );
    }
    let projected = Plan::Project(
        Box::new(scan()),
        vec![("x".into(), Expr::Literal(Value::I64(1)))],
    );
    assert_eq!(
        execute(
            &environment(),
            &Plan::Filter(Box::new(Plan::Take(Box::new(projected), 0)), field("x"))
        )
        .err(),
        Some(Error::Scalar("TYPE_MISMATCH"))
    );
}

#[test]
fn dropping_a_context_row_removes_the_entire_payload() {
    let context = Plan::ContextPack {
        input: Box::new(scan()),
        owner: "s".into(),
        output: "package".into(),
        tokenizer: "scalar-v1".into(),
        budget: 100,
    };
    let result = execute(&environment(), &Plan::Take(Box::new(context), 0)).unwrap();
    assert!(result.rows.is_empty());
    assert!(!format!("{result:?}").contains("red red"));
}
#[test]
fn union_rejects_incompatible_scalar_column_types() {
    let rows = |v| Plan::Values(vec![BTreeMap::from([("x".into(), v)])]);
    assert_eq!(
        execute(
            &environment(),
            &Plan::UnionAll(
                Box::new(rows(Value::I64(1))),
                Box::new(rows(Value::Text("1".into())))
            )
        )
        .err(),
        Some(Error::Schema)
    );
}

#[test]
fn empty_domain_columns_do_not_acquire_undeclared_sort_or_numeric_operators() {
    let empty = Plan::Take(Box::new(scan()), 0);
    let sorted = Plan::Sort(
        Box::new(empty.clone()),
        vec![SortKey {
            expr: field("s"),
            descending: false,
            nulls_first: false,
        }],
    );
    assert_eq!(
        execute(&environment(), &sorted).err(),
        Some(Error::Scalar("TYPE_MISMATCH"))
    );
    for agg in [
        Aggregate::Sum(field("s")),
        Aggregate::Avg(field("s")),
        Aggregate::Min(field("s")),
        Aggregate::Max(field("s")),
    ] {
        let query = Plan::Aggregate {
            input: Box::new(empty.clone()),
            keys: vec![],
            aggregates: vec![("value".into(), agg)],
        };
        assert_eq!(
            execute(&environment(), &query).err(),
            Some(Error::Scalar("TYPE_MISMATCH"))
        );
    }
}
#[test]
fn union_does_not_label_an_ordinary_number_as_a_ranked_score() {
    let ranked = Plan::Project(
        Box::new(knn(scan(), 1)),
        vec![("d".into(), field("seed_distance"))],
    );
    let ordinary = Plan::Values(vec![BTreeMap::from([("d".into(), Value::F64(42.0))])]);
    assert_eq!(
        execute(
            &environment(),
            &Plan::UnionAll(Box::new(ranked), Box::new(ordinary))
        )
        .err(),
        Some(Error::Schema)
    );
}

#[test]
fn lexical_absent_text_document_is_nonmatching_not_an_owner_error() {
    let mut env = environment();
    env.ranking.documents.retain(|d| d.id != "C");
    let query = Plan::LexicalMatch {
        input: Box::new(scan()),
        owner: "s".into(),
        output: "score".into(),
        request: rank::LexicalRequest {
            analyzer_fingerprint: "pretok".into(),
            terms: vec!["red".into()],
            k1: 1.2,
            b: 0.75,
            top: 3,
            source: "lex".into(),
        },
    };
    let out = execute(&env, &query).unwrap();
    assert_eq!(out.rows.len(), 2);
    assert_eq!(out.rows[0]["s"], ref_value("B"));
    assert_eq!(out.rows[1]["s"], ref_value("A"));
}
#[test]
fn count_and_collect_validate_argument_expression_types_on_empty_input() {
    for empty in [false, true] {
        for agg in [
            Aggregate::Count(Expr::Not(Box::new(Expr::Literal(Value::I64(1))))),
            Aggregate::Collect(Expr::Not(Box::new(Expr::Literal(Value::I64(1)))), 10),
        ] {
            let input = if empty {
                Plan::Take(Box::new(scan()), 0)
            } else {
                scan()
            };
            let query = Plan::Aggregate {
                input: Box::new(input),
                keys: vec![],
                aggregates: vec![("a".into(), agg)],
            };
            assert_eq!(
                execute(&environment(), &query).err(),
                Some(Error::Scalar("TYPE_MISMATCH"))
            );
        }
    }
}
