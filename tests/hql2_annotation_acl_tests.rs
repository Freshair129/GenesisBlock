use genesis_block_native::{
    query::hql2::QueryOutcomeV2,
    uee_v2::{AnnotationPutMutationV2, RecordKindV2},
    AccessAction, AccessContext, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource,
    NodeInput, OpenOptions, PolicyAdminActor, Storage,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::Path;
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

fn add_target(storage: &Storage, path: &Path) -> String {
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
    let conn = Connection::open(path.join("projection.sqlite")).unwrap();
    conn.query_row(
        "SELECT revision_id FROM hql2_record_revisions
         WHERE namespace='default' AND kind='node' AND record_id='doc:one' AND tx_to IS NULL",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

fn annotation(database_id: &str, target_revision: &str) -> Value {
    json!({
        "id": "review:1",
        "namespace": "default",
        "kind": "review",
        "targets": [{
            "ref": {
                "database_id": database_id,
                "namespace": "default",
                "kind": "node",
                "id": "doc:one",
                "revision": target_revision
            },
            "binding": "frozen",
            "selector": {"type": "whole"}
        }],
        "evidence": [{
            "ref": {
                "database_id": database_id,
                "namespace": "default",
                "kind": "node",
                "id": "doc:one",
                "revision": target_revision
            },
            "binding": "frozen",
            "selector": {"type": "whole"}
        }],
        "body": {"type": "text", "text": "Reviewed against source revision.", "language": "en"},
        "author": "asserted-reviewer",
        "created_at": "2026-09-22T00:00:00Z",
        "valid_from": "2026-09-22T00:00:00Z",
        "valid_to": null,
        "confidence": 0.9,
        "status": "approved"
    })
}

fn put(
    storage: &Storage,
    annotation: Value,
    expected_revision: Option<Uuid>,
) -> genesis_block_native::uee_v2::RecordRefV2 {
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation,
                expected_revision,
                valid: None,
            },
        )
        .unwrap()
}

fn database_id(storage: &Storage) -> String {
    let request = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-database-id",
        "namespace":"default",
        "ir":{"contract_version":"query-ir.v2","nodes":[{"id":"v","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}}],"root":"v","parameter_types":{"xs":"List<I64>"}},
        "params":{"xs":{"type":"List<I64>","value":[]}}
    }))
    .unwrap();
    let genesis_block_native::query::hql2::QueryOutcomeV2::Rows(result) = storage
        .query_v2(
            AccessContext {
                principal: "annotation-test".into(),
                namespace: "default".into(),
            },
            request,
        )
        .unwrap()
    else {
        panic!("values request must return rows")
    };
    result.snapshot.database_id
}

#[test]
fn annotation_put_persists_engine_actor_and_separate_target_evidence_rows() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let target_revision = add_target(&storage, dir.path());
    let database_id = database_id(&storage);
    let stored = put(&storage, annotation(&database_id, &target_revision), None);

    assert_eq!(stored.database_id, database_id);
    assert_eq!(stored.namespace, "default");
    assert_eq!(stored.kind, RecordKindV2::Annotation);
    assert_eq!(
        Uuid::parse_str(&stored.revision).unwrap().get_version_num(),
        4
    );

    let conn = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let (payload, operation, predecessor): (String, String, Option<String>) = conn
        .query_row(
            "SELECT payload_json, operation, predecessor_revision_id
             FROM hql2_record_revisions
             WHERE database_id=?1 AND namespace='default' AND kind='annotation'
               AND record_id='review:1' AND revision_id=?2",
            rusqlite::params![database_id, stored.revision],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let payload: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(operation, "upsert");
    assert_eq!(predecessor, None);
    assert_eq!(payload["verified_actor"], "reviewer");
    assert_eq!(payload["author"], "asserted-reviewer");
    assert!(payload.get("revision_id").is_none());

    let role_counts: (i64, i64) = conn
        .query_row(
            "SELECT SUM(is_evidence=0), SUM(is_evidence=1)
             FROM hql2_annotation_targets
             WHERE database_id=?1 AND annotation_id='review:1' AND annotation_revision_id=?2",
            rusqlite::params![database_id, stored.revision],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(role_counts, (1, 1));

    storage.compact().unwrap();
    drop(storage);
    let reopened = open(dir.path());
    assert!(reopened.events_since_seq(0).iter().any(|event| {
        serde_json::to_value(&event.event)
            .ok()
            .and_then(|value| value.get("Transaction").cloned())
            .and_then(|value| value.get("record_revision_transaction").cloned())
            .is_some_and(|value| value.to_string().contains("review:1"))
    }));
}

#[test]
fn annotation_put_rejects_foreign_lineage_and_client_verified_actor_before_append() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let target_revision = add_target(&storage, dir.path());
    let database_id = database_id(&storage);
    let before = storage.events_since_seq(0).len();

    let mut foreign = annotation(&database_id, &target_revision);
    foreign["targets"][0]["ref"]["database_id"] = json!("f".repeat(64));
    assert!(storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: foreign,
                expected_revision: None,
                valid: None,
            },
        )
        .is_err());

    let mut spoofed = annotation(&database_id, &target_revision);
    spoofed["verified_actor"] = json!("spoofed");
    assert!(storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: spoofed,
                expected_revision: None,
                valid: None,
            },
        )
        .is_err());
    assert_eq!(storage.events_since_seq(0).len(), before);
}

#[test]
fn annotation_updates_are_revision_cas_and_stale_writes_do_not_append() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let target_revision = add_target(&storage, dir.path());
    let database_id = database_id(&storage);
    let first = put(&storage, annotation(&database_id, &target_revision), None);
    let mut amendment = annotation(&database_id, &target_revision);
    amendment["body"]["text"] = json!("Amended review.");
    amendment["supersedes"] = json!(first.revision);
    let second = put(
        &storage,
        amendment.clone(),
        Some(Uuid::parse_str(&first.revision).unwrap()),
    );
    assert_ne!(first.revision, second.revision);

    let before = storage.events_since_seq(0).len();
    assert!(storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: amendment,
                expected_revision: Some(Uuid::new_v4()),
                valid: None,
            },
        )
        .is_err());
    assert_eq!(storage.events_since_seq(0).len(), before);

    let conn = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let history: Vec<(String, Option<i64>, Option<String>)> = conn
        .prepare(
            "SELECT revision_id, tx_to, predecessor_revision_id
             FROM hql2_record_revisions WHERE database_id=?1 AND kind='annotation'
               AND record_id='review:1' ORDER BY tx_from",
        )
        .unwrap()
        .query_map([database_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(history.len(), 2);
    assert!(history[0].1.is_some());
    assert_eq!(history[1].1, None);
    assert_eq!(history[1].2.as_deref(), Some(first.revision.as_str()));
}

#[test]
fn annotation_put_rejects_duplicate_refs_within_either_role() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let target_revision = add_target(&storage, dir.path());
    let database_id = database_id(&storage);
    let mut duplicate = annotation(&database_id, &target_revision);
    let duplicate_target = duplicate["targets"][0].clone();
    duplicate["targets"]
        .as_array_mut()
        .unwrap()
        .push(duplicate_target);
    let before = storage.events_since_seq(0).len();
    assert!(storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: duplicate,
                expected_revision: None,
                valid: None,
            },
        )
        .is_err());
    assert_eq!(storage.events_since_seq(0).len(), before);
}

#[test]
fn annotation_put_rejects_missing_frozen_revision_before_append() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let target_revision = add_target(&storage, dir.path());
    let database_id = database_id(&storage);
    let mut missing = annotation(&database_id, &target_revision);
    missing["targets"][0]["ref"]["revision"] = json!(Uuid::new_v4().to_string());
    let before = storage.events_since_seq(0).len();

    assert!(storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: missing,
                expected_revision: None,
                valid: None,
            },
        )
        .is_err());
    assert_eq!(storage.events_since_seq(0).len(), before);
}

#[test]
fn annotation_put_rejects_malformed_live_refs_and_evidence_shape() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let target_revision = add_target(&storage, dir.path());
    let database_id = database_id(&storage);
    let mut malformed = annotation(&database_id, &target_revision);
    malformed["targets"][0]["binding"] = json!("live");
    malformed["targets"][0]["ref"]["revision"] = Value::Null;
    let before = storage.events_since_seq(0).len();

    assert!(storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: malformed,
                expected_revision: None,
                valid: None,
            },
        )
        .is_err());
    let mut malformed = annotation(&database_id, &target_revision);
    malformed["evidence"] = json!("not-an-array");
    assert!(storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: malformed,
                expected_revision: None,
                valid: None,
            },
        )
        .is_err());
    assert_eq!(storage.events_since_seq(0).len(), before);
}

#[test]
fn annotation_put_rejects_annotation_cycles_before_append() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let target_revision = add_target(&storage, dir.path());
    let database_id = database_id(&storage);
    let mut first = annotation(&database_id, &target_revision);
    first["targets"][0]["ref"]["kind"] = json!("annotation");
    first["targets"][0]["ref"]["id"] = json!("review:2");
    first["targets"][0]["ref"]
        .as_object_mut()
        .unwrap()
        .remove("revision");
    first["targets"][0]["binding"] = json!("live");
    put(&storage, first, None);

    let mut second = annotation(&database_id, &target_revision);
    second["id"] = json!("review:2");
    second["targets"][0]["ref"]["kind"] = json!("annotation");
    second["targets"][0]["ref"]["id"] = json!("review:1");
    second["targets"][0]["ref"]
        .as_object_mut()
        .unwrap()
        .remove("revision");
    second["targets"][0]["binding"] = json!("live");
    let before = storage.events_since_seq(0).len();

    assert!(storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: second,
                expected_revision: None,
                valid: None,
            },
        )
        .is_err());
    assert_eq!(storage.events_since_seq(0).len(), before);
}

#[test]
fn annotation_acl_policy_v2_survives_compaction_and_reopen() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let policy = |revision| AccessPolicy {
        revision,
        mode: AccessPolicyMode::Enforced,
        grants: vec![
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
        ],
    };
    storage
        .replace_access_policy(
            PolicyAdminActor {
                access: AccessContext {
                    principal: "local-owner".into(),
                    namespace: "default".into(),
                },
            },
            0,
            policy(1),
        )
        .unwrap();
    storage.compact().unwrap();
    drop(storage);

    let reopened = open(dir.path());
    reopened
        .replace_access_policy(
            PolicyAdminActor {
                access: AccessContext {
                    principal: "local-owner".into(),
                    namespace: "default".into(),
                },
            },
            1,
            policy(2),
        )
        .unwrap();
}

#[test]
fn annotation_scan_requires_annotation_grant_in_addition_to_namespace_query_grant() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let target_revision = add_target(&storage, dir.path());
    let database_id = database_id(&storage);
    let after_seq = storage.stable_frontier();
    put(&storage, annotation(&database_id, &target_revision), None);

    let policy = |revision, include_annotation| {
        let mut grants = vec![
            AccessGrant {
                principal: "reviewer".into(),
                action: AccessAction::Read,
                resource: AccessResource::Namespace("default".into()),
            },
            AccessGrant {
                principal: "local-owner".into(),
                action: AccessAction::ManagePolicy,
                resource: AccessResource::Namespace("default".into()),
            },
        ];
        if include_annotation {
            grants.push(AccessGrant {
                principal: "reviewer".into(),
                action: AccessAction::Read,
                resource: AccessResource::Annotation("default".into()),
            });
        }
        AccessPolicy {
            revision,
            mode: AccessPolicyMode::Enforced,
            grants,
        }
    };
    let admin = || PolicyAdminActor {
        access: AccessContext {
            principal: "local-owner".into(),
            namespace: "default".into(),
        },
    };
    storage
        .replace_access_policy(admin(), 0, policy(1, false))
        .unwrap();

    let request = |request_id| {
        serde_json::from_value(json!({
            "contract_version":"genesis.api.v2",
            "request_id":request_id,
            "namespace":"default",
            "language_version":"hql.v2",
            "hql":"USE default FROM ANNOTATIONS AS a |> RETURN a.id AS id",
            "params":{}
        }))
        .unwrap()
    };
    let change_request = |request_id| {
        serde_json::from_value(json!({
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
        .unwrap()
    };
    let error = storage
        .query_v2(actor(), request("annotation-without-grant"))
        .unwrap_err();
    assert_eq!(error.code, "FORBIDDEN");

    let QueryOutcomeV2::Rows(changes) = storage
        .query_v2(actor(), change_request("annotation-change-without-grant"))
        .unwrap()
    else {
        panic!("change scan must return rows")
    };
    assert!(changes.rows.is_empty());

    storage
        .replace_access_policy(admin(), 1, policy(2, true))
        .unwrap();
    let QueryOutcomeV2::Rows(result) = storage
        .query_v2(actor(), request("annotation-with-grant"))
        .unwrap()
    else {
        panic!("annotation scan must return rows")
    };
    assert_eq!(result.rows.len(), 1);

    let QueryOutcomeV2::Rows(changes) = storage
        .query_v2(actor(), change_request("annotation-change-with-grant"))
        .unwrap()
    else {
        panic!("change scan must return rows")
    };
    assert_eq!(changes.rows.len(), 1);
}

#[test]
fn change_scan_budget_ignores_annotation_revisions_hidden_by_acl() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let target_revision = add_target(&storage, dir.path());
    let database_id = database_id(&storage);
    let after_seq = storage.stable_frontier();

    for index in 1..=3 {
        let mut payload = annotation(&database_id, &target_revision);
        payload["id"] = json!(format!("review:{index}"));
        put(&storage, payload, None);
    }
    storage
        .add_node(NodeInput {
            id: Some("visible:node".into()),
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
                        principal: "reviewer".into(),
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

    let request = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-hidden-budget",
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
        "params":{},
        "budget":{"max_expanded_nodes":1}
    }))
    .unwrap();

    let QueryOutcomeV2::Rows(changes) = storage.query_v2(actor(), request).unwrap() else {
        panic!("change scan must return rows")
    };
    assert_eq!(changes.rows.len(), 1);
}

#[test]
fn exact_node_grant_does_not_create_hql2_query_and_denial_precedes_parse() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_target(&storage, dir.path());
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
                        principal: "reviewer".into(),
                        action: AccessAction::Read,
                        resource: AccessResource::Node("doc:one".into()),
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

    for (index, hql) in [
        "USE default MATCH (a {id: \"doc:one\"})-[:LINK]->(b) |> RETURN a",
        "USE default MATCH (a {id: \"doc:missing\"})-[:LINK]->(b) |> RETURN a",
        "USE default MATCH (",
    ]
    .into_iter()
    .enumerate()
    {
        let request = serde_json::from_value(json!({
            "contract_version": "genesis.api.v2",
            "request_id": format!("exact-node-{index}"),
            "namespace": "default",
            "hql": hql,
            "language_version": "hql.v2",
            "params": {}
        }))
        .unwrap();
        let error = storage.query_v2(actor(), request).unwrap_err();
        assert_eq!(error.code, "FORBIDDEN");
        assert_eq!(error.stage, "authorize");
    }

    for (index, id) in ["doc:one", "doc:missing"].into_iter().enumerate() {
        let request = serde_json::from_value(json!({
            "contract_version":"genesis.api.v2",
            "request_id":format!("exact-node-ir-{index}"),
            "namespace":"default",
            "params":{},
            "ir":{
                "contract_version":"query-ir.v2",
                "nodes":[
                    {"id":"match","op":"Match","inputs":[],"config":{
                        "pattern":{
                            "form":"sequence",
                            "start":{"alias":"a","id":{"literal":id,"type":"Utf8"},"labels":[],"properties":{}},
                            "steps":[{"edge":{"relations":["LINK"],"direction":"out","min_hops":1,"max_hops":1,"properties":{}},"node":{"alias":"b","labels":[],"properties":{}}}],
                            "mode":"trail"
                        },
                        "anchors":{},"shortest":false
                    }}
                ],
                "root":"match","parameter_types":{}
            }
        }))
        .unwrap();
        let error = storage.query_v2(actor(), request).unwrap_err();
        assert_eq!(error.code, "FORBIDDEN");
        assert_eq!(error.stage, "authorize");
    }

    let malformed_ir = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"exact-node-malformed-ir",
        "namespace":"default",
        "params":{},
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"n","unapproved":true}}],
            "root":"scan","parameter_types":{}
        }
    }))
    .unwrap();
    let error = storage.query_v2(actor(), malformed_ir).unwrap_err();
    assert_eq!(error.code, "FORBIDDEN");
    assert_eq!(error.stage, "authorize");
}
