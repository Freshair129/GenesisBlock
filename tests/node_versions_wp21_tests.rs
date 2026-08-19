// WP-2.1 node_versions (ADR--GENESISDB-JOURNAL-HISTORY D2/D4, GNSE plan
// Phase 2): a per-entity tx-time version chain in the SQLite projection,
// keyed by the LOCAL frame seq.
//
// - Every Node frame this replica commits appends a chain row (NOT
//   clock-LWW-gated — the chain records what was committed, in frame order).
// - Retractions append a marker row, so resolve-at-commit past the
//   retraction answers "retracted", not the last live version.
// - D4 rule 1: rows below history_horizon() are never served (a projection
//   rebuild would not recover them). D4 rule 2: at_seq below the horizon
//   fails `beyond_horizon` explicitly — never silently the current state.
// - Lookup is by id string, so a retracted node's chain stays addressable
//   after its interning entry is gone.

use genesis_block_native::{NodeInput, OpenOptions, Storage};
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

fn add_node(s: &Storage, id: &str, v: i64) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec!["THING".to_string()],
        props: Some(json!({ "v": v })),
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: Some("en".to_string()),
        valid_from: Some("2024-01-01T00:00:00Z".to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

fn versions(s: &Storage, id: &str) -> Vec<serde_json::Value> {
    s.node_versions(id, None).unwrap()["versions"]
        .as_array()
        .unwrap()
        .clone()
}

/// add + 2× supersede = 5 frames (each supersede closes the old version and
/// writes the new one). The chain is frame-ordered and complete.
#[test]
fn supersede_builds_a_version_chain() {
    let path = fresh("wp21_chain");
    let s = open_with(&path, Some("full"));
    add_node(&s, "doc", 1);
    s.supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
        .unwrap();
    s.supersede_node("doc".to_string(), Some(json!({ "v": 3 })), None)
        .unwrap();

    let vs = versions(&s, "doc");
    assert_eq!(
        vs.len(),
        5,
        "add(1) + supersede(close+new)×2 = 5 chain rows, got {vs:?}"
    );
    assert_eq!(vs[0]["props"]["v"], 1, "first version is the original");
    assert_eq!(vs[4]["props"]["v"], 3, "last version is the current one");
    assert!(vs.iter().all(|v| v["retracted"] == false));
    let seqs: Vec<u64> = vs
        .iter()
        .map(|v| v["frame_seq"].as_u64().unwrap())
        .collect();
    let mut sorted = seqs.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(seqs, sorted, "chain must be strictly frame-ordered");
}

/// resolve-at-commit answers "what did THIS replica believe at ITS commit N"
/// (D2: replica-local tx-time).
#[test]
fn resolve_at_commit_returns_historical_version() {
    let path = fresh("wp21_resolve");
    let s = open_with(&path, Some("full"));
    add_node(&s, "doc", 1);
    let seq_v1 = s.stable_frontier();
    s.supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
        .unwrap();
    let seq_v2 = s.stable_frontier();

    let at_v1 = s.node_versions("doc", Some(seq_v1)).unwrap();
    assert_eq!(at_v1["resolved"]["props"]["v"], 1);
    let at_v2 = s.node_versions("doc", Some(seq_v2)).unwrap();
    assert_eq!(at_v2["resolved"]["props"]["v"], 2);
}

/// The retraction is part of the chain: resolving past it answers
/// "retracted", and the chain stays addressable by id string even though the
/// interning entry is gone.
#[test]
fn retraction_appears_in_chain_and_resolves_as_retracted() {
    let path = fresh("wp21_retract");
    let s = open_with(&path, Some("full"));
    add_node(&s, "doc", 1);
    let seq_live = s.stable_frontier();
    s.retract_node("doc").unwrap();
    let seq_after = s.stable_frontier();

    let vs = versions(&s, "doc");
    assert_eq!(vs.len(), 2);
    assert_eq!(
        vs[1]["retracted"], true,
        "retraction marker missing: {vs:?}"
    );

    let live = s.node_versions("doc", Some(seq_live)).unwrap();
    assert_eq!(live["resolved"]["props"]["v"], 1);
    let after = s.node_versions("doc", Some(seq_after)).unwrap();
    assert_eq!(
        after["resolved"]["retracted"], true,
        "resolve past the retraction must answer retracted, not the last live version"
    );
}

/// The chain is strictly rebuildable from the journal (D4 rule 1's
/// precondition): delete projection.sqlite, reopen under full retention, and
/// the chain is identical.
#[test]
fn chain_rebuilds_from_journal() {
    let path = fresh("wp21_rebuild");
    let before = {
        let s = open_with(&path, Some("full"));
        add_node(&s, "doc", 1);
        s.supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
            .unwrap();
        s.retract_node("doc").unwrap();
        versions(&s, "doc")
    };
    fs::remove_file(Path::new(&path).join("projection.sqlite")).unwrap();
    let s = open_with(&path, Some("full"));
    let after = versions(&s, "doc");
    assert_eq!(
        before, after,
        "projection rebuild must reproduce the identical version chain"
    );
}

/// D4 under frontier_only: after a fold, only the base-frame version remains
/// servable, and resolve below the horizon fails explicitly.
#[test]
fn beyond_horizon_fails_explicitly_after_fold() {
    let path = fresh("wp21_horizon");
    let s = open_with(&path, None); // frontier_only default
    add_node(&s, "doc", 1);
    s.supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
        .unwrap();
    s.save_state().unwrap(); // fold: history destroyed, horizon advances
    let horizon = s.history_horizon();
    assert!(horizon > 0);

    let err = s
        .node_versions("doc", Some(horizon - 1))
        .expect_err("resolve below the horizon must fail, not silently answer");
    assert!(
        format!("{err}").contains("beyond_horizon"),
        "error must name beyond_horizon, got: {err}"
    );

    // Listing filters to >= horizon: pre-fold chain rows may still be
    // resident, but a rebuild would not recover them — they must not serve.
    let vs = versions(&s, "doc");
    assert!(
        vs.iter()
            .all(|v| v["frame_seq"].as_u64().unwrap() >= horizon),
        "versions below the horizon served: {vs:?}"
    );
    assert!(
        !vs.is_empty(),
        "the folded base frame itself is at the horizon and must serve"
    );
}
