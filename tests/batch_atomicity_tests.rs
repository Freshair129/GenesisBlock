use genesis_block_native::{BatchInput, EdgeInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

#[test]
fn test_mark_ix_batch_atomicity() {
    let db_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_batch_db");
    if Path::new(db_path).exists() {
        fs::remove_dir_all(db_path).unwrap();
    }

    let storage = Storage::open(OpenOptions {
        path: db_path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
        retention: None,
    })
    .unwrap();

    // 1. Execute a successful batch (1 Node + 1 Edge)
    let batch = BatchInput {
        nodes: vec![NodeInput {
            id: Some("batched_node".to_string()),
            labels: vec!["BATCH".to_string()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        }],
        edges: vec![EdgeInput {
            id: Some("batched_edge".to_string()),
            from: "batched_node".to_string(),
            to: "batched_node".to_string(), // Self-loop for simplicity
            rel: "SELF".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        }],
    };

    let output = storage.execute_batch(batch).unwrap();
    assert_eq!(output.nodes.len(), 1);
    assert_eq!(output.edges.len(), 1);

    // Verify existence in memory. Nodes live in id_to_u32; edges are keyed by
    // their deterministic u64 hash and are NOT in id_to_u32
    // (ADR--GENESISDB-EDGE-NUMERIC-KEYS).
    assert!(storage.get_u32("batched_node").is_some());
    assert!(storage
        .edges
        .get(&Storage::edge_key("batched_edge"))
        .is_some());

    // 2. Test Atomic Failure (Invalid Governance Tier)
    let bad_batch = BatchInput {
        nodes: vec![
            NodeInput {
                id: Some("valid_node".to_string()),
                labels: vec!["USER".to_string()],
                props: None,
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            },
            NodeInput {
                id: Some("invalid_master_node".to_string()),
                labels: vec!["MASTER".to_string()], // Forbidden for agents (not is_system)
                props: None,
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            },
        ],
        edges: vec![],
    };

    let fail_res = storage.execute_batch(bad_batch);
    assert!(
        fail_res.is_err(),
        "Batch should fail due to MASTER tier violation"
    );

    // CRITICAL: Verify "valid_node" from the failed batch was NOT created
    assert!(
        storage.get_u32("valid_node").is_none(),
        "Atomicity failure: valid_node was created despite batch failure"
    );

    println!("Mark IX: Batch Atomicity verification successful.");
}
