// WP-3.1 (GNSE plan Phase 3 "Prove or kill", interview ROUND2 G3-e bar):
// the bitemporal correctness suite. This is the matrix the DIY SQLite
// assembly must also pass in the moat bench (WP-3.2) — retraction, two-axis
// AS OF, interval overlap, correction-after-the-fact, audit reconstruction —
// written against the WP-2.2 Query IR contract (`temporal.valid_at` +
// `temporal.tx_as_of`) and the WP-2.1 `node_versions` chain.
//
// Axis semantics under test (the engine's documented rules):
// - valid time: `valid_from <= as_of < valid_to` — start inclusive, end
//   exclusive (`is_valid_as_of`).
// - tx time: replica-local commit seq; results re-resolved through the
//   version chain at N ("implemented_post_resolution", WP-2.2). Nodes
//   absent or retracted at N drop.
// - both axes together: valid-time selects the version window, tx-time
//   selects the belief — "what did we believe at commit N about time V".

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

fn open_full(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some("full".to_string()),
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

fn link_at(s: &Storage, id: &str, from: &str, to: &str, valid_from: &str) {
    s.add_edge(EdgeInput {
        id: Some(id.to_string()),
        from: from.to_string(),
        to: to.to_string(),
        rel: "KNOWS".to_string(),
        props: None,
        valid_from: Some(valid_from.to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
}

/// TRAVERSE hub -KNOWS-> * through the Query IR, with optional selectors on
/// both temporal axes. Returns the `data` rows.
fn traverse(s: &Storage, valid_at: Option<&str>, tx_as_of: Option<u64>) -> serde_json::Value {
    let mut req = json!({
        "contract_version": "query-ir.v1",
        "request_id": "wp31",
        "operation": {
            "kind": "traverse",
            "seed_id": "hub",
            "depth": 1,
            "relations": ["KNOWS"],
            "direction": "out"
        }
    });
    if valid_at.is_some() || tx_as_of.is_some() {
        req["temporal"] = json!({ "valid_at": valid_at, "tx_as_of": tx_as_of });
    }
    s.execute_query_ir_json(req).unwrap()["data"].clone()
}

fn rows(v: &serde_json::Value) -> &Vec<serde_json::Value> {
    v.as_array().unwrap()
}

/// The four quadrants of the valid×tx matrix on one superseded node.
///
/// Timeline: doc v1 (valid from 2020) committed at S1, then superseded to v2
/// (v1's window closes at ~now, v2 valid from ~now).
///
/// | valid_at   | tx_as_of | expectation                                    |
/// |------------|----------|------------------------------------------------|
/// | now (none) | now      | v2, open window                                |
/// | 2022       | now      | v1, CLOSED window (we now know it ended)       |
/// | now (none) | S1       | v1, open window (belief at S1)                 |
/// | 2022       | S1       | v1, OPEN window (at S1 we didn't know it ends) |
#[test]
fn matrix_valid_by_tx_four_quadrants() {
    let path = fresh("wp31_quadrants");
    let s = open_full(&path);
    add_node(&s, "hub", 0);
    add_node(&s, "doc", 1);
    link_at(&s, "e1", "hub", "doc", "2020-01-01T00:00:00Z");
    let s1 = s.stable_frontier();
    s.supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
        .unwrap();

    // (now, now): the current version.
    let q1 = traverse(&s, None, None);
    assert_eq!(rows(&q1).len(), 1);
    assert_eq!(q1[0]["node"]["props"]["v"], 2);
    assert!(
        q1[0]["node"]["valid_to"].is_null(),
        "current version is open"
    );

    // (2022, now): the historically valid version, with the closed window
    // we know about TODAY (WP-2.2 as_of fix).
    let q2 = traverse(&s, Some("2022-01-01T00:00:00Z"), None);
    assert_eq!(rows(&q2).len(), 1, "v1 must resolve, not vanish: {q2}");
    assert_eq!(q2[0]["node"]["props"]["v"], 1);
    assert!(
        q2[0]["node"]["valid_to"].is_string(),
        "today we know v1's window closed: {q2}"
    );

    // (now, S1): what this replica believed at commit S1.
    let q3 = traverse(&s, None, Some(s1));
    assert_eq!(rows(&q3).len(), 1);
    assert_eq!(q3[0]["node"]["props"]["v"], 1, "belief at S1 is v1: {q3}");
    assert!(
        q3[0]["node"]["valid_to"].is_null(),
        "at S1 v1 had not been closed yet: {q3}"
    );

    // (2022, S1): both axes — the belief at S1 about valid-time 2022.
    let q4 = traverse(&s, Some("2022-01-01T00:00:00Z"), Some(s1));
    assert_eq!(rows(&q4).len(), 1, "two-axis query must answer: {q4}");
    assert_eq!(q4[0]["node"]["props"]["v"], 1);
    assert!(
        q4[0]["node"]["valid_to"].is_null(),
        "at S1 the recorded belief had an open window: {q4}"
    );
}

/// Retraction across tx time, the implemented half: a retracted node is
/// gone from the current view and from any tx_as_of at-or-after the
/// retraction. (The belief-before half is the ignored RED test below.)
#[test]
fn matrix_retraction_current_and_after_tx() {
    let path = fresh("wp31_retract_tx");
    let s = open_full(&path);
    add_node(&s, "hub", 0);
    add_node(&s, "doc", 1);
    link_at(&s, "e1", "hub", "doc", "2020-01-01T00:00:00Z");
    s.retract_node("doc").unwrap();
    let after = s.stable_frontier();

    assert_eq!(
        rows(&traverse(&s, None, None)).len(),
        0,
        "current view hides the retracted node"
    );
    assert_eq!(
        rows(&traverse(&s, None, Some(after))).len(),
        0,
        "belief at-or-after the retraction drops it"
    );
}

/// TDD RED (phase-scale, per plan §Phase 3): `tx_as_of` BEFORE a retraction
/// should still serve the node — "what did we believe at commit N" must not
/// depend on what happened after N. Today it cannot: the disclosed
/// "implemented_post_resolution" semantics (WP-2.2, capabilities
/// `tx_as_of`) enumerates candidates from CURRENT indexes, and a retracted
/// node is absent from them, so the chain never gets a chance to resurrect
/// it. Un-ignore when epoch-segmented indexes land (GNSE backlog, WP-3.3
/// decision gate). Deliberately NOT rewritten to assert the current
/// behavior — that would codify the gap (storage-readiness audit rule).
#[test]
#[ignore = "known WP-2.2 disclosed gap: tx_as_of cannot resurrect retracted nodes until epoch-segmented indexes (WP-3.3 gate)"]
fn matrix_retraction_belief_before_still_serves() {
    let path = fresh("wp31_retract_tx_red");
    let s = open_full(&path);
    add_node(&s, "hub", 0);
    add_node(&s, "doc", 1);
    link_at(&s, "e1", "hub", "doc", "2020-01-01T00:00:00Z");
    let before = s.stable_frontier();
    s.retract_node("doc").unwrap();

    let believed = traverse(&s, None, Some(before));
    assert_eq!(
        rows(&believed).len(),
        1,
        "belief before the retraction must still serve the node: {believed}"
    );
    assert_eq!(believed[0]["node"]["props"]["v"], 1);
}

/// Correction-after-the-fact on the valid axis: a retroactive `retract_edge`
/// changes the answer to the SAME valid-time question across tx time —
/// exactly the two-axis behavior single-axis audit-history patterns get
/// wrong (interview ROUND2).
#[test]
fn correction_after_the_fact_retroactive_edge_retract() {
    let path = fresh("wp31_correction");
    let s = open_full(&path);
    add_node(&s, "hub", 0);
    add_node(&s, "doc", 1);
    link_at(&s, "e1", "hub", "doc", "2020-01-01T00:00:00Z");

    // Before the correction: "was hub linked to doc during 2022?" — yes.
    assert_eq!(
        rows(&traverse(&s, Some("2022-01-01T00:00:00Z"), None)).len(),
        1
    );

    // The correction: we later learn the link actually ended mid-2021.
    s.retract_edge("e1".to_string(), Some("2021-06-01T00:00:00Z".to_string()))
        .unwrap();

    // Same valid-time question, new tx-time answer: no.
    assert_eq!(
        rows(&traverse(&s, Some("2022-01-01T00:00:00Z"), None)).len(),
        0,
        "after the correction, 2022 no longer overlaps the link's window"
    );
    // And the window BEFORE the corrected end still answers yes.
    assert_eq!(
        rows(&traverse(&s, Some("2021-01-01T00:00:00Z"), None)).len(),
        1,
        "the link remains true for valid times before the corrected end"
    );
    // Current view hides it too (valid_to is in the past).
    assert_eq!(rows(&traverse(&s, None, None)).len(), 0);
}

/// Interval-overlap boundary semantics: `valid_from <= as_of < valid_to` —
/// start inclusive, end exclusive.
#[test]
fn interval_boundaries_inclusive_start_exclusive_end() {
    let path = fresh("wp31_boundaries");
    let s = open_full(&path);
    add_node(&s, "hub", 0);
    add_node(&s, "doc", 1);
    link_at(&s, "e1", "hub", "doc", "2021-01-01T00:00:00Z");
    s.retract_edge("e1".to_string(), Some("2023-01-01T00:00:00Z".to_string()))
        .unwrap();

    let visible_at = |as_of: &str| rows(&traverse(&s, Some(as_of), None)).len();
    assert_eq!(visible_at("2020-12-31T23:59:59Z"), 0, "before valid_from");
    assert_eq!(visible_at("2021-01-01T00:00:00Z"), 1, "start is inclusive");
    assert_eq!(visible_at("2022-06-15T00:00:00Z"), 1, "inside the window");
    assert_eq!(visible_at("2023-01-01T00:00:00Z"), 0, "end is exclusive");
}

/// Audit reconstruction: the full life of an entity — create, two
/// supersessions, retraction — is reconstructable from the `node_versions`
/// chain alone, and the WP-2.3 `caused_by` auto-chain links each version to
/// the exact frame that closed its predecessor.
#[test]
fn audit_reconstruction_full_chain() {
    let path = fresh("wp31_audit");
    let s = open_full(&path);
    add_node(&s, "doc", 1);
    let v2 = s
        .supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
        .unwrap();
    let v3 = s
        .supersede_node("doc".to_string(), Some(json!({ "v": 3 })), None)
        .unwrap();
    s.retract_node("doc").unwrap();

    let chain = s.node_versions("doc", None).unwrap();
    let versions = chain["versions"].as_array().unwrap();
    // create + (close v1, open v2) + (close v2, open v3) + retract marker.
    assert_eq!(versions.len(), 6, "full audit trail: {chain}");
    let seqs: Vec<i64> = versions
        .iter()
        .map(|r| r["frame_seq"].as_i64().unwrap())
        .collect();
    let mut sorted = seqs.clone();
    sorted.sort_unstable();
    assert_eq!(seqs, sorted, "chain is ordered by frame_seq");
    assert_eq!(
        versions.last().unwrap()["retracted"],
        true,
        "the retraction marker closes the trail"
    );

    // Walk the provenance chain backwards: v3 -> the frame that closed v2 ->
    // v2 -> the frame that closed v1 -> v1. Purely from stored identities.
    let seq_of = |caused_by: &Option<String>| -> u64 {
        caused_by
            .as_deref()
            .and_then(|c| c.split_once('@'))
            .and_then(|(_, s)| s.parse().ok())
            .expect("auto-chain identity must parse")
    };
    let closed_v2 = s.node_versions("doc", Some(seq_of(&v3.caused_by))).unwrap();
    assert_eq!(
        closed_v2["resolved"]["props"]["v"], 2,
        "v3's provenance resolves the closed v2: {closed_v2}"
    );
    let closed_v1 = s.node_versions("doc", Some(seq_of(&v2.caused_by))).unwrap();
    assert_eq!(
        closed_v1["resolved"]["props"]["v"], 1,
        "v2's provenance resolves the closed v1: {closed_v1}"
    );
}

/// The audit chain and both temporal axes survive a process restart: reopen
/// the database and re-run a two-axis query plus the chain walk.
#[test]
fn audit_chain_survives_reopen() {
    let path = fresh("wp31_reopen");
    let (s1_seq, versions_before) = {
        let s = open_full(&path);
        add_node(&s, "hub", 0);
        add_node(&s, "doc", 1);
        link_at(&s, "e1", "hub", "doc", "2020-01-01T00:00:00Z");
        let s1 = s.stable_frontier();
        s.supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
            .unwrap();
        s.save_state().unwrap();
        let n = s.node_versions("doc", None).unwrap()["versions"]
            .as_array()
            .unwrap()
            .len();
        (s1, n)
    };

    let s = open_full(&path);
    let versions_after = s.node_versions("doc", None).unwrap()["versions"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(
        versions_after, versions_before,
        "the version chain must survive reopen"
    );

    let believed = traverse(&s, None, Some(s1_seq));
    assert_eq!(
        rows(&believed).len(),
        1,
        "tx_as_of before the supersession still answers after reopen: {believed}"
    );
    assert_eq!(believed[0]["node"]["props"]["v"], 1);

    let historical = traverse(&s, Some("2022-01-01T00:00:00Z"), None);
    assert_eq!(rows(&historical).len(), 1);
    assert_eq!(
        historical[0]["node"]["props"]["v"], 1,
        "valid-time resolution still answers after reopen: {historical}"
    );
}
