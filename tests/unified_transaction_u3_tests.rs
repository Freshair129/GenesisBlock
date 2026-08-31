use genesis_block_native::{
    BatchInput, GenesisTransaction, NodeInput, OpenOptions, RelationalColumn, RelationalColumnType,
    RelationalFilter, RelationalMutationGroup, RelationalMutationKind, RelationalQuery,
    RelationalRowMutation, RelationalSchemaPackage, RelationalTable, Storage,
};
use serde_json::json;
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&path).exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

fn schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "fung".to_string(),
        schema_version: 1,
        previous_version: None,
        package_id: "00000000-0000-4000-8000-000000000002".to_string(),
        schema_hash: String::new(),
        tables: vec![RelationalTable {
            name: "notes".to_string(),
            columns: vec![
                RelationalColumn::required("id", RelationalColumnType::Text),
                RelationalColumn::required("title", RelationalColumnType::Text),
            ],
            primary_key: vec!["id".to_string()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        named_queries: vec![],
    }
}

#[test]
fn one_wal_transaction_advances_stable_frontier_for_row_and_graph() {
    let path = fresh("unified_u3_row_graph");
    let storage = open(&path);
    storage.register_relational_schema(schema()).unwrap();

    let transaction = GenesisTransaction {
        transaction_id: "tx-note-1".to_string(),
        expected_frontier: Some(0),
        relational: vec![RelationalMutationGroup {
            namespace: "fung".to_string(),
            mutations: vec![RelationalRowMutation {
                table: "notes".to_string(),
                kind: RelationalMutationKind::Upsert,
                values: json!({"id": "note-1", "title": "Unified"}),
                key: None,
            }],
        }],
        graph: BatchInput {
            nodes: vec![NodeInput {
                id: Some("note-1".to_string()),
                labels: vec!["Note".to_string()],
                props: Some(json!({"title": "Unified"})),
                embedding: None,
                lang: Some("en".to_string()),
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            }],
            edges: vec![],
        },
        vectors: vec![],
    };
    let committed = storage.commit_transaction(transaction.clone()).unwrap();

    // WP-1.2 (ADR D2.3): commit_sequence is the transaction's FRAME seq. The
    // schema registration above already wrote a frame, so absolute values are
    // not stable — assert the frontier relationships instead: the txn frontier
    // IS this commit, and no frame landed after it.
    assert!(committed.commit_sequence >= 1);
    assert_eq!(committed.commit_sequence, storage.txn_frontier());
    assert_eq!(storage.stable_frontier(), committed.commit_sequence);
    assert!(storage.node_view("note-1").is_some());
    let rows = storage
        .query_relational(RelationalQuery {
            namespace: "fung".to_string(),
            table: "notes".to_string(),
            columns: vec!["notes.title".to_string()],
            joins: vec![],
            filters: vec![RelationalFilter::equal("notes.id", json!("note-1"))],
            limit: Some(1),
            offset: None,
        })
        .unwrap();
    assert_eq!(rows, vec![json!({"notes.title": "Unified"})]);

    let retry = storage.commit_transaction(transaction).unwrap();
    assert_eq!(retry.commit_sequence, committed.commit_sequence);
    assert_eq!(storage.stable_frontier(), committed.commit_sequence);
}

#[test]
fn stale_expected_frontier_is_rejected_before_commit() {
    let path = fresh("unified_u3_frontier_conflict");
    let storage = open(&path);
    let result = storage.commit_transaction(GenesisTransaction {
        transaction_id: "tx-stale".to_string(),
        expected_frontier: Some(4),
        relational: vec![],
        graph: BatchInput {
            nodes: vec![],
            edges: vec![],
        },
        vectors: vec![],
    });
    assert!(result.is_err());
    assert_eq!(storage.stable_frontier(), 0);
}

#[test]
fn compacted_wal_restores_frontier_and_transaction_identity() {
    let path = fresh("unified_u3_reopen");
    let transaction = GenesisTransaction {
        transaction_id: "tx-persisted".to_string(),
        expected_frontier: Some(0),
        relational: vec![],
        graph: BatchInput {
            nodes: vec![NodeInput {
                id: Some("persisted-node".to_string()),
                labels: vec!["Note".to_string()],
                props: Some(json!({"title": "Persisted"})),
                embedding: None,
                lang: Some("en".to_string()),
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            }],
            edges: vec![],
        },
        vectors: vec![],
    };
    {
        let storage = open(&path);
        storage.commit_transaction(transaction.clone()).unwrap();
        storage.compact().unwrap();
    }

    fs::remove_file(Path::new(&path).join("projection.sqlite")).unwrap();
    let storage = open(&path);
    assert_eq!(storage.stable_frontier(), 1);
    assert!(storage.node_view("persisted-node").is_some());
    let retry = storage.commit_transaction(transaction).unwrap();
    assert_eq!(retry.commit_sequence, 1);
}
