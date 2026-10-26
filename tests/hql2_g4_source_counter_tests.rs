use genesis_block_native::{
    query::hql2::{result::QueryResultV2, value::QueryValueV2, QueryOutcomeV2},
    uee_v2::{AnnotationPutMutationV2, ExplainV2, QueryRequestV2},
    AccessAction, AccessContext, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource,
    EdgeInput, GenerationInfo, NodeInput, OpenOptions, PolicyAdminActor, RelationalColumn,
    RelationalColumnType, RelationalMutationBatch, RelationalMutationKind, RelationalRowMutation,
    RelationalSchemaPackage, RelationalTable, Storage,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
use tempfile::TempDir;
use uuid::Uuid;

#[allow(dead_code)]
#[path = "support/hql2_diagnostics.rs"]
mod diagnostics;
// This target uses only SourceScan pieces of the shared reference oracle.
#[allow(dead_code)]
#[path = "support/hql2_graph_reference.rs"]
mod p7_graph;

fn actor(principal: &str) -> AccessContext {
    AccessContext {
        principal: principal.into(),
        namespace: "default".into(),
    }
}

fn request(op: &str, config: Value) -> QueryRequestV2 {
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2", "request_id":"source-diagnostics",
        "namespace":"default", "params":{},
        "ir":{"contract_version":"query-ir.v2", "nodes":[
            {"id":"scan","op":op,"inputs":[],"config":config}
        ],"root":"scan","parameter_types":{}}
    }))
    .unwrap()
}

fn fixture(populated: bool) -> (TempDir, Storage) {
    let dir = TempDir::new().unwrap();
    let db = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap();
    db.register_relational_schema(RelationalSchemaPackage {
        namespace: "default".into(),
        schema_version: 1,
        previous_version: None,
        package_id: Uuid::new_v4().to_string(),
        schema_hash: String::new(),
        tables: vec![RelationalTable {
            name: "records".into(),
            columns: vec![RelationalColumn::required("id", RelationalColumnType::Text)],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        named_queries: vec![],
    })
    .unwrap();
    if populated {
        for id in ["one", "two"] {
            db.add_node(NodeInput {
                id: Some(id.into()),
                labels: vec!["Document".into()],
                props: Some(json!({"name":id})),
                embedding: None,
                lang: None,
                valid_from: Some("2026-09-22T00:00:00Z".into()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
        }
        db.add_edge(EdgeInput {
            id: Some("edge:one".into()),
            from: "one".into(),
            to: "two".into(),
            rel: "LINK".into(),
            props: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
        db.apply_relational_batch(RelationalMutationBatch {
            mutation_id: Uuid::new_v4().to_string(),
            namespace: "default".into(),
            schema_version: 1,
            operations: ["row:one", "row:two"]
                .into_iter()
                .map(|id| RelationalRowMutation {
                    table: "records".into(),
                    kind: RelationalMutationKind::Insert,
                    values: json!({"id":id}),
                    key: None,
                })
                .collect(),
        })
        .unwrap();
        let QueryOutcomeV2::Rows(nodes) = db
            .query_v2(
                actor("source-reader"),
                request("NodeScan", json!({"as":"x","label":"Document"})),
            )
            .unwrap()
        else {
            panic!("rows")
        };
        let QueryValueV2::Entity(target) = &nodes.rows[0]["x"] else {
            panic!("entity")
        };
        db.put_annotation(actor("source-reader"), AnnotationPutMutationV2 {
            annotation:json!({"id":"review:one","namespace":"default","kind":"review",
                "targets":[{"ref":target,"binding":"frozen","selector":{"type":"whole"}}],
                "evidence":[],"body":{"type":"text","text":"Reviewed."},"author":"fixture",
                "created_at":"2026-09-22T00:00:00Z","valid_from":"2026-09-22T00:00:00Z","valid_to":null}),
            expected_revision:None, valid:None,
        }).unwrap();
    }
    (dir, db)
}

#[test]
fn source_rows_are_exact_in_hql_ir_and_all_diagnostic_modes_including_empty() {
    for populated in [false, true] {
        let (_dir, db) = fixture(populated);
        for (op, config, source, count) in [
            (
                "NodeScan",
                json!({"as":"x","label":"Document"}),
                "NODES Document",
                2,
            ),
            (
                "EdgeScan",
                json!({"as":"x","relation":"LINK"}),
                "EDGES LINK",
                1,
            ),
            (
                "RowScan",
                json!({"as":"x","table":"records"}),
                "TABLE records",
                2,
            ),
            ("AnnotationScan", json!({"as":"x"}), "ANNOTATIONS", 1),
        ] {
            let expected = if populated { count } else { 0 };
            let ir = diagnostics::three_modes_with_access(
                &db,
                actor("source-reader"),
                request(op, config),
            );
            diagnostics::measured(&ir, "scan", "rows_in", "rows", 0);
            diagnostics::measured(&ir, "scan", "rows_out", "rows", expected);
            let hql: QueryRequestV2 = serde_json::from_value(json!({
                "contract_version":"genesis.api.v2","request_id":"source-hql-diagnostics",
                "namespace":"default","language_version":"hql.v2",
                "hql":format!("USE default FROM {source} AS x |> RETURN x"),"params":{}
            }))
            .unwrap();
            let h = diagnostics::three_modes_with_access(&db, actor("source-reader"), hql);
            diagnostics::measured(&h, "h0", "rows_in", "rows", 0);
            diagnostics::measured(&h, "h0", "rows_out", "rows", expected);
            assert_eq!(h.rows, ir.rows, "{op}");
            assert_eq!(h.columns, ir.columns);
            assert_eq!(h.semantics, ir.semantics);
            assert_eq!(h.completeness, ir.completeness);
            assert_eq!(ir.rows.len() as u64, expected);
            // Identity expectations come from fixture setup, not diagnostics.
            if populated && op != "RowScan" {
                let mut ids: Vec<_> = ir
                    .rows
                    .iter()
                    .map(|row| match &row["x"] {
                        QueryValueV2::Entity(entity) => entity.id.as_str(),
                        _ => panic!("entity"),
                    })
                    .collect();
                ids.sort();
                assert_eq!(
                    ids,
                    match op {
                        "NodeScan" => vec!["one", "two"],
                        "EdgeScan" => vec!["edge:one"],
                        "AnnotationScan" => vec!["review:one"],
                        _ => unreachable!(),
                    }
                );
            }
        }
    }
}

#[test]
fn source_budget_failures_return_errors_in_both_execution_modes() {
    let (_dir, db) = fixture(true);
    let original = request("NodeScan", json!({"as":"x","label":"Document"}));
    diagnostics::three_modes_with_access(&db, actor("source-reader"), original.clone());
    for limit in [
        json!({"max_expanded_nodes":1}),
        json!({"max_result_rows":1}),
        json!({"max_result_bytes":1}),
    ] {
        for mode in [None, Some(ExplainV2::Analyze)] {
            let mut limited = original.clone();
            limited.explain = mode;
            limited.budget = Some(serde_json::from_value(limit.clone()).unwrap());
            assert_eq!(
                db.query_v2(actor("source-reader"), limited)
                    .unwrap_err()
                    .code,
                "QUERY_BUDGET_EXCEEDED"
            );
        }
    }
}

#[test]
fn source_diagnostics_respect_explicit_actor_authority() {
    let (_dir, db) = fixture(true);
    db.replace_access_policy(
        PolicyAdminActor {
            access: actor("local-owner"),
        },
        0,
        AccessPolicy {
            revision: 1,
            mode: AccessPolicyMode::Enforced,
            grants: vec![
                AccessGrant {
                    principal: "source-reader".into(),
                    action: AccessAction::Read,
                    resource: AccessResource::Namespace("default".into()),
                },
                AccessGrant {
                    principal: "source-reader".into(),
                    action: AccessAction::Read,
                    resource: AccessResource::Annotation("default".into()),
                },
                AccessGrant {
                    principal: "namespace-only-reader".into(),
                    action: AccessAction::Read,
                    resource: AccessResource::Namespace("default".into()),
                },
                AccessGrant {
                    principal: "local-owner".into(),
                    action: AccessAction::ManagePolicy,
                    resource: AccessResource::Namespace("default".into()),
                },
            ],
        },
    )
    .unwrap();
    let original = request("AnnotationScan", json!({"as":"x"}));
    let allowed =
        diagnostics::three_modes_with_access(&db, actor("source-reader"), original.clone());
    diagnostics::measured(&allowed, "scan", "rows_out", "rows", 1);
    let lookup:QueryRequestV2=serde_json::from_value(json!({
        "contract_version":"genesis.api.v2","request_id":"actor-lookup",
        "namespace":"default","language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS d |> OPTIONAL ANNOTATIONS OF d AS a |> RETURN d.id AS document_id, a.id AS annotation_id","params":{}
    })).unwrap();
    let result = diagnostics::three_modes_with_access(&db, actor("source-reader"), lookup.clone());
    let lookup_node = result
        .explain
        .as_ref()
        .unwrap()
        .plan
        .iter()
        .find(|node| node.physical_op == "AuthorizedAnnotationLookup")
        .unwrap();
    diagnostics::measured(&result, &lookup_node.id, "rows_in", "rows", 2);
    diagnostics::measured(&result, &lookup_node.id, "rows_out", "rows", 2);
    for original in [original, lookup] {
        // Namespace access permits planning; annotation data requires its own grant.
        let mut plan_only = original.clone();
        plan_only.explain = Some(ExplainV2::Plan);
        let QueryOutcomeV2::Plan(plan) = db
            .query_v2(actor("namespace-only-reader"), plan_only)
            .unwrap()
        else {
            panic!("catalog-only plan")
        };
        assert!(plan.plan.iter().all(|node| node.actual.is_none()));
        for mode in [None, Some(ExplainV2::Analyze)] {
            let mut denied = original.clone();
            denied.explain = mode;
            let error = db
                .query_v2(actor("namespace-only-reader"), denied)
                .unwrap_err();
            assert_eq!(error.code, "FORBIDDEN");
            assert_eq!(error.stage, "authorize");
        }
        for mode in [None, Some(ExplainV2::Plan), Some(ExplainV2::Analyze)] {
            let mut denied = original.clone();
            denied.explain = mode;
            let error = db
                .query_v2(actor("unauthorized-reader"), denied)
                .unwrap_err();
            assert_eq!(error.code, "FORBIDDEN");
        }
    }
}

fn source_metadata_disk_tree(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path
                .file_name()
                .is_some_and(|name| name == "genesis.lock" || name == "projection.sqlite-shm")
            {
                out.insert(
                    format!("{}:size", path.file_name().unwrap().to_string_lossy()),
                    std::fs::metadata(&path)
                        .unwrap()
                        .len()
                        .to_le_bytes()
                        .to_vec(),
                );
                continue;
            }
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_string_lossy().into(),
                    std::fs::read(&path)
                        .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
                );
            }
        }
    }

    let mut files = BTreeMap::new();
    visit(path, path, &mut files);
    files
}

fn source_metadata_state(db: &Storage, path: &Path) -> (u64, BTreeMap<String, Vec<u8>>) {
    (db.stable_frontier(), source_metadata_disk_tree(path))
}

fn assert_source_metadata_state(
    db: &Storage,
    path: &Path,
    before: &(u64, BTreeMap<String, Vec<u8>>),
) {
    assert_eq!(
        db.stable_frontier(),
        before.0,
        "Plan changed durable frontier"
    );
    assert_eq!(
        source_metadata_disk_tree(path),
        before.1,
        "Plan changed observed fixture files"
    );
}

fn source_metadata_row_ids(path: &Path) -> Vec<String> {
    let connection = rusqlite::Connection::open(path.join("projection.sqlite")).unwrap();
    let mut statement = connection
        .prepare(
            "SELECT record_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='row' AND schema_ref='records' AND tx_to IS NULL
             ORDER BY record_id",
        )
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<String>>>()
        .unwrap()
}

fn source_metadata_policy(db: &Storage) {
    db.replace_access_policy(
        PolicyAdminActor {
            access: actor("local-owner"),
        },
        0,
        AccessPolicy {
            revision: 1,
            mode: AccessPolicyMode::Enforced,
            grants: vec![
                AccessGrant {
                    principal: "source-reader".into(),
                    action: AccessAction::Read,
                    resource: AccessResource::Namespace("default".into()),
                },
                AccessGrant {
                    principal: "source-reader".into(),
                    action: AccessAction::Read,
                    resource: AccessResource::Annotation("default".into()),
                },
                AccessGrant {
                    principal: "local-owner".into(),
                    action: AccessAction::ManagePolicy,
                    resource: AccessResource::Namespace("default".into()),
                },
            ],
        },
    )
    .unwrap();
}

fn source_metadata_p7_revision(
    entity: p7_graph::EntityRef,
    fields: BTreeMap<String, p7_graph::Scalar>,
    data: p7_graph::RecordData,
) -> p7_graph::Revision {
    p7_graph::Revision {
        entity,
        transaction: p7_graph::Interval {
            start: 1,
            end: None,
        },
        valid: p7_graph::Interval {
            start: 0,
            end: None,
        },
        retracted: false,
        fields,
        data,
    }
}

fn source_metadata_p7_entity(
    kind: p7_graph::Kind,
    id: &str,
    revision: &str,
) -> p7_graph::EntityRef {
    p7_graph::EntityRef {
        namespace: "default".into(),
        kind,
        id: id.into(),
        revision: revision.into(),
    }
}

fn source_metadata_p7_bag(
    revisions: &[p7_graph::Revision],
    kind: p7_graph::Kind,
    predicate: p7_graph::Predicate,
) -> BTreeMap<String, usize> {
    let catalog = p7_graph::Catalog {
        frontier: 10,
        history: [
            p7_graph::Kind::Node,
            p7_graph::Kind::Edge,
            p7_graph::Kind::Row,
            p7_graph::Kind::Annotation,
        ]
        .into_iter()
        .map(|kind| {
            (
                kind,
                p7_graph::HistoryCapability {
                    horizon: 0,
                    available: true,
                },
            )
        })
        .collect(),
        revisions: revisions.to_vec(),
    };
    let view = p7_graph::View {
        namespace: "default".into(),
        transaction: 10,
        valid_at: 5,
        permissions: p7_graph::Permissions {
            read: revisions
                .iter()
                .map(|revision| revision.entity.identity())
                .collect(),
            annotation_body: revisions
                .iter()
                .filter(|revision| revision.entity.kind == p7_graph::Kind::Annotation)
                .map(|revision| revision.entity.identity())
                .collect(),
        },
    };
    let plan = p7_graph::Plan {
        source: p7_graph::Source::Scan {
            kind,
            alias: "scan".into(),
            predicate,
        },
        stages: vec![],
    };
    let mut bag = BTreeMap::new();
    for row in p7_graph::execute(&catalog, &view, &plan, p7_graph::Limits::default())
        .unwrap()
        .rows
    {
        let p7_graph::Binding::Entity(entity) = row.get("scan").unwrap() else {
            panic!("P7 source scan must bind an entity")
        };
        *bag.entry(entity.id.clone()).or_insert(0) += 1;
    }
    bag
}

fn source_metadata_p7_revisions(row_ids: &[String], populated: bool) -> Vec<p7_graph::Revision> {
    if !populated {
        return vec![];
    }

    let node_one = source_metadata_p7_entity(p7_graph::Kind::Node, "one", "node-rev-one");
    let node_two = source_metadata_p7_entity(p7_graph::Kind::Node, "two", "node-rev-two");
    let mut revisions = vec![
        source_metadata_p7_revision(
            node_one.clone(),
            BTreeMap::from([("label".into(), p7_graph::Scalar::Text("Document".into()))]),
            p7_graph::RecordData::Plain,
        ),
        source_metadata_p7_revision(
            node_two.clone(),
            BTreeMap::from([("label".into(), p7_graph::Scalar::Text("Document".into()))]),
            p7_graph::RecordData::Plain,
        ),
        source_metadata_p7_revision(
            source_metadata_p7_entity(p7_graph::Kind::Edge, "edge:one", "edge-rev-one"),
            BTreeMap::from([("relation".into(), p7_graph::Scalar::Text("LINK".into()))]),
            p7_graph::RecordData::Edge {
                source: node_one.identity(),
                target: node_two.identity(),
                relation: "LINK".into(),
            },
        ),
    ];
    revisions.extend(row_ids.iter().enumerate().map(|(index, id)| {
        source_metadata_p7_revision(
            source_metadata_p7_entity(p7_graph::Kind::Row, id, &format!("row-rev-{index}")),
            BTreeMap::new(),
            p7_graph::RecordData::Plain,
        )
    }));
    revisions.push(source_metadata_p7_revision(
        source_metadata_p7_entity(
            p7_graph::Kind::Annotation,
            "review:one",
            "annotation-rev-one",
        ),
        BTreeMap::new(),
        p7_graph::RecordData::Annotation {
            targets: vec![p7_graph::Target {
                binding: p7_graph::TargetBinding::Frozen(node_one),
                selector: p7_graph::Selector::Whole,
            }],
        },
    ));
    revisions
}

fn source_metadata_bag(ids: &[String]) -> BTreeMap<String, usize> {
    let mut bag = BTreeMap::new();
    for id in ids {
        *bag.entry(id.clone()).or_insert(0) += 1;
    }
    bag
}

struct SourceMetadataRequestSpec<'a> {
    frontend: &'a str,
    op: &'a str,
    config: Value,
    hql_source: &'a str,
    request_id: &'a str,
    valid_at: &'a str,
    tx_as_of: u64,
    mode: ExplainV2,
}

fn source_metadata_request(spec: SourceMetadataRequestSpec<'_>) -> QueryRequestV2 {
    let SourceMetadataRequestSpec {
        frontend,
        op,
        config,
        hql_source,
        request_id,
        valid_at,
        tx_as_of,
        mode,
    } = spec;
    let mut query = if frontend == "HQL" {
        serde_json::from_value(json!({
            "contract_version":"genesis.api.v2",
            "request_id":request_id,
            "namespace":"default",
            "language_version":"hql.v2",
            "hql":format!("USE default FROM {hql_source} AS x |> RETURN x"),
            "params":{}
        }))
        .unwrap()
    } else {
        request(op, config)
    };
    query.request_id = request_id.into();
    query.temporal = Some(
        serde_json::from_value(json!({
            "valid_at":valid_at,
            "tx_as_of":tx_as_of.to_string()
        }))
        .unwrap(),
    );
    query.explain = Some(mode);
    query
}

fn assert_source_metadata_plan(
    plan: &genesis_block_native::query::hql2::ExplainResultV2,
    physical_op: &str,
    root_columns: &Value,
    request_id: &str,
    policy_revision: u64,
    observed_frontier: u64,
    expect_no_actuals: bool,
) {
    assert_eq!(plan.request_id, request_id);
    assert_eq!(plan.catalog.policy_revision, policy_revision);
    assert_eq!(plan.catalog.observed_frontier, observed_frontier);
    assert_eq!(plan.catalog.schema_fingerprint.len(), 64);
    assert!(plan
        .catalog
        .schema_fingerprint
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    let root = plan
        .plan
        .iter()
        .find(|node| node.id == plan.root)
        .expect("plan root");
    assert_eq!(serde_json::to_value(&root.columns).unwrap(), *root_columns);
    let source = plan
        .plan
        .iter()
        .find(|node| node.physical_op == physical_op)
        .expect("source physical operator");
    assert!(source.inputs.is_empty());
    assert_eq!(
        serde_json::to_value(&source.columns).unwrap(),
        *root_columns
    );
    if root.id == source.id {
        assert_eq!(plan.plan.len(), 1);
    } else {
        assert_eq!(root.physical_op, "Project");
        assert_eq!(root.inputs, vec![source.id.clone()]);
        assert_eq!(plan.plan.len(), 2);
    }
    assert!(plan.plan.iter().all(|node| node.estimates.is_none()));
    if expect_no_actuals {
        assert!(plan.plan.iter().all(|node| node.actual.is_none()));
    } else {
        assert!(source.actual.is_some());
    }
}

struct SourceMetadataExpectations<'a> {
    request_id: &'a str,
    database_id: &'a str,
    generation: &'a GenerationInfo,
    valid_at: &'a str,
    root_columns: &'a Value,
    expected_ids: &'a [String],
    p7_bag: &'a BTreeMap<String, usize>,
    kind: &'a str,
    analyze: bool,
    physical_op: &'a str,
}

fn assert_source_metadata_result(result: &QueryResultV2, expected: SourceMetadataExpectations<'_>) {
    let SourceMetadataExpectations {
        request_id,
        database_id,
        generation,
        valid_at,
        root_columns,
        expected_ids,
        p7_bag,
        kind,
        analyze,
        physical_op,
    } = expected;
    let wire = serde_json::to_value(result).unwrap();
    assert_eq!(wire["request_id"], request_id);
    assert_eq!(wire["columns"], *root_columns);
    assert_eq!(
        wire["snapshot"],
        json!({
            "database_id":database_id,
            "tx":generation.wal_frontier.to_string(),
            "valid_at":valid_at,
            "catalog_generation":generation.generation_id.to_string(),
            "policy_version":generation.acl_revision.to_string()
        })
    );
    assert_eq!(
        wire["semantics"],
        json!({
            "candidate_search":"exact",
            "distance_fidelity":"not_applicable",
            "index_coverage":"not_applicable",
            "scope":"whole_input"
        })
    );
    assert_eq!(
        wire["completeness"],
        json!({"status":"complete","reason":null,"eligible_count_known":true})
    );
    assert_eq!(wire["index_frontiers"], json!({}));
    assert_eq!(wire.get("cursor"), Some(&Value::Null));
    assert_eq!(result.rows.len(), expected_ids.len());

    let mut actual_bag = BTreeMap::new();
    for row in &result.rows {
        let QueryValueV2::Entity(entity) = row.get("x").expect("root entity") else {
            panic!("source root must be an entity")
        };
        assert_eq!(entity.database_id, database_id);
        assert_eq!(entity.namespace, "default");
        assert_eq!(serde_json::to_value(&entity.kind).unwrap(), json!(kind));
        assert!(!entity.id.is_empty());
        assert!(!entity.revision.is_empty());
        *actual_bag.entry(entity.id.clone()).or_insert(0) += 1;
    }
    let literal_bag = source_metadata_bag(expected_ids);
    assert_eq!(actual_bag, literal_bag, "literal source identities");
    assert_eq!(actual_bag, *p7_bag, "independent P7 source bag");

    if analyze {
        let explain = result.explain.as_ref().expect("Analyze explain result");
        assert_source_metadata_plan(
            explain,
            physical_op,
            root_columns,
            request_id,
            generation.acl_revision,
            generation.publication_seq,
            false,
        );
        let node = explain
            .plan
            .iter()
            .find(|node| node.physical_op == physical_op)
            .expect("Analyze source physical operator");
        let counters =
            serde_json::to_value(node.actual.as_ref().expect("Analyze counters")).unwrap();
        assert_eq!(
            counters["rows_in"],
            json!({"unit":"rows","measurement":{"status":"measured","value":0}})
        );
        assert_eq!(
            counters["rows_out"],
            json!({"unit":"rows","measurement":{"status":"measured","value":expected_ids.len()}})
        );
        assert_eq!(
            counters["elapsed_ns"],
            json!({"unit":"nanoseconds","measurement":{"status":"unknown","reason":"not_instrumented"}})
        );
    } else {
        assert!(result.explain.is_none());
    }
}

#[test]
fn source_result_metadata_has_independent_fixture_expectations_in_hql_and_ir() {
    let sources = [
        (
            "NodeScan",
            json!({"as":"x","label":"Document"}),
            "NODES Document",
            "AuthorizedNodeScan",
            p7_graph::Kind::Node,
            "node",
        ),
        (
            "EdgeScan",
            json!({"as":"x","relation":"LINK"}),
            "EDGES LINK",
            "AuthorizedEdgeScan",
            p7_graph::Kind::Edge,
            "edge",
        ),
        (
            "RowScan",
            json!({"as":"x","table":"records"}),
            "TABLE records",
            "AuthorizedRowScan",
            p7_graph::Kind::Row,
            "row",
        ),
        (
            "AnnotationScan",
            json!({"as":"x"}),
            "ANNOTATIONS",
            "AuthorizedAnnotationScan",
            p7_graph::Kind::Annotation,
            "annotation",
        ),
    ];

    for populated in [false, true] {
        let (dir, db) = fixture(populated);
        source_metadata_policy(&db);
        let row_ids = if populated {
            source_metadata_row_ids(dir.path())
        } else {
            vec![]
        };
        assert_eq!(row_ids.len(), if populated { 2 } else { 0 });
        let p7_revisions = source_metadata_p7_revisions(&row_ids, populated);
        let generation = db.publish_generation().unwrap();
        assert_eq!(generation.acl_revision, 1);
        assert_eq!(db.stable_frontier(), generation.publication_seq);
        let valid_at = chrono::Utc::now().to_rfc3339();
        let plan_frontier = db.stable_frontier();
        let database_id = {
            let mut digest = Sha256::new();
            digest.update(b"genesis.api.v2.database:");
            digest.update(db.verifying_key.as_bytes());
            hex::encode(digest.finalize())
        };
        let root_columns = json!([{"name":"x","type":"Entity","nullable":false}]);
        let mut plan_requests = 0;
        let mut normal_requests = 0;
        let mut analyze_requests = 0;

        // The complete selected Plan matrix precedes every selected data execution.
        for (op, config, hql_source, physical_op, _, source_name) in &sources {
            for frontend in ["HQL", "IR"] {
                let request_id = format!(
                    "source-meta-{}-{source_name}-{frontend}-plan",
                    if populated { "populated" } else { "empty" }
                );
                let query = source_metadata_request(SourceMetadataRequestSpec {
                    frontend,
                    op,
                    config: config.clone(),
                    hql_source,
                    request_id: &request_id,
                    valid_at: &valid_at,
                    tx_as_of: generation.wal_frontier,
                    mode: ExplainV2::Plan,
                });
                let before = source_metadata_state(&db, dir.path());
                assert_eq!(before.0, plan_frontier);
                let QueryOutcomeV2::Plan(plan) =
                    db.query_v2(actor("source-reader"), query).unwrap()
                else {
                    panic!("Plan request must return a plan")
                };
                assert_source_metadata_plan(
                    &plan,
                    physical_op,
                    &root_columns,
                    &request_id,
                    1,
                    plan_frontier,
                    true,
                );
                assert_source_metadata_state(&db, dir.path(), &before);
                plan_requests += 1;
            }
        }

        for (op, config, hql_source, physical_op, p7_kind, source_name) in &sources {
            let (predicate, expected_ids) = match *source_name {
                "node" => (
                    p7_graph::Predicate {
                        equals: BTreeMap::from([(
                            "label".into(),
                            p7_graph::Scalar::Text("Document".into()),
                        )]),
                        ..p7_graph::Predicate::default()
                    },
                    if populated {
                        vec!["one".into(), "two".into()]
                    } else {
                        vec![]
                    },
                ),
                "edge" => (
                    p7_graph::Predicate {
                        equals: BTreeMap::from([(
                            "relation".into(),
                            p7_graph::Scalar::Text("LINK".into()),
                        )]),
                        ..p7_graph::Predicate::default()
                    },
                    if populated {
                        vec!["edge:one".into()]
                    } else {
                        vec![]
                    },
                ),
                "row" => (p7_graph::Predicate::default(), row_ids.clone()),
                "annotation" => (
                    p7_graph::Predicate::default(),
                    if populated {
                        vec!["review:one".into()]
                    } else {
                        vec![]
                    },
                ),
                _ => unreachable!(),
            };
            let p7_bag = source_metadata_p7_bag(&p7_revisions, *p7_kind, predicate);
            assert_eq!(p7_bag, source_metadata_bag(&expected_ids));
            for frontend in ["HQL", "IR"] {
                for (mode_name, mode) in
                    [("normal", ExplainV2::None), ("analyze", ExplainV2::Analyze)]
                {
                    let request_id = format!(
                        "source-meta-{}-{source_name}-{frontend}-{mode_name}",
                        if populated { "populated" } else { "empty" }
                    );
                    let query = source_metadata_request(SourceMetadataRequestSpec {
                        frontend,
                        op,
                        config: config.clone(),
                        hql_source,
                        request_id: &request_id,
                        valid_at: &valid_at,
                        tx_as_of: generation.wal_frontier,
                        mode,
                    });
                    let QueryOutcomeV2::Rows(result) =
                        db.query_v2(actor("source-reader"), query).unwrap()
                    else {
                        panic!("Normal/Analyze request must return rows")
                    };
                    assert_source_metadata_result(
                        &result,
                        SourceMetadataExpectations {
                            request_id: &request_id,
                            database_id: &database_id,
                            generation: &generation,
                            valid_at: &valid_at,
                            root_columns: &root_columns,
                            expected_ids: &expected_ids,
                            p7_bag: &p7_bag,
                            kind: match *source_name {
                                "node" => "node",
                                "edge" => "edge",
                                "row" => "row",
                                "annotation" => "annotation",
                                _ => unreachable!(),
                            },
                            analyze: mode_name == "analyze",
                            physical_op,
                        },
                    );
                    if mode_name == "normal" {
                        normal_requests += 1;
                    } else {
                        analyze_requests += 1;
                    }
                }
            }
        }

        assert_eq!(plan_requests, 8);
        assert_eq!(normal_requests, 8);
        assert_eq!(analyze_requests, 8);
    }
}
