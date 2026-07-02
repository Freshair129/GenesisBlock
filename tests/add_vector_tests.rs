// Same-node multi-vector (`add_vector` + `Event::Vector`): a node added with a
// primary embedding can carry ADDITIONAL vectors in other collections (e.g. a
// `code` embedding and a `text` embedding). The vector is durable (WAL
// `Event::Vector`) and survives reload. See ADR--GENESISDB-ADD-VECTOR.

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

fn add_node(s: &Storage, id: &str, emb: Vec<f64>, collection: Option<&str>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: Some(emb),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: collection.map(|c| c.to_string()),
    })
    .unwrap();
}

fn search(s: &Storage, q: Vec<f64>, k: u32, collection: Option<&str>) -> Vec<String> {
    s.flush_index();
    s.hybrid_search(HybridSearchInput {
        query_vector: q,
        k,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: collection.map(|c| c.to_string()),
        ef_search: None,
        oversample: None,
    })
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .collect()
}

/// A node carries a vector in its primary collection AND a second vector in
/// another collection; each is searchable in its own space.
#[test]
fn node_carries_vectors_in_two_collections() {
    let s = open_dim(&fresh("test_av_two"), 4);
    s.create_collection(
        "code".to_string(),
        "jina-code".to_string(),
        3,
        Some("L2".to_string()),
        None,
        None,
        None,
    )
    .unwrap();

    // Primary embedding in the default collection (dim 4).
    add_node(&s, "N1", vec![1.0, 0.0, 0.0, 0.0], None);
    // Second vector for the same node in "code" (dim 3).
    s.add_vector("N1".to_string(), "code".to_string(), vec![0.0, 1.0, 0.0])
        .unwrap();

    let in_default = search(&s, vec![1.0, 0.0, 0.0, 0.0], 5, None);
    assert!(
        in_default.contains(&"N1".to_string()),
        "primary vector searchable in default"
    );

    let in_code = search(&s, vec![0.0, 1.0, 0.0], 5, Some("code"));
    assert!(
        in_code.contains(&"N1".to_string()),
        "attached vector searchable in code"
    );
}

/// Attaching a vector to a node that doesn't exist is a typed error.
#[test]
fn add_vector_to_missing_node_errors() {
    let s = open_dim(&fresh("test_av_missing"), 4);
    s.create_collection(
        "code".to_string(),
        "m".to_string(),
        3,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let r = s.add_vector("ghost".to_string(), "code".to_string(), vec![1.0, 0.0, 0.0]);
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("not found"));
}

/// A wrong-dim embedding for the target collection is rejected (nothing staged).
#[test]
fn add_vector_dim_mismatch_errors() {
    let s = open_dim(&fresh("test_av_dim"), 4);
    s.create_collection(
        "code".to_string(),
        "m".to_string(),
        3,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    add_node(&s, "N1", vec![1.0, 0.0, 0.0, 0.0], None);
    let r = s.add_vector("N1".to_string(), "code".to_string(), vec![1.0, 0.0]); // 2 != 3
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("dim"));
}

/// An unknown target collection errors (the live path is strict; only WAL
/// replay / CRDT sync auto-provisions).
#[test]
fn add_vector_unknown_collection_errors() {
    let s = open_dim(&fresh("test_av_unknown"), 4);
    add_node(&s, "N1", vec![1.0, 0.0, 0.0, 0.0], None);
    let r = s.add_vector("N1".to_string(), "nope".to_string(), vec![1.0, 0.0, 0.0]);
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("not found"));
}

/// The attached vector is durable: a `Event::Vector` replays on reopen (pure WAL,
/// no snapshot) into its (auto-provisioned) collection and stays searchable.
#[test]
fn attached_vector_survives_wal_replay() {
    let path = fresh("test_av_wal");
    {
        let s = open_dim(&path, 4);
        s.create_collection(
            "code".to_string(),
            "jina-code".to_string(),
            3,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        add_node(&s, "N1", vec![1.0, 0.0, 0.0, 0.0], None);
        s.add_vector("N1".to_string(), "code".to_string(), vec![0.0, 0.0, 1.0])
            .unwrap();
        // No save_state() -> reopen replays the WAL (Event::Node + Event::Vector).
    }
    let s2 = open_dim(&path, 4);
    let in_code = search(&s2, vec![0.0, 0.0, 1.0], 5, Some("code"));
    assert!(
        in_code.contains(&"N1".to_string()),
        "attached vector replayed from WAL and searchable"
    );
    // Primary vector also intact.
    let in_default = search(&s2, vec![1.0, 0.0, 0.0, 0.0], 5, None);
    assert!(in_default.contains(&"N1".to_string()));
}

/// Re-adding a vector for the same node in a collection it already has supersedes
/// the node->arena mapping but leaves the old arena/HNSW slot until compaction, so
/// the raw index can surface the node twice. Search must dedupe by node id.
#[test]
fn readding_vector_does_not_duplicate_search_hits() {
    let s = open_dim(&fresh("test_av_dedup"), 4);
    add_node(&s, "N1", vec![1.0, 0.0, 0.0, 0.0], None); // primary vector in default
                                                        // Attach a second vector for N1 in the SAME (default) collection.
    s.add_vector(
        "N1".to_string(),
        "default".to_string(),
        vec![1.0, 0.0, 0.0, 0.0],
    )
    .unwrap();

    let hits = search(&s, vec![1.0, 0.0, 0.0, 0.0], 5, None);
    let n1_count = hits.iter().filter(|id| id.as_str() == "N1").count();
    assert_eq!(
        n1_count, 1,
        "a superseded vector must not surface the node twice"
    );
}
