use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::{AnnotationPutMutationV2, QueryRequestV2},
    AccessAction, AccessContext, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource,
    EdgeInput, NodeInput, OpenOptions, PolicyAdminActor, Storage,
};
use serde_json::json;
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

    let hql_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-scan-hql-parity",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM ANNOTATIONS AS a |> RETURN a.id AS id",
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
}
