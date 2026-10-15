use genesis_block_native::{
    query::hql2::{QueryOutcomeV2, QueryResultV2},
    uee_v2::{AnnotationPutMutationV2, QueryRequestV2},
    AccessContext, BackupExportRequest, BackupRestoreRequest, NodeInput, OpenOptions,
    RelationalColumn, RelationalColumnType, RelationalMutationBatch, RelationalMutationKind,
    RelationalRowMutation, RelationalSchemaPackage, RelationalTable, Storage,
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

fn access() -> AccessContext {
    AccessContext {
        principal: "backup-restore-reader".into(),
        namespace: "default".into(),
    }
}

fn run_hql(storage: &Storage, request_id: &str, hql: &str) -> QueryResultV2 {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":hql,
        "params":{}
    }))
    .unwrap();
    match storage.query_v2(access(), request).unwrap() {
        QueryOutcomeV2::Rows(result) => result,
        QueryOutcomeV2::Plan(_) => panic!("history/change query must return rows"),
    }
}

fn run_ir_history(
    storage: &Storage,
    request_id: &str,
    kind: &str,
    record_id: &str,
    valid_at: &str,
) -> QueryResultV2 {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "temporal":{"valid_at":valid_at,"tx_as_of":null},
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
    match storage.query_v2(access(), request).unwrap() {
        QueryOutcomeV2::Rows(result) => result,
        QueryOutcomeV2::Plan(_) => panic!("history query must return rows"),
    }
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
        QueryOutcomeV2::Plan(_) => panic!("change query must return rows"),
    }
}

fn database_id(storage: &Storage) -> String {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"backup-restore-database-id",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{"id":"values","op":"Values","inputs":[],"config":{"param":"items","as":"item"}}],
            "root":"values",
            "parameter_types":{"items":"List<I64>"}
        },
        "params":{"items":{"type":"List<I64>","value":[]}}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("values query must return rows")
    };
    result.snapshot.database_id
}

fn projection_state(path: &Path, database_id: &str) -> Value {
    let connection = Connection::open(path.join("projection.sqlite")).unwrap();
    let revisions: Vec<Value> = connection
        .prepare(
            "SELECT namespace, kind, record_id, revision_id,
                    predecessor_revision_id, operation, valid_from, valid_to,
                    tx_from, tx_to, schema_ref, schema_version,
                    origin_database_id, payload_json
             FROM hql2_record_revisions WHERE database_id=?1
             ORDER BY namespace, kind, record_id, tx_from, revision_id",
        )
        .unwrap()
        .query_map([database_id], |row| {
            Ok(json!([
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<i64>>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?
            ]))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let schemas: Vec<Value> = connection
        .prepare(
            "SELECT namespace, schema_version, package_id, schema_hash, package_json
             FROM relational_schema_registry ORDER BY namespace",
        )
        .unwrap()
        .query_map([], |row| {
            Ok(json!([
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?
            ]))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let row_identities: Vec<Value> = connection
        .prepare(
            "SELECT namespace, table_name, row_id, key_codec_version,
                    key_bytes, created_revision_id
             FROM hql2_row_identity_registry WHERE database_id=?1
             ORDER BY namespace, table_name, row_id",
        )
        .unwrap()
        .query_map([database_id], |row| {
            Ok(json!([
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Vec<u8>>(4)?,
                row.get::<_, String>(5)?
            ]))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let annotation_selectors: Vec<Value> = connection
        .prepare(
            "SELECT namespace, annotation_id, annotation_revision_id, target_kind,
                    target_id, target_revision_id, binding, selector_json, is_evidence
             FROM hql2_annotation_targets WHERE database_id=?1
             ORDER BY namespace, annotation_id, annotation_revision_id,
                      target_kind, target_id, target_revision_id, is_evidence",
        )
        .unwrap()
        .query_map([database_id], |row| {
            Ok(json!([
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, i64>(8)?
            ]))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let history_floors: Vec<Value> = connection
        .prepare(
            "SELECT namespace, source, history_floor
             FROM hql2_source_history_floors ORDER BY namespace, source",
        )
        .unwrap()
        .query_map([], |row| {
            Ok(json!([
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?
            ]))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    json!({
        "revisions":revisions,
        "schemas":schemas,
        "row_identities":row_identities,
        "annotation_selectors":annotation_selectors,
        "history_floors":history_floors
    })
}

fn relational_schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "default".into(),
        schema_version: 1,
        previous_version: None,
        package_id: Uuid::new_v4().to_string(),
        schema_hash: String::new(),
        tables: vec![RelationalTable {
            name: "backup_records".into(),
            columns: vec![
                RelationalColumn::required("id", RelationalColumnType::Text),
                RelationalColumn::required("title", RelationalColumnType::Text),
            ],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        named_queries: vec![],
    }
}

#[test]
fn h2_d11_revision_identity_schema_floors_and_history_survive_backup_restore() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source");
    let bundle_path = dir.path().join("backup.genesis");
    let restore_root = dir.path().join("restored");
    let source = open(&source_root);
    source
        .add_node(NodeInput {
            id: Some("backup:owner".into()),
            labels: vec!["Document".into()],
            props: Some(json!({"title":"backup source"})),
            embedding: Some(vec![0.25, 0.5]),
            lang: Some("en".into()),
            valid_from: Some("2010-01-01T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: Some("default".into()),
        })
        .unwrap();
    source
        .add_vector("backup:owner".into(), "default".into(), vec![0.75, 0.25])
        .unwrap();
    source
        .register_relational_schema(relational_schema())
        .unwrap();
    for (kind, values, key) in [
        (
            RelationalMutationKind::Insert,
            json!({"id":"row:backup","title":"original"}),
            None,
        ),
        (
            RelationalMutationKind::Update,
            json!({"title":"revised"}),
            Some(json!({"id":"row:backup"})),
        ),
    ] {
        source
            .apply_relational_batch(RelationalMutationBatch {
                mutation_id: Uuid::new_v4().to_string(),
                namespace: "default".into(),
                schema_version: 1,
                operations: vec![RelationalRowMutation {
                    table: "backup_records".into(),
                    kind,
                    values,
                    key,
                }],
            })
            .unwrap();
    }

    let source_database_id = database_id(&source);
    let projection = Connection::open(source_root.join("projection.sqlite")).unwrap();
    let node_revision: String = projection
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE database_id=?1 AND namespace='default' AND kind='node'
               AND record_id='backup:owner' AND tx_to IS NULL",
            [&source_database_id],
            |row| row.get(0),
        )
        .unwrap();
    let row_id: String = projection
        .query_row(
            "SELECT record_id FROM hql2_record_revisions
             WHERE database_id=?1 AND namespace='default' AND kind='row'
               AND schema_ref='backup_records' AND tx_to IS NULL",
            [&source_database_id],
            |row| row.get(0),
        )
        .unwrap();
    drop(projection);
    source
        .put_annotation(
            access(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":"review:backup",
                    "namespace":"default",
                    "kind":"review",
                    "targets":[{
                    "ref":{"database_id":source_database_id,"namespace":"default","kind":"node","id":"backup:owner","revision":node_revision},
                        "binding":"frozen",
                        "selector":{"type":"whole"}
                    }],
                    "evidence":[{
                    "ref":{"database_id":source_database_id,"namespace":"default","kind":"node","id":"backup:owner","revision":node_revision},
                        "binding":"frozen",
                        "selector":{"type":"whole"}
                    }],
                    "body":{"type":"text","text":"Backup selector fixture."},
                    "author":"asserted-reviewer",
                    "created_at":"2010-01-01T00:00:00Z",
                    "valid_from":"2010-01-01T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();

    let frontier = source.stable_frontier();
    let exported = source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone(),
        })
        .unwrap();
    assert_eq!(exported.stable_frontier, frontier);
    let restored_bundle = Storage::restore_backup(BackupRestoreRequest {
        bundle_path,
        target_root: restore_root.clone(),
    })
    .unwrap();
    assert_eq!(restored_bundle.sha256, exported.sha256);
    assert_eq!(restored_bundle.stable_frontier, frontier);

    let restored = open(&restore_root);
    assert!(
        restored.stable_frontier() >= frontier,
        "restore may append a local P6 generation-publication frame"
    );
    assert_eq!(database_id(&restored), source_database_id);
    assert_eq!(source.history_horizon(), restored.history_horizon());
    assert_eq!(
        projection_state(&source_root, &source_database_id),
        projection_state(&restore_root, &source_database_id),
        "revision IDs, row IDs, schemas, annotation selectors and source floors must survive restore"
    );
    assert_ne!(
        row_id, "row:backup",
        "row revision IDs must be engine-issued"
    );

    let vector_id = serde_json::to_string(&("backup:owner", "default")).unwrap();
    let histories = [
        ("NODE", "node", "backup:owner"),
        ("ROW", "row", row_id.as_str()),
        ("VECTOR", "vector", vector_id.as_str()),
        ("ANNOTATION", "annotation", "review:backup"),
    ];
    let valid_at = chrono::Utc::now().to_rfc3339();
    for (index, (hql_kind, ir_kind, record_id)) in histories.into_iter().enumerate() {
        let hql = format!(
            "USE default AT VALID {} HISTORY {hql_kind} {} AS h |> RETURN h",
            serde_json::to_string(&valid_at).unwrap(),
            serde_json::to_string(record_id).unwrap()
        );
        let source_hql = run_hql(&source, &format!("history-source-hql-{index}"), &hql);
        let restored_hql = run_hql(&restored, &format!("history-restored-hql-{index}"), &hql);
        let source_ir = run_ir_history(
            &source,
            &format!("history-source-ir-{index}"),
            ir_kind,
            record_id,
            &valid_at,
        );
        let restored_ir = run_ir_history(
            &restored,
            &format!("history-restored-ir-{index}"),
            ir_kind,
            record_id,
            &valid_at,
        );
        assert_eq!(source_hql.snapshot.tx, restored_hql.snapshot.tx);
        assert_eq!(source_ir.snapshot.tx, restored_ir.snapshot.tx);
        assert_eq!(source_hql.columns, restored_hql.columns);
        assert_eq!(source_hql.rows, restored_hql.rows);
        assert_eq!(source_ir.columns, restored_ir.columns);
        assert_eq!(source_ir.rows, restored_ir.rows);
        assert_eq!(source_hql.columns, source_ir.columns);
        assert_eq!(source_hql.rows, source_ir.rows);
    }

    let source_hql_changes = run_hql(
        &source,
        "changes-source-hql",
        "USE default CHANGES SINCE 0 AS c |> RETURN c",
    );
    let restored_hql_changes = run_hql(
        &restored,
        "changes-restored-hql",
        "USE default CHANGES SINCE 0 AS c |> RETURN c",
    );
    let source_ir_changes = run_ir_changes(&source, "changes-source-ir", 0);
    let restored_ir_changes = run_ir_changes(&restored, "changes-restored-ir", 0);
    assert_eq!(source_hql_changes.rows, restored_hql_changes.rows);
    assert_eq!(source_ir_changes.rows, restored_ir_changes.rows);
    assert_eq!(source_hql_changes.columns, source_ir_changes.columns);
    assert_eq!(source_hql_changes.rows, source_ir_changes.rows);
}
