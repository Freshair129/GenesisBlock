// E1 (SPEC--GENESISDB-EPOCH-HNSW §3.2): the retired-adjacency overlay.
// The headline behavior — tx_as_of BEFORE a retraction resurrects the node —
// is pinned by the (formerly RED, now green) WP-3.1 matrix test
// `matrix_retraction_belief_before_still_serves`. This file covers the
// overlay's lifecycle: multi-hop through a retracted intermediate, snapshot
// reopen, journal-replay rebuild, and the fold-clears-with-horizon rule.

use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use serde_json::json;
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open_with(path: &str, retention: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some(retention.to_string()),
    })
    .unwrap()
}

fn add_node(s: &Storage, id: &str, v: i64) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec!["THING".to_string()],
        props: Some(json!({ "v": v })),
        embedding: None,
        lang: Some("en".to_string()),
        valid_from: Some("2020-01-01T00:00:00Z".to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

fn link(s: &Storage, id: &str, from: &str, to: &str) {
    s.add_edge(EdgeInput {
        id: Some(id.to_string()),
        from: from.to_string(),
        to: to.to_string(),
        rel: "KNOWS".to_string(),
        props: None,
        valid_from: Some("2020-01-01T00:00:00Z".to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
}

fn traverse_tx(s: &Storage, seed: &str, depth: u32, t: u64) -> Vec<serde_json::Value> {
    s.execute_query_ir_json(json!({
        "contract_version": "query-ir.v1",
        "request_id": "e1",
        "operation": {
            "kind": "traverse",
            "seed_id": seed,
            "depth": depth,
            "relations": ["KNOWS"],
            "direction": "out"
        },
        "temporal": { "tx_as_of": t }
    }))
    .unwrap()["data"]
        .as_array()
        .unwrap()
        .clone()
}

/// A retracted INTERMEDIATE node must not sever the historical path: the BFS
/// resurrects it AND keeps walking its (retired) out-edges to the live leaf.
#[test]
fn tx_resurrect_multihop_through_retracted_intermediate() {
    let path = fresh("e1_multihop");
    let s = open_with(&path, "full");
    add_node(&s, "hub", 0);
    add_node(&s, "mid", 1);
    add_node(&s, "leaf", 2);
    link(&s, "e1", "hub", "mid");
    link(&s, "e2", "mid", "leaf");
    let before = s.stable_frontier();
    s.retract_node("mid").unwrap();

    // Current view: the whole path through mid is gone.
    let now_rows = s
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "e1-now",
            "operation": {
                "kind": "traverse", "seed_id": "hub", "depth": 2,
                "relations": ["KNOWS"], "direction": "out"
            }
        }))
        .unwrap()["data"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(now_rows, 0, "current view must hide the retracted path");

    // Belief before the retraction: both hops answer.
    let rows = traverse_tx(&s, "hub", 2, before);
    assert_eq!(rows.len(), 2, "mid AND leaf must resolve: {rows:?}");
    let ids: Vec<&str> = rows
        .iter()
        .map(|r| r["node"]["id"].as_str().unwrap())
        .collect();
    assert!(
        ids.contains(&"mid"),
        "retracted intermediate served: {ids:?}"
    );
    assert!(
        ids.contains(&"leaf"),
        "live leaf reached THROUGH it: {ids:?}"
    );
    let mid_row = rows.iter().find(|r| r["node"]["id"] == "mid").unwrap();
    assert_eq!(mid_row["node"]["props"]["v"], 1, "chain-resolved fields");
}

/// The overlay must survive a snapshot instant-load (edges_retired.bin):
/// the load path replays only frames past the frontier, so without the file
/// the resurrection would silently die at the first checkpoint.
#[test]
fn tx_resurrect_survives_snapshot_reopen() {
    let path = fresh("e1_reopen_snapshot");
    let before = {
        let s = open_with(&path, "full");
        add_node(&s, "hub", 0);
        add_node(&s, "doc", 1);
        link(&s, "e1", "hub", "doc");
        let before = s.stable_frontier();
        s.retract_node("doc").unwrap();
        s.save_state().unwrap(); // full profile: checkpoint without fold
        before
    };

    let s = open_with(&path, "full");
    assert_eq!(
        s.edges_retired.len(),
        1,
        "overlay reloaded from edges_retired.bin"
    );
    let rows = traverse_tx(&s, "hub", 1, before);
    assert_eq!(
        rows.len(),
        1,
        "belief before retraction after reopen: {rows:?}"
    );
    assert_eq!(rows[0]["node"]["id"], "doc");
    assert_eq!(rows[0]["node"]["props"]["v"], 1);
}

/// No snapshot at all: a reopen goes through full journal replay, and the
/// NodeRetract frame's own seq must rebuild the overlay identically.
#[test]
fn tx_resurrect_survives_journal_replay() {
    let path = fresh("e1_reopen_replay");
    let before = {
        let s = open_with(&path, "full");
        add_node(&s, "hub", 0);
        add_node(&s, "doc", 1);
        link(&s, "e1", "hub", "doc");
        let before = s.stable_frontier();
        s.retract_node("doc").unwrap();
        before
        // dropped WITHOUT save_state → next open replays the journal
    };

    let s = open_with(&path, "full");
    let rows = traverse_tx(&s, "hub", 1, before);
    assert_eq!(
        rows.len(),
        1,
        "journal replay must rebuild the overlay: {rows:?}"
    );
    assert_eq!(rows[0]["node"]["props"]["v"], 1);
}

/// The fold is the single destruction boundary (ADR--JOURNAL-HISTORY I6):
/// under frontier_only every checkpoint folds, the horizon advances to the
/// frontier, the overlay is cleared with the history it belonged to — and
/// the tx question itself now fails loudly, never silently empty.
#[test]
fn fold_clears_overlay_and_horizon_guards() {
    let path = fresh("e1_fold_clears");
    let s = open_with(&path, "frontier_only");
    add_node(&s, "hub", 0);
    add_node(&s, "doc", 1);
    link(&s, "e1", "hub", "doc");
    let before = s.stable_frontier();
    s.retract_node("doc").unwrap();
    assert_eq!(s.edges_retired.len(), 1, "overlay populated pre-fold");

    s.save_state().unwrap(); // frontier_only: this checkpoint folds

    assert_eq!(s.edges_retired.len(), 0, "fold swept the overlay");
    assert_eq!(s.out_idx_retired.len(), 0);
    assert_eq!(s.in_idx_retired.len(), 0);
    let err = s
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "e1-horizon",
            "operation": {
                "kind": "traverse", "seed_id": "hub", "depth": 1,
                "relations": ["KNOWS"], "direction": "out"
            },
            "temporal": { "tx_as_of": before }
        }))
        .unwrap_err();
    assert!(
        err.to_string().contains("beyond_horizon"),
        "pre-fold tx question fails loudly, not empty: {err}"
    );
}

/// A node re-created after retraction attaches to its own id history: the
/// old retired edges stay historical (tx-only), the current view sees only
/// the new wiring.
#[test]
fn recreate_after_retract_keeps_views_separated() {
    let path = fresh("e1_recreate");
    let s = open_with(&path, "full");
    add_node(&s, "hub", 0);
    add_node(&s, "doc", 1);
    link(&s, "e1", "hub", "doc");
    let before = s.stable_frontier();
    s.retract_node("doc").unwrap();
    add_node(&s, "doc", 99); // legitimate re-creation, clears the tombstone
    link(&s, "e2", "hub", "doc");

    // Current view: exactly the new edge, new props.
    let now_rows = s
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "e1-recreate-now",
            "operation": {
                "kind": "traverse", "seed_id": "hub", "depth": 1,
                "relations": ["KNOWS"], "direction": "out"
            }
        }))
        .unwrap()["data"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(now_rows.len(), 1);
    assert_eq!(now_rows[0]["node"]["props"]["v"], 99);

    // Belief before the retraction: the OLD version, via the retired edge.
    let rows = traverse_tx(&s, "hub", 1, before);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        rows[0]["node"]["props"]["v"], 1,
        "old belief, not the re-creation"
    );
}
