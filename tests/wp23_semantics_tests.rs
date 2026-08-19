// WP-2.3 (GNSE plan, Phase 2 semantics fixes):
//
// 1. `caused_by` auto-chain — `supersede_node` with `caused_by: None` no
//    longer leaves the new version's provenance empty: it defaults to the
//    identity of the version the supersession closed, `<id>@<frame_seq>` of
//    the closing frame, resolvable back through the WP-2.1 node_versions
//    tx-time chain. An explicit caller value always wins.
//
// 2. `recorded_at` queryable — HQL pattern clauses gain a `recorded_at`
//    accessor (`e.recorded_at` in WHERE / ORDER BY / RETURN). Edge bindings
//    carry the tx-time ingestion timestamp; node bindings resolve to null
//    (NodeOutput has no such field), mirroring the score/depth convention.

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

/// supersede with `caused_by: None` chains to the closed version's identity,
/// and the embedded frame seq resolves that exact version through the
/// WP-2.1 chain.
#[test]
fn supersede_caused_by_auto_chains_to_closed_version() {
    let path = fresh("wp23_auto_chain");
    let s = open_full(&path);
    add_node(&s, "doc", 1);

    let new_ver = s
        .supersede_node("doc".to_string(), Some(json!({ "v": 2 })), None)
        .unwrap();
    let caused_by = new_ver
        .caused_by
        .expect("caused_by must be auto-filled when the caller passes None");
    let (chained_id, seq_str) = caused_by
        .split_once('@')
        .expect("auto-chain must use the <id>@<frame_seq> form");
    assert_eq!(chained_id, "doc");
    let closed_seq: u64 = seq_str.parse().expect("frame seq must be numeric");

    // The identity resolves: at that frame the chain serves the closed v1
    // version (the frame that set its valid_to), not the v2 successor.
    let at_closed = s.node_versions("doc", Some(closed_seq)).unwrap();
    assert_eq!(
        at_closed["resolved"]["props"]["v"], 1,
        "the chained frame must resolve to the version the supersession closed: {at_closed}"
    );
    assert!(
        at_closed["resolved"]["valid_to"].is_string(),
        "the closed version carries its closing valid_to: {at_closed}"
    );

    // Chaining again links v3 -> v2's closing frame, strictly after v1's.
    let third = s
        .supersede_node("doc".to_string(), Some(json!({ "v": 3 })), None)
        .unwrap();
    let caused_by_3 = third.caused_by.unwrap();
    let seq_3: u64 = caused_by_3.split_once('@').unwrap().1.parse().unwrap();
    assert!(
        seq_3 > closed_seq,
        "each supersession chains to a later frame"
    );
    let at_v2 = s.node_versions("doc", Some(seq_3)).unwrap();
    assert_eq!(at_v2["resolved"]["props"]["v"], 2);
}

/// An explicit caller-provided `caused_by` is passed through untouched.
#[test]
fn supersede_caused_by_explicit_wins() {
    let path = fresh("wp23_explicit");
    let s = open_full(&path);
    add_node(&s, "doc", 1);

    let new_ver = s
        .supersede_node(
            "doc".to_string(),
            Some(json!({ "v": 2 })),
            Some("agent:reviewer".to_string()),
        )
        .unwrap();
    assert_eq!(
        new_ver.caused_by.as_deref(),
        Some("agent:reviewer"),
        "explicit provenance must never be overwritten by the auto-chain"
    );
}

/// RETURN e.recorded_at projects the edge's tx-time timestamp; the same
/// accessor on a node binding is null.
#[test]
fn hql_recorded_at_projects_edge_tx_time() {
    let path = fresh("wp23_return");
    let s = open_full(&path);
    add_node(&s, "hub", 0);
    add_node(&s, "doc", 1);
    link(&s, "e1", "hub", "doc");

    let res = s
        .execute_hql(
            "MATCH (a {id: \"hub\"})-[e:KNOWS]->(b) RETURN a.recorded_at, e.recorded_at, b.id",
        )
        .unwrap();
    let rows = res.as_array().unwrap();
    assert_eq!(rows.len(), 1, "one bound row expected: {res}");
    let row = &rows[0];
    assert_eq!(row["b.id"], "doc");
    let ts = row["e.recorded_at"]
        .as_str()
        .expect("edge recorded_at must project as a timestamp string");
    assert!(
        ts.starts_with("20"),
        "recorded_at must be an RFC3339 tx-time timestamp, got {ts}"
    );
    assert!(
        row["a.recorded_at"].is_null(),
        "node bindings have no recorded_at and must resolve to null: {row}"
    );
}

/// WHERE over e.recorded_at filters lexicographically (RFC3339 == chrono
/// order); a node-binding recorded_at predicate is null ⇒ false and drops
/// every row.
#[test]
fn hql_recorded_at_where_filters() {
    let path = fresh("wp23_where");
    let s = open_full(&path);
    add_node(&s, "hub", 0);
    add_node(&s, "doc", 1);
    link(&s, "e1", "hub", "doc");

    let all = s
        .execute_hql(
            "MATCH (a {id: \"hub\"})-[e:KNOWS]->(b) \
             WHERE e.recorded_at > \"2000-01-01T00:00:00Z\" RETURN b.id",
        )
        .unwrap();
    assert_eq!(
        all.as_array().unwrap().len(),
        1,
        "ingested after 2000: {all}"
    );

    let none = s
        .execute_hql(
            "MATCH (a {id: \"hub\"})-[e:KNOWS]->(b) \
             WHERE e.recorded_at > \"2999-01-01T00:00:00Z\" RETURN b.id",
        )
        .unwrap();
    assert_eq!(
        none.as_array().unwrap().len(),
        0,
        "nothing after 2999: {none}"
    );

    let node_side = s
        .execute_hql(
            "MATCH (a {id: \"hub\"})-[e:KNOWS]->(b) \
             WHERE b.recorded_at > \"2000-01-01T00:00:00Z\" RETURN b.id",
        )
        .unwrap();
    assert_eq!(
        node_side.as_array().unwrap().len(),
        0,
        "node recorded_at is null and null never satisfies a predicate: {node_side}"
    );
}
