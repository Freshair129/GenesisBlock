// ADR--GENESISDB-ASYNC-INDEXING: HNSW insertion runs off the write hot path on
// a dedicated thread. Vectors are durable + in the arena synchronously, but
// become searchable only after the indexing queue drains (flush_index()).

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open_dim(path: &str, dim: u32) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(dim),
    })
    .unwrap()
}

fn add(s: &Storage, id: &str, emb: Vec<f64>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: Some(emb),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

fn search_ids(s: &Storage, q: Vec<f64>, k: u32) -> Vec<String> {
    s.hybrid_search(HybridSearchInput {
        query_vector: q,
        k,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: None,
        ef_search: None,
        oversample: None,
    })
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .collect()
}

/// After add + flush_index, the vector is searchable.
#[test]
fn searchable_after_flush() {
    let s = open_dim(&fresh("test_async_basic"), 3);
    add(&s, "v1", vec![1.0, 0.0, 0.0]);
    s.flush_index();
    assert_eq!(index_top(&s, vec![0.9, 0.1, 0.0]), Some("v1".to_string()));
}

fn index_top(s: &Storage, q: Vec<f64>) -> Option<String> {
    search_ids(s, q, 1).into_iter().next()
}

/// flush_index drives the lag counter to zero; the add path does not block on
/// HNSW construction (it returns; the vector is staged, indexing is deferred).
#[test]
fn flush_clears_index_lag() {
    let s = open_dim(&fresh("test_async_lag"), 3);
    for i in 0..50 {
        add(&s, &format!("n{i}"), vec![i as f64, 0.0, 0.0]);
    }
    s.flush_index();
    assert_eq!(
        s.index_lag(),
        0,
        "no vectors should remain unindexed after flush"
    );
}

/// A bulk load, then flush, leaves every vector searchable — none dropped.
#[test]
fn bulk_then_flush_indexes_all() {
    let s = open_dim(&fresh("test_async_bulk"), 3);
    let inputs: Vec<NodeInput> = (0..30)
        .map(|i| NodeInput {
            id: Some(format!("b{i}")),
            labels: vec![],
            props: None,
            embedding: Some(vec![i as f64, 1.0, 0.0]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .collect();
    s.bulk_add_nodes(inputs).unwrap();
    s.flush_index();

    // Each vector is in the index: an exact-match query (distance 0) returns
    // its own row as the top hit. (Count-based assertions are unreliable —
    // HNSW search is approximate — so we probe specific rows instead.)
    for i in [0u32, 7, 17, 29] {
        assert_eq!(
            index_top(&s, vec![i as f64, 1.0, 0.0]),
            Some(format!("b{i}")),
            "bulk vector b{i} must be indexed + retrievable after flush"
        );
    }
}

/// Crash/reopen (pure WAL replay, no snapshot) reconstructs the full HNSW
/// synchronously at open — searchable immediately, without an explicit flush.
#[test]
fn reopen_rebuilds_full_index() {
    let path = fresh("test_async_reopen");
    {
        let s = open_dim(&path, 3);
        add(&s, "p1", vec![1.0, 0.0, 0.0]);
        add(&s, "p2", vec![0.0, 1.0, 0.0]);
        s.flush_index();
        // no save_state -> reopen replays the WAL and rehydrates the index.
    }
    let s2 = open_dim(&path, 3);
    assert_eq!(index_top(&s2, vec![0.9, 0.1, 0.0]), Some("p1".to_string()));
    assert_eq!(index_top(&s2, vec![0.1, 0.9, 0.0]), Some("p2".to_string()));
}
