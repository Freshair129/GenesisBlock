use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::{AnnotationPutMutationV2, QueryRequestV2},
    AccessAction, AccessContext, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource,
    EdgeInput, NodeInput, OpenOptions, PolicyAdminActor, Storage,
};
#[allow(dead_code)]
#[path = "support/hql2_graph_reference.rs"]
mod p7_graph;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use tempfile::TempDir;
use uuid::Uuid;

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

fn actor() -> AccessContext {
    AccessContext {
        principal: "reviewer".into(),
        namespace: "default".into(),
    }
}

fn database_id(storage: &Storage) -> String {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-source-database-id",
        "namespace":"default",
        "ir":{"contract_version":"query-ir.v2","nodes":[{"id":"v","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}}],"root":"v","parameter_types":{"xs":"List<I64>"}},
        "params":{"xs":{"type":"List<I64>","value":[]}}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("values query must return rows")
    };
    result.snapshot.database_id
}

fn scan_count(storage: &Storage) -> usize {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-source-acl-scan",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM ANNOTATIONS AS a |> RETURN a",
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("annotation scan must execute")
    };
    result.rows.len()
}

fn revision_mutations(storage: &Storage, kind: &str) -> Vec<Value> {
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
fn annotation_history_scan_matches_p7_for_wal_revision_and_reference_acl() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(NodeInput {
            id: Some("doc:history".into()),
            labels: vec!["Document".into()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    let node_tx = storage.stable_frontier();
    let database_id = database_id(&storage);
    let node_mutations = revision_mutations(&storage, "node");
    assert_eq!(node_mutations.len(), 1);
    let node_revision = node_mutations[0]["revision_id"].as_str().unwrap();
    let target = json!({
        "ref": {
            "database_id": database_id,
            "namespace": "default",
            "kind": "node",
            "id": "doc:history",
            "revision": node_revision
        },
        "binding": "frozen",
        "selector": {"type": "whole"}
    });
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":"review:history",
                    "namespace":"default",
                    "kind":"review",
                    "targets":[target],
                    "evidence":[target],
                    "body":{"type":"text","text":"Reviewed history."},
                    "author":"asserted-reviewer",
                    "created_at":"2026-09-22T00:00:00Z",
                    "valid_from":"2026-09-22T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();
    let frontier = storage.stable_frontier();
    let valid_at_text = "2026-10-03T00:00:00Z";
    let hql = format!(
        "USE default AT VALID \"{valid_at_text}\" HISTORY ANNOTATION \"review:history\" AS h |> RETURN h"
    );
    let hql_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-history-p7-hql",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":hql,
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(actor(), hql_request).unwrap() else {
        panic!("HQL history query must return rows")
    };
    let ir_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-history-p7-ir",
        "namespace":"default",
        "temporal":{"valid_at":valid_at_text},
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{
                "id":"history",
                "op":"HistoryScan",
                "inputs":[],
                "config":{"kind":"annotation","id":{"literal":"review:history","type":"Utf8"},"as":"h"}
            }],
            "root":"history",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(actor(), ir_request).unwrap() else {
        panic!("typed IR history query must return rows")
    };
    assert_eq!(hql_result.columns, ir_result.columns);
    assert_eq!(hql_result.rows, ir_result.rows);

    let annotation_mutations = revision_mutations(&storage, "annotation");
    assert_eq!(annotation_mutations.len(), 1);
    let node = p7_graph::Revision {
        entity: p7_graph::EntityRef {
            namespace: "default".into(),
            kind: p7_graph::Kind::Node,
            id: "doc:history".into(),
            revision: node_revision.into(),
        },
        transaction: p7_graph::Interval {
            start: node_tx,
            end: None,
        },
        valid: p7_graph::Interval {
            start: chrono::DateTime::parse_from_rfc3339(
                node_mutations[0]["valid_from"].as_str().unwrap(),
            )
            .unwrap()
            .timestamp_micros(),
            end: None,
        },
        retracted: false,
        fields: p7_graph::Fields::new(),
        data: p7_graph::RecordData::Plain,
    };
    let mutation = &annotation_mutations[0];
    let mut targets = Vec::new();
    for role in ["targets", "evidence"] {
        for value in mutation["payload"][role].as_array().unwrap() {
            let reference = &value["ref"];
            let target = p7_graph::Target {
                binding: p7_graph::TargetBinding::Frozen(p7_graph::EntityRef {
                    namespace: reference["namespace"].as_str().unwrap().into(),
                    kind: p7_graph::Kind::Node,
                    id: reference["id"].as_str().unwrap().into(),
                    revision: reference["revision"].as_str().unwrap().into(),
                }),
                selector: p7_graph::Selector::Whole,
            };
            if !targets.contains(&target) {
                targets.push(target);
            }
        }
    }
    let annotation = p7_graph::Revision {
        entity: p7_graph::EntityRef {
            namespace: mutation["namespace"].as_str().unwrap().into(),
            kind: p7_graph::Kind::Annotation,
            id: mutation["id"].as_str().unwrap().into(),
            revision: mutation["revision_id"].as_str().unwrap().into(),
        },
        transaction: p7_graph::Interval {
            start: frontier,
            end: None,
        },
        valid: p7_graph::Interval {
            start: chrono::DateTime::parse_from_rfc3339(mutation["valid_from"].as_str().unwrap())
                .unwrap()
                .timestamp_micros(),
            end: mutation["valid_to"].as_str().map(|value| {
                chrono::DateTime::parse_from_rfc3339(value)
                    .unwrap()
                    .timestamp_micros()
            }),
        },
        retracted: false,
        fields: p7_graph::Fields::new(),
        data: p7_graph::RecordData::Annotation { targets },
    };
    let annotation_identity = annotation.entity.identity();
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
                p7_graph::Kind::Annotation,
                p7_graph::HistoryCapability {
                    horizon: 0,
                    available: true,
                },
            ),
        ]),
        revisions: vec![node, annotation],
    };
    let valid_at = chrono::DateTime::parse_from_rfc3339(valid_at_text)
        .unwrap()
        .timestamp_micros();
    let p7_plan = p7_graph::Plan {
        source: p7_graph::Source::HistoryScan {
            kind: p7_graph::Kind::Annotation,
            alias: "h".into(),
            predicate: p7_graph::Predicate {
                id: Some("review:history".into()),
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
            read: BTreeSet::from([
                p7_graph::Identity {
                    namespace: "default".into(),
                    kind: p7_graph::Kind::Node,
                    id: "doc:history".into(),
                },
                annotation_identity.clone(),
            ]),
            annotation_body: BTreeSet::from([annotation_identity]),
        },
    };
    let p7_result =
        p7_graph::execute(&p7_catalog, &p7_view, &p7_plan, p7_graph::Limits::default()).unwrap();
    assert_eq!(hql_result.rows.len(), p7_result.rows.len());
    for (actual, expected) in hql_result.rows.iter().zip(&p7_result.rows) {
        let p7_graph::Binding::Entity(entity) = &expected["h"] else {
            panic!("P7 HistoryScan binds annotation revision identities")
        };
        let history = serde_json::to_value(&actual["h"]).unwrap();
        let history = &history["value"];
        assert_eq!(history["subject"]["namespace"].as_str(), Some("default"));
        assert_eq!(history["subject"]["kind"].as_str(), Some("annotation"));
        assert_eq!(history["subject"]["id"].as_str(), Some(entity.id.as_str()));
        assert_eq!(
            history["subject"]["revision"].as_str(),
            Some(entity.revision.as_str())
        );
        assert_eq!(
            history["tx_from"].as_str().unwrap().parse::<u64>().unwrap(),
            frontier
        );
    }
}

#[test]
fn annotation_scan_returns_the_revision_visible_in_its_p6_snapshot() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(NodeInput {
            id: Some("doc:one".into()),
            labels: vec!["Document".into()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    let database_id = database_id(&storage);
    let conn = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let target_revision: String = conn
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='node' AND record_id='doc:one' AND tx_to IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":"review:1",
                    "namespace":"default",
                    "kind":"review",
                    "targets":[{"ref":{"database_id":database_id,"namespace":"default","kind":"node","id":"doc:one","revision":target_revision},"binding":"frozen","selector":{"type":"whole"}}],
                    "evidence":[],
                    "body":{"type":"text","text":"Reviewed."},
                    "author":"asserted-reviewer",
                    "created_at":"2026-09-22T00:00:00Z",
                    "valid_from":"2026-09-22T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-source-scan",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM ANNOTATIONS AS a |> RETURN a",
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("annotation scan must execute")
    };
    assert_eq!(result.rows.len(), 1);
    let encoded = serde_json::to_value(&result).unwrap();
    let annotation = &encoded["rows"][0]["a"];
    assert_eq!(annotation["type"], "Entity");
    assert_eq!(annotation["value"]["database_id"], database_id);
    assert_eq!(annotation["value"]["kind"], "annotation");
    assert_eq!(annotation["value"]["id"], "review:1");
    assert_eq!(
        Uuid::parse_str(annotation["value"]["revision"].as_str().unwrap())
            .unwrap()
            .get_version_num(),
        4
    );
}

#[test]
fn annotation_scan_hides_parent_when_nested_annotation_references_are_incomplete() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(NodeInput {
            id: Some("doc:nested".into()),
            labels: vec!["Document".into()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    let database_id = database_id(&storage);
    let conn = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let node_revision: String = conn
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='node' AND record_id='doc:nested' AND tx_to IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":"review:child",
                    "namespace":"default",
                    "kind":"review",
                    "targets":[{"ref":{"database_id":database_id,"namespace":"default","kind":"node","id":"doc:nested","revision":node_revision},"binding":"frozen","selector":{"type":"whole"}}],
                    "evidence":[],
                    "body":{"type":"text","text":"Child."},
                    "author":"asserted-reviewer",
                    "created_at":"2026-09-22T00:00:00Z",
                    "valid_from":"2026-09-22T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();
    let child_revision: String = conn
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='annotation' AND record_id='review:child' AND tx_to IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":"review:parent",
                    "namespace":"default",
                    "kind":"review",
                    "targets":[{"ref":{"database_id":database_id,"namespace":"default","kind":"annotation","id":"review:child","revision":child_revision},"binding":"frozen","selector":{"type":"whole"}}],
                    "evidence":[],
                    "body":{"type":"text","text":"Parent."},
                    "author":"asserted-reviewer",
                    "created_at":"2026-09-22T00:00:00Z",
                    "valid_from":"2026-09-22T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();

    conn.execute(
        "DELETE FROM hql2_annotation_targets
         WHERE namespace='default' AND annotation_id='review:child'",
        [],
    )
    .unwrap();
    assert_eq!(scan_count(&storage), 0);
}

#[test]
fn annotation_scan_checks_edge_endpoint_access_for_target_and_evidence() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for id in ["doc:left", "doc:right"] {
        storage
            .add_node(NodeInput {
                id: Some(id.into()),
                labels: vec!["Document".into()],
                props: None,
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
        .add_edge(EdgeInput {
            id: Some("edge:review".into()),
            from: "doc:left".into(),
            to: "doc:right".into(),
            rel: "CITES".into(),
            props: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
    let database_id = database_id(&storage);
    let conn = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let edge_revision: String = conn
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='edge' AND record_id='edge:review' AND tx_to IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let edge_reference = json!({
        "ref":{"database_id":database_id,"namespace":"default","kind":"edge","id":"edge:review","revision":edge_revision},
        "binding":"frozen",
        "selector":{"type":"whole"}
    });
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":"review:edge",
                    "namespace":"default",
                    "kind":"review",
                    "targets":[edge_reference.clone()],
                    "evidence":[edge_reference],
                    "body":{"type":"text","text":"Edge evidence."},
                    "author":"asserted-reviewer",
                    "created_at":"2026-09-22T00:00:00Z",
                    "valid_from":"2026-09-22T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();
    assert_eq!(scan_count(&storage), 1);
}

#[test]
fn annotation_scan_uses_namespace_read_for_target_and_evidence_references() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for id in ["doc:target", "doc:evidence"] {
        storage
            .add_node(NodeInput {
                id: Some(id.into()),
                labels: vec!["Document".into()],
                props: None,
                embedding: None,
                lang: None,
                valid_from: Some("2026-09-22T00:00:00Z".into()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
    }
    let database_id = database_id(&storage);
    let conn = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let revision = |id: &str| {
        conn.query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='node' AND record_id=?1 AND tx_to IS NULL",
            [id],
            |row| row.get::<_, String>(0),
        )
        .unwrap()
    };
    let reference = |id: &str| {
        json!({
            "ref": {
                "database_id":database_id,
                "namespace":"default",
                "kind":"node",
                "id":id,
                "revision":revision(id)
            },
            "binding":"frozen",
            "selector":{"type":"whole"}
        })
    };
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":"review:acl",
                    "namespace":"default",
                    "kind":"review",
                    "targets":[reference("doc:target")],
                    "evidence":[reference("doc:evidence")],
                    "body":{"type":"text","text":"Evidence-backed review."},
                    "author":"asserted-reviewer",
                    "created_at":"2026-09-22T00:00:00Z",
                    "valid_from":"2026-09-22T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();

    let owner = PolicyAdminActor {
        access: AccessContext {
            principal: "local-owner".into(),
            namespace: "default".into(),
        },
    };
    let grants = vec![
        AccessGrant {
            principal: "reviewer".into(),
            action: AccessAction::Read,
            resource: AccessResource::Namespace("default".into()),
        },
        AccessGrant {
            principal: "reviewer".into(),
            action: AccessAction::Read,
            resource: AccessResource::Annotation("default".into()),
        },
        AccessGrant {
            principal: "local-owner".into(),
            action: AccessAction::ManagePolicy,
            resource: AccessResource::Namespace("default".into()),
        },
    ];
    let policy = AccessPolicy {
        revision: 1,
        mode: AccessPolicyMode::Enforced,
        grants,
    };
    storage.replace_access_policy(owner, 0, policy).unwrap();
    // Namespace Read is the P6 broad grant for all same-namespace resources.
    assert_eq!(scan_count(&storage), 1);

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-separate-evidence",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM ANNOTATIONS AS a |> RETURN prop(a, \"targets\") AS targets, prop(a, \"evidence\") AS evidence",
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("annotation property projection must return rows")
    };
    assert_eq!(result.rows.len(), 1);
    let targets = &result.rows[0]["targets"];
    let evidence = &result.rows[0]["evidence"];
    assert_eq!(
        targets,
        &genesis_block_native::query::hql2::value::QueryValueV2::Json(json!([reference(
            "doc:target"
        )]))
    );
    assert_eq!(
        evidence,
        &genesis_block_native::query::hql2::value::QueryValueV2::Json(json!([reference(
            "doc:evidence"
        )]))
    );
}

#[test]
fn annotation_scan_paginates_without_skipping_or_repeating_revisions() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(NodeInput {
            id: Some("doc:page".into()),
            labels: vec!["Document".into()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    let database_id = database_id(&storage);
    let conn = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let target_revision: String = conn
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='node' AND record_id='doc:page' AND tx_to IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    for index in 0..260 {
        storage
            .put_annotation(
                actor(),
                AnnotationPutMutationV2 {
                    annotation: json!({
                        "id":format!("review:{index:03}"),
                        "namespace":"default",
                        "kind":"review",
                        "targets":[{"ref":{"database_id":database_id,"namespace":"default","kind":"node","id":"doc:page","revision":target_revision},"binding":"frozen","selector":{"type":"whole"}}],
                        "evidence":[],
                        "body":{"type":"text","text":"Paged scan fixture."},
                        "author":"asserted-reviewer",
                        "created_at":"2026-09-22T00:00:00Z",
                        "valid_from":"2026-09-22T00:00:00Z",
                        "valid_to":null
                    }),
                    expected_revision: None,
                    valid: None,
                },
            )
            .unwrap();
    }

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-source-pages",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM ANNOTATIONS AS a |> RETURN a",
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("annotation scan must execute")
    };
    assert_eq!(result.rows.len(), 260);
    let ids: std::collections::BTreeSet<_> = result
        .rows
        .iter()
        .map(|row| match row.get("a").unwrap() {
            genesis_block_native::query::hql2::value::QueryValueV2::Entity(record) => {
                record.id.clone()
            }
            other => panic!("expected annotation entity, got {other:?}"),
        })
        .collect();
    assert_eq!(ids.len(), 260);
}

#[test]
fn hql_and_ir_annotation_scans_have_identical_results() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(NodeInput {
            id: Some("doc:parity".into()),
            labels: vec!["Document".into()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    let database_id = database_id(&storage);
    let connection = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let target_revision: String = connection
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='node' AND record_id='doc:parity' AND tx_to IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":"review:parity",
                    "namespace":"default",
                    "kind":"review",
                    "targets":[{"ref":{"database_id":database_id,"namespace":"default","kind":"node","id":"doc:parity","revision":target_revision},"binding":"frozen","selector":{"type":"whole"}}],
                    "evidence":[],
                    "body":{"type":"text","text":"Parity fixture."},
                    "author":"asserted-reviewer",
                    "created_at":"2026-09-22T00:00:00Z",
                    "valid_from":"2026-09-22T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();
    let annotation_revision: String = connection
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='annotation'
               AND record_id='review:parity' AND tx_to IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let hql_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-scan-hql-parity",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM ANNOTATIONS AS a |> RETURN a AS entity, a.id AS id",
        "params":{}
    }))
    .unwrap();
    let ir_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-scan-ir-parity",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"AnnotationScan","inputs":[],"config":{"as":"a"}},
                {"id":"project","op":"Project","inputs":["scan"],"config":{"fields":[
                    {"as":"entity","expression":{"field":{"alias":"a","path":[]}}},
                    {"as":"id","expression":{"field":{"alias":"a","path":["id"]}}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();

    let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(actor(), hql_request).unwrap() else {
        panic!("HQL annotation scan must return rows")
    };
    let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(actor(), ir_request).unwrap() else {
        panic!("IR annotation scan must return rows")
    };
    assert_eq!(hql_result.columns, ir_result.columns);
    assert_eq!(hql_result.rows, ir_result.rows);
    assert_eq!(hql_result.semantics, ir_result.semantics);
    assert_eq!(hql_result.rows.len(), 1);
    assert_eq!(
        hql_result.rows[0]["id"],
        QueryValueV2::Utf8("review:parity".into())
    );

    let node = p7_graph::Identity {
        namespace: "default".into(),
        kind: p7_graph::Kind::Node,
        id: "doc:parity".into(),
    };
    let annotation = p7_graph::Identity {
        namespace: "default".into(),
        kind: p7_graph::Kind::Annotation,
        id: "review:parity".into(),
    };
    let target = p7_graph::EntityRef {
        namespace: node.namespace.clone(),
        kind: node.kind,
        id: node.id.clone(),
        revision: target_revision,
    };
    let p7 = p7_graph::execute(
        &p7_graph::Catalog {
            frontier: 1,
            history: [p7_graph::Kind::Node, p7_graph::Kind::Annotation]
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
            revisions: vec![
                p7_graph::Revision {
                    entity: target.clone(),
                    transaction: p7_graph::Interval {
                        start: 1,
                        end: None,
                    },
                    valid: p7_graph::Interval {
                        start: i64::MIN,
                        end: None,
                    },
                    retracted: false,
                    fields: p7_graph::Fields::new(),
                    data: p7_graph::RecordData::Plain,
                },
                p7_graph::Revision {
                    entity: p7_graph::EntityRef {
                        namespace: annotation.namespace.clone(),
                        kind: annotation.kind,
                        id: annotation.id.clone(),
                        revision: annotation_revision,
                    },
                    transaction: p7_graph::Interval {
                        start: 1,
                        end: None,
                    },
                    valid: p7_graph::Interval {
                        start: i64::MIN,
                        end: None,
                    },
                    retracted: false,
                    fields: p7_graph::Fields::new(),
                    data: p7_graph::RecordData::Annotation {
                        targets: vec![p7_graph::Target {
                            binding: p7_graph::TargetBinding::Frozen(target),
                            selector: p7_graph::Selector::Whole,
                        }],
                    },
                },
            ],
        },
        &p7_graph::View {
            namespace: "default".into(),
            transaction: 1,
            valid_at: 0,
            permissions: p7_graph::Permissions {
                read: BTreeSet::from([node, annotation.clone()]),
                annotation_body: BTreeSet::from([annotation]),
            },
        },
        &p7_graph::Plan {
            source: p7_graph::Source::Scan {
                kind: p7_graph::Kind::Annotation,
                alias: "a".into(),
                predicate: p7_graph::Predicate::default(),
            },
            stages: vec![],
        },
        p7_graph::Limits::default(),
    )
    .unwrap();
    assert_eq!(hql_result.rows.len(), p7.rows.len());
    assert_eq!(ir_result.rows.len(), p7.rows.len());
    for actual_rows in [&hql_result.rows, &ir_result.rows] {
        for (actual, expected) in actual_rows.iter().zip(&p7.rows) {
            let p7_graph::Binding::Entity(expected) = &expected["a"] else {
                panic!("P7 AnnotationScan binds annotation identities")
            };
            let QueryValueV2::Entity(actual) = &actual["entity"] else {
                panic!("HQL/IR AnnotationScan must return an entity identity")
            };
            assert_eq!(actual.database_id, database_id);
            assert_eq!(actual.namespace, expected.namespace);
            assert_eq!(actual.id, expected.id);
            assert_eq!(actual.revision, expected.revision);
            assert_eq!(
                actual.kind,
                genesis_block_native::uee_v2::RecordKindV2::Annotation
            );
        }
    }
}
