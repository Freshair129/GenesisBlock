//! Standalone: rustc --edition=2021 --test tests/hql2_graph_oracle_tests.rs
//!     -o target/hql2-graph-oracle.exe
//! Synthetic fixtures, not copied production outputs. See support module API docs.
#[path = "support/hql2_graph_reference.rs"]
mod reference;
use reference::*;
use std::collections::{BTreeMap, BTreeSet};

fn entity(kind: Kind, id: &str, revision: &str) -> EntityRef {
    EntityRef {
        namespace: "fixture".into(),
        kind,
        id: id.into(),
        revision: revision.into(),
    }
}
fn record(kind: Kind, id: &str) -> Revision {
    Revision {
        entity: entity(kind, id, "r1"),
        transaction: Interval {
            start: 1,
            end: None,
        },
        valid: Interval {
            start: 0,
            end: None,
        },
        retracted: false,
        fields: Fields::new(),
        data: RecordData::Plain,
    }
}
fn edge(id: &str, from: &str, to: &str) -> Revision {
    let mut r = record(Kind::Edge, id);
    r.data = RecordData::Edge {
        source: entity(Kind::Node, from, "r1").identity(),
        target: entity(Kind::Node, to, "r1").identity(),
        relation: "LINK".into(),
    };
    r
}
fn catalog(revisions: Vec<Revision>) -> Catalog {
    Catalog {
        frontier: 10,
        revisions,
        history: [
            Kind::Node,
            Kind::Edge,
            Kind::Row,
            Kind::Vector,
            Kind::Annotation,
            Kind::Artifact,
            Kind::Event,
        ]
        .into_iter()
        .map(|k| {
            (
                k,
                HistoryCapability {
                    horizon: 1,
                    available: true,
                },
            )
        })
        .collect(),
    }
}
fn view(c: &Catalog) -> View {
    View {
        namespace: "fixture".into(),
        transaction: 10,
        valid_at: 5,
        permissions: Permissions {
            read: c.revisions.iter().map(|r| r.entity.identity()).collect(),
            annotation_body: c
                .revisions
                .iter()
                .filter(|r| r.entity.kind == Kind::Annotation)
                .map(|r| r.entity.identity())
                .collect(),
        },
    }
}
fn graph() -> Catalog {
    catalog(vec![
        record(Kind::Node, "a"),
        record(Kind::Node, "b"),
        record(Kind::Node, "c"),
        edge("ab1", "a", "b"),
        edge("ab2", "a", "b"),
        edge("bc", "b", "c"),
        edge("ca", "c", "a"),
    ])
}
fn scan(kind: Kind, alias: &str, id: Option<&str>) -> Plan {
    Plan {
        source: Source::Scan {
            kind,
            alias: alias.into(),
            predicate: Predicate {
                id: id.map(str::to_owned),
                ..Predicate::default()
            },
        },
        stages: vec![],
    }
}
fn segment(end: &str, min: usize, max: usize) -> Segment {
    Segment {
        end_alias: end.into(),
        edge_alias: Some("e".into()),
        direction: Direction::Out,
        relations: BTreeSet::new(),
        min_hops: min,
        max_hops: max,
        node_predicate: Predicate::default(),
        edge_predicate: Predicate::default(),
    }
}
fn expand(min: usize, max: usize) -> Expand {
    Expand {
        start_alias: "s".into(),
        segments: vec![segment("t", min, max)],
        path_alias: Some("p".into()),
        mode: PathMode::Trail,
        optional: false,
        shortest: false,
    }
}
fn run(c: &Catalog, p: &Plan) -> Relation {
    execute(c, &view(c), p, Limits::default()).unwrap()
}
fn binding(kind: Kind, id: &str) -> Binding {
    Binding::Entity(entity(kind, id, "r1"))
}
fn row(bindings: &[(&str, Binding)]) -> Row {
    bindings
        .iter()
        .map(|(a, b)| (a.to_string(), b.clone()))
        .collect()
}
fn input(rows: Vec<Row>, columns: &[(&str, BindingType)]) -> Relation {
    Relation {
        schema: columns.iter().map(|(a, t)| (a.to_string(), *t)).collect(),
        rows,
    }
}
fn paths(result: &Relation) -> Vec<Vec<String>> {
    result
        .rows
        .iter()
        .map(|r| match &r["p"] {
            Binding::Path(p) => p.edges.iter().map(|e| e.id.clone()).collect(),
            other => panic!("not path: {other:?}"),
        })
        .collect()
}
fn expected_paths(paths: &[&[&str]]) -> Vec<Vec<String>> {
    paths
        .iter()
        .map(|p| p.iter().map(|s| s.to_string()).collect())
        .collect()
}
fn annotation(id: &str, targets: Vec<Target>) -> Revision {
    let mut r = record(Kind::Annotation, id);
    r.data = RecordData::Annotation { targets };
    r
}
fn live(kind: Kind, id: &str) -> Target {
    Target {
        binding: TargetBinding::Live(entity(kind, id, "r1").identity()),
        selector: Selector::Whole,
    }
}
fn frozen(kind: Kind, id: &str, rev: &str) -> Target {
    Target {
        binding: TargetBinding::Frozen(entity(kind, id, rev)),
        selector: Selector::Whole,
    }
}
fn lookup(optional: bool) -> Stage {
    Stage::Annotations(AnnotationLookup {
        target_alias: "t".into(),
        alias: "a".into(),
        predicate: Predicate::default(),
        optional,
    })
}

#[test]
fn scan_has_explicit_rows_order_and_namespace_kind_revision_identity() {
    let mut c = graph();
    c.revisions.reverse();
    let result = run(&c, &scan(Kind::Node, "s", None));
    assert_eq!(
        result.rows,
        vec![
            row(&[("s", binding(Kind::Node, "a"))]),
            row(&[("s", binding(Kind::Node, "b"))]),
            row(&[("s", binding(Kind::Node, "c"))])
        ]
    );
    assert_eq!(
        result.schema,
        BTreeMap::from([("s".into(), BindingType::Entity(Kind::Node))])
    );
}

#[test]
fn direction_parallel_edges_zero_hop_and_named_edge_types() {
    let c = graph();
    let mut p = scan(Kind::Node, "s", Some("a"));
    p.stages = vec![Stage::Expand(expand(0, 1))];
    let result = run(&c, &p);
    assert_eq!(paths(&result), expected_paths(&[&[], &["ab1"], &["ab2"]]));
    assert_eq!(result.rows[0]["t"], binding(Kind::Node, "a"));
    assert_eq!(result.rows[0]["e"], Binding::Edges(vec![]));
    let mut x = expand(1, 1);
    x.segments[0].direction = Direction::In;
    p.stages = vec![Stage::Expand(x.clone())];
    assert_eq!(paths(&run(&c, &p)), expected_paths(&[&["ca"]]));
    assert_eq!(run(&c, &p).rows[0]["e"], binding(Kind::Edge, "ca"));
    x.segments[0].direction = Direction::Both;
    p.stages = vec![Stage::Expand(x)];
    assert_eq!(
        paths(&run(&c, &p)),
        expected_paths(&[&["ab1"], &["ab2"], &["ca"]])
    );
}

#[test]
fn cycle_modes_distinguish_edge_and_vertex_reuse() {
    let c = graph();
    let mut p = scan(Kind::Node, "s", Some("a"));
    let mut x = expand(3, 4);
    p.stages = vec![Stage::Expand(x.clone())];
    assert_eq!(
        paths(&run(&c, &p)),
        expected_paths(&[
            &["ab1", "bc", "ca"],
            &["ab2", "bc", "ca"],
            &["ab1", "bc", "ca", "ab2"],
            &["ab2", "bc", "ca", "ab1"]
        ])
    );
    x.mode = PathMode::Simple;
    p.stages = vec![Stage::Expand(x.clone())];
    assert!(run(&c, &p).rows.is_empty());
    x.mode = PathMode::Walk;
    p.stages = vec![Stage::Expand(x)];
    assert_eq!(
        paths(&run(&c, &p)),
        expected_paths(&[
            &["ab1", "bc", "ca"],
            &["ab2", "bc", "ca"],
            &["ab1", "bc", "ca", "ab1"],
            &["ab1", "bc", "ca", "ab2"],
            &["ab2", "bc", "ca", "ab1"],
            &["ab2", "bc", "ca", "ab2"]
        ])
    );
}

#[test]
fn compound_optional_null_extends_once_and_uniqueness_spans_segments() {
    let c = graph();
    let mut p = scan(Kind::Node, "s", Some("a"));
    let mut x = expand(1, 1);
    x.optional = true;
    let mut second = segment("u", 1, 1);
    second.edge_alias = Some("f".into());
    second.node_predicate.id = Some("missing".into());
    x.segments.push(second);
    p.stages = vec![Stage::Expand(x.clone())];
    assert_eq!(
        run(&c, &p).rows,
        vec![row(&[
            ("s", binding(Kind::Node, "a")),
            ("t", Binding::Null),
            ("u", Binding::Null),
            ("e", Binding::Null),
            ("f", Binding::Null),
            ("p", Binding::Null)
        ])]
    );
    x.segments[1].node_predicate.id = Some("c".into());
    p.stages = vec![Stage::Expand(x.clone())];
    assert_eq!(
        paths(&run(&c, &p)),
        expected_paths(&[&["ab1", "bc"], &["ab2", "bc"]])
    );
    x.segments[1].node_predicate.id = Some("a".into());
    x.segments[1].direction = Direction::In;
    x.optional = false;
    p.stages = vec![Stage::Expand(x.clone())];
    assert_eq!(
        paths(&run(&c, &p)),
        expected_paths(&[&["ab1", "ab2"], &["ab2", "ab1"]])
    );
    x.mode = PathMode::Simple;
    p.stages = vec![Stage::Expand(x)];
    assert!(run(&c, &p).rows.is_empty());
}

#[test]
fn shortest_is_one_per_anchored_pair_with_edge_identity_tie_break() {
    let mut c = graph();
    c.revisions.reverse();
    let i = input(
        vec![
            row(&[
                ("s", binding(Kind::Node, "a")),
                ("t", binding(Kind::Node, "c"))
            ]);
            2
        ],
        &[
            ("s", BindingType::Entity(Kind::Node)),
            ("t", BindingType::Entity(Kind::Node)),
        ],
    );
    let mut x = expand(0, 4);
    x.shortest = true;
    let result = transform(
        &c,
        &view(&c),
        &i,
        &[Stage::Expand(x.clone())],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        paths(&result),
        expected_paths(&[&["ab1", "bc"], &["ab1", "bc"]])
    );
    let mut p = scan(Kind::Node, "s", Some("a"));
    p.stages = vec![Stage::Expand(x)];
    assert!(matches!(
        execute(&c, &view(&c), &p, Limits::default()),
        Err(Error::Alias(_))
    ));
}

#[test]
fn historical_traversal_enumerates_retired_edges_and_endpoints() {
    let mut c = graph();
    for r in &mut c.revisions {
        if r.entity.id == "b" || r.entity.id == "ab1" {
            r.transaction.end = Some(7);
        }
    }
    let mut p = scan(Kind::Node, "s", Some("a"));
    p.stages = vec![Stage::Expand(expand(1, 1))];
    assert!(run(&c, &p).rows.is_empty()); // ab2 is live, but b is retired.
    let mut v = view(&c);
    v.transaction = 6;
    assert_eq!(
        paths(&execute(&c, &v, &p, Limits::default()).unwrap()),
        expected_paths(&[&["ab1"], &["ab2"]])
    );
    v.transaction = 7;
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
    c.revisions
        .iter_mut()
        .find(|r| r.entity.id == "b")
        .unwrap()
        .transaction
        .end = None;
    c.revisions
        .iter_mut()
        .find(|r| r.entity.id == "b")
        .unwrap()
        .valid
        .end = Some(5);
    v.transaction = 6;
    v.valid_at = 5;
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
    v.valid_at = 4;
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows.len(),
        2
    );
}

#[test]
fn correction_is_immutable_and_splits_half_open_valid_intervals() {
    let mut old = record(Kind::Node, "a");
    old.valid.end = Some(100);
    old.fields.insert("value".into(), Scalar::Integer(1));
    let before = old.clone();
    let history = correct_revision(
        &old,
        7,
        Interval {
            start: 20,
            end: Some(40),
        },
        Fields::from([("value".into(), Scalar::Integer(2))]),
        ["left", "fix", "right"],
    )
    .unwrap();
    assert_eq!(old, before);
    assert_eq!(
        history
            .iter()
            .map(|r| (
                r.entity.revision.as_str(),
                r.valid.start,
                r.valid.end,
                r.transaction.start,
                r.transaction.end
            ))
            .collect::<Vec<_>>(),
        vec![
            ("r1", 0, Some(100), 1, Some(7)),
            ("left", 0, Some(20), 7, None),
            ("fix", 20, Some(40), 7, None),
            ("right", 40, Some(100), 7, None)
        ]
    );
    let c = catalog(history);
    let mut v = view(&c);
    let p = scan(Kind::Node, "s", None);
    for (s, t, revision) in [
        (6, 20, "r1"),
        (7, 19, "left"),
        (7, 20, "fix"),
        (7, 39, "fix"),
        (7, 40, "right"),
    ] {
        v.transaction = s;
        v.valid_at = t;
        assert_eq!(
            execute(&c, &v, &p, Limits::default()).unwrap().rows,
            vec![row(&[(
                "s",
                Binding::Entity(entity(Kind::Node, "a", revision))
            )])]
        );
    }
    v.valid_at = 100;
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
    assert_eq!(
        correct_revision(
            &old,
            7,
            Interval {
                start: 20,
                end: Some(20)
            },
            Fields::new(),
            ["l", "m", "r"]
        ),
        Err(Error::InvalidInterval)
    );
    assert_eq!(
        correct_revision(
            &old,
            1,
            Interval {
                start: 0,
                end: Some(100)
            },
            Fields::new(),
            ["l", "m", "r"]
        ),
        Err(Error::InvalidInterval)
    );
    let full = correct_revision(
        &old,
        7,
        Interval {
            start: 0,
            end: Some(100),
        },
        Fields::new(),
        ["l", "m", "r"],
    )
    .unwrap();
    assert_eq!(full.len(), 2);
}

#[test]
fn history_capabilities_checked_even_for_empty_inputs() {
    let mut c = graph();
    let mut v = view(&c);
    v.transaction = 4;
    let mut p = scan(Kind::Node, "s", Some("missing"));
    p.stages = vec![Stage::Expand(expand(0, 1))];
    c.history.get_mut(&Kind::Edge).unwrap().horizon = 5;
    assert_eq!(
        execute(&c, &v, &p, Limits::default()),
        Err(Error::BeyondHorizon(Kind::Edge))
    );
    c.history.get_mut(&Kind::Edge).unwrap().horizon = 1;
    c.history.get_mut(&Kind::Edge).unwrap().available = false;
    assert_eq!(
        execute(&c, &v, &p, Limits::default()),
        Err(Error::HistoryUnavailable(Kind::Edge))
    );
    v.transaction = 11;
    assert_eq!(
        execute(&c, &v, &p, Limits::default()),
        Err(Error::FutureSnapshot)
    );
}

#[test]
fn graph_annotation_composition_preserves_path_and_input_multiplicity() {
    let mut c = graph();
    c.revisions.push(annotation(
        "review",
        vec![live(Kind::Node, "b"), frozen(Kind::Node, "b", "r1")],
    ));
    let mut p = scan(Kind::Node, "s", Some("a"));
    p.stages = vec![Stage::Expand(expand(1, 1)), lookup(false)];
    let result = run(&c, &p);
    assert_eq!(paths(&result), expected_paths(&[&["ab1"], &["ab2"]]));
    assert!(result
        .rows
        .iter()
        .all(|r| r["a"] == binding(Kind::Annotation, "review")));
    let i = input(
        vec![row(&[("t", binding(Kind::Node, "b"))]); 2],
        &[("t", BindingType::Entity(Kind::Node))],
    );
    assert_eq!(
        transform(&c, &view(&c), &i, &[lookup(false)], Limits::default())
            .unwrap()
            .rows,
        vec![
            row(&[
                ("t", binding(Kind::Node, "b")),
                ("a", binding(Kind::Annotation, "review"))
            ]);
            2
        ]
    );
}

#[test]
fn frozen_revision_never_drifts_to_live_identity() {
    let mut old = record(Kind::Node, "b");
    old.transaction.end = Some(7);
    let mut new = record(Kind::Node, "b");
    new.entity.revision = "r2".into();
    new.transaction.start = 7;
    let c = catalog(vec![
        old,
        new,
        annotation("frozen", vec![frozen(Kind::Node, "b", "r1")]),
        annotation("live", vec![live(Kind::Node, "b")]),
    ]);
    let mut p = scan(Kind::Node, "t", None);
    p.stages = vec![lookup(false)];
    assert_eq!(
        run(&c, &p).rows,
        vec![row(&[
            ("t", Binding::Entity(entity(Kind::Node, "b", "r2"))),
            ("a", binding(Kind::Annotation, "live"))
        ])]
    );
    let mut v = view(&c);
    v.transaction = 6;
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        vec![
            row(&[
                ("t", binding(Kind::Node, "b")),
                ("a", binding(Kind::Annotation, "frozen"))
            ]),
            row(&[
                ("t", binding(Kind::Node, "b")),
                ("a", binding(Kind::Annotation, "live"))
            ])
        ]
    );
}

#[test]
fn annotation_optional_permission_intersection_and_no_status_bypass() {
    let mut c = graph();
    let mut a = annotation("review", vec![live(Kind::Node, "b")]);
    a.fields
        .insert("status".into(), Scalar::Text("approved".into()));
    c.revisions.push(a);
    let mut p = scan(Kind::Node, "t", Some("b"));
    p.stages = vec![lookup(true)];
    for remove_body in [false, true] {
        let mut v = view(&c);
        if remove_body {
            v.permissions.annotation_body.clear();
        } else {
            v.permissions
                .read
                .remove(&entity(Kind::Annotation, "review", "r1").identity());
        }
        assert_eq!(
            execute(&c, &v, &p, Limits::default()).unwrap().rows,
            vec![row(&[
                ("t", binding(Kind::Node, "b")),
                ("a", Binding::Null)
            ])]
        );
    }
    let mut v = view(&c);
    v.permissions
        .read
        .remove(&entity(Kind::Node, "b", "r1").identity());
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
    assert!(execute(
        &c,
        &v,
        &scan(Kind::Annotation, "a", None),
        Limits::default()
    )
    .unwrap()
    .rows
    .is_empty());
    let mut q = scan(Kind::Node, "s", Some("a"));
    q.stages = vec![Stage::Expand(expand(2, 2))];
    assert!(execute(&c, &v, &q, Limits::default())
        .unwrap()
        .rows
        .is_empty());
}

#[test]
fn unicode_scalar_offsets_and_source_metadata_are_validated() {
    let s = TextSource {
        text: "ก้😀Z".into(),
        extraction_id: "extract-1".into(),
        normalization_id: "fixture/raw-v1".into(),
    };
    let selector = Selector::TextPosition {
        start: 1,
        end: 3,
        source_hash: source_hash(&s.text),
        extraction_id: s.extraction_id.clone(),
        normalization_id: s.normalization_id.clone(),
    };
    assert_eq!(s.text.chars().skip(1).take(2).collect::<String>(), "้😀");
    assert_eq!(validate_selector(&s, &selector), Ok(()));
    for bad in [
        Selector::TextPosition {
            start: 0,
            end: 5,
            source_hash: source_hash(&s.text),
            extraction_id: s.extraction_id.clone(),
            normalization_id: s.normalization_id.clone(),
        },
        Selector::TextPosition {
            start: 3,
            end: 2,
            source_hash: source_hash(&s.text),
            extraction_id: s.extraction_id.clone(),
            normalization_id: s.normalization_id.clone(),
        },
        Selector::TextPosition {
            start: 0,
            end: 1,
            source_hash: "wrong".into(),
            extraction_id: s.extraction_id.clone(),
            normalization_id: s.normalization_id.clone(),
        },
        Selector::TextPosition {
            start: 0,
            end: 1,
            source_hash: source_hash(&s.text),
            extraction_id: "other".into(),
            normalization_id: s.normalization_id.clone(),
        },
        Selector::TextPosition {
            start: 0,
            end: 1,
            source_hash: source_hash(&s.text),
            extraction_id: s.extraction_id.clone(),
            normalization_id: "other".into(),
        },
    ] {
        assert!(validate_selector(&s, &bad).is_err());
    }
    let mut artifact = record(Kind::Artifact, "source");
    artifact.data = RecordData::Text(s);
    let mut target = frozen(Kind::Artifact, "source", "r1");
    target.selector = selector;
    let c = catalog(vec![artifact, annotation("span", vec![target])]);
    let mut p = scan(Kind::Artifact, "t", None);
    p.stages = vec![lookup(false)];
    assert_eq!(
        run(&c, &p).rows,
        vec![row(&[
            ("t", binding(Kind::Artifact, "source")),
            ("a", binding(Kind::Annotation, "span"))
        ])]
    );
}

#[test]
fn sha256_is_pinned_to_independent_known_answers() {
    assert_eq!(
        source_hash(""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        source_hash("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        source_hash("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn namespace_mismatch_and_duplicate_targets_fail_closed() {
    let mut c = graph();
    let mut bad = live(Kind::Node, "b");
    if let TargetBinding::Live(id) = &mut bad.binding {
        id.namespace = "other".into();
    }
    c.revisions.push(annotation("bad", vec![bad]));
    assert_eq!(
        execute(
            &c,
            &view(&c),
            &scan(Kind::Node, "s", None),
            Limits::default()
        ),
        Err(Error::NamespaceMismatch)
    );
    c.revisions.pop();
    c.revisions.push(annotation(
        "duplicate",
        vec![live(Kind::Node, "b"), live(Kind::Node, "b")],
    ));
    assert_eq!(
        execute(
            &c,
            &view(&c),
            &scan(Kind::Node, "s", None),
            Limits::default()
        ),
        Err(Error::InvalidCatalog)
    );
}

#[test]
fn invalid_bounds_aliases_and_resource_limits_reject_without_partial_output() {
    let c = graph();
    let mut p = scan(Kind::Node, "s", Some("missing"));
    for (min, max) in [(2, 1), (0, 9), (1, usize::MAX)] {
        p.stages = vec![Stage::Expand(expand(min, max))];
        assert_eq!(
            execute(&c, &view(&c), &p, Limits::default()),
            Err(Error::InvalidBounds)
        );
    }
    let mut x = expand(1, 1);
    x.start_alias = "unbound".into();
    p.stages = vec![Stage::Expand(x)];
    assert!(matches!(
        execute(&c, &view(&c), &p, Limits::default()),
        Err(Error::Alias(_))
    ));
    p = scan(Kind::Node, "s", None);
    assert_eq!(
        execute(
            &c,
            &view(&c),
            &p,
            Limits {
                max_work: 0,
                ..Limits::default()
            }
        ),
        Err(Error::BudgetExceeded)
    );
    assert_eq!(
        execute(
            &c,
            &view(&c),
            &p,
            Limits {
                max_rows: 1,
                ..Limits::default()
            }
        ),
        Err(Error::BudgetExceeded)
    );
}

#[test]
fn history_scan_enumerates_revisions_not_only_current_candidates() {
    let mut old = record(Kind::Node, "x");
    old.transaction.end = Some(6);
    let mut new = record(Kind::Node, "x");
    new.entity.revision = "r2".into();
    new.transaction.start = 6;
    let mut dead = record(Kind::Node, "y");
    dead.retracted = true;
    dead.transaction.start = 8;
    let c: Fixture = catalog(vec![dead, new, old]);
    let mut v: Context = view(&c);
    let p = Plan {
        source: Source::HistoryScan {
            kind: Kind::Node,
            alias: "h".into(),
            predicate: Predicate::default(),
            transactions: Interval {
                start: 1,
                end: Some(9),
            },
            valid: Interval {
                start: 0,
                end: Some(10),
            },
        },
        stages: vec![],
    };
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        vec![
            row(&[("h", binding(Kind::Node, "x"))]),
            row(&[("h", Binding::Entity(entity(Kind::Node, "x", "r2")))]),
            row(&[("h", binding(Kind::Node, "y"))])
        ]
    );
    v.transaction = 5;
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        vec![row(&[("h", binding(Kind::Node, "x"))])]
    );
    let mut c = c;
    c.history.get_mut(&Kind::Node).unwrap().horizon = 2;
    assert_eq!(
        execute(&c, &v, &p, Limits::default()),
        Err(Error::BeyondHorizon(Kind::Node))
    );
}

#[test]
fn change_scan_uses_explicit_events_order_window_and_subject_permissions() {
    let mut subject = record(Kind::Node, "x");
    subject.transaction.end = Some(6);
    let mut events = Vec::new();
    for (id, seq, operation) in [
        ("z", 2, ChangeKind::Upsert),
        ("a", 6, ChangeKind::Retract),
        ("b", 6, ChangeKind::Correct),
    ] {
        let mut event = record(Kind::Event, id);
        event.transaction.start = seq;
        event.data = RecordData::Change {
            subject: subject.entity.clone(),
            operation,
        };
        events.push(event);
    }
    events.push(subject);
    let c = catalog(events);
    let mut v = view(&c);
    let mut p = Plan {
        source: Source::ChangeScan {
            alias: "event".into(),
            predicate: Predicate::default(),
            after: 2,
            through: 6,
        },
        stages: vec![],
    };
    assert_eq!(
        run(&c, &p).rows,
        vec![
            row(&[("event", binding(Kind::Event, "a"))]),
            row(&[("event", binding(Kind::Event, "b"))])
        ]
    );
    v.transaction = 5;
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
    v.transaction = 10;
    v.permissions
        .read
        .remove(&entity(Kind::Node, "x", "r1").identity());
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
    p.source = Source::ChangeScan {
        alias: "event".into(),
        predicate: Predicate::default(),
        after: 7,
        through: 6,
    };
    assert_eq!(
        execute(&c, &v, &p, Limits::default()),
        Err(Error::InvalidBounds)
    );
}

#[test]
fn edge_and_annotation_scans_are_typed_composable_sources() {
    let mut c = graph();
    c.revisions.push(annotation(
        "edge-note",
        vec![frozen(Kind::Edge, "ab1", "r1")],
    ));
    let mut p = scan(Kind::Edge, "t", Some("ab1"));
    p.stages = vec![lookup(false)];
    assert_eq!(
        run(&c, &p).rows,
        vec![row(&[
            ("t", binding(Kind::Edge, "ab1")),
            ("a", binding(Kind::Annotation, "edge-note"))
        ])]
    );
    assert_eq!(
        run(&c, &scan(Kind::Annotation, "a", None)).rows,
        vec![row(&[("a", binding(Kind::Annotation, "edge-note"))])]
    );
    let r = run(&c, &scan(Kind::Edge, "t", Some("ab1")));
    assert_eq!(
        transform(&c, &view(&c), &r, &p.stages, Limits::default()).unwrap(),
        run(&c, &p)
    );
}

#[test]
fn distinct_spans_on_same_frozen_revision_yield_one_annotation_per_input() {
    let text = TextSource {
        text: "ก้😀Z".into(),
        extraction_id: "extract".into(),
        normalization_id: "raw-v1".into(),
    };
    let mut artifact = record(Kind::Artifact, "source");
    artifact.data = RecordData::Text(text.clone());
    let targets = [(0, 1), (1, 3)]
        .into_iter()
        .map(|(start, end)| Target {
            binding: TargetBinding::Frozen(artifact.entity.clone()),
            selector: Selector::TextPosition {
                start,
                end,
                source_hash: source_hash(&text.text),
                extraction_id: text.extraction_id.clone(),
                normalization_id: text.normalization_id.clone(),
            },
        })
        .collect();
    let c = catalog(vec![artifact, annotation("spans", targets)]);
    let mut p = scan(Kind::Artifact, "t", None);
    p.stages = vec![lookup(false)];
    assert_eq!(
        run(&c, &p).rows,
        vec![row(&[
            ("t", binding(Kind::Artifact, "source")),
            ("a", binding(Kind::Annotation, "spans"))
        ])]
    );
}

#[test]
fn empty_transform_validates_schema_bounds_collisions_and_returns_output_schema() {
    let c = graph();
    let i = input(vec![], &[("s", BindingType::Entity(Kind::Node))]);
    let result = transform(
        &c,
        &view(&c),
        &i,
        &[Stage::Expand(expand(1, 2)), lookup(true)],
        Limits::default(),
    )
    .unwrap();
    assert!(result.rows.is_empty());
    assert_eq!(
        result.schema,
        BTreeMap::from([
            ("s".into(), BindingType::Entity(Kind::Node)),
            ("t".into(), BindingType::Entity(Kind::Node)),
            ("e".into(), BindingType::Edges),
            ("p".into(), BindingType::Path),
            ("a".into(), BindingType::Entity(Kind::Annotation))
        ])
    );
    for alias in ["s", "t", "e"] {
        let mut x = expand(1, 1);
        x.path_alias = Some(alias.into());
        assert!(matches!(
            transform(&c, &view(&c), &i, &[Stage::Expand(x)], Limits::default()),
            Err(Error::Alias(_))
        ));
    }
    assert_eq!(
        transform(
            &c,
            &view(&c),
            &i,
            &[Stage::Expand(expand(2, 1))],
            Limits::default()
        ),
        Err(Error::InvalidBounds)
    );
    let wrong = input(vec![], &[("s", BindingType::Entity(Kind::Edge))]);
    assert_eq!(
        transform(
            &c,
            &view(&c),
            &wrong,
            &[Stage::Expand(expand(1, 1))],
            Limits::default()
        ),
        Err(Error::Type("s".into()))
    );
}

#[test]
fn input_bags_preserve_order_and_optional_null_anchors_remain_null() {
    let c = graph();
    let i = input(
        vec![
            row(&[("s", binding(Kind::Node, "b"))]),
            row(&[("s", binding(Kind::Node, "a"))]),
            row(&[("s", binding(Kind::Node, "a"))]),
        ],
        &[("s", BindingType::Entity(Kind::Node))],
    );
    let result = transform(
        &c,
        &view(&c),
        &i,
        &[Stage::Expand(expand(1, 1))],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        paths(&result),
        expected_paths(&[&["bc"], &["ab1"], &["ab2"], &["ab1"], &["ab2"]])
    );
    let i = input(
        vec![row(&[("s", Binding::Null)])],
        &[("s", BindingType::Entity(Kind::Node))],
    );
    let mut x = expand(0, 1);
    x.optional = true;
    assert_eq!(
        transform(&c, &view(&c), &i, &[Stage::Expand(x)], Limits::default())
            .unwrap()
            .rows,
        vec![row(&[
            ("s", Binding::Null),
            ("t", Binding::Null),
            ("e", Binding::Null),
            ("p", Binding::Null)
        ])]
    );
}

#[test]
fn zero_hop_shortest_preserves_anchored_end_and_optional_failure_preserves_bindings() {
    let c = graph();
    let mut x = expand(0, 3);
    x.shortest = true;
    x.optional = true;
    let i = input(
        vec![row(&[
            ("s", binding(Kind::Node, "a")),
            ("t", binding(Kind::Node, "a")),
        ])],
        &[
            ("s", BindingType::Entity(Kind::Node)),
            ("t", BindingType::Entity(Kind::Node)),
        ],
    );
    let result = transform(
        &c,
        &view(&c),
        &i,
        &[Stage::Expand(x.clone())],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(paths(&result), expected_paths(&[&[]]));
    x.segments[0].min_hops = 1;
    x.segments[0].max_hops = 1;
    assert_eq!(
        transform(&c, &view(&c), &i, &[Stage::Expand(x)], Limits::default())
            .unwrap()
            .rows,
        vec![row(&[
            ("s", binding(Kind::Node, "a")),
            ("t", binding(Kind::Node, "a")),
            ("e", Binding::Null),
            ("p", Binding::Null)
        ])]
    );
}

#[test]
fn predicates_and_relation_constraints_are_inside_the_optional_pattern() {
    let mut c = graph();
    c.revisions
        .iter_mut()
        .find(|r| r.entity.id == "b")
        .unwrap()
        .fields
        .insert("enabled".into(), Scalar::Boolean(true));
    c.revisions
        .iter_mut()
        .find(|r| r.entity.id == "ab2")
        .unwrap()
        .fields
        .insert("rank".into(), Scalar::Integer(7));
    let mut x = expand(1, 1);
    x.optional = true;
    x.segments[0].relations.insert("LINK".into());
    x.segments[0]
        .node_predicate
        .equals
        .insert("enabled".into(), Scalar::Boolean(true));
    x.segments[0]
        .edge_predicate
        .equals
        .insert("rank".into(), Scalar::Integer(7));
    let mut p = scan(Kind::Node, "s", Some("a"));
    p.stages = vec![Stage::Expand(x.clone())];
    assert_eq!(paths(&run(&c, &p)), expected_paths(&[&["ab2"]]));
    x.segments[0].relations = BTreeSet::from(["link".into()]);
    p.stages = vec![Stage::Expand(x)];
    assert_eq!(
        run(&c, &p).rows,
        vec![row(&[
            ("s", binding(Kind::Node, "a")),
            ("t", Binding::Null),
            ("e", Binding::Null),
            ("p", Binding::Null)
        ])]
    );
}

#[test]
fn live_span_drift_and_missing_frozen_revision_fail_without_retargeting() {
    let old_text = TextSource {
        text: "old".into(),
        extraction_id: "extract".into(),
        normalization_id: "raw-v1".into(),
    };
    let mut old = record(Kind::Artifact, "source");
    old.data = RecordData::Text(old_text.clone());
    old.transaction.end = Some(7);
    let mut new = old.clone();
    new.entity.revision = "r2".into();
    new.transaction = Interval {
        start: 7,
        end: None,
    };
    new.data = RecordData::Text(TextSource {
        text: "new".into(),
        ..old_text.clone()
    });
    let mut target = live(Kind::Artifact, "source");
    target.selector = Selector::TextPosition {
        start: 0,
        end: 3,
        source_hash: source_hash(&old_text.text),
        extraction_id: old_text.extraction_id,
        normalization_id: old_text.normalization_id,
    };
    let mut c = catalog(vec![old, new, annotation("span", vec![target])]);
    let mut p = scan(Kind::Artifact, "t", None);
    p.stages = vec![lookup(false)];
    assert_eq!(
        execute(&c, &view(&c), &p, Limits::default()),
        Err(Error::InvalidSource)
    );
    let mut v = view(&c);
    v.transaction = 6;
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows.len(),
        1
    );
    c.revisions.pop();
    c.revisions.push(annotation(
        "missing",
        vec![frozen(Kind::Artifact, "source", "missing")],
    ));
    assert_eq!(
        execute(&c, &view(&c), &p, Limits::default()),
        Err(Error::MissingRevision)
    );
}

#[test]
fn invalid_intervals_overlap_and_imported_reference_namespace_are_rejected() {
    let mut c = graph();
    let mut overlap = record(Kind::Node, "a");
    overlap.entity.revision = "duplicate-window".into();
    c.revisions.push(overlap);
    assert_eq!(
        execute(
            &c,
            &view(&c),
            &scan(Kind::Node, "s", None),
            Limits::default()
        ),
        Err(Error::InvalidCatalog)
    );
    c.revisions.pop();
    c.revisions[0].valid.end = Some(0);
    assert_eq!(
        execute(
            &c,
            &view(&c),
            &scan(Kind::Node, "s", None),
            Limits::default()
        ),
        Err(Error::InvalidInterval)
    );
    let c = graph();
    let mut other = entity(Kind::Node, "a", "r1");
    other.namespace = "other".into();
    let i = input(
        vec![row(&[("s", Binding::Entity(other))])],
        &[("s", BindingType::Entity(Kind::Node))],
    );
    assert_eq!(
        transform(&c, &view(&c), &i, &[], Limits::default()),
        Err(Error::NamespaceMismatch)
    );
    let i = input(vec![Row::new()], &[("s", BindingType::Entity(Kind::Node))]);
    assert_eq!(
        transform(&c, &view(&c), &i, &[], Limits::default()),
        Err(Error::InvalidSource)
    );
}

#[test]
fn current_permissions_filter_historical_sources_and_tombstones_hide_current_nodes() {
    let mut old = record(Kind::Node, "a");
    old.transaction.end = Some(7);
    let mut tombstone = record(Kind::Node, "a");
    tombstone.entity.revision = "deleted".into();
    tombstone.transaction.start = 7;
    tombstone.retracted = true;
    let c = catalog(vec![old, tombstone]);
    let mut v = view(&c);
    assert!(run(&c, &scan(Kind::Node, "s", None)).rows.is_empty());
    v.transaction = 6;
    assert_eq!(
        execute(&c, &v, &scan(Kind::Node, "s", None), Limits::default())
            .unwrap()
            .rows
            .len(),
        1
    );
    v.permissions.read.clear();
    let p = Plan {
        source: Source::HistoryScan {
            kind: Kind::Node,
            alias: "h".into(),
            predicate: Predicate::default(),
            transactions: Interval {
                start: 1,
                end: None,
            },
            valid: Interval {
                start: 0,
                end: None,
            },
        },
        stages: vec![],
    };
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
}

// Independent baseline: enumerate the full Cartesian product of oriented edges
// at each length, THEN check continuity and uniqueness. No reference helpers,
// adjacency traversal, pruning, visibility, or ordering functions are reused.
fn baseline(
    edges: &[(usize, usize, usize)],
    start: usize,
    direction: Direction,
    mode: PathMode,
) -> Vec<Vec<String>> {
    let mut oriented = Vec::new();
    for &(id, a, b) in edges {
        match direction {
            Direction::Out => oriented.push((id, a, b)),
            Direction::In => oriented.push((id, b, a)),
            Direction::Both => {
                oriented.push((id, a, b));
                if a != b {
                    oriented.push((id, b, a));
                }
            }
        }
    }
    let mut answer = vec![vec![]];
    for length in 1..=3u32 {
        for mut code in 0..oriented.len().pow(length) {
            let mut tuple = Vec::new();
            for _ in 0..length {
                tuple.push(oriented[code % oriented.len()]);
                code /= oriented.len();
            }
            let mut at = start;
            let mut nodes = vec![start];
            let mut used = Vec::new();
            let mut ok = true;
            for &(id, a, b) in &tuple {
                if a != at
                    || (mode == PathMode::Trail && used.contains(&id))
                    || (mode == PathMode::Simple && nodes.contains(&b))
                {
                    ok = false;
                    break;
                }
                at = b;
                nodes.push(b);
                used.push(id);
            }
            if ok {
                answer.push(used.iter().map(|id| format!("e{id}")).collect());
            }
        }
    }
    answer.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));
    answer
}

#[test]
fn annotation_edge_targets_require_endpoint_permissions_recursively() {
    let mut c = graph();
    c.revisions.push(annotation(
        "edge-frozen",
        vec![frozen(Kind::Edge, "ab1", "r1")],
    ));
    c.revisions
        .push(annotation("edge-live", vec![live(Kind::Edge, "ab1")]));
    let mut v = view(&c);
    v.permissions
        .read
        .remove(&entity(Kind::Node, "b", "r1").identity());
    assert!(execute(
        &c,
        &v,
        &scan(Kind::Annotation, "note", None),
        Limits::default()
    )
    .unwrap()
    .rows
    .is_empty());
    v.permissions
        .read
        .insert(entity(Kind::Node, "b", "r1").identity());
    assert_eq!(
        execute(
            &c,
            &v,
            &scan(Kind::Annotation, "note", None),
            Limits::default()
        )
        .unwrap()
        .rows,
        vec![
            row(&[("note", binding(Kind::Annotation, "edge-frozen"))]),
            row(&[("note", binding(Kind::Annotation, "edge-live"))])
        ]
    );
}

#[test]
fn change_subject_edges_require_endpoint_permissions_recursively() {
    let mut c = graph();
    let mut event = record(Kind::Event, "edge-change");
    event.data = RecordData::Change {
        subject: entity(Kind::Edge, "ab1", "r1"),
        operation: ChangeKind::Upsert,
    };
    event.transaction.start = 2;
    c.revisions.push(event);
    let p = Plan {
        source: Source::ChangeScan {
            alias: "event".into(),
            predicate: Predicate::default(),
            after: 1,
            through: 10,
        },
        stages: vec![],
    };
    let mut v = view(&c);
    v.permissions
        .read
        .remove(&entity(Kind::Node, "b", "r1").identity());
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
    v.permissions
        .read
        .insert(entity(Kind::Node, "b", "r1").identity());
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        vec![row(&[("event", binding(Kind::Event, "edge-change"))])]
    );
}

#[test]
fn nested_annotation_and_event_subjects_cannot_bypass_denied_leaf_scope() {
    let mut c = graph();
    c.revisions
        .push(annotation("inner", vec![live(Kind::Node, "b")]));
    c.revisions
        .push(annotation("outer", vec![live(Kind::Annotation, "inner")]));
    let mut event = record(Kind::Event, "nested-change");
    event.data = RecordData::Change {
        subject: entity(Kind::Annotation, "outer", "r1"),
        operation: ChangeKind::Correct,
    };
    c.revisions.push(event);
    let mut v = view(&c);
    v.permissions
        .read
        .remove(&entity(Kind::Node, "b", "r1").identity());
    let annotations = scan(Kind::Annotation, "note", None);
    assert!(execute(&c, &v, &annotations, Limits::default())
        .unwrap()
        .rows
        .is_empty());
    assert!(
        execute(&c, &v, &scan(Kind::Event, "event", None), Limits::default())
            .unwrap()
            .rows
            .is_empty()
    );
    let i = input(
        vec![row(&[("t", binding(Kind::Annotation, "inner"))])],
        &[("t", BindingType::Entity(Kind::Annotation))],
    );
    assert!(transform(&c, &v, &i, &[lookup(false)], Limits::default())
        .unwrap()
        .rows
        .is_empty());
    v.permissions
        .read
        .insert(entity(Kind::Node, "b", "r1").identity());
    assert_eq!(
        execute(&c, &v, &annotations, Limits::default())
            .unwrap()
            .rows
            .len(),
        2
    );
    assert_eq!(
        execute(&c, &v, &scan(Kind::Event, "event", None), Limits::default())
            .unwrap()
            .rows
            .len(),
        1
    );
    v.permissions
        .annotation_body
        .remove(&entity(Kind::Annotation, "inner", "r1").identity());
    assert!(execute(&c, &v, &annotations, Limits::default())
        .unwrap()
        .rows
        .is_empty());
}

#[test]
fn annotation_authorization_cycles_need_an_independently_authorized_target() {
    let mut c = catalog(vec![
        record(Kind::Node, "root"),
        annotation("one", vec![live(Kind::Annotation, "two")]),
        annotation("two", vec![live(Kind::Annotation, "one")]),
    ]);
    let p = scan(Kind::Annotation, "note", None);
    assert!(
        run(&c, &p).rows.is_empty(),
        "a cycle alone cannot establish target authorization"
    );
    let RecordData::Annotation { targets } = &mut c.revisions[2].data else {
        unreachable!()
    };
    targets.push(live(Kind::Node, "root"));
    assert_eq!(
        run(&c, &p).rows,
        vec![
            row(&[("note", binding(Kind::Annotation, "one"))]),
            row(&[("note", binding(Kind::Annotation, "two"))])
        ]
    );
    let mut v = view(&c);
    v.permissions
        .read
        .remove(&entity(Kind::Node, "root", "r1").identity());
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
}

#[test]
fn recursive_authorization_depth_has_a_bounded_failure() {
    let mut revisions = vec![record(Kind::Node, "root")];
    for index in 0..130 {
        let target = if index == 129 {
            live(Kind::Node, "root")
        } else {
            live(Kind::Annotation, &format!("note-{}", index + 1))
        };
        revisions.push(annotation(&format!("note-{index}"), vec![target]));
    }
    let c = catalog(revisions);
    assert_eq!(
        execute(
            &c,
            &view(&c),
            &scan(Kind::Annotation, "note", Some("note-0")),
            Limits {
                max_work: 1_000_000,
                ..Limits::default()
            }
        ),
        Err(Error::BudgetExceeded)
    );
}

#[test]
fn future_denied_and_retired_annotations_do_not_add_scan_history_dependencies() {
    let mut c = catalog(vec![
        record(Kind::Node, "node"),
        record(Kind::Artifact, "artifact"),
        annotation("visible", vec![live(Kind::Node, "node")]),
        annotation("other", vec![frozen(Kind::Artifact, "artifact", "r1")]),
    ]);
    c.history.get_mut(&Kind::Artifact).unwrap().available = false;
    c.revisions[3].transaction.start = 9;
    let mut v = view(&c);
    v.transaction = 5;
    let p = scan(Kind::Annotation, "note", None);
    let expected = vec![row(&[("note", binding(Kind::Annotation, "visible"))])];
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        expected
    );
    c.revisions[3].transaction.start = 1;
    v.permissions
        .read
        .remove(&entity(Kind::Annotation, "other", "r1").identity());
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        expected
    );
    v.permissions
        .read
        .insert(entity(Kind::Annotation, "other", "r1").identity());
    v.permissions
        .annotation_body
        .remove(&entity(Kind::Annotation, "other", "r1").identity());
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        expected
    );
    v.permissions
        .annotation_body
        .insert(entity(Kind::Annotation, "other", "r1").identity());
    v.permissions
        .read
        .remove(&entity(Kind::Artifact, "artifact", "r1").identity());
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        expected
    );
    v.permissions
        .read
        .insert(entity(Kind::Artifact, "artifact", "r1").identity());
    assert_eq!(
        execute(&c, &v, &p, Limits::default()),
        Err(Error::HistoryUnavailable(Kind::Artifact))
    );
    c.revisions[3].transaction.end = Some(4);
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        expected
    );
    let history = Plan {
        source: Source::HistoryScan {
            kind: Kind::Annotation,
            alias: "note".into(),
            predicate: Predicate::default(),
            transactions: Interval {
                start: 1,
                end: Some(6),
            },
            valid: Interval {
                start: 0,
                end: None,
            },
        },
        stages: vec![],
    };
    assert_eq!(
        execute(&c, &v, &history, Limits::default()),
        Err(Error::HistoryUnavailable(Kind::Artifact))
    );
}

#[test]
fn future_denied_and_out_of_window_events_do_not_add_change_history_dependencies() {
    let mut c = catalog(vec![
        record(Kind::Node, "node"),
        record(Kind::Artifact, "artifact"),
    ]);
    for (id, kind, subject, at) in [
        ("visible", Kind::Node, "node", 2),
        ("other", Kind::Artifact, "artifact", 9),
    ] {
        let mut event = record(Kind::Event, id);
        event.transaction.start = at;
        event.data = RecordData::Change {
            subject: entity(kind, subject, "r1"),
            operation: ChangeKind::Upsert,
        };
        c.revisions.push(event);
    }
    c.history.get_mut(&Kind::Artifact).unwrap().available = false;
    let mut v = view(&c);
    v.transaction = 5;
    let p = Plan {
        source: Source::ChangeScan {
            alias: "event".into(),
            predicate: Predicate::default(),
            after: 1,
            through: 5,
        },
        stages: vec![],
    };
    let expected = vec![row(&[("event", binding(Kind::Event, "visible"))])];
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        expected
    );
    c.revisions[3].transaction.start = 3;
    v.permissions
        .read
        .remove(&entity(Kind::Event, "other", "r1").identity());
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        expected
    );
    v.permissions
        .read
        .insert(entity(Kind::Event, "other", "r1").identity());
    assert_eq!(
        execute(&c, &v, &p, Limits::default()),
        Err(Error::HistoryUnavailable(Kind::Artifact))
    );
    c.revisions[3].transaction.start = 1; // Excluded by the explicit change cursor.
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        expected
    );
}

#[test]
fn imported_paths_require_connected_edges_in_either_orientation() {
    let c = graph();
    let make = |vertices: &[&str]| {
        input(
            vec![row(&[(
                "p",
                Binding::Path(Path {
                    vertices: vertices
                        .iter()
                        .map(|id| entity(Kind::Node, id, "r1"))
                        .collect(),
                    edges: vec![entity(Kind::Edge, "ab1", "r1")],
                }),
            )])],
            &[("p", BindingType::Path)],
        )
    };
    let forged = make(&["a", "c"]);
    assert_eq!(
        transform(&c, &view(&c), &forged, &[], Limits::default()),
        Err(Error::InvalidSource)
    );
    for vertices in [["a", "b"], ["b", "a"]] {
        let valid = make(&vertices);
        assert_eq!(
            transform(&c, &view(&c), &valid, &[], Limits::default()).unwrap(),
            valid
        );
    }
    let mut cross_namespace = c.clone();
    let edge = cross_namespace
        .revisions
        .iter_mut()
        .find(|r| r.entity.id == "ab1")
        .unwrap();
    let RecordData::Edge { target, .. } = &mut edge.data else {
        unreachable!()
    };
    target.namespace = "foreign".into();
    assert_eq!(
        transform(
            &cross_namespace,
            &view(&cross_namespace),
            &make(&["a", "b"]),
            &[],
            Limits::default()
        ),
        Err(Error::NamespaceMismatch)
    );
}

#[test]
fn metered_execute_reports_work_and_preserves_results_at_exact_budget() {
    let c = graph();
    let v = view(&c);
    let mut p = scan(Kind::Node, "s", Some("a"));
    p.stages = vec![Stage::Expand(expand(0, 2))];
    let (relation, used) = execute_metered(&c, &v, &p, Limits::default()).unwrap();
    assert_eq!(relation, execute(&c, &v, &p, Limits::default()).unwrap());
    assert!(used > 0, "successful execution must expose runtime work");
    assert_eq!(
        execute_metered(
            &c,
            &v,
            &p,
            Limits {
                max_work: used,
                ..Limits::default()
            }
        )
        .unwrap(),
        (relation, used)
    );
    assert_eq!(
        execute_metered(
            &c,
            &v,
            &p,
            Limits {
                max_work: used - 1,
                ..Limits::default()
            }
        ),
        Err(Error::BudgetExceeded)
    );
    eprintln!("metered execute consumed {used} work units");
}

#[test]
fn metered_transform_exhausts_shared_remaining_work_across_individually_fitting_calls() {
    let c = graph();
    let v = view(&c);
    let i = input(
        vec![row(&[("s", binding(Kind::Node, "a"))])],
        &[("s", BindingType::Entity(Kind::Node))],
    );
    let stages = [Stage::Expand(expand(1, 1))];
    let (expected, used) = transform_metered(&c, &v, &i, &stages, Limits::default()).unwrap();
    assert_eq!(
        expected,
        transform(&c, &v, &i, &stages, Limits::default()).unwrap()
    );
    assert!(used > 1);
    let total = used.checked_mul(2).unwrap() - 1;
    for _ in 0..2 {
        assert_eq!(
            transform_metered(
                &c,
                &v,
                &i,
                &stages,
                Limits {
                    max_work: total,
                    ..Limits::default()
                }
            )
            .unwrap(),
            (expected.clone(), used)
        );
    }
    let mut remaining = total;
    let (first, consumed) = transform_metered(
        &c,
        &v,
        &i,
        &stages,
        Limits {
            max_work: remaining,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(first, expected);
    remaining = remaining.checked_sub(consumed).unwrap();
    assert_eq!(remaining, used - 1);
    assert_eq!(
        transform_metered(
            &c,
            &v,
            &i,
            &stages,
            Limits {
                max_work: remaining,
                ..Limits::default()
            }
        ),
        Err(Error::BudgetExceeded)
    );
    eprintln!("metered transform consumed {used}; shared quota {total}; second call refused with {remaining} remaining");
}

#[test]
fn metered_empty_stage_and_empty_input_calls_charge_validation_work() {
    let c = graph();
    let v = view(&c);
    let i = input(vec![], &[("s", BindingType::Entity(Kind::Node))]);
    let (empty, preflight_work) = transform_metered(&c, &v, &i, &[], Limits::default()).unwrap();
    assert_eq!(empty, i);
    assert!(
        preflight_work > 0,
        "empty calls still validate the catalog and capabilities"
    );
    let mut populated = i.clone();
    populated.rows.push(row(&[("s", binding(Kind::Node, "a"))]));
    let (identity, populated_work) =
        transform_metered(&c, &v, &populated, &[], Limits::default()).unwrap();
    assert_eq!(identity, populated);
    assert!(populated_work > preflight_work);
    let total = preflight_work.checked_add(populated_work).unwrap() - 1;
    let (_, used) = transform_metered(
        &c,
        &v,
        &i,
        &[],
        Limits {
            max_work: total,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        transform_metered(
            &c,
            &v,
            &populated,
            &[],
            Limits {
                max_work: total - used,
                ..Limits::default()
            }
        ),
        Err(Error::BudgetExceeded)
    );
    assert_eq!(
        transform_metered(
            &c,
            &v,
            &i,
            &[],
            Limits {
                max_work: 0,
                ..Limits::default()
            }
        ),
        Err(Error::BudgetExceeded)
    );
    eprintln!("empty-input validation consumed {preflight_work}; populated identity transform consumed {populated_work}");
}

#[test]
fn exhaustive_tiny_graphs_match_separate_cartesian_baseline() {
    // All 32 subsets of two-node directed multigraph slots, including loops,
    // parallel edges, cycles and empty graphs: 32*2*3*3 = 576 comparisons.
    let slots = [(0, 0, 0), (1, 0, 1), (2, 0, 1), (3, 1, 0), (4, 1, 1)];
    for mask in 0..32 {
        let selected: Vec<_> = slots
            .iter()
            .copied()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, e)| e)
            .collect();
        let mut records = vec![record(Kind::Node, "0"), record(Kind::Node, "1")];
        for &(id, a, b) in &selected {
            records.push(edge(&format!("e{id}"), &a.to_string(), &b.to_string()));
        }
        let c = catalog(records);
        for start in 0..2 {
            for direction in [Direction::Out, Direction::In, Direction::Both] {
                for mode in [PathMode::Trail, PathMode::Simple, PathMode::Walk] {
                    let mut p = scan(Kind::Node, "s", Some(&start.to_string()));
                    let mut x = expand(0, 3);
                    x.segments[0].direction = direction;
                    x.mode = mode;
                    p.stages = vec![Stage::Expand(x)];
                    assert_eq!(
                        paths(&run(&c, &p)),
                        baseline(&selected, start, direction, mode),
                        "mask={mask} start={start} {direction:?} {mode:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn vector_history_uses_h2_d11_identity_and_owner_read_permission() {
    let owner = entity(Kind::Node, "owner:one", "owner-r1").identity();
    let vector_id = r#"["owner:one","default"]"#;
    let mut first = record(Kind::Vector, vector_id);
    first.entity.revision = "vector-r1".into();
    first.transaction = Interval {
        start: 2,
        end: Some(4),
    };
    first.data = RecordData::Vector {
        owner: owner.clone(),
    };
    let mut second = record(Kind::Vector, vector_id);
    second.entity.revision = "vector-r2".into();
    second.transaction.start = 4;
    second.data = RecordData::Vector {
        owner: owner.clone(),
    };

    let c = catalog(vec![record(Kind::Node, "owner:one"), first, second]);
    let mut v = view(&c);
    let p = Plan {
        source: Source::HistoryScan {
            kind: Kind::Vector,
            alias: "h".into(),
            predicate: Predicate {
                id: Some(vector_id.into()),
                ..Predicate::default()
            },
            transactions: Interval {
                start: 1,
                end: Some(10),
            },
            valid: Interval {
                start: 5,
                end: Some(6),
            },
        },
        stages: vec![],
    };
    assert_eq!(
        execute(&c, &v, &p, Limits::default()).unwrap().rows,
        vec![
            row(&[(
                "h",
                Binding::Entity(entity(Kind::Vector, vector_id, "vector-r1")),
            )]),
            row(&[(
                "h",
                Binding::Entity(entity(Kind::Vector, vector_id, "vector-r2")),
            )]),
        ]
    );

    v.permissions.read.remove(&owner);
    assert!(execute(&c, &v, &p, Limits::default())
        .unwrap()
        .rows
        .is_empty());
}
