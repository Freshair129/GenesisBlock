use genesis_block_native::{
    BatchInput, EdgeInput, GenesisTransaction, HybridSearchInput, NeighborInput, NodeInput,
    OpenOptions, RelationalColumn, RelationalColumnType, RelationalFilter, RelationalMutationGroup,
    RelationalMutationKind, RelationalQuery, RelationalRowMutation, RelationalSchemaPackage,
    RelationalTable, SidecarReader, Storage,
};
use parking_lot::RwLock;
use serde_json::json;
use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;

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
        package_id: "00000000-0000-4000-8000-000000000005".to_string(),
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

fn transaction() -> GenesisTransaction {
    GenesisTransaction {
        transaction_id: "tx-p5-row-graph-vector".to_string(),
        expected_frontier: Some(0),
        relational: vec![RelationalMutationGroup {
            namespace: "fung".to_string(),
            mutations: vec![RelationalRowMutation {
                table: "notes".to_string(),
                kind: RelationalMutationKind::Upsert,
                values: json!({"id": "note-1", "title": "Unified P5"}),
                key: None,
            }],
        }],
        graph: BatchInput {
            nodes: vec![
                NodeInput {
                    id: Some("note-1".to_string()),
                    labels: vec!["Note".to_string()],
                    props: Some(json!({"title": "Unified P5"})),
                    embedding: None,
                    lang: Some("en".to_string()),
                    valid_from: None,
                    caused_by: None,
                    ttl: None,
                    collection: None,
                },
                NodeInput {
                    id: Some("project-1".to_string()),
                    labels: vec!["Project".to_string()],
                    props: Some(json!({"name": "Genesis"})),
                    embedding: None,
                    lang: Some("en".to_string()),
                    valid_from: None,
                    caused_by: None,
                    ttl: None,
                    collection: None,
                },
            ],
            edges: vec![EdgeInput {
                id: Some("edge-note-project".to_string()),
                from: "note-1".to_string(),
                to: "project-1".to_string(),
                rel: "BELONGS_TO".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: Some(0.5),
                caused_by: None,
            }],
        },
        vectors: vec![genesis_block_native::VectorUpsertInput {
            node_id: "note-1".to_string(),
            collection: "default".to_string(),
            embedding: vec![1.0, 0.0, 0.0, 0.0],
        }],
    }
}

fn assert_unified_state(storage: &Storage) {
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
    assert_eq!(rows, vec![json!({"notes.title": "Unified P5"})]);

    assert_eq!(
        storage.node_view("note-1").unwrap().props,
        json!({"title": "Unified P5"})
    );
    let neighbors = storage
        .neighbors(
            "note-1".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: Some("BELONGS_TO".to_string()),
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: Some(false),
                limit: Some(10),
            },
            false,
        )
        .unwrap();
    assert_eq!(
        neighbors
            .iter()
            .map(|neighbor| neighbor.node.id.as_str())
            .collect::<Vec<_>>(),
        vec!["project-1"]
    );

    storage.flush_index();
    let hits = storage
        .hybrid_search(HybridSearchInput {
            query_vector: vec![1.0, 0.0, 0.0, 0.0],
            k: 1,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: Some("default".to_string()),
            ef_search: None,
            oversample: None,
        })
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node.id, "note-1");

    let collection = storage
        .list_collections()
        .into_iter()
        .find(|collection| collection.name == "default")
        .unwrap();
    assert_eq!(collection.count, 1);
    assert_eq!(collection.indexed, 1);
}

fn remove_rebuildable_projections(path: &str) {
    for name in [
        "projection.sqlite",
        "projection.sqlite-wal",
        "projection.sqlite-shm",
        "state.json",
        "vec_default.bin",
        "meta_default.bin",
    ] {
        let file = Path::new(path).join(name);
        if file.exists() {
            fs::remove_file(file).unwrap();
        }
    }
}

#[test]
fn unified_transaction_commits_row_graph_vector_and_recovers_idempotently() {
    let path = fresh("unified_transaction_p5_row_graph_vector");
    let transaction = transaction();
    let committed = {
        let storage = open(&path);
        storage.register_relational_schema(schema()).unwrap();
        let committed = storage.commit_transaction(transaction.clone()).unwrap();
        assert!(committed.stable);
        assert_eq!(committed.commit_sequence, storage.txn_frontier());
        assert_eq!(committed.commit_sequence, storage.stable_frontier());
        assert_unified_state(&storage);

        let retry = storage.commit_transaction(transaction.clone()).unwrap();
        assert_eq!(retry.commit_sequence, committed.commit_sequence);
        assert_eq!(storage.list_collections()[0].count, 1);
        committed
    };

    // Removing the snapshot and every rebuildable projection forces the next
    // open to recover rows/graph/vector from the canonical WAL alone.
    remove_rebuildable_projections(&path);
    let storage = open(&path);
    assert_unified_state(&storage);
    assert_eq!(storage.txn_frontier(), committed.commit_sequence);
    assert_eq!(storage.stable_frontier(), committed.commit_sequence);

    let retry = storage.commit_transaction(transaction).unwrap();
    assert_eq!(retry.commit_sequence, committed.commit_sequence);
    assert_eq!(storage.list_collections()[0].count, 1);
}

#[test]
fn compacted_transaction_retry_keeps_original_local_frame_after_later_write() {
    let path = fresh("unified_transaction_p5_receipt_frontier");
    let transaction = transaction();
    let original_sequence = {
        let storage = open(&path);
        storage.register_relational_schema(schema()).unwrap();
        let committed = storage.commit_transaction(transaction.clone()).unwrap();
        storage
            .add_node(NodeInput {
                id: Some("ordinary-write".to_string()),
                labels: vec!["Other".to_string()],
                props: Some(json!({"kind": "later"})),
                embedding: None,
                lang: Some("en".to_string()),
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
        assert!(storage.stable_frontier() > committed.commit_sequence);
        storage.compact().unwrap();
        committed.commit_sequence
    };

    // The fold is the only remaining transaction receipt source. The receipt
    // must retain the transaction's original local frame, not the fold frame.
    let projection = Path::new(&path).join("projection.sqlite");
    if projection.exists() {
        fs::remove_file(projection).unwrap();
    }
    let storage = open(&path);
    let retry = storage.commit_transaction(transaction).unwrap();
    assert_eq!(retry.commit_sequence, original_sequence);
    assert_eq!(storage.txn_frontier(), original_sequence);
}

#[test]
fn vector_materialization_failure_is_not_reported_as_stable_and_recovers() {
    let path = fresh("unified_transaction_p5_vector_apply_failure");
    let transaction = GenesisTransaction {
        transaction_id: "tx-p5-vector-apply-failure".to_string(),
        expected_frontier: Some(0),
        relational: vec![],
        graph: BatchInput {
            nodes: vec![],
            edges: vec![],
        },
        vectors: vec![genesis_block_native::VectorUpsertInput {
            node_id: "vector-only".to_string(),
            collection: "rerank".to_string(),
            embedding: vec![1.0, 0.0, 0.0, 0.0],
        }],
    };

    {
        let storage = open(&path);
        storage
            .create_collection(
                "rerank".to_string(),
                "test-model".to_string(),
                4,
                Some("l2".to_string()),
                Some("sq8".to_string()),
                None,
                Some(true),
            )
            .unwrap();

        let readonly_fixture = Path::new(&path).join("readonly-sidecar.bin");
        File::create(&readonly_fixture).unwrap();
        let readonly_file = File::open(&readonly_fixture).unwrap();
        {
            let mut collection = storage.collections.get_mut("rerank").unwrap();
            let collection = Arc::get_mut(collection.value_mut()).unwrap();
            collection.f32_sidecar = Some(RwLock::new(SidecarReader::new(readonly_file, 4)));
        }

        let error = storage
            .commit_transaction(transaction.clone())
            .unwrap_err()
            .to_string();
        assert!(error.contains("DURABLE_COMMIT_APPLY_FAILED"), "{error}");
        assert!(error.contains("RECOVERY_REQUIRED"), "{error}");
    }

    let storage = open(&path);
    let collection = storage
        .list_collections()
        .into_iter()
        .find(|collection| collection.name == "rerank")
        .unwrap();
    assert_eq!(collection.count, 1);
    let retry = storage.commit_transaction(transaction).unwrap();
    assert!(retry.stable);
    assert_eq!(retry.commit_sequence, storage.txn_frontier());
}
