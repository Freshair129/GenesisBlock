use genesis_block_native::{
    query::hql2::{QueryOutcomeV2, QueryResultV2},
    uee_v2::QueryRequestV2,
    AccessContext, EdgeInput, NodeInput, OpenOptions, Storage,
};
#[allow(dead_code)]
#[path = "support/hql2_graph_reference.rs"]
mod p7_graph;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use tempfile::TempDir;

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
        principal: "history-change-reader".into(),
        namespace: "default".into(),
    }
}

fn run_hql(
    storage: &Storage,
    request_id: &str,
    hql: &str,
) -> Result<QueryResultV2, genesis_block_native::query::hql2::QueryErrorV2> {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":hql,
        "params":{}
    }))
    .unwrap();
    match storage.query_v2(access(), request)? {
        QueryOutcomeV2::Rows(result) => Ok(result),
        QueryOutcomeV2::Plan(_) => panic!("read query must return rows"),
    }
}

fn run_hql_with_budget(
    storage: &Storage,
    request_id: &str,
    hql: &str,
    budget: Value,
) -> Result<QueryResultV2, genesis_block_native::query::hql2::QueryErrorV2> {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":hql,
        "params":{},
        "budget":budget
    }))
    .unwrap();
    match storage.query_v2(access(), request)? {
        QueryOutcomeV2::Rows(result) => Ok(result),
        QueryOutcomeV2::Plan(_) => panic!("read query must return rows"),
    }
}

fn run_ir_history_source(
    storage: &Storage,
    request_id: &str,
    valid_at: &str,
    kind: &str,
    record_id: &str,
    tx_as_of: Option<&str>,
) -> Result<QueryResultV2, genesis_block_native::query::hql2::QueryErrorV2> {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "temporal":{"valid_at":valid_at,"tx_as_of":tx_as_of},
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{
                "id":"history",
                "op":"HistoryScan",
                "inputs":[],
                "config":{
                    "kind":kind,
                    "id":{"literal":record_id,"type":"Utf8"},
                    "as":"h"
                }
            }],
            "root":"history",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    match storage.query_v2(access(), request)? {
        QueryOutcomeV2::Rows(result) => Ok(result),
        QueryOutcomeV2::Plan(_) => panic!("read query must return rows"),
    }
}

fn run_ir_history(storage: &Storage, request_id: &str, valid_at: &str) -> QueryResultV2 {
    run_ir_history_source(storage, request_id, valid_at, "node", "history:one", None).unwrap()
}

fn run_ir_changes(storage: &Storage, request_id: &str, after_seq: u64) -> QueryResultV2 {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{
                "id":"changes",
                "op":"ChangeScan",
                "inputs":[],
                "config":{"after_seq":after_seq.to_string(),"as":"c"}
            }],
            "root":"changes",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    match storage.query_v2(access(), request).unwrap() {
        QueryOutcomeV2::Rows(result) => result,
        QueryOutcomeV2::Plan(_) => panic!("read query must return rows"),
    }
}

fn add_node(storage: &Storage) {
    storage
        .add_node(NodeInput {
            id: Some("history:one".into()),
            labels: vec!["Document".into()],
            props: Some(json!({"title":"original"})),
            embedding: None,
            lang: None,
            valid_from: Some("2010-01-01T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn tagged(
    row: &std::collections::BTreeMap<
        String,
        genesis_block_native::query::hql2::value::QueryValueV2,
    >,
    field: &str,
) -> Value {
    serde_json::to_value(&row[field]).unwrap()
}

fn record_revision_mutations(storage: &Storage, kind: &str) -> Vec<Value> {
    storage
        .events_since_seq(0)
        .into_iter()
        .filter_map(|signed| serde_json::to_value(signed.event).ok())
        .filter_map(|event| event.get("Transaction").cloned())
        .filter_map(|transaction| transaction.get("record_revision_transaction").cloned())
        .filter_map(|revisions| revisions.get("mutations").cloned())
        .filter_map(|mutations| mutations.as_array().cloned())
        .flatten()
        .filter(|mutation| mutation.get("kind").and_then(Value::as_str) == Some(kind))
        .collect()
}

#[test]
fn history_scan_reads_closed_revisions_and_hydrates_the_exact_revision() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let first_tx = storage.stable_frontier();
    storage
        .supersede_node(
            "history:one".into(),
            Some(json!({"title":"replacement"})),
            None,
        )
        .unwrap();
    let second_tx = storage.stable_frontier();

    let result = run_hql(
        &storage,
        "history-closed-revision",
        "USE default AT VALID \"2020-01-01T00:00:00Z\" HISTORY NODE \"history:one\" AS h |> ORDER BY h.tx_from ASC NULLS LAST |> RETURN h.tx_from AS tx, h.tx_to AS tx_to, h.operation AS operation, h.revision_id AS revision, prop(h.subject, \"title\") AS title",
    )
    .expect("HistoryScan should enumerate retained closed revisions");

    assert_eq!(result.rows.len(), 2);
    let first = &result.rows[0];
    let second = &result.rows[1];
    for row in [first, second] {
        assert_eq!(tagged(row, "tx")["type"], "DecimalU64");
        assert_eq!(tagged(row, "operation")["value"], "upsert");
        assert_eq!(tagged(row, "title")["value"], "original");
        assert!(!tagged(row, "revision")["value"]
            .as_str()
            .unwrap()
            .is_empty());
    }
    assert_eq!(tagged(first, "title"), tagged(second, "title"));
    assert_ne!(tagged(first, "revision"), tagged(second, "revision"));
    assert!(
        tagged(first, "tx")["value"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            < tagged(second, "tx")["value"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
    );
    assert_ne!(tagged(first, "tx_to")["type"], "Null");
    assert_ne!(tagged(second, "tx_to")["type"], "Null");

    let hql_typed = run_hql(
        &storage,
        "history-hql-ir-parity-hql",
        "USE default AT VALID \"2020-01-01T00:00:00Z\" HISTORY NODE \"history:one\" AS h |> RETURN h",
    )
    .unwrap();
    let ir_typed = run_ir_history(&storage, "history-hql-ir-parity-ir", "2020-01-01T00:00:00Z");
    assert_eq!(hql_typed.columns, ir_typed.columns);
    assert_eq!(hql_typed.rows, ir_typed.rows);

    let mutations = record_revision_mutations(&storage, "node");
    assert_eq!(second_tx - first_tx, 2);
    assert_eq!(mutations.len(), 3, "WAL mutations: {mutations:#?}");
    let mut p7_revisions = Vec::with_capacity(mutations.len());
    for (index, mutation) in mutations.iter().enumerate() {
        let valid_from =
            chrono::DateTime::parse_from_rfc3339(mutation["valid_from"].as_str().unwrap())
                .unwrap()
                .timestamp_micros();
        let valid_to = mutation["valid_to"].as_str().map(|value| {
            chrono::DateTime::parse_from_rfc3339(value)
                .unwrap()
                .timestamp_micros()
        });
        p7_revisions.push(p7_graph::Revision {
            entity: p7_graph::EntityRef {
                namespace: mutation["namespace"].as_str().unwrap().into(),
                kind: p7_graph::Kind::Node,
                id: mutation["id"].as_str().unwrap().into(),
                revision: mutation["revision_id"].as_str().unwrap().into(),
            },
            transaction: p7_graph::Interval {
                start: first_tx + index as u64,
                end: [Some(first_tx + 1), Some(second_tx), None][index],
            },
            valid: p7_graph::Interval {
                start: valid_from,
                end: valid_to,
            },
            retracted: false,
            fields: p7_graph::Fields::new(),
            data: p7_graph::RecordData::Plain,
        });
    }
    let node = p7_graph::Identity {
        namespace: "default".into(),
        kind: p7_graph::Kind::Node,
        id: "history:one".into(),
    };
    let p7_catalog = p7_graph::Catalog {
        frontier: second_tx,
        history: BTreeMap::from([(
            p7_graph::Kind::Node,
            p7_graph::HistoryCapability {
                horizon: 0,
                available: true,
            },
        )]),
        revisions: p7_revisions,
    };
    let p7_plan = p7_graph::Plan {
        source: p7_graph::Source::HistoryScan {
            kind: p7_graph::Kind::Node,
            alias: "h".into(),
            predicate: p7_graph::Predicate {
                id: Some("history:one".into()),
                ..p7_graph::Predicate::default()
            },
            transactions: p7_graph::Interval {
                start: 0,
                end: second_tx.checked_add(1),
            },
            valid: p7_graph::Interval {
                start: chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
                    .unwrap()
                    .timestamp_micros(),
                end: Some(
                    chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
                        .unwrap()
                        .timestamp_micros()
                        + 1,
                ),
            },
        },
        stages: vec![],
    };
    let p7_view = p7_graph::View {
        namespace: "default".into(),
        transaction: second_tx,
        valid_at: chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
            .unwrap()
            .timestamp_micros(),
        permissions: p7_graph::Permissions {
            read: BTreeSet::from([node]),
            annotation_body: BTreeSet::new(),
        },
    };
    let p7_result =
        p7_graph::execute(&p7_catalog, &p7_view, &p7_plan, p7_graph::Limits::default()).unwrap();
    assert_eq!(hql_typed.rows.len(), p7_result.rows.len());
    for (actual, expected) in hql_typed.rows.iter().zip(&p7_result.rows) {
        let p7_graph::Binding::Entity(entity) = &expected["h"] else {
            panic!("P7 HistoryScan binds node revision identities")
        };
        let revision = p7_catalog
            .revisions
            .iter()
            .find(|revision| &revision.entity == entity)
            .unwrap();
        let history = tagged(actual, "h")["value"].clone();
        let subject = &history["subject"];
        assert_eq!(
            subject["namespace"].as_str(),
            Some(entity.namespace.as_str())
        );
        assert_eq!(subject["kind"].as_str(), Some("node"));
        assert_eq!(subject["id"].as_str(), Some(entity.id.as_str()));
        assert_eq!(subject["revision"].as_str(), Some(entity.revision.as_str()));
        assert_eq!(history["operation"].as_str(), Some("upsert"));
        assert_eq!(
            history["tx_from"].as_str().unwrap().parse::<u64>().unwrap(),
            revision.transaction.start
        );
        assert_eq!(
            history["tx_to"]
                .as_str()
                .map(|value| value.parse::<u64>().unwrap()),
            revision.transaction.end
        );
        assert_eq!(
            chrono::DateTime::parse_from_rfc3339(history["valid_from"].as_str().unwrap())
                .unwrap()
                .timestamp_micros(),
            revision.valid.start
        );
        assert_eq!(
            history["valid_to"]
                .as_str()
                .map(|value| chrono::DateTime::parse_from_rfc3339(value)
                    .unwrap()
                    .timestamp_micros()),
            revision.valid.end
        );
    }
}

#[test]
fn history_scan_matches_p7_for_edge_revisions_and_endpoint_acl() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let source_tx = storage.stable_frontier();
    storage
        .add_node(NodeInput {
            id: Some("history:two".into()),
            labels: vec!["Document".into()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: Some("2010-01-01T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    let target_tx = storage.stable_frontier();
    storage
        .add_edge(EdgeInput {
            id: Some("history:edge".into()),
            from: "history:one".into(),
            to: "history:two".into(),
            rel: "LINK".into(),
            props: Some(json!({"weight":1})),
            valid_from: Some("2010-01-01T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
    let edge_from = storage.stable_frontier();
    storage
        .retract_edge("history:edge".into(), Some("2020-02-01T00:00:00Z".into()))
        .unwrap();
    let frontier = storage.stable_frontier();
    let valid_at = "2020-01-15T00:00:00Z";

    let hql = format!(
        "USE default AT VALID \"{valid_at}\" HISTORY EDGE \"history:edge\" AS h |> RETURN h"
    );
    let hql_result = run_hql(&storage, "history-edge-p7-hql", &hql).unwrap();
    let ir_result = run_ir_history_source(
        &storage,
        "history-edge-p7-ir",
        valid_at,
        "edge",
        "history:edge",
        None,
    )
    .unwrap();
    assert_eq!(hql_result.columns, ir_result.columns);
    assert_eq!(hql_result.rows, ir_result.rows);

    let node_mutations = record_revision_mutations(&storage, "node");
    assert_eq!(node_mutations.len(), 2);
    let mut revisions = Vec::with_capacity(4);
    for (index, mutation) in node_mutations.iter().enumerate() {
        let valid_from =
            chrono::DateTime::parse_from_rfc3339(mutation["valid_from"].as_str().unwrap())
                .unwrap()
                .timestamp_micros();
        revisions.push(p7_graph::Revision {
            entity: p7_graph::EntityRef {
                namespace: mutation["namespace"].as_str().unwrap().into(),
                kind: p7_graph::Kind::Node,
                id: mutation["id"].as_str().unwrap().into(),
                revision: mutation["revision_id"].as_str().unwrap().into(),
            },
            transaction: p7_graph::Interval {
                start: [source_tx, target_tx][index],
                end: None,
            },
            valid: p7_graph::Interval {
                start: valid_from,
                end: mutation["valid_to"].as_str().map(|value| {
                    chrono::DateTime::parse_from_rfc3339(value)
                        .unwrap()
                        .timestamp_micros()
                }),
            },
            retracted: false,
            fields: p7_graph::Fields::new(),
            data: p7_graph::RecordData::Plain,
        });
    }
    let edge_mutations = record_revision_mutations(&storage, "edge");
    assert_eq!(edge_mutations.len(), 2);
    for (index, mutation) in edge_mutations.iter().enumerate() {
        let valid_from =
            chrono::DateTime::parse_from_rfc3339(mutation["valid_from"].as_str().unwrap())
                .unwrap()
                .timestamp_micros();
        let valid_to = mutation["valid_to"].as_str().map(|value| {
            chrono::DateTime::parse_from_rfc3339(value)
                .unwrap()
                .timestamp_micros()
        });
        revisions.push(p7_graph::Revision {
            entity: p7_graph::EntityRef {
                namespace: mutation["namespace"].as_str().unwrap().into(),
                kind: p7_graph::Kind::Edge,
                id: mutation["id"].as_str().unwrap().into(),
                revision: mutation["revision_id"].as_str().unwrap().into(),
            },
            transaction: p7_graph::Interval {
                start: [edge_from, frontier][index],
                end: [Some(frontier), None][index],
            },
            valid: p7_graph::Interval {
                start: valid_from,
                end: valid_to,
            },
            retracted: mutation["operation"].as_str() == Some("retract"),
            fields: p7_graph::Fields::new(),
            data: p7_graph::RecordData::Edge {
                source: p7_graph::Identity {
                    namespace: "default".into(),
                    kind: p7_graph::Kind::Node,
                    id: mutation["payload"]["from"].as_str().unwrap().into(),
                },
                target: p7_graph::Identity {
                    namespace: "default".into(),
                    kind: p7_graph::Kind::Node,
                    id: mutation["payload"]["to"].as_str().unwrap().into(),
                },
                relation: mutation["payload"]["rel"].as_str().unwrap().into(),
            },
        });
    }

    let edge = p7_graph::Identity {
        namespace: "default".into(),
        kind: p7_graph::Kind::Edge,
        id: "history:edge".into(),
    };
    let source = p7_graph::Identity {
        namespace: "default".into(),
        kind: p7_graph::Kind::Node,
        id: "history:one".into(),
    };
    let target = p7_graph::Identity {
        namespace: "default".into(),
        kind: p7_graph::Kind::Node,
        id: "history:two".into(),
    };
    let p7_catalog = p7_graph::Catalog {
        frontier,
        history: BTreeMap::from([
            (
                p7_graph::Kind::Node,
                p7_graph::HistoryCapability {
                    horizon: 0,
                    available: true,
                },
            ),
            (
                p7_graph::Kind::Edge,
                p7_graph::HistoryCapability {
                    horizon: 0,
                    available: true,
                },
            ),
        ]),
        revisions,
    };
    let valid_at = chrono::DateTime::parse_from_rfc3339(valid_at)
        .unwrap()
        .timestamp_micros();
    let p7_plan = p7_graph::Plan {
        source: p7_graph::Source::HistoryScan {
            kind: p7_graph::Kind::Edge,
            alias: "h".into(),
            predicate: p7_graph::Predicate {
                id: Some("history:edge".into()),
                ..p7_graph::Predicate::default()
            },
            transactions: p7_graph::Interval {
                start: 0,
                end: frontier.checked_add(1),
            },
            valid: p7_graph::Interval {
                start: valid_at,
                end: valid_at.checked_add(1),
            },
        },
        stages: vec![],
    };
    let p7_view = p7_graph::View {
        namespace: "default".into(),
        transaction: frontier,
        valid_at,
        permissions: p7_graph::Permissions {
            read: BTreeSet::from([source, target, edge]),
            annotation_body: BTreeSet::new(),
        },
    };
    let p7_result =
        p7_graph::execute(&p7_catalog, &p7_view, &p7_plan, p7_graph::Limits::default()).unwrap();
    assert_eq!(hql_result.rows.len(), p7_result.rows.len());
    for (actual, expected) in hql_result.rows.iter().zip(&p7_result.rows) {
        let p7_graph::Binding::Entity(entity) = &expected["h"] else {
            panic!("P7 HistoryScan binds edge revision identities")
        };
        let revision = p7_catalog
            .revisions
            .iter()
            .find(|revision| &revision.entity == entity)
            .unwrap();
        let history = tagged(actual, "h")["value"].clone();
        let subject = &history["subject"];
        assert_eq!(
            subject["namespace"].as_str(),
            Some(entity.namespace.as_str())
        );
        assert_eq!(subject["kind"].as_str(), Some("edge"));
        assert_eq!(subject["id"].as_str(), Some(entity.id.as_str()));
        assert_eq!(subject["revision"].as_str(), Some(entity.revision.as_str()));
        assert_eq!(
            history["operation"].as_str(),
            Some(if revision.retracted {
                "retract"
            } else {
                "upsert"
            })
        );
        assert_eq!(
            history["tx_from"].as_str().unwrap().parse::<u64>().unwrap(),
            revision.transaction.start
        );
        assert_eq!(
            history["tx_to"]
                .as_str()
                .map(|value| value.parse::<u64>().unwrap()),
            revision.transaction.end
        );
        assert_eq!(
            chrono::DateTime::parse_from_rfc3339(history["valid_from"].as_str().unwrap())
                .unwrap()
                .timestamp_micros(),
            revision.valid.start
        );
        assert_eq!(
            history["valid_to"]
                .as_str()
                .map(|value| chrono::DateTime::parse_from_rfc3339(value)
                    .unwrap()
                    .timestamp_micros()),
            revision.valid.end
        );
    }
}

#[test]
fn transaction_time_uses_one_selected_frontier_for_sources_hydration_and_snapshot() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let selected_tx: i64 = projection
        .query_row(
            "SELECT tx_from FROM hql2_record_revisions
             WHERE kind='node' AND record_id='history:one' AND operation='upsert'
             ORDER BY tx_from LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let selected_tx = u64::try_from(selected_tx).unwrap();
    let selected_tx_text = selected_tx.to_string();
    storage
        .supersede_node(
            "history:one".into(),
            Some(json!({"title":"replacement"})),
            None,
        )
        .unwrap();

    let history_hql = format!(
        "USE default AT TX {selected_tx} AT VALID \"2020-01-01T00:00:00Z\" HISTORY NODE \"history:one\" AS h |> RETURN h"
    );
    let history_hql_result = run_hql(&storage, "history-at-tx-hql", &history_hql).unwrap();
    let history_ir_result = run_ir_history_source(
        &storage,
        "history-at-tx-ir",
        "2020-01-01T00:00:00Z",
        "node",
        "history:one",
        Some(&selected_tx_text),
    )
    .unwrap();
    assert_eq!(history_hql_result.snapshot.tx, selected_tx_text);
    assert_eq!(history_ir_result.snapshot.tx, selected_tx_text);
    assert_eq!(history_hql_result.rows, history_ir_result.rows);
    assert_eq!(history_hql_result.rows.len(), 1);

    let changes_hql =
        format!("USE default AT TX {selected_tx} CHANGES SINCE {selected_tx} AS c |> RETURN c");
    let changes_hql_result = run_hql(&storage, "changes-at-tx-hql", &changes_hql).unwrap();
    let changes_ir_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"changes-at-tx-ir",
        "namespace":"default",
        "temporal":{"tx_as_of":selected_tx_text},
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{"id":"changes","op":"ChangeScan","inputs":[],"config":{"after_seq":selected_tx_text,"as":"c"}}],
            "root":"changes",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(changes_ir_result) =
        storage.query_v2(access(), changes_ir_request).unwrap()
    else {
        panic!("read query must return rows");
    };
    assert_eq!(changes_hql_result.snapshot.tx, selected_tx_text);
    assert_eq!(changes_ir_result.snapshot.tx, selected_tx_text);
    assert_eq!(changes_hql_result.columns, changes_ir_result.columns);
    assert_eq!(changes_hql_result.rows, changes_ir_result.rows);
    assert!(changes_hql_result.rows.is_empty());

    let scan_hql = format!(
        "USE default AT TX {selected_tx} AT VALID \"2020-01-01T00:00:00Z\" FROM NODES Document AS n |> RETURN n.id AS id, prop(n, \"title\") AS title"
    );
    let scan_hql_result = run_hql(&storage, "node-at-tx-hql", &scan_hql).unwrap();
    let scan_ir_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"node-at-tx-ir",
        "namespace":"default",
        "temporal":{"valid_at":"2020-01-01T00:00:00Z","tx_as_of":selected_tx_text},
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"n","label":"Document"}},
                {"id":"project","op":"Project","inputs":["scan"],"config":{"fields":[
                    {"as":"id","expression":{"field":{"alias":"n","path":["id"]}}},
                    {"as":"title","expression":{"call":"prop","args":[
                        {"field":{"alias":"n","path":[]}},
                        {"type":"Utf8","literal":"title"}
                    ]}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(scan_ir_result) = storage.query_v2(access(), scan_ir_request).unwrap()
    else {
        panic!("read query must return rows");
    };
    assert_eq!(scan_hql_result.snapshot.tx, selected_tx_text);
    assert_eq!(scan_ir_result.snapshot.tx, selected_tx_text);
    assert_eq!(scan_hql_result.columns, scan_ir_result.columns);
    assert_eq!(scan_hql_result.rows, scan_ir_result.rows);
    assert_eq!(scan_hql_result.rows.len(), 1);
    assert_eq!(
        tagged(&scan_hql_result.rows[0], "title")["value"],
        "original"
    );

    let current = run_hql(
        &storage,
        "node-at-current-tx-hql",
        "USE default FROM NODES Document AS n |> RETURN prop(n, \"title\") AS title",
    )
    .unwrap();
    assert_eq!(tagged(&current.rows[0], "title")["value"], "replacement");
    assert!(current.snapshot.tx.parse::<u64>().unwrap() > selected_tx);

    let future_tx = storage.stable_frontier() + 1;
    let future_query =
        format!("USE default AT TX {future_tx} FROM NODES Document AS n |> RETURN n.id AS id");
    let future_error = run_hql(&storage, "future-transaction-time", &future_query).unwrap_err();
    assert_eq!(future_error.code, "BIND_ERROR");
    let horizon_error = run_hql(
        &storage,
        "transaction-time-below-horizon",
        "USE default AT TX 0 FROM NODES Document AS n |> RETURN n.id AS id",
    )
    .unwrap_err();
    assert_eq!(horizon_error.code, "BEYOND_HORIZON");
}

#[test]
fn history_scan_reads_vector_revisions_with_hql_ir_and_p7_parity() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    storage
        .add_vector("history:one".into(), "default".into(), vec![0.5, 0.75])
        .unwrap();
    let first_tx = storage.stable_frontier();
    storage
        .add_vector("history:one".into(), "default".into(), vec![0.25, 0.5])
        .unwrap();
    let second_tx = storage.stable_frontier();
    assert!(first_tx < second_tx);

    let vector_id = serde_json::to_string(&("history:one", "default")).unwrap();
    let hql_id = serde_json::to_string(&vector_id).unwrap();
    let selected_tx = second_tx;
    let selected_tx_text = selected_tx.to_string();
    let hql = format!(
        "USE default AT TX {selected_tx} AT VALID \"2020-01-01T00:00:00Z\" HISTORY VECTOR {hql_id} AS h |> RETURN h"
    );
    let hql_result = run_hql(&storage, "history-vector-hql", &hql)
        .expect("HQL HistoryScan should support durable vector identities");
    let ir_result = run_ir_history_source(
        &storage,
        "history-vector-ir",
        "2020-01-01T00:00:00Z",
        "vector",
        &vector_id,
        Some(&selected_tx_text),
    )
    .expect("typed-IR HistoryScan should support durable vector identities");

    assert_eq!(hql_result.columns, ir_result.columns);
    assert_eq!(hql_result.rows, ir_result.rows);
    assert_eq!(hql_result.snapshot.tx, selected_tx_text);
    assert_eq!(hql_result.rows.len(), 2);

    let mutations = record_revision_mutations(&storage, "vector");
    assert_eq!(mutations.len(), 2);
    assert!(mutations.iter().all(|mutation| mutation["id"] == vector_id));
    let owner = p7_graph::Identity {
        namespace: "default".into(),
        kind: p7_graph::Kind::Node,
        id: "history:one".into(),
    };
    let mut p7_revisions = vec![p7_graph::Revision {
        entity: p7_graph::EntityRef {
            namespace: "default".into(),
            kind: p7_graph::Kind::Node,
            id: "history:one".into(),
            revision: "fixture-owner".into(),
        },
        transaction: p7_graph::Interval {
            start: 0,
            end: None,
        },
        valid: p7_graph::Interval {
            start: i64::MIN,
            end: None,
        },
        retracted: false,
        fields: p7_graph::Fields::new(),
        data: p7_graph::RecordData::Plain,
    }];
    for (index, mutation) in mutations.iter().enumerate() {
        let valid_from =
            chrono::DateTime::parse_from_rfc3339(mutation["valid_from"].as_str().unwrap())
                .unwrap()
                .timestamp_micros();
        let valid_to = mutation["valid_to"].as_str().map(|value| {
            chrono::DateTime::parse_from_rfc3339(value)
                .unwrap()
                .timestamp_micros()
        });
        let tx_from = [first_tx, second_tx][index];
        p7_revisions.push(p7_graph::Revision {
            entity: p7_graph::EntityRef {
                namespace: mutation["namespace"].as_str().unwrap().into(),
                kind: p7_graph::Kind::Vector,
                id: mutation["id"].as_str().unwrap().into(),
                revision: mutation["revision_id"].as_str().unwrap().into(),
            },
            transaction: p7_graph::Interval {
                start: tx_from,
                end: [Some(second_tx), None][index],
            },
            valid: p7_graph::Interval {
                start: valid_from,
                end: valid_to,
            },
            retracted: false,
            fields: p7_graph::Fields::new(),
            data: p7_graph::RecordData::Vector {
                owner: owner.clone(),
            },
        });
    }
    let p7_catalog = p7_graph::Catalog {
        frontier: selected_tx,
        history: BTreeMap::from([(
            p7_graph::Kind::Vector,
            p7_graph::HistoryCapability {
                horizon: 0,
                available: true,
            },
        )]),
        revisions: p7_revisions,
    };
    let valid_at = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
        .unwrap()
        .timestamp_micros();
    let p7_view = p7_graph::View {
        namespace: "default".into(),
        transaction: selected_tx,
        valid_at,
        permissions: p7_graph::Permissions {
            read: BTreeSet::from([owner]),
            annotation_body: BTreeSet::new(),
        },
    };
    let p7_plan = p7_graph::Plan {
        source: p7_graph::Source::HistoryScan {
            kind: p7_graph::Kind::Vector,
            alias: "h".into(),
            predicate: p7_graph::Predicate {
                id: Some(vector_id.clone()),
                ..p7_graph::Predicate::default()
            },
            transactions: p7_graph::Interval {
                start: 0,
                end: selected_tx.checked_add(1),
            },
            valid: p7_graph::Interval {
                start: valid_at,
                end: valid_at.checked_add(1),
            },
        },
        stages: vec![],
    };
    let p7_result =
        p7_graph::execute(&p7_catalog, &p7_view, &p7_plan, p7_graph::Limits::default()).unwrap();
    assert_eq!(hql_result.rows.len(), p7_result.rows.len());
    let database_id = tagged(&hql_result.rows[0], "h")["value"]["subject"]["database_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!database_id.is_empty());
    for (actual, expected) in hql_result.rows.iter().zip(&p7_result.rows) {
        let p7_graph::Binding::Entity(entity) = &expected["h"] else {
            panic!("P7 HistoryScan binds vector revision identities")
        };
        let revision = p7_catalog
            .revisions
            .iter()
            .find(|revision| &revision.entity == entity)
            .unwrap();
        let history = tagged(actual, "h")["value"].clone();
        let subject = &history["subject"];
        assert_eq!(subject["database_id"].as_str(), Some(database_id.as_str()));
        assert_eq!(
            subject["namespace"].as_str(),
            Some(entity.namespace.as_str())
        );
        assert_eq!(subject["kind"].as_str(), Some("vector"));
        assert_eq!(subject["id"].as_str(), Some(entity.id.as_str()));
        assert_eq!(subject["revision"].as_str(), Some(entity.revision.as_str()));
        assert_eq!(history["operation"].as_str(), Some("upsert"));
        assert_eq!(
            history["tx_from"].as_str().unwrap().parse::<u64>().unwrap(),
            revision.transaction.start
        );
        assert_eq!(
            history["tx_to"]
                .as_str()
                .map(|value| value.parse::<u64>().unwrap()),
            revision.transaction.end
        );
        assert_eq!(
            chrono::DateTime::parse_from_rfc3339(history["valid_from"].as_str().unwrap())
                .unwrap()
                .timestamp_micros(),
            revision.valid.start
        );
        assert_eq!(
            history["valid_to"]
                .as_str()
                .map(|value| chrono::DateTime::parse_from_rfc3339(value)
                    .unwrap()
                    .timestamp_micros()),
            revision.valid.end
        );
    }
}

#[test]
fn history_scan_includes_retract_tombstones_without_user_properties() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let current = storage
        .supersede_node("history:one".into(), Some(json!({"title":"current"})), None)
        .unwrap();
    storage.retract_node("history:one").unwrap();

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let (from, to): (String, String) = projection
        .query_row(
            "SELECT valid_from, valid_to FROM hql2_record_revisions
             WHERE kind='node' AND record_id='history:one' AND operation='retract'
             ORDER BY tx_from DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let from = chrono::DateTime::parse_from_rfc3339(&from).unwrap();
    let to = chrono::DateTime::parse_from_rfc3339(&to).unwrap();
    let valid_at = (from + (to - from) / 2).to_rfc3339();
    assert!(chrono::DateTime::parse_from_rfc3339(&current.valid_from).unwrap() < to);

    let query = format!(
        "USE default AT VALID \"{valid_at}\" HISTORY NODE \"history:one\" AS h |> RETURN h.operation AS operation, prop(h.subject, \"title\") AS title, has_prop(h.subject, \"title\") AS has_title"
    );
    let result = run_hql(&storage, "history-tombstone", &query)
        .expect("HistoryScan should include the retained retract row");
    let tombstone = result
        .rows
        .iter()
        .find(|row| tagged(row, "operation")["value"] == "retract")
        .expect("retract metadata should be present");
    assert_eq!(tagged(tombstone, "title")["type"], "Null");
    assert_eq!(tagged(tombstone, "has_title")["value"], false);
}

#[test]
fn change_scan_emits_ordered_revision_events_with_typed_sequences_and_operations() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let after = storage.stable_frontier();
    add_node(&storage);
    storage
        .supersede_node(
            "history:one".into(),
            Some(json!({"title":"replacement"})),
            None,
        )
        .unwrap();
    storage.retract_node("history:one").unwrap();

    let query = format!(
        "USE default CHANGES SINCE {after} AS c |> ORDER BY c.sequence ASC NULLS LAST |> RETURN c.subject AS subject, c.sequence AS sequence, c.operation AS operation"
    );
    let result = run_hql(&storage, "change-events", &query)
        .expect("ChangeScan should emit retained revisions after the cursor");

    assert_eq!(result.rows.len(), 4);
    let operations: Vec<_> = result
        .rows
        .iter()
        .map(|row| {
            tagged(row, "operation")["value"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(operations, ["upsert", "correct", "correct", "retract"]);
    let sequences: Vec<_> = result
        .rows
        .iter()
        .map(|row| {
            let value = tagged(row, "sequence");
            assert_eq!(value["type"], "DecimalU64");
            assert_eq!(tagged(row, "subject")["value"]["id"], "history:one");
            value["value"].as_str().unwrap().parse::<u64>().unwrap()
        })
        .collect();
    assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(result
        .rows
        .iter()
        .all(|row| tagged(row, "subject")["type"] == "Entity"));

    let hql_result = run_hql(
        &storage,
        "change-hql-ir-parity-hql",
        &format!("USE default CHANGES SINCE {after} AS c |> RETURN c"),
    )
    .unwrap();
    let ir_result = run_ir_changes(&storage, "change-hql-ir-parity-ir", after);
    assert_eq!(hql_result.columns, ir_result.columns);
    assert_eq!(hql_result.rows, ir_result.rows);
}

#[test]
fn hql_change_scan_accepts_decimal_u64_floor_and_take_parameters() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let floor = storage.stable_frontier();
    add_node(&storage);
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"hql-change-u64-parameters",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default CHANGES SINCE $floor AS c |> TAKE $limit |> RETURN c.sequence AS sequence",
        "params":{
            "floor":{"type":"DecimalU64","value":floor.to_string()},
            "limit":{"type":"DecimalU64","value":"1"}
        }
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("read query must return rows")
    };
    assert_eq!(result.rows.len(), 1);
    assert!(matches!(
        result.rows[0]["sequence"],
        genesis_block_native::query::hql2::value::QueryValueV2::DecimalU64(_)
    ));

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"hql-change-u64-wrong-type",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default CHANGES SINCE $floor AS c |> RETURN c.sequence AS sequence",
        "params":{"floor":{"type":"I64","value":"0"}}
    }))
    .unwrap();
    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "parameter_type_mismatch");
}

#[test]
fn change_scan_floor_is_an_exclusive_cursor_and_equality_is_valid() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let floor = storage.stable_frontier();
    assert!(floor > 0);
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    projection
        .execute(
            "UPDATE hql2_source_history_floors SET history_floor=?1
             WHERE namespace='default' AND source IN ('row','vector','annotation')",
            [i64::try_from(floor).unwrap()],
        )
        .unwrap();

    let below = run_hql(
        &storage,
        "change-below-floor",
        &format!("USE default CHANGES SINCE {} AS c |> RETURN c", floor - 1),
    )
    .unwrap_err();
    assert_eq!(below.code, "HISTORY_UNAVAILABLE");

    let at_floor = run_hql(
        &storage,
        "change-at-floor",
        &format!("USE default CHANGES SINCE {floor} AS c |> RETURN c"),
    )
    .unwrap();
    assert!(at_floor.rows.is_empty());
}

#[test]
fn change_scan_includes_owner_bound_vector_revision_events() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let after = storage.stable_frontier();
    storage
        .add_node(NodeInput {
            id: Some("history:vector-owner".into()),
            labels: vec!["Document".into()],
            props: None,
            embedding: Some(vec![0.125, 0.25]),
            lang: None,
            valid_from: Some("2010-01-01T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: Some("default".into()),
        })
        .unwrap();

    let result = run_hql(
        &storage,
        "change-vector-owner",
        &format!("USE default CHANGES SINCE {after} AS c |> RETURN c"),
    )
    .unwrap();
    let kinds: Vec<_> = result
        .rows
        .iter()
        .map(|row| {
            tagged(row, "c")["value"]["subject"]["kind"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(kinds, ["node", "vector"]);
}

#[test]
fn history_scan_rejects_a_source_floor_beyond_the_pinned_frontier() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let floor = storage.stable_frontier() + 1;
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    projection
        .execute(
            "UPDATE hql2_source_history_floors SET history_floor=?1
             WHERE namespace='default' AND source='annotation'",
            [i64::try_from(floor).unwrap()],
        )
        .unwrap();

    let error = run_hql(
        &storage,
        "history-floor-beyond-snapshot",
        "USE default HISTORY ANNOTATION \"missing:annotation\" AS h |> RETURN h",
    )
    .unwrap_err();
    assert_eq!(error.code, "HISTORY_UNAVAILABLE");
}

#[test]
fn vector_history_scan_rejects_a_vector_floor_beyond_the_pinned_frontier() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    storage
        .add_vector("history:one".into(), "default".into(), vec![0.5, 0.75])
        .unwrap();
    let floor = storage.stable_frontier() + 1;
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    projection
        .execute(
            "UPDATE hql2_source_history_floors SET history_floor=?1
             WHERE namespace='default' AND source='vector'",
            [i64::try_from(floor).unwrap()],
        )
        .unwrap();

    let vector_id = serde_json::to_string(&("history:one", "default")).unwrap();
    let hql_id = serde_json::to_string(&vector_id).unwrap();
    let hql = format!(
        "USE default AT VALID \"2020-01-01T00:00:00Z\" HISTORY VECTOR {hql_id} AS h |> RETURN h"
    );
    let hql_error = run_hql(&storage, "history-vector-floor-hql", &hql).unwrap_err();
    assert_eq!(hql_error.code, "HISTORY_UNAVAILABLE");

    let ir_error = run_ir_history_source(
        &storage,
        "history-vector-floor-ir",
        "2020-01-01T00:00:00Z",
        "vector",
        &vector_id,
        None,
    )
    .unwrap_err();
    assert_eq!(ir_error.code, "HISTORY_UNAVAILABLE");
}

#[test]
fn transaction_time_history_scan_rejects_a_selected_frontier_below_vector_floor() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let selected_tx: i64 = projection
        .query_row(
            "SELECT tx_from FROM hql2_record_revisions
             WHERE kind='node' AND record_id='history:one' AND operation='upsert'
             ORDER BY tx_from LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    storage
        .add_vector("history:one".into(), "default".into(), vec![0.5, 0.75])
        .unwrap();
    let floor = selected_tx + 1;
    assert!(u64::try_from(floor).unwrap() <= storage.stable_frontier());
    projection
        .execute(
            "UPDATE hql2_source_history_floors SET history_floor=?1
             WHERE namespace='default' AND source='vector'",
            [floor],
        )
        .unwrap();

    let selected_tx = u64::try_from(selected_tx).unwrap();
    let selected_tx_text = selected_tx.to_string();
    let vector_id = serde_json::to_string(&("history:one", "default")).unwrap();
    let hql_id = serde_json::to_string(&vector_id).unwrap();
    let hql = format!(
        "USE default AT TX {selected_tx} AT VALID \"2020-01-01T00:00:00Z\" HISTORY VECTOR {hql_id} AS h |> RETURN h"
    );
    let hql_error = run_hql(&storage, "history-vector-at-tx-floor-hql", &hql).unwrap_err();
    assert_eq!(hql_error.code, "HISTORY_UNAVAILABLE");
    let ir_error = run_ir_history_source(
        &storage,
        "history-vector-at-tx-floor-ir",
        "2020-01-01T00:00:00Z",
        "vector",
        &vector_id,
        Some(&selected_tx_text),
    )
    .unwrap_err();
    assert_eq!(ir_error.code, "HISTORY_UNAVAILABLE");
}

#[test]
fn history_artifact_source_and_unknown_change_families_fail_closed() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let artifact_error = run_hql(
        &storage,
        "history-artifact-unavailable",
        "USE default HISTORY ARTIFACT \"artifact:one\" AS h |> RETURN h",
    )
    .unwrap_err();
    assert_eq!(artifact_error.code, "CAPABILITY_UNSUPPORTED");

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let (database_id, sequence): (String, i64) = projection
        .query_row(
            "SELECT database_id, tx_from FROM hql2_record_revisions
             WHERE kind='node' ORDER BY tx_from LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    projection
        .execute(
            "INSERT INTO hql2_record_revisions(
                 database_id, namespace, kind, record_id, revision_id, predecessor_revision_id,
                 operation, valid_from, valid_to, tx_from, tx_to, schema_ref, schema_version,
                 origin_database_id, payload_json
             ) VALUES (?1, 'default', 'future_family', 'future:one', 'future-revision', NULL,
                       'upsert', '2000-01-01T00:00:00Z', NULL, ?2, NULL, NULL, NULL, ?1, '{}')",
            rusqlite::params![database_id, sequence],
        )
        .unwrap();

    let error = run_hql(
        &storage,
        "change-unknown-family",
        "USE default CHANGES SINCE 0 AS c |> RETURN c",
    )
    .unwrap_err();
    assert_eq!(error.code, "CAPABILITY_UNSUPPORTED");
}

#[test]
fn change_scan_rejects_a_cursor_beyond_the_pinned_frontier() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let error = run_hql(
        &storage,
        "change-invalid-cursor",
        &format!("USE default CHANGES SINCE {} AS c |> RETURN c", u64::MAX),
    )
    .unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.stage, "bind");
    assert_eq!(
        error
            .detail
            .as_ref()
            .and_then(|detail| detail.get("reason")),
        Some(&json!("invalid_change_bounds"))
    );
}

#[test]
fn history_and_change_scans_fail_without_partial_rows_when_source_budget_is_exhausted() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let after = storage.stable_frontier();
    add_node(&storage);
    storage
        .supersede_node(
            "history:one".into(),
            Some(json!({"title":"replacement"})),
            None,
        )
        .unwrap();

    let change_error = run_hql_with_budget(
        &storage,
        "change-source-budget",
        &format!("USE default CHANGES SINCE {after} AS c |> RETURN c"),
        json!({"max_expanded_nodes":1}),
    )
    .unwrap_err();
    assert_eq!(change_error.code, "QUERY_BUDGET_EXCEEDED");

    let history_error = run_hql_with_budget(
        &storage,
        "history-source-budget",
        "USE default HISTORY NODE \"history:one\" AS h |> RETURN h",
        json!({"max_expanded_nodes":1}),
    )
    .unwrap_err();
    assert_eq!(history_error.code, "QUERY_BUDGET_EXCEEDED");
}
