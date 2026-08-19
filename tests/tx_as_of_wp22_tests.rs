// WP-2.2 (ADR D2/D4, GNSE Phase 2): `temporal.tx_as_of` on the Typed Query
// IR + the as_of valid-time semantics fix.
//
// tx_as_of = replica-local commit-seq selector ("what did THIS replica
// believe at ITS commit N"). Interim semantics, disclosed by capabilities as
// "implemented_post_resolution": candidates come from CURRENT indexes, then
// each result node is re-resolved through the WP-2.1 version chain at N —
// nodes with no committed version at-or-below N (or retracted at N) drop.
// Selectors below history_horizon() fail `beyond_horizon` (D4 rule 2).
//
// The as_of fix (hybrid_search + neighbors): a node whose current version
// postdates the valid-time selector now resolves its historically valid
// version from the chain instead of being silently hidden. The old hiding
// behavior was codified by temporal_queries_tests (updated in this change).

use genesis_block_native::{EdgeInput, HybridSearchInput, NodeInput, OpenOptions, Storage};
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

fn open_with(path: &str, retention: Option<&str>) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: retention.map(|r| r.to_string()),
    })
    .unwrap()
}

fn add_node(s: &Storage, id: &str, v: i64, embedding: Option<Vec<f64>>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec!["THING".to_string()],
        props: Some(json!({ "v": v })),
        embedding,
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

fn traverse_at(s: &Storage, seed: &str, tx_as_of: Option<u64>) -> serde_json::Value {
    let mut req = json!({
        "contract_version": "query-ir.v1",
        "request_id": "wp22",
        "operation": {
            "kind": "traverse",
            "seed_id": seed,
            "depth": 1,
            "relations": ["KNOWS"],
            "direction": "out"
        }
    });
    if let Some(t) = tx_as_of {
        req["temporal"] = json!({ "tx_as_of": t });
    }
    s.execute_query_ir_json(req).unwrap()
}

/// tx_as_of resolves each result through the version chain at that commit.
#[test]
fn tx_as_of_resolves_historical_version() {
    let path = fresh("wp22_resolve");
    let s = open_with(&path, Some("full"));
    add_node(&s, "hub", 0, None);
    add_node(&s, "doc", 1, None);
    link(&s, "e1", "hub", "doc");
    let seq_v1 = s.stable_frontier();
    s.supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
        .unwrap();

    let now = traverse_at(&s, "hub", None);
    assert_eq!(now["data"][0]["node"]["props"]["v"], 2);

    let then = traverse_at(&s, "hub", Some(seq_v1));
    assert_eq!(
        then["data"][0]["node"]["props"]["v"], 1,
        "tx_as_of at the v1 commit must serve the v1 version: {then}"
    );
}

/// A node with no committed version at-or-below the selector drops from the
/// result set, even though current topology reaches it.
#[test]
fn tx_as_of_drops_nodes_not_yet_committed() {
    let path = fresh("wp22_drop");
    let s = open_with(&path, Some("full"));
    add_node(&s, "hub", 0, None);
    let before_doc = s.stable_frontier();
    add_node(&s, "doc", 1, None);
    link(&s, "e1", "hub", "doc");

    let now = traverse_at(&s, "hub", None);
    assert_eq!(now["data"].as_array().unwrap().len(), 1);

    let then = traverse_at(&s, "hub", Some(before_doc));
    assert_eq!(
        then["data"].as_array().unwrap().len(),
        0,
        "doc was not committed at that frontier and must drop: {then}"
    );
}

/// D4 rule 2: a selector below the horizon fails explicitly.
#[test]
fn tx_as_of_below_horizon_fails_beyond_horizon() {
    let path = fresh("wp22_horizon");
    let s = open_with(&path, None); // frontier_only default
    add_node(&s, "hub", 0, None);
    add_node(&s, "doc", 1, None);
    link(&s, "e1", "hub", "doc");
    s.save_state().unwrap();
    let horizon = s.history_horizon();
    assert!(horizon > 0);

    let err = s
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "wp22-h",
            "temporal": { "tx_as_of": horizon - 1 },
            "operation": {
                "kind": "traverse",
                "seed_id": "hub",
                "depth": 1,
                "relations": ["KNOWS"],
                "direction": "out"
            }
        }))
        .expect_err("tx_as_of below the horizon must fail, not silently answer");
    assert!(
        format!("{err}").contains("beyond_horizon"),
        "error must name beyond_horizon: {err}"
    );
}

/// The as_of fix on the search path: a superseded node's historically valid
/// version is served (with its closed window), not hidden.
#[test]
fn hybrid_search_as_of_resolves_superseded_version() {
    let path = fresh("wp22_search_asof");
    let s = open_with(&path, Some("full"));
    add_node(&s, "doc", 1, Some(vec![1.0, 0.0, 0.0, 0.0]));
    s.supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
        .unwrap();
    s.flush_index();

    let hits = s
        .hybrid_search(HybridSearchInput {
            query_vector: vec![1.0, 0.0, 0.0, 0.0],
            k: 5,
            alpha: Some(0.0),
            lang: None,
            as_of: Some("2022-01-01T00:00:00Z".to_string()),
            collection: None,
            ef_search: None,
            oversample: None,
        })
        .unwrap();
    let doc = hits
        .iter()
        .find(|n| n.node.id == "doc")
        .expect("superseded node must resolve its 2022-valid version, not vanish from search");
    assert_eq!(doc.node.props["v"], 1, "must serve the v1 props");
    assert!(
        doc.node.valid_to.is_some(),
        "resolved historical version carries its closed validity window"
    );
}
