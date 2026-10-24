//! HQL2 table joins against the independent P7 bag interpreter and typed IR.
#[path = "support/hql2_reference.rs"]
mod reference;

use genesis_block_native::{
    query::hql2::{value::QueryValueV2 as V, QueryOutcomeV2},
    uee_v2::{ExplainV2, QueryOpV2, QueryRequestV2},
    AccessAction, AccessContext, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource,
    NodeInput, OpenOptions, PolicyAdminActor, RelationalColumn, RelationalColumnType,
    RelationalMutationKind, RelationalRowMutation, RelationalSchemaPackage, RelationalTable,
    Storage,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
use tempfile::TempDir;
use uuid::Uuid;

const P7_JSON_NULL: &str = "__p7_json_null__";
const P7_JSON_NUMBER_PREFIX: &str = "__p7_json_number__:";

#[derive(Clone, Copy)]
enum JoinKey {
    Number(i64),
    Missing,
    JsonNull,
}

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
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

fn hql_request(request_id: &str, kind: &str) -> QueryRequestV2 {
    let projection = if matches!(kind, "semi" | "anti") {
        "prop(n, \"k\") AS left_key"
    } else {
        "prop(n, \"k\") AS left_key, prop(r, \"k\") AS right_key"
    };
    let query = format!(
        "USE default FROM NODES JoinKey AS n |> {kind} JOIN TABLE right_rows AS r ON prop(n, \"k\") = prop(r, \"k\") |> RETURN {projection}"
    );
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":query,
        "params":{}
    }))
    .unwrap()
}

fn typed_ir_request(request_id: &str, kind: &str) -> QueryRequestV2 {
    let property = |alias| {
        json!({"call":"prop","args":[
            {"field":{"alias":alias,"path":[]}},
            {"type":"Utf8","literal":"k"}
        ]})
    };
    let mut nodes = vec![
        json!({"id":"left","op":"NodeScan","inputs":[],"config":{"as":"n","label":"JoinKey"}}),
        json!({"id":"right","op":"RowScan","inputs":[],"config":{"table":"right_rows","as":"r"}}),
        json!({"id":"join","op":"Join","inputs":["left","right"],"config":{
            "kind":kind,
            "condition":{"binary":"eq","left":property("n"),"right":property("r")}
        }}),
    ];
    let mut fields = vec![json!({"as":"left_key","expression":property("n")})];
    if !matches!(kind, "semi" | "anti") {
        fields.push(json!({"as":"right_key","expression":property("r")}));
    }
    nodes.push(json!({"id":"project","op":"Project","inputs":["join"],"config":{"fields":fields}}));
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":nodes,
            "root":"project",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap()
}

fn from_p7(value: &reference::Value) -> V {
    match value {
        reference::Value::Null => V::Null,
        reference::Value::Text(value) if value == P7_JSON_NULL => V::Json(Value::Null),
        reference::Value::Text(value) if value.starts_with(P7_JSON_NUMBER_PREFIX) => {
            let number = value[P7_JSON_NUMBER_PREFIX.len()..]
                .parse::<i64>()
                .expect("P7 JSON-number sentinel is generated locally");
            V::Json(json!(number))
        }
        other => panic!("unexpected P7 HQL join value: {other:?}"),
    }
}

fn p7_value(key: JoinKey) -> reference::Value {
    match key {
        JoinKey::Number(value) => reference::Value::Text(format!("{P7_JSON_NUMBER_PREFIX}{value}")),
        JoinKey::Missing => reference::Value::Null,
        // P7 has no JSON value variant; reserve a text sentinel for JSON null.
        JoinKey::JsonNull => reference::Value::Text(P7_JSON_NULL.into()),
    }
}

fn expected_rows(
    left: &[JoinKey],
    right: &[JoinKey],
    kind: reference::JoinKind,
) -> Vec<BTreeMap<String, V>> {
    use reference::{Expr as E, Plan as P};

    let values = |field: &str, values: &[JoinKey]| {
        P::Values(
            values
                .iter()
                .map(|value| BTreeMap::from([(field.into(), p7_value(*value))]))
                .collect(),
        )
    };
    let join = P::Join {
        left: Box::new(values("n.k", left)),
        right: Box::new(values("r.k", right)),
        on: E::Eq(
            Box::new(E::Field("n.k".into())),
            Box::new(E::Field("r.k".into())),
        ),
        kind,
        right_fields: vec!["r.k".into()],
    };
    let fields = if matches!(kind, reference::JoinKind::Semi | reference::JoinKind::Anti) {
        vec![("left_key".into(), E::Field("n.k".into()))]
    } else {
        vec![
            ("left_key".into(), E::Field("n.k".into())),
            ("right_key".into(), E::Field("r.k".into())),
        ]
    };
    reference::execute(&P::Project(Box::new(join), fields))
        .unwrap()
        .iter()
        .map(|row| {
            row.iter()
                .map(|(key, value)| (key.clone(), from_p7(value)))
                .collect()
        })
        .collect()
}

fn sorted_rows(rows: Vec<BTreeMap<String, V>>) -> Vec<String> {
    let mut rows = rows
        .into_iter()
        .map(|row| format!("{row:?}"))
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

fn populate_join(storage: &Storage, left: &[JoinKey], right: &[JoinKey]) {
    storage
        .register_relational_schema(RelationalSchemaPackage {
            namespace: "default".into(),
            schema_version: 1,
            previous_version: None,
            package_id: Uuid::new_v4().to_string(),
            schema_hash: String::new(),
            tables: vec![RelationalTable {
                name: "right_rows".into(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::Text),
                    RelationalColumn {
                        name: "k".into(),
                        column_type: RelationalColumnType::Json,
                        nullable: true,
                        default: None,
                    },
                ],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            }],
            named_queries: vec![],
        })
        .unwrap();

    for (index, key) in left.iter().enumerate() {
        let props = match key {
            JoinKey::Number(value) => json!({"k":value}),
            JoinKey::Missing => json!({}),
            JoinKey::JsonNull => json!({"k":null}),
        };
        storage
            .add_node(NodeInput {
                id: Some(format!("join-left-{index}")),
                labels: vec!["JoinKey".into()],
                props: Some(props),
                embedding: None,
                lang: None,
                valid_from: Some("2026-09-22T00:00:00Z".into()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
    }
    storage
        .apply_relational_rows(
            "default",
            right
                .iter()
                .enumerate()
                .map(|(index, value)| RelationalRowMutation {
                    table: "right_rows".into(),
                    kind: RelationalMutationKind::Insert,
                    values: json!({
                        "id":format!("right-{index}"),
                        "k":match value {
                            JoinKey::Number(value) => json!(value),
                            JoinKey::Missing | JoinKey::JsonNull => Value::Null,
                        }
                    }),
                    key: None,
                })
                .collect(),
        )
        .unwrap();
}

#[test]
fn storage_hql_join_kinds_match_typed_ir_and_independent_p7() {
    let directory = TempDir::new().unwrap();
    let storage = open(directory.path());
    let left = [
        JoinKey::Number(1),
        JoinKey::Number(1),
        JoinKey::Missing,
        JoinKey::Number(2),
        JoinKey::JsonNull,
    ];
    let right = [
        JoinKey::Number(1),
        JoinKey::Number(1),
        JoinKey::JsonNull,
        JoinKey::Number(99),
    ];
    populate_join(&storage, &left, &right);

    for (index, query) in [
        "USE default FROM NODES JoinKey AS n |> RETURN prop(n, \"k\") AS k",
        "USE default FROM TABLE right_rows AS r |> RETURN prop(r, \"k\") AS k",
    ]
    .into_iter()
    .enumerate()
    {
        let QueryOutcomeV2::Rows(result) = storage
            .query_v2(
                access(),
                serde_json::from_value(json!({
                    "contract_version":"genesis.api.v2",
                    "request_id":format!("null-source-{index}"),
                    "namespace":"default",
                    "language_version":"hql.v2",
                    "hql":query,
                    "params":{}
                }))
                .unwrap(),
            )
            .unwrap()
        else {
            panic!("source property query should return rows")
        };
        if query.contains("NODES") {
            assert!(result.rows.iter().any(|row| row["k"] == V::Null));
            assert!(result
                .rows
                .iter()
                .any(|row| row["k"] == V::Json(Value::Null)));
        } else {
            assert!(result
                .rows
                .iter()
                .any(|row| row["k"] == V::Json(Value::Null)));
        }
    }

    for (kind, oracle_kind, expected_count) in [
        ("inner", reference::JoinKind::Inner, 5),
        ("left", reference::JoinKind::Left, 7),
        ("semi", reference::JoinKind::Semi, 3),
        ("anti", reference::JoinKind::Anti, 2),
    ] {
        let expected = sorted_rows(expected_rows(&left, &right, oracle_kind));
        let QueryOutcomeV2::Rows(hql) = storage
            .query_v2(access(), hql_request(&format!("hql-join-{kind}"), kind))
            .unwrap()
        else {
            panic!("HQL join should return rows for {kind}");
        };
        let QueryOutcomeV2::Rows(ir) = storage
            .query_v2(access(), typed_ir_request(&format!("ir-join-{kind}"), kind))
            .unwrap()
        else {
            panic!("typed-IR join should return rows for {kind}");
        };

        assert_eq!(hql.rows.len(), expected_count, "{kind} P7 row count");
        assert_eq!(
            sorted_rows(hql.rows.clone()),
            expected,
            "{kind} differs from P7"
        );
        assert_eq!(
            sorted_rows(hql.rows),
            sorted_rows(ir.rows),
            "{kind} differs from typed IR"
        );
    }

    let QueryOutcomeV2::Rows(bare_join) = storage
        .query_v2(access(), hql_request("hql-bare-join", ""))
        .unwrap()
    else {
        panic!("bare JOIN should return rows")
    };
    assert_eq!(
        sorted_rows(bare_join.rows),
        sorted_rows(expected_rows(&left, &right, reference::JoinKind::Inner)),
        "bare JOIN must default to INNER"
    );

    for kind in ["semi", "anti"] {
        let query = format!(
            "USE default FROM NODES JoinKey AS n |> {kind} JOIN TABLE right_rows AS r ON prop(n, \"k\") = prop(r, \"k\") |> RETURN prop(r, \"k\") AS leaked"
        );
        let error = storage
            .query_v2(
                access(),
                serde_json::from_value(json!({
                    "contract_version":"genesis.api.v2",
                    "request_id":format!("{kind}-right-scope"),
                    "namespace":"default",
                    "language_version":"hql.v2",
                    "hql":query,
                    "params":{}
                }))
                .unwrap(),
            )
            .unwrap_err();
        assert_eq!(error.code, "BIND_ERROR", "{kind} must hide right scope");
    }
}

fn disk_tree(path: &Path) -> BTreeMap<String, Vec<u8>> {
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

    let mut out = BTreeMap::new();
    visit(path, path, &mut out);
    out
}

fn state(db: &Storage, path: &Path) -> (u64, BTreeMap<String, Vec<u8>>) {
    (db.stable_frontier(), disk_tree(path))
}
fn assert_state(db: &Storage, path: &Path, before: &(u64, BTreeMap<String, Vec<u8>>)) {
    assert_eq!(
        db.stable_frontier(),
        before.0,
        "query changed durable frontier"
    );
    assert_eq!(disk_tree(path), before.1, "query changed fixture files");
}

fn join_policy(storage: &Storage) {
    storage
        .replace_access_policy(
            PolicyAdminActor {
                access: AccessContext {
                    principal: "local-owner".into(),
                    namespace: "default".into(),
                },
            },
            0,
            AccessPolicy {
                revision: 1,
                mode: AccessPolicyMode::Enforced,
                grants: vec![
                    AccessGrant {
                        principal: "reader".into(),
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
}

#[test]
fn storage_join_diagnostics_preserve_p7_bags_counters_authority_and_state() {
    let full_left = [
        JoinKey::Number(1),
        JoinKey::Number(1),
        JoinKey::Missing,
        JoinKey::Number(2),
        JoinKey::JsonNull,
    ];
    let full_right = [
        JoinKey::Number(1),
        JoinKey::Number(1),
        JoinKey::JsonNull,
        JoinKey::Number(99),
    ];
    // Literal outputs are hand-derived, independently cross-checked by P7.
    for (left, right, outputs) in [
        (&full_left[..], &full_right[..], [5, 7, 3, 2]),
        (&[][..], &full_right[..], [0, 0, 0, 0]),
        (&full_left[..], &[][..], [0, 5, 0, 5]),
        (&[][..], &[][..], [0, 0, 0, 0]),
    ] {
        let directory = TempDir::new().unwrap();
        let storage = open(directory.path());
        populate_join(&storage, left, right);
        join_policy(&storage);
        // Row revisions use insertion time. Pin once after setup, then reuse in
        // every mode/frontend so the fixture cannot outlive a calendar literal.
        let valid_at = chrono::Utc::now().to_rfc3339();
        // All allowed/denied Plan requests precede any query execution or explicit
        // publication in this fixture. Capturing state does not publish a generation.
        let before_plan = state(&storage, directory.path());
        let mut cases = Vec::new();
        for (index, (kind, oracle)) in [
            ("inner", reference::JoinKind::Inner),
            ("left", reference::JoinKind::Left),
            ("semi", reference::JoinKind::Semi),
            ("anti", reference::JoinKind::Anti),
        ]
        .into_iter()
        .enumerate()
        {
            let expected = sorted_rows(expected_rows(left, right, oracle));
            assert_eq!(expected.len(), outputs[index]);
            for mut request in [
                hql_request("join-diagnostic-hql", kind),
                typed_ir_request("join-diagnostic-ir", kind),
            ] {
                request.temporal =
                    Some(serde_json::from_value(json!({"valid_at":valid_at})).unwrap());
                let mut planned = request.clone();
                planned.explain = Some(ExplainV2::Plan);
                let QueryOutcomeV2::Plan(plan) =
                    storage.query_v2(access(), planned.clone()).unwrap()
                else {
                    panic!("Plan")
                };
                assert!(plan
                    .plan
                    .iter()
                    .all(|node| node.actual.is_none() && node.estimates.is_none()));
                assert_state(&storage, directory.path(), &before_plan);
                let denied = AccessContext {
                    principal: "no-namespace".into(),
                    namespace: "default".into(),
                };
                let error = storage.query_v2(denied, planned).unwrap_err();
                assert_eq!(error.code, "FORBIDDEN");
                assert_eq!(error.stage, "authorize");
                assert_state(&storage, directory.path(), &before_plan);
                cases.push((request, plan, expected.clone(), outputs[index] as u64));
            }
        }
        // Execution may legitimately publish lazily; establish that boundary once
        // before guarding normal/Analyze errors, never before the Plan control.
        storage.publish_generation().unwrap();
        for (request, plan, expected, output_count) in cases {
            let QueryOutcomeV2::Rows(normal) = storage.query_v2(access(), request.clone()).unwrap()
            else {
                panic!("rows")
            };
            assert!(normal.explain.is_none());
            assert_eq!(sorted_rows(normal.rows.clone()), expected);
            let mut analyzed_request = request.clone();
            analyzed_request.explain = Some(ExplainV2::Analyze);
            let QueryOutcomeV2::Rows(analyzed) =
                storage.query_v2(access(), analyzed_request).unwrap()
            else {
                panic!("Analyze")
            };
            assert_eq!(sorted_rows(analyzed.rows.clone()), expected);
            assert_eq!(analyzed.columns, normal.columns);
            assert_eq!(analyzed.semantics, normal.semantics);
            assert_eq!(analyzed.snapshot, normal.snapshot);
            assert_eq!(analyzed.completeness, normal.completeness);
            assert_eq!(analyzed.index_frontiers, normal.index_frontiers);
            assert_eq!(analyzed.cursor, normal.cursor);
            let mut actual = analyzed.explain.unwrap();
            assert_eq!(actual.plan_hash, plan.plan_hash);
            assert_eq!(actual.planner_version, plan.planner_version);
            assert_eq!(actual.plan.len(), 4);
            for (op, input, output) in [
                (QueryOpV2::NodeScan, 0, left.len() as u64),
                (QueryOpV2::RowScan, 0, right.len() as u64),
                (
                    QueryOpV2::Join,
                    (left.len() + right.len()) as u64,
                    output_count,
                ),
                (QueryOpV2::Project, output_count, output_count),
            ] {
                let nodes: Vec<_> = actual
                    .plan
                    .iter_mut()
                    .filter(|node| node.logical_op == op)
                    .collect();
                assert_eq!(nodes.len(), 1, "one {op:?}");
                let node = nodes.into_iter().next().unwrap();
                let counters = serde_json::to_value(node.actual.take().unwrap()).unwrap();
                for (name, count) in [("rows_in", input), ("rows_out", output)] {
                    assert_eq!(
                        counters[name],
                        json!({"unit":"rows","measurement":{"status":"measured","value":count}})
                    );
                }
                for (name, unit, reason) in [
                    ("bytes_read", "bytes", "not_instrumented"),
                    ("work_units", "work_units", "not_instrumented"),
                    ("memory_peak_bytes", "bytes", "not_instrumented"),
                    ("index_probes", "index_probes", "not_applicable"),
                    (
                        "distance_evaluations",
                        "distance_evaluations",
                        "not_applicable",
                    ),
                    ("graph_expansions", "graph_expansions", "not_applicable"),
                    ("spill_bytes", "bytes", "not_applicable"),
                ] {
                    assert_eq!(
                        counters[name],
                        json!({"unit":unit,"measurement":{"status":"unknown","reason":reason}})
                    );
                }
                assert_eq!(counters["elapsed_ns"]["unit"], "nanoseconds");
                if matches!(op, QueryOpV2::NodeScan | QueryOpV2::RowScan) {
                    assert_eq!(
                        counters["elapsed_ns"]["measurement"],
                        json!({"status":"unknown","reason":"not_instrumented"})
                    );
                    assert_eq!(counters["elapsed_scope"], "operator_execution");
                } else {
                    assert_eq!(counters["elapsed_ns"]["measurement"]["status"], "measured");
                    assert!(counters["elapsed_ns"]["measurement"]["value"]
                        .as_u64()
                        .is_some());
                    assert_eq!(counters["elapsed_scope"], "operator_execution");
                }
                assert_eq!(counters["clock_source"], "monotonic_instant");
                assert_eq!(counters["sampling"], "complete");
            }
            assert_eq!(actual.plan, plan.plan);
            for mode in [None, Some(ExplainV2::Analyze)] {
                let mut denied = request.clone();
                denied.explain = mode.clone();
                let before = state(&storage, directory.path());
                let error = storage
                    .query_v2(
                        AccessContext {
                            principal: "no-namespace".into(),
                            namespace: "default".into(),
                        },
                        denied,
                    )
                    .unwrap_err();
                assert_eq!(error.code, "FORBIDDEN");
                assert_eq!(error.stage, "authorize");
                assert_state(&storage, directory.path(), &before);
                let mut limited = request.clone();
                limited.explain = mode;
                limited.budget =
                    Some(serde_json::from_value(json!({"max_result_bytes":1})).unwrap());
                let before = state(&storage, directory.path());
                let error = storage.query_v2(access(), limited.clone()).unwrap_err();
                assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
                assert_state(&storage, directory.path(), &before);
                if output_count == 0 {
                    // The Storage boundary budgets the complete envelope, not only [].
                    limited.budget =
                        Some(serde_json::from_value(json!({"max_result_bytes":2})).unwrap());
                    let error = storage.query_v2(access(), limited.clone()).unwrap_err();
                    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
                    assert_eq!(error.detail.unwrap()["reason"], "output_limit");
                    assert_state(&storage, directory.path(), &before);
                    limited.budget =
                        Some(serde_json::from_value(json!({"max_result_bytes":1048576})).unwrap());
                    let QueryOutcomeV2::Rows(empty) = storage.query_v2(access(), limited).unwrap()
                    else {
                        panic!("empty rows")
                    };
                    assert!(empty.rows.is_empty());
                    assert_eq!(empty.completeness, normal.completeness);
                    assert_state(&storage, directory.path(), &before);
                }
            }
        }
    }
}

// Literal schemas are fixture expectations, not copied from another mode.
fn assert_join_metadata_schemas(
    plan: &genesis_block_native::query::hql2::ExplainResultV2,
    root_columns: &Value,
    join_columns: &Value,
) {
    assert_eq!(plan.plan.len(), 4);
    let find = |id: &str| plan.plan.iter().find(|n| n.id == id).unwrap();
    let root = find(&plan.root);
    assert_eq!(root.logical_op, QueryOpV2::Project);
    assert_eq!(root.inputs.len(), 1);
    assert_eq!(serde_json::to_value(&root.columns).unwrap(), *root_columns);
    let join = find(&root.inputs[0]);
    assert_eq!(join.logical_op, QueryOpV2::Join);
    assert_eq!(join.inputs.len(), 2);
    assert_ne!(join.inputs[0], join.inputs[1]);
    assert_eq!(find(&join.inputs[0]).logical_op, QueryOpV2::NodeScan);
    assert_eq!(find(&join.inputs[1]).logical_op, QueryOpV2::RowScan);
    assert_eq!(serde_json::to_value(&join.columns).unwrap(), *join_columns);
}

fn qualify_join_result_metadata(
    storage: &Storage,
    path: &Path,
    mut request: QueryRequestV2,
    root_columns: Value,
    join_columns: Value,
    expected: Vec<BTreeMap<String, V>>,
) {
    use sha2::{Digest, Sha256};
    // Guard optional-null member presence; JSON indexing alone misses absence.
    assert_ne!(json!({}).get("cursor"), Some(&Value::Null));
    assert_eq!(json!({"cursor":null}).get("cursor"), Some(&Value::Null));
    let mut identity = Sha256::new();
    identity.update(b"genesis.api.v2.database:");
    identity.update(storage.verifying_key.as_bytes());
    let database_id = hex::encode(identity.finalize());
    // Rows use insertion time. One post-setup value is reused before/after Plan.
    let valid_at = chrono::Utc::now().to_rfc3339();
    request.temporal = Some(serde_json::from_value(json!({"valid_at":valid_at})).unwrap());
    let before = state(storage, path);
    let mut planned = request.clone();
    planned.explain = Some(ExplainV2::Plan);
    let QueryOutcomeV2::Plan(plan) = storage.query_v2(access(), planned).unwrap() else {
        panic!("Plan")
    };
    assert_eq!(plan.request_id, request.request_id);
    assert_eq!(plan.catalog.policy_revision, 1);
    assert_eq!(plan.catalog.observed_frontier, before.0);
    assert_join_metadata_schemas(&plan, &root_columns, &join_columns);
    assert!(plan
        .plan
        .iter()
        .all(|n| n.actual.is_none() && n.estimates.is_none()));
    assert_state(storage, path, &before);
    let generation = storage.publish_generation().unwrap();
    let execution_catalog_frontier = storage.stable_frontier();
    assert_eq!(execution_catalog_frontier, generation.publication_seq);
    assert_eq!(generation.acl_revision, 1);
    request.temporal = Some(
        serde_json::from_value(
            json!({"valid_at":valid_at,"tx_as_of":generation.wal_frontier.to_string()}),
        )
        .unwrap(),
    );
    let snapshot = json!({"database_id":database_id,"tx":generation.wal_frontier.to_string(),"valid_at":valid_at,"catalog_generation":generation.generation_id.to_string(),"policy_version":generation.acl_revision.to_string()});
    for mode in [None, Some(ExplainV2::Analyze)] {
        request.explain = mode.clone();
        let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request.clone()).unwrap()
        else {
            panic!("rows")
        };
        let actual = serde_json::to_value(&result).unwrap();
        assert_eq!(actual["request_id"], request.request_id);
        assert_eq!(actual["columns"], root_columns);
        assert_eq!(actual["snapshot"], snapshot);
        assert_eq!(
            actual["semantics"],
            json!({"candidate_search":"exact","distance_fidelity":"not_applicable","index_coverage":"not_applicable","scope":"whole_input"})
        );
        assert_eq!(
            actual["completeness"],
            json!({"status":"complete","reason":null,"eligible_count_known":true})
        );
        assert_eq!(actual["index_frontiers"], json!({}));
        assert_eq!(actual.get("cursor"), Some(&Value::Null));
        assert_eq!(
            sorted_rows(result.rows.clone()),
            sorted_rows(expected.clone())
        );
        if mode.is_none() {
            assert!(result.explain.is_none());
        } else {
            let explain = result.explain.unwrap();
            assert_join_metadata_schemas(&explain, &root_columns, &join_columns);
            assert_eq!(explain.request_id, request.request_id);
            assert_eq!(explain.catalog.policy_revision, 1);
            assert_eq!(
                explain.catalog.observed_frontier,
                execution_catalog_frontier
            );
            for node in explain.plan {
                assert!(node.estimates.is_none());
                let counters = serde_json::to_value(node.actual.unwrap()).unwrap();
                for name in ["rows_in", "rows_out"] {
                    assert_eq!(counters[name]["unit"], "rows");
                    assert_eq!(counters[name]["measurement"]["status"], "measured");
                    assert!(counters[name]["measurement"]["value"].as_u64().is_some());
                }
                for (name, unit, reason) in [
                    ("bytes_read", "bytes", "not_instrumented"),
                    ("work_units", "work_units", "not_instrumented"),
                    ("memory_peak_bytes", "bytes", "not_instrumented"),
                    ("index_probes", "index_probes", "not_applicable"),
                    (
                        "distance_evaluations",
                        "distance_evaluations",
                        "not_applicable",
                    ),
                    ("graph_expansions", "graph_expansions", "not_applicable"),
                    ("spill_bytes", "bytes", "not_applicable"),
                ] {
                    assert_eq!(
                        counters[name],
                        json!({"unit":unit,"measurement":{"status":"unknown","reason":reason}})
                    );
                }
                assert_eq!(counters["elapsed_ns"]["unit"], "nanoseconds");
                if matches!(node.logical_op, QueryOpV2::NodeScan | QueryOpV2::RowScan) {
                    assert_eq!(
                        counters["elapsed_ns"]["measurement"],
                        json!({"status":"unknown","reason":"not_instrumented"})
                    );
                } else {
                    assert_eq!(counters["elapsed_ns"]["measurement"]["status"], "measured");
                    assert!(counters["elapsed_ns"]["measurement"]["value"]
                        .as_u64()
                        .is_some());
                }
                assert_eq!(counters["elapsed_scope"], "operator_execution");
                assert_eq!(counters["clock_source"], "monotonic_instant");
                assert_eq!(counters["sampling"], "complete");
            }
        }
    }
}

#[test]
fn storage_join_result_metadata_has_independent_fixture_expectations() {
    let full_left = [
        JoinKey::Number(1),
        JoinKey::Number(1),
        JoinKey::Missing,
        JoinKey::Number(2),
        JoinKey::JsonNull,
    ];
    let full_right = [
        JoinKey::Number(1),
        JoinKey::Number(1),
        JoinKey::JsonNull,
        JoinKey::Number(99),
    ];
    let j1 = V::Json(json!(1));
    let j2 = V::Json(json!(2));
    let jnull = V::Json(Value::Null);
    for (case, left, right, counts) in [
        ("full", &full_left[..], &full_right[..], [5, 7, 3, 2]),
        ("left-empty", &[][..], &full_right[..], [0, 0, 0, 0]),
        ("right-empty", &full_left[..], &[][..], [0, 5, 0, 5]),
        ("both-empty", &[][..], &[][..], [0, 0, 0, 0]),
    ] {
        for (index, (kind, oracle_kind)) in [
            ("inner", reference::JoinKind::Inner),
            ("left", reference::JoinKind::Left),
            ("semi", reference::JoinKind::Semi),
            ("anti", reference::JoinKind::Anti),
        ]
        .into_iter()
        .enumerate()
        {
            let one_side = matches!(kind, "semi" | "anti");
            let pair = |left: V, right: V| {
                let mut row = BTreeMap::from([("left_key".into(), left)]);
                if !one_side {
                    row.insert("right_key".into(), right);
                }
                row
            };
            // Hand-derived bags, independent of the production/P7 join loops.
            let literal = match (case, kind) {
                ("full", "inner") => vec![
                    pair(j1.clone(), j1.clone()),
                    pair(j1.clone(), j1.clone()),
                    pair(j1.clone(), j1.clone()),
                    pair(j1.clone(), j1.clone()),
                    pair(jnull.clone(), jnull.clone()),
                ],
                ("full", "left") => vec![
                    pair(j1.clone(), j1.clone()),
                    pair(j1.clone(), j1.clone()),
                    pair(j1.clone(), j1.clone()),
                    pair(j1.clone(), j1.clone()),
                    pair(jnull.clone(), jnull.clone()),
                    pair(V::Null, V::Null),
                    pair(j2.clone(), V::Null),
                ],
                ("full", "semi") => vec![
                    pair(j1.clone(), V::Null),
                    pair(j1.clone(), V::Null),
                    pair(jnull.clone(), V::Null),
                ],
                ("full", "anti") => vec![pair(V::Null, V::Null), pair(j2.clone(), V::Null)],
                ("right-empty", "left" | "anti") => vec![
                    pair(j1.clone(), V::Null),
                    pair(j1.clone(), V::Null),
                    pair(V::Null, V::Null),
                    pair(j2.clone(), V::Null),
                    pair(jnull.clone(), V::Null),
                ],
                _ => vec![],
            };
            assert_eq!(literal.len(), counts[index]);
            assert_eq!(
                sorted_rows(expected_rows(left, right, oracle_kind)),
                sorted_rows(literal.clone())
            );
            let mut join_columns = vec![json!({"name":"n","type":"Entity","nullable":false})];
            let mut root_columns =
                vec![json!({"name":"left_key","type":"Nullable<Json>","nullable":true})];
            if !one_side {
                join_columns.push(if kind == "left" {
                    json!({"name":"r","type":"Nullable<Entity>","nullable":true})
                } else {
                    json!({"name":"r","type":"Entity","nullable":false})
                });
                root_columns
                    .push(json!({"name":"right_key","type":"Nullable<Json>","nullable":true}));
            }
            for hql in [false, true] {
                let dir = TempDir::new().unwrap();
                let storage = open(dir.path());
                populate_join(&storage, left, right);
                join_policy(&storage);
                let id = format!("join-metadata-{case}-{kind}-{hql}");
                let request = if hql {
                    hql_request(&id, kind)
                } else {
                    typed_ir_request(&id, kind)
                };
                qualify_join_result_metadata(
                    &storage,
                    dir.path(),
                    request,
                    json!(root_columns),
                    json!(join_columns),
                    literal.clone(),
                );
            }
        }
    }
}
