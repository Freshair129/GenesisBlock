//! HQL P0 refinement tests (PLAN--HQL-REFINEMENT P0-T1..T5): search-by-node
//! target semantics, hybrid `K <n>`, `EF`/`OVERSAMPLE` clauses, TRAVERSE
//! direction + multi-rel, strict numeric parse errors, and colon-qualified
//! seed/target ids. Grammar/semantics SSOT: docs/DESIGN--HQL-P0-DECISIONS.md.

use genesis_block_native::query::ast::HqlCommand;
use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use std::convert::TryFrom;
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
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(dim),
        retention: None,
    })
    .unwrap()
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap()
}

fn add(s: &Storage, id: &str, emb: Option<Vec<f64>>, collection: Option<&str>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: emb,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: collection.map(|c| c.to_string()),
    })
    .unwrap();
}

fn node(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

fn edge(s: &Storage, from: &str, to: &str, rel: &str) {
    s.add_edge(EdgeInput {
        id: None,
        from: from.to_string(),
        to: to.to_string(),
        rel: rel.to_string(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
}

/// Extract node ids from an `execute_hql` search/hybrid result value, in order.
fn ids(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect()
}

// =========================================================================
// P0-T1 — search-by-node target semantics
// =========================================================================

/// Seed `q` and three candidates at increasing, non-collinear distance, plus
/// enough far-away filler nodes to clear HNSW's `max_nb_connection` (16) by a
/// comfortable margin. RCA--HQL-P0-SEARCH-BY-NODE-FLAKE: an exactly-collinear
/// 4-point graph is the documented degenerate case for `hnsw_rs`'s
/// neighbor-pruning heuristic (same hazard class as
/// RCA--BULK-PARALLEL-INSERT-UNREACHABLE); breaking collinearity and growing
/// past the tiny-graph regime removes that hazard without loosening what the
/// tests assert. Filler nodes sit far past `far` so they can never enter a
/// top-3 result.
fn seed_search_by_node_fixture(s: &Storage) {
    add(s, "q", Some(vec![0.0, 0.0]), None);
    add(s, "near", Some(vec![0.1, 0.01]), None);
    add(s, "mid", Some(vec![1.0, -0.02]), None);
    add(s, "far", Some(vec![10.0, 0.03]), None);
    for i in 0..20 {
        let x = 50.0 + i as f64;
        let y = (i as f64 % 7.0) - 3.0;
        add(s, &format!("filler{i}"), Some(vec![x, y]), None);
    }
}

/// `SEARCH <known-node-id> K 3` (no vector) returns nearest-by-that-node's
/// embedding. Fixture: seed `q` and three candidates at increasing distance;
/// top-1 (other than `q` itself, k excludes nothing but we assert the closest
/// non-self candidate ranks first) is unambiguous.
#[test]
fn search_by_node_no_vector_returns_nearest_by_embedding() {
    let p = fresh("p0_search_by_node");
    let s = open_dim(&p, 2);
    seed_search_by_node_fixture(&s);
    s.flush_index();

    let res = s.execute_hql("SEARCH q K 3").unwrap();
    let hits = ids(&res);
    assert_eq!(
        hits.len(),
        3,
        "SEARCH q K 3 must return exactly 3 hits (q, near, mid), got {:?}",
        hits
    );
    assert_eq!(
        hits[0], "q",
        "top-1 must be q itself (distance 0 to its own embedding), got {:?}",
        hits
    );
    assert_eq!(
        hits[1], "near",
        "second-nearest must be the closest OTHER node to q's own embedding, got {:?}",
        hits
    );
}

/// Same semantics via the hybrid `MATCH <id> ALPHA 0.0` form (no vector).
#[test]
fn hybrid_by_node_no_vector_returns_nearest_by_embedding() {
    let p = fresh("p0_hybrid_by_node");
    let s = open_dim(&p, 2);
    seed_search_by_node_fixture(&s);
    s.flush_index();

    let res = s.execute_hql("MATCH q ALPHA 0.0").unwrap();
    let hits = ids(&res);
    // MATCH's default K is 10 (src/query/ast.rs); the fixture's 24 total nodes
    // comfortably cover that, so a short result set is a real regression, not
    // just "K wasn't reached" — assert the exact count before indexing.
    assert_eq!(
        hits.len(),
        10,
        "MATCH q ALPHA 0.0 must return exactly 10 hits (default K), got {:?}",
        hits
    );
    assert_eq!(
        hits[0], "q",
        "top-1 must be q itself (distance 0 to its own embedding), got {:?}",
        hits
    );
    assert_eq!(
        hits[1], "near",
        "second-nearest must be the closest OTHER node to q's own embedding, got {:?}",
        hits
    );
}

/// Verify against a direct `hybrid_search` call using the same embedding
/// (review-gate item 2 of P0-T1): the no-vector path must produce the same
/// top-1 as manually fetching the node's embedding and searching with it.
#[test]
fn search_by_node_matches_direct_hybrid_search_with_same_embedding() {
    let p = fresh("p0_search_by_node_direct_check");
    let s = open_dim(&p, 2);
    add(&s, "q", Some(vec![0.0, 0.0]), None);
    add(&s, "near", Some(vec![0.1, 0.0]), None);
    add(&s, "mid", Some(vec![1.0, 0.0]), None);
    s.flush_index();

    let via_hql = s.execute_hql("SEARCH q K 3").unwrap();
    let via_hql_ids = ids(&via_hql);

    let direct = s
        .hybrid_search(genesis_block_native::HybridSearchInput {
            query_vector: vec![0.0, 0.0],
            k: 3,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: None,
            ef_search: None,
            oversample: None,
        })
        .unwrap();
    let direct_ids: Vec<String> = direct.iter().map(|n| n.node.id.clone()).collect();

    assert_eq!(
        via_hql_ids, direct_ids,
        "search-by-node must match a direct hybrid_search using the node's own embedding"
    );
}

/// Unresolvable target (no vector) -> Err containing the design doc's message.
#[test]
fn search_by_node_unresolvable_target_errors() {
    let p = fresh("p0_search_unresolvable");
    let s = open_dim(&p, 2);
    add(&s, "q", Some(vec![0.0, 0.0]), None);
    s.flush_index();

    let res = s.execute_hql("SEARCH does-not-exist K 3");
    assert!(res.is_err(), "unresolvable target must error");
    let msg = res.unwrap_err().to_string();
    assert!(
        msg.contains("does not resolve to a node and no vector was given"),
        "unexpected message: {msg}"
    );
}

/// Node without a stored embedding (no vector given) -> Err.
#[test]
fn search_by_node_no_embedding_errors() {
    let p = fresh("p0_search_no_embedding");
    let s = open_dim(&p, 2);
    node(&s, "bare"); // no embedding
    s.flush_index();

    let res = s.execute_hql("SEARCH bare K 3");
    assert!(res.is_err(), "node without embedding must error");
    let msg = res.unwrap_err().to_string();
    assert!(
        msg.contains("has no stored embedding and no vector was given"),
        "unexpected message: {msg}"
    );
}

/// `IN <other-collection>` conflicting with the node's own collection errors.
#[test]
fn search_by_node_in_conflicting_collection_errors() {
    let p = fresh("p0_search_in_conflict");
    let s = open_dim(&p, 2);
    s.create_collection(
        "other".to_string(),
        "model".to_string(),
        2,
        Some("L2".to_string()),
        None,
        None,
        None,
    )
    .unwrap();
    add(&s, "q", Some(vec![0.0, 0.0]), None); // lives in "default"
    s.flush_index();

    let res = s.execute_hql("SEARCH q K 3 IN other");
    assert!(res.is_err(), "IN naming a different collection must error");
    let msg = res.unwrap_err().to_string();
    assert!(
        msg.contains("lives in collection")
            && msg.contains("but IN")
            && msg.contains("omit IN or match the node's collection"),
        "unexpected message: {msg}"
    );
}

/// Literal-vector SEARCH: existing-style query still returns the same shape
/// (back-compat oracle — asserted directly here, not just via unedited
/// pre-existing tests).
#[test]
fn search_literal_vector_unchanged_shape() {
    let p = fresh("p0_search_literal_vector");
    let s = open_dim(&p, 2);
    add(&s, "a", Some(vec![1.0, 0.0]), None);
    add(&s, "b", Some(vec![0.0, 1.0]), None);
    s.flush_index();

    let res = s.execute_hql("SEARCH q SIMILAR TO [1.0,0.0] K 2").unwrap();
    let hits = ids(&res);
    assert_eq!(hits[0], "a", "literal vector search must rank a first");
}

/// Literal-vector MATCH…SIMILAR: byte-identical result shape.
#[test]
fn match_literal_vector_unchanged_shape() {
    let p = fresh("p0_match_literal_vector");
    let s = open_dim(&p, 2);
    add(&s, "a", Some(vec![1.0, 0.0]), None);
    add(&s, "b", Some(vec![0.0, 1.0]), None);
    s.flush_index();

    let res = s
        .execute_hql("MATCH q SIMILAR TO [1.0,0.0] ALPHA 0.0")
        .unwrap();
    let hits = ids(&res);
    assert_eq!(hits[0], "a", "literal vector hybrid must rank a first");
}

// =========================================================================
// P0-T2 — hybrid K <n>
// =========================================================================

/// Hybrid `K 50` must widen the candidate pool beyond the default 10. The
/// assertion deliberately avoids requiring any exact ANN neighbor: HNSW is
/// approximate even when `EF` is larger than this fixture.
#[test]
fn hybrid_k_widens_pool_past_default_10() {
    let p = fresh("p0_hybrid_k");
    let s = open_dim(&p, 2);
    // 30 decoy nodes closer to the query than the real target, all within the
    // same tight cluster as target-hit (not a far outlier) so HNSW graph
    // connectivity — not just search-time `ef` — reliably reaches it; a
    // far-outlier fixture is flaky under concurrent-test CPU contention
    // because greedy graph routing (not just ef) can miss a poorly-connected
    // distant point, which is a real ANN property, not a K-clause bug.
    for i in 0..30 {
        let d = 0.01 * (i as f64 + 1.0);
        add(&s, &format!("decoy{i}"), Some(vec![d, 0.0]), None);
    }
    // The target hit: farther than every decoy but still inside the same
    // local neighborhood (0.31..0.5 vs decoys' 0.01..0.30).
    add(&s, "target-hit", Some(vec![0.5, 0.0]), None);
    s.flush_index();

    // Default K (10, absent from grammar) misses target-hit: only the 10
    // closest decoys are in the pool.
    let default_res = s
        .execute_hql("MATCH q SIMILAR TO [0.0,0.0] ALPHA 0.0")
        .unwrap();
    let default_hits = ids(&default_res);
    assert!(
        !default_hits.contains(&"target-hit".to_string()),
        "default K=10 pool must NOT contain target-hit (fixture invariant), got {:?}",
        default_hits
    );

    // K 50 widens the candidate pool. EF 200 improves recall, but does not
    // make HNSW exhaustive; compare pool sizes instead of requiring one
    // particular approximate neighbor.
    let wide_res = s
        .execute_hql("MATCH q SIMILAR TO [0.0,0.0] ALPHA 0.0 K 50 EF 200")
        .unwrap();
    let wide_hits = ids(&wide_res);
    assert!(
        wide_hits.len() > default_hits.len(),
        "K 50 must widen the default pool ({}), got {}: {:?}",
        default_hits.len(),
        wide_hits.len(),
        wide_hits
    );
}

/// Hybrid without K is unchanged (still parses/executes, default pool = 10).
#[test]
fn hybrid_without_k_unchanged() {
    let p = fresh("p0_hybrid_no_k");
    let s = open_dim(&p, 2);
    add(&s, "a", Some(vec![1.0, 0.0]), None);
    s.flush_index();

    let res = s
        .execute_hql("MATCH q SIMILAR TO [1.0,0.0] ALPHA 0.0")
        .unwrap();
    let hits = ids(&res);
    assert_eq!(hits[0], "a");
}

/// K composes with ALPHA/IN/LANGUAGE/AS OF/clauses in the documented order.
#[test]
fn hybrid_k_composes_with_other_clauses() {
    let p = fresh("p0_hybrid_k_compose");
    let s = open_dim(&p, 2);
    add(&s, "a", Some(vec![1.0, 0.0]), None);
    add(&s, "b", Some(vec![0.9, 0.1]), None);
    s.flush_index();

    let res = s
        .execute_hql(
            "MATCH q SIMILAR TO [1.0,0.0] ALPHA 0.5 K 50 LANGUAGE \"en\" AS OF \"2099-01-01T00:00:00Z\" LIMIT 5",
        )
        .unwrap();
    let hits = ids(&res);
    assert!(hits.len() <= 5);
}

// =========================================================================
// P0-T3 — EF / OVERSAMPLE clauses
// =========================================================================

/// Parse-level assertions: EF/OVERSAMPLE fields land in the AST for both
/// SEARCH and hybrid.
#[test]
fn ef_oversample_parse_into_ast_search() {
    let q = "SEARCH mynode SIMILAR TO [1.0,2.0] K 5 EF 512 OVERSAMPLE 8";
    match HqlCommand::try_from(q).unwrap() {
        HqlCommand::Search {
            ef_search,
            oversample,
            ..
        } => {
            assert_eq!(ef_search, Some(512));
            assert_eq!(oversample, Some(8));
        }
        _ => panic!("expected Search variant"),
    }
}

#[test]
fn ef_oversample_parse_into_ast_hybrid() {
    let q = "MATCH mynode SIMILAR TO [1.0,2.0] ALPHA 0.5 EF 256 OVERSAMPLE 4";
    match HqlCommand::try_from(q).unwrap() {
        HqlCommand::Hybrid {
            ef_search,
            oversample,
            ..
        } => {
            assert_eq!(ef_search, Some(256));
            assert_eq!(oversample, Some(4));
        }
        _ => panic!("expected Hybrid variant"),
    }
}

/// Omitted EF/OVERSAMPLE ⇒ None.
#[test]
fn ef_oversample_omitted_is_none() {
    let q = "SEARCH mynode SIMILAR TO [1.0,2.0] K 5";
    match HqlCommand::try_from(q).unwrap() {
        HqlCommand::Search {
            ef_search,
            oversample,
            ..
        } => {
            assert_eq!(ef_search, None);
            assert_eq!(oversample, None);
        }
        _ => panic!("expected Search variant"),
    }
}

/// End-to-end query with both clauses returns valid results (deterministic
/// recall-miss fixtures are out of scope per the task spec).
#[test]
fn ef_oversample_end_to_end_returns_results() {
    let p = fresh("p0_ef_oversample_e2e");
    let s = open_dim(&p, 2);
    add(&s, "a", Some(vec![1.0, 0.0]), None);
    add(&s, "b", Some(vec![0.0, 1.0]), None);
    s.flush_index();

    let res = s
        .execute_hql("SEARCH q SIMILAR TO [1.0,0.0] K 2 EF 128 OVERSAMPLE 2")
        .unwrap();
    let hits = ids(&res);
    assert_eq!(hits[0], "a");
}

// =========================================================================
// P0-T4 — TRAVERSE direction + multi-rel
// =========================================================================

/// `DIRECTION in` returns the reverse-edge neighbor the default (`out`) misses.
#[test]
fn traverse_direction_in_returns_reverse_neighbor() {
    let p = fresh("p0_traverse_dir_in");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    edge(&s, "B", "A", "KNOWS"); // B -> A

    let default_res = s.execute_hql("TRAVERSE FROM A DEPTH 1 REL KNOWS").unwrap();
    let default_ids: Vec<String> = default_res
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !default_ids.contains(&"B".to_string()),
        "default (out) direction must NOT see the reverse edge, got {:?}",
        default_ids
    );

    let in_res = s
        .execute_hql("TRAVERSE FROM A DEPTH 1 REL KNOWS DIRECTION in")
        .unwrap();
    let in_ids: Vec<String> = in_res
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        in_ids.contains(&"B".to_string()),
        "DIRECTION in must see the reverse edge, got {:?}",
        in_ids
    );
}

/// `DIRECTION both` returns the union of in+out neighbors.
#[test]
fn traverse_direction_both_returns_union() {
    let p = fresh("p0_traverse_dir_both");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "A", "B", "KNOWS"); // out
    edge(&s, "C", "A", "KNOWS"); // in

    let res = s
        .execute_hql("TRAVERSE FROM A DEPTH 1 REL KNOWS DIRECTION both")
        .unwrap();
    let hit_ids: Vec<String> = res
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        hit_ids.contains(&"B".to_string()),
        "must include out neighbor B"
    );
    assert!(
        hit_ids.contains(&"C".to_string()),
        "must include in neighbor C"
    );
}

/// `REL a|b` returns the union of both rel types.
#[test]
fn traverse_rel_alternation_returns_union() {
    let p = fresh("p0_traverse_rel_alt");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    node(&s, "D");
    edge(&s, "A", "B", "KNOWS");
    edge(&s, "A", "C", "LIKES");
    edge(&s, "A", "D", "OTHER");

    let res = s
        .execute_hql("TRAVERSE FROM A DEPTH 1 REL KNOWS|LIKES")
        .unwrap();
    let hit_ids: Vec<String> = res
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        hit_ids.contains(&"B".to_string()),
        "must include KNOWS target B"
    );
    assert!(
        hit_ids.contains(&"C".to_string()),
        "must include LIKES target C"
    );
    assert!(
        !hit_ids.contains(&"D".to_string()),
        "must NOT include OTHER-rel target D"
    );
}

/// `REL ANY` maps to wildcard traversal, while a concrete rel stays filtered.
#[test]
fn traverse_rel_any_restores_wildcard() {
    let p = fresh("p0_traverse_rel_any");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    node(&s, "D");
    edge(&s, "A", "B", "LINK");
    edge(&s, "A", "C", "REF");
    edge(&s, "A", "D", "OTHER");

    let any_res = s.execute_hql("TRAVERSE FROM A DEPTH 1 REL ANY").unwrap();
    let any_ids: Vec<String> = any_res
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        any_ids.contains(&"B".to_string()),
        "REL ANY must include LINK target B, got {:?}",
        any_ids
    );
    assert!(
        any_ids.contains(&"C".to_string()),
        "REL ANY must include REF target C, got {:?}",
        any_ids
    );
    assert!(
        any_ids.contains(&"D".to_string()),
        "REL ANY must include OTHER target D, got {:?}",
        any_ids
    );

    let link_res = s.execute_hql("TRAVERSE FROM A DEPTH 1 REL LINK").unwrap();
    let link_ids: Vec<String> = link_res
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(link_ids, vec!["B".to_string()]);
}

/// Omitted DIRECTION ⇒ "out" (back-compat unedited-oracle check inline here).
#[test]
fn traverse_direction_omitted_defaults_to_out() {
    let p = fresh("p0_traverse_dir_default");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    edge(&s, "A", "B", "KNOWS");

    let res = s.execute_hql("TRAVERSE FROM A DEPTH 1 REL KNOWS").unwrap();
    let hit_ids: Vec<String> = res
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(hit_ids, vec!["B".to_string()]);
}

// =========================================================================
// P0-T5 — strict numeric parse errors
// =========================================================================

#[test]
fn k_overflow_errors_not_silent_default() {
    let r = HqlCommand::try_from("SEARCH x SIMILAR TO [1.0] K 99999999999999999999");
    assert!(r.is_err(), "K overflow must error (was silent default 5)");
    let msg = r.unwrap_err();
    assert!(
        msg.contains("K value out of range"),
        "unexpected message: {msg}"
    );
}

#[test]
fn depth_overflow_errors() {
    let r = HqlCommand::try_from("TRAVERSE FROM x DEPTH 99999999999999999999 REL KNOWS");
    assert!(r.is_err(), "DEPTH overflow must error");
    let msg = r.unwrap_err();
    assert!(
        msg.contains("DEPTH value out of range"),
        "unexpected message: {msg}"
    );
}

#[test]
fn alpha_overflow_errors() {
    let huge = "1".repeat(400);
    let q = format!("MATCH x SIMILAR TO [1.0] ALPHA {huge}");
    let err = HqlCommand::try_from(q.as_str()).unwrap_err();
    assert!(err.contains("ALPHA value out of range"), "{err}");
}

#[test]
fn budget_overflow_errors() {
    let r = HqlCommand::try_from("CONTEXT FOR x TIER H1 BUDGET 99999999999999999999");
    assert!(r.is_err(), "BUDGET overflow must error");
    let msg = r.unwrap_err();
    assert!(
        msg.contains("BUDGET value out of range"),
        "unexpected message: {msg}"
    );
}

#[test]
fn vector_component_overflow_errors() {
    let huge = "1".repeat(400);
    let q = format!("SEARCH x SIMILAR TO [{huge}] K 5");
    let err = HqlCommand::try_from(q.as_str()).unwrap_err();
    assert!(err.contains("vector component value out of range"), "{err}");
}

#[test]
fn filter_value_overflow_errors() {
    let huge = "1".repeat(400);
    let q = format!("TRAVERSE FROM x DEPTH 1 REL KNOWS WHERE prop.n > {huge}");
    let err = HqlCommand::try_from(q.as_str()).unwrap_err();
    assert!(err.contains("filter number value out of range"), "{err}");
}

/// `LIMIT` still saturates on overflow (Ok), never errors — the one
/// deliberate exception in the strict-number policy.
#[test]
fn limit_overflow_still_saturates_ok() {
    let r = HqlCommand::try_from("TRAVERSE FROM x DEPTH 1 REL KNOWS LIMIT 99999999999999999999");
    assert!(
        r.is_ok(),
        "LIMIT overflow must still saturate (Ok), not error"
    );
    if let Ok(HqlCommand::Traverse { clauses, .. }) = r {
        assert_eq!(clauses.limit, Some(usize::MAX));
    } else {
        panic!("expected Traverse variant");
    }
}

// =========================================================================
// Colon-id (qualified_id)
// =========================================================================

/// `TRAVERSE FROM user:5 DEPTH 1 REL X` parses and resolves a node literally
/// named `user:5` (unquoted colon-id in the seed position).
#[test]
fn colon_id_traverse_seed_resolves_literal_node() {
    let p = fresh("p0_colon_id_traverse");
    let s = open(&p);
    node(&s, "user:5");
    node(&s, "peer");
    edge(&s, "user:5", "peer", "X");

    let res = s.execute_hql("TRAVERSE FROM user:5 DEPTH 1 REL X").unwrap();
    let hit_ids: Vec<String> = res
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(hit_ids, vec!["peer".to_string()]);
}

/// `SEARCH user:5 K 3` (no vector) works — colon-id in the target position of
/// the search-by-node form.
#[test]
fn colon_id_search_by_node_target() {
    let p = fresh("p0_colon_id_search");
    let s = open_dim(&p, 2);
    add(&s, "user:5", Some(vec![0.0, 0.0]), None);
    add(&s, "near", Some(vec![0.1, 0.0]), None);
    add(&s, "far", Some(vec![10.0, 0.0]), None);
    s.flush_index();

    let res = s.execute_hql("SEARCH user:5 K 3").unwrap();
    let hits = ids(&res);
    assert_eq!(hits[0], "user:5", "top-1 must be the node itself");
    assert_eq!(hits[1], "near");
}

/// A `MATCH (a:Label)` pattern query still parses identically — colon-id does
/// not bleed into the pattern grammar.
#[test]
fn colon_id_does_not_affect_pattern_grammar() {
    let p = fresh("p0_colon_id_pattern");
    let s = open(&p);
    s.add_node(NodeInput {
        id: Some("u1".to_string()),
        labels: vec!["User".to_string()],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    let res = s.execute_hql("MATCH (u:User) RETURN u.id").unwrap();
    let arr = res.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["u.id"].as_str().unwrap(), "u1");
}
