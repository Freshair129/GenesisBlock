// Recall floor beneath the approximate index (RCA--HNSW-UNDER-RETURN-SMALL-GRAPH).
//
// `hnsw_rs` can return ONLY the entry point on a small graph — when that entry
// point's layer-0 neighbour list was pruned empty, the greedy expansion
// terminates immediately. Recall then collapses rather than degrading: a
// 24-vector collection asked for 10 results returned exactly 1 (the query node
// itself) in 2 of 150 fresh graphs, and never 2-9. Layer assignment is random,
// which is why it is intermittent — it surfaced as a recurring CI flake in
// tests/hql_p0_tests.rs.
//
// `hybrid_search` now detects an under-return and rebuilds the candidate set by
// exhaustive scan when the collection is small enough (EXACT_SCAN_MAX_SLOTS).
// These tests pin that contract: on a collection small enough to scan, a search
// returns exactly min(k, live vectors) — the engine never silently hands back a
// near-empty result set for a well-populated collection.
//
// Each iteration builds a FRESH graph because the defect depends on the random
// layer assignment drawn at insert time; a single graph proves nothing.

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

/// Fresh graphs per test. The pre-fix defect hit ~1.3% of graphs, so this is
/// sized to catch a regression with high probability while staying quick.
const GRAPHS: usize = 60;

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
        page_cache_mb: Some(32),
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

fn search(s: &Storage, q: Vec<f64>, k: u32) -> Vec<String> {
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

/// The query vector is an indexed point itself — the case that failed, because
/// the query node is the one result a stalled expansion still returns.
#[test]
fn small_collection_returns_full_k_on_every_fresh_graph() {
    let mut short: Vec<(usize, usize, Vec<String>)> = Vec::new();
    for g in 0..GRAPHS {
        let p = fresh(&format!("recall_floor_k_{g}"));
        {
            let s = open_dim(&p, 2);
            add(&s, "q", vec![0.0, 0.0]);
            add(&s, "near", vec![0.1, 0.01]);
            add(&s, "mid", vec![1.0, -0.02]);
            add(&s, "far", vec![10.0, 0.03]);
            for i in 0..20 {
                add(
                    &s,
                    &format!("filler{i}"),
                    vec![50.0 + i as f64, (i as f64 % 7.0) - 3.0],
                );
            }
            s.flush_index();

            let hits = search(&s, vec![0.0, 0.0], 10);
            if hits.len() != 10 {
                short.push((g, hits.len(), hits));
            }
        }
        let _ = fs::remove_dir_all(&p);
    }
    assert!(
        short.is_empty(),
        "{} of {GRAPHS} fresh graphs returned fewer than the requested 10 hits \
         from a 24-vector collection — the exact-scan recall floor did not \
         engage: {:?}",
        short.len(),
        &short[..short.len().min(4)]
    );
}

/// `k` larger than the collection must return every vector, not a truncated
/// walk of whatever the graph happened to reach.
#[test]
fn k_larger_than_collection_returns_every_vector() {
    for g in 0..GRAPHS {
        let p = fresh(&format!("recall_floor_all_{g}"));
        {
            let s = open_dim(&p, 2);
            for i in 0..12 {
                add(&s, &format!("n{i}"), vec![i as f64 * 0.5, (i % 3) as f64]);
            }
            s.flush_index();

            let hits = search(&s, vec![0.0, 0.0], 50);
            assert_eq!(
                hits.len(),
                12,
                "graph {g}: k=50 over a 12-vector collection must return all 12, got {:?}",
                hits
            );
        }
        let _ = fs::remove_dir_all(&p);
    }
}
