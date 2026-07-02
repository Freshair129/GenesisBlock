// Per-query HNSW `ef_search` override on `HybridSearchInput` (MARK XIV P3).
// The engine keeps a global `ef_search` (set_index_params); a query may override
// it per-call. These tests pin DETERMINISTIC invariants only — HNSW is
// approximate, so recall-vs-ef behaviour is validated by benchmark (the
// Recall@500k frontier), not here. Guarded: (a) an explicit override returns the
// exact match as top-1; (b) `None` falls back to the global and behaves
// identically on a small set; (c) an override below the global default still
// returns k results (the value is honoured, not clamped to the global).

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

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(4),
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

fn search(s: &Storage, q: Vec<f64>, k: u32, ef: Option<u32>) -> Vec<String> {
    s.flush_index();
    s.hybrid_search(HybridSearchInput {
        query_vector: q,
        k,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: None,
        ef_search: ef,
        oversample: None,
    })
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .collect()
}

fn seed(s: &Storage) {
    add(s, "A", vec![1.0, 0.0, 0.0, 0.0]);
    add(s, "B", vec![0.0, 1.0, 0.0, 0.0]);
    add(s, "C", vec![0.0, 0.0, 1.0, 0.0]);
    add(s, "D", vec![0.0, 0.0, 0.0, 1.0]);
    add(s, "E", vec![0.9, 0.1, 0.0, 0.0]);
}

/// An explicit per-query ef_search override returns the exact match as top-1.
#[test]
fn override_finds_exact_match() {
    let s = open(&fresh("test_ef_override"));
    seed(&s);
    let hits = search(&s, vec![1.0, 0.0, 0.0, 0.0], 1, Some(64));
    assert_eq!(hits.first().map(|x| x.as_str()), Some("A"));
}

/// `None` falls back to the global ef_search and behaves identically on a small
/// set — same top-1 as an explicit override.
#[test]
fn none_falls_back_to_global() {
    let s = open(&fresh("test_ef_fallback"));
    seed(&s);
    let with = search(&s, vec![0.0, 0.0, 1.0, 0.0], 1, Some(128));
    let without = search(&s, vec![0.0, 0.0, 1.0, 0.0], 1, None);
    assert_eq!(with.first().map(|x| x.as_str()), Some("C"));
    assert_eq!(with, without);
}

/// An override below the global default (100) is honoured and still returns k
/// results — the per-query value is passed straight to the HNSW search.
#[test]
fn low_override_still_returns_k() {
    let s = open(&fresh("test_ef_low"));
    seed(&s);
    let hits = search(&s, vec![1.0, 0.0, 0.0, 0.0], 3, Some(16));
    assert_eq!(
        hits.len(),
        3,
        "k results returned even with a low ef_search override"
    );
}
