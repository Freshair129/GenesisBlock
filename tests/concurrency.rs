//! Concurrency and bulk-operation integration tests for GenesisBlockDB.
//!
//! Validates thread-safety of Storage (DashMap-backed, Send+Sync) under
//! concurrent writes, reads-while-writing, and bulk ingestion paths.

use genesis_block_native::{EdgeInput, HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

fn fresh(name: &str) -> String {
    let db_path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&db_path).exists() {
        fs::remove_dir_all(&db_path).unwrap();
    }
    db_path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

fn make_node(id: &str) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec!["Test".to_string()],
        props: None,
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

fn make_edge(id: &str, from: &str, to: &str) -> EdgeInput {
    EdgeInput {
        id: Some(id.to_string()),
        from: from.to_string(),
        to: to.to_string(),
        rel: "related_to".to_string(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    }
}

// ---------------------------------------------------------------------------
// 1. concurrent_node_writes
//    10 threads x 100 nodes = 1000 nodes total, no duplicates.
// ---------------------------------------------------------------------------
#[test]
fn concurrent_node_writes() {
    let db = Arc::new(open(&fresh("conc_node_writes")));
    let handles: Vec<_> = (0..10)
        .map(|t| {
            let db = Arc::clone(&db);
            thread::spawn(move || {
                for i in 0..100 {
                    let id = format!("t{}-n{}", t, i);
                    db.add_node(make_node(&id)).unwrap();
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("thread panicked");
    }

    assert_eq!(db.nodes.len(), 1000, "expected exactly 1000 nodes");
}

// ---------------------------------------------------------------------------
// 2. concurrent_edge_writes
//    20 seed nodes, 10 threads x 50 edges = 500 edges total.
// ---------------------------------------------------------------------------
#[test]
fn concurrent_edge_writes() {
    let db = Arc::new(open(&fresh("conc_edge_writes")));

    // Create 20 seed nodes
    let node_ids: Vec<String> = (0..20)
        .map(|i| {
            let id = format!("node-{}", i);
            db.add_node(make_node(&id)).unwrap();
            id
        })
        .collect();

    let node_ids = Arc::new(node_ids);
    let handles: Vec<_> = (0..10)
        .map(|t| {
            let db = Arc::clone(&db);
            let ids = Arc::clone(&node_ids);
            thread::spawn(move || {
                for i in 0..50 {
                    let edge_id = format!("e-t{}-{}", t, i);
                    let from_idx = (t * 50 + i) % ids.len();
                    let to_idx = (t * 50 + i + 1) % ids.len();
                    db.add_edge(make_edge(&edge_id, &ids[from_idx], &ids[to_idx]))
                        .unwrap();
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("thread panicked");
    }

    assert_eq!(db.edges.len(), 500, "expected exactly 500 edges");
}

// ---------------------------------------------------------------------------
// 3. concurrent_read_while_write
//    Writer inserts 200 nodes while a reader polls nodes.len() in a loop.
//    No panics, no deadlocks; writer finishes all 200.
// ---------------------------------------------------------------------------
#[test]
fn concurrent_read_while_write() {
    let db = Arc::new(open(&fresh("conc_read_write")));
    let db_w = Arc::clone(&db);
    let db_r = Arc::clone(&db);

    let writer = thread::spawn(move || {
        for i in 0..200 {
            db_w.add_node(make_node(&format!("rw-{}", i))).unwrap();
        }
    });

    let reader = thread::spawn(move || {
        let mut counts = Vec::with_capacity(100);
        for _ in 0..100 {
            counts.push(db_r.nodes.len());
        }
        counts
    });

    writer.join().expect("writer panicked");
    let counts = reader.join().expect("reader panicked");

    // After writer joins, all 200 nodes are in.
    assert_eq!(db.nodes.len(), 200);
    // Reader saw monotonically non-decreasing counts (inserts never retract).
    for window in counts.windows(2) {
        assert!(
            window[0] <= window[1],
            "node count should never decrease: {} -> {}",
            window[0],
            window[1]
        );
    }
}

// ---------------------------------------------------------------------------
// 4. bulk_add_nodes_1000
//    Insert 1000 nodes via bulk_add_nodes; verify count.
// ---------------------------------------------------------------------------
#[test]
fn bulk_add_nodes_1000() {
    let db = open(&fresh("bulk_nodes_1000"));

    let nodes: Vec<NodeInput> = (0..1000)
        .map(|i| make_node(&format!("bulk-n-{}", i)))
        .collect();

    db.bulk_add_nodes(nodes).unwrap();
    assert!(
        db.nodes.len() >= 1000,
        "expected at least 1000 nodes, got {}",
        db.nodes.len()
    );
}

// ---------------------------------------------------------------------------
// 5. bulk_add_edges_1000
//    100 seed nodes, then 1000 edges via bulk_add_edges.
// ---------------------------------------------------------------------------
#[test]
fn bulk_add_edges_1000() {
    let db = open(&fresh("bulk_edges_1000"));

    // Create 100 seed nodes
    let node_ids: Vec<String> = (0..100)
        .map(|i| {
            let id = format!("seed-{}", i);
            db.add_node(make_node(&id)).unwrap();
            id
        })
        .collect();

    let edges: Vec<EdgeInput> = (0..1000)
        .map(|i| {
            make_edge(
                &format!("bulk-e-{}", i),
                &node_ids[i % 100],
                &node_ids[(i + 1) % 100],
            )
        })
        .collect();

    db.bulk_add_edges(edges).unwrap();
    assert_eq!(db.edges.len(), 1000, "expected exactly 1000 edges");
}

// ---------------------------------------------------------------------------
// 6. concurrent_search_while_write
//    50 initial vector nodes (dim=4), flush, then writer adds 50 more while
//    reader runs hybrid_search 10 times. No panics.
// ---------------------------------------------------------------------------
#[test]
fn concurrent_search_while_write() {
    let db = Arc::new(open(&fresh("conc_search_write")));

    // Insert 50 initial nodes with varied embeddings
    for i in 0..50 {
        let angle = (i as f64) * 0.1;
        db.add_node(NodeInput {
            id: Some(format!("init-{}", i)),
            labels: vec!["Vec".to_string()],
            props: None,
            embedding: Some(vec![angle.cos(), angle.sin(), 0.0, 0.0]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    }
    db.flush_index();

    let db_w = Arc::clone(&db);
    let db_r = Arc::clone(&db);

    let writer = thread::spawn(move || {
        for i in 50..100 {
            let angle = (i as f64) * 0.1;
            db_w.add_node(NodeInput {
                id: Some(format!("late-{}", i)),
                labels: vec!["Vec".to_string()],
                props: None,
                embedding: Some(vec![angle.cos(), angle.sin(), 0.0, 0.0]),
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
        }
    });

    let reader = thread::spawn(move || {
        for _ in 0..10 {
            // Search may return partial results while writer is active; that is fine.
            let _results = db_r.hybrid_search(HybridSearchInput {
                query_vector: vec![1.0, 0.0, 0.0, 0.0],
                k: 5,
                alpha: None,
                lang: None,
                as_of: None,
                collection: None,
                ef_search: None,
                oversample: None,
            });
            // We don't assert result count — just that it doesn't panic.
        }
    });

    writer.join().expect("writer panicked");
    reader.join().expect("reader panicked");

    // All 100 nodes ingested
    assert_eq!(db.nodes.len(), 100);
}
