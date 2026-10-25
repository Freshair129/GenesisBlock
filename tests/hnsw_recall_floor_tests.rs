// Recall floors beneath the approximate index (RCA--HNSW-UNDER-RETURN-SMALL-GRAPH;
// RCA--HNSW-SMALL-COLLECTION-RANK-RECALL).
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
        retention: None,
    })
    .unwrap()
}

fn node_input(id: &str, emb: Vec<f64>, collection: Option<&str>) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: Some(emb),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: collection.map(str::to_string),
    }
}

fn add_to(s: &Storage, id: &str, emb: Vec<f64>, collection: Option<&str>) {
    s.add_node(node_input(id, emb, collection)).unwrap();
}

fn add(s: &Storage, id: &str, emb: Vec<f64>) {
    add_to(s, id, emb, None);
}

fn search_in(
    s: &Storage,
    q: Vec<f64>,
    k: u32,
    collection: Option<&str>,
    ef_search: Option<u32>,
) -> Vec<String> {
    s.hybrid_search(HybridSearchInput {
        query_vector: q,
        k,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: collection.map(str::to_string),
        ef_search,
        oversample: None,
    })
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .collect()
}

fn search(s: &Storage, q: Vec<f64>, k: u32) -> Vec<String> {
    search_in(s, q, k, None, None)
}

fn next_unit(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let unit = (*seed >> 11) as f64 / ((1u64 << 53) as f64);
    unit * 2.0 - 1.0
}

fn seeded_vectors(count: usize, dim: usize, mut seed: u64) -> Vec<Vec<f64>> {
    (0..count)
        .map(|_| (0..dim).map(|_| next_unit(&mut seed)).collect())
        .collect()
}

fn prepare_oracle_vector(values: &[f64], cosine: bool) -> Vec<f32> {
    let mut vector: Vec<f32> = values.iter().map(|value| *value as f32).collect();
    if cosine {
        let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm > 0.0 {
            vector.iter_mut().for_each(|value| *value /= norm);
        }
    }
    vector
}

fn exact_top_k(vectors: &[Vec<f64>], query: &[f64], cosine: bool, k: usize) -> Vec<String> {
    let query = prepare_oracle_vector(query, cosine);
    let mut distances: Vec<(usize, f32)> = vectors
        .iter()
        .enumerate()
        .map(|(index, values)| {
            let vector = prepare_oracle_vector(values, cosine);
            let distance = query
                .iter()
                .zip(vector)
                .map(|(left, right)| {
                    let delta = left - right;
                    delta * delta
                })
                .sum::<f32>()
                .sqrt();
            (index, distance)
        })
        .collect();
    distances.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    distances
        .into_iter()
        .take(k)
        .map(|(index, _)| format!("v{index:03}"))
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

/// A full-sized approximate shortlist can still miss the exact nearest rows.
/// Rebuild fresh graphs so this checks recall, not just the old under-return
/// fallback. The vectors and query set are seeded and independent of corpus
/// data; low ef_search makes the approximate path's bounded candidate window
/// visible without changing its configured floor.
#[test]
fn small_lossless_l2_and_cosine_top_k_matches_exhaustive_oracle() {
    const DIM: usize = 24;
    const NODES: usize = 256;
    const QUERIES: usize = 6;
    const K: usize = 8;
    const FRESH_GRAPHS: usize = 4;

    let vectors = seeded_vectors(NODES, DIM, 0x51A7_2026_1026);
    let queries = seeded_vectors(QUERIES, DIM, 0xA11C_E5ED_2026);

    for (metric, cosine) in [("l2", false), ("cosine", true)] {
        let collection = if cosine { Some("cosine") } else { None };
        for graph in 0..FRESH_GRAPHS {
            let path = fresh(&format!("exact_recall_{metric}_{graph}"));
            {
                let storage = open_dim(&path, DIM as u32);
                if cosine {
                    storage
                        .create_collection(
                            "cosine".into(),
                            "synthetic-recall-test".into(),
                            DIM as u32,
                            Some("cosine".into()),
                            None,
                            Some(1),
                            Some(false),
                        )
                        .unwrap()
                }
                storage
                    .bulk_add_nodes(
                        vectors
                            .iter()
                            .enumerate()
                            .map(|(index, vector)| {
                                node_input(&format!("v{index:03}"), vector.clone(), collection)
                            })
                            .collect(),
                    )
                    .unwrap();
                storage.flush_index();
                drop(storage);
            }
            {
                // Reopening rebuilds the approximate graph from durable arena
                // state, which is the lifecycle that varied in the paired run.
                let storage = open_dim(&path, DIM as u32);

                for (query_index, query) in queries.iter().enumerate() {
                    let expected = exact_top_k(&vectors, query, cosine, K);
                    let actual = search_in(&storage, query.clone(), K as u32, collection, Some(1));
                    assert_eq!(
                        actual,
                        expected,
                        "metric={metric}, graph={graph}, query={query_index}: low-ef small-collection results must match exhaustive top-{K}"
                    );
                }
            }
            let _ = fs::remove_dir_all(&path);
        }
    }
}

/// Exact work is still subject to the existing candidate-window budget. The
/// floor changes how the window is populated, not how many candidates a query
/// may request or return.
#[test]
fn exact_recall_floor_preserves_candidate_budget_rejection() {
    let path = fresh("exact_recall_budget");
    let storage = open_dim(&path, 2);
    add(&storage, "a", vec![0.1, 0.0]);
    add(&storage, "b", vec![0.2, 0.0]);

    let error = storage
        .execute_query_ir_json(serde_json::json!({
            "contract_version": "query-ir.v1",
            "request_id": "small-exact-recall-budget",
            "budget": { "max_vector_candidates": 1 },
            "operation": {
                "kind": "search",
                "mode": "vector",
                "query_vector": [0.0, 0.0],
                "k": 1
            }
        }))
        .expect_err("fetch=2 must fail before exact scan under a one-candidate budget");
    assert_eq!(
        error.to_string(),
        "QUERY_BUDGET_EXCEEDED: reason=candidates"
    );
}

/// A current-view refill after retired near rows must still find enough live
/// candidates. The initial exact window is deliberately consumed by retired
/// vectors; widening it must rerun the exact ranking over the larger window.
#[test]
fn exact_small_view_refills_after_retired_nearest_rows() {
    let path = fresh("exact_recall_retired_refill");
    let storage = open_dim(&path, 2);
    let mut inputs = Vec::new();
    for index in 0..3 {
        inputs.push(node_input(
            &format!("live-{index}"),
            vec![0.0, 10.0 + index as f64],
            None,
        ));
    }
    for index in 0..4 {
        inputs.push(node_input(
            &format!("retired-{index}"),
            vec![1.0, 0.001 * index as f64],
            None,
        ));
    }
    storage.bulk_add_nodes(inputs).unwrap();
    storage.flush_index();
    for index in 0..4 {
        storage.retract_node(&format!("retired-{index}")).unwrap();
    }

    let actual = search_in(&storage, vec![1.0, 0.0], 3, None, Some(1));
    assert_eq!(actual, vec!["live-0", "live-1", "live-2"]);

    drop(storage);
    let _ = fs::remove_dir_all(&path);
}
