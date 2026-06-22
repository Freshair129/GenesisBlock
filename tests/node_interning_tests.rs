// Node interning Layer A (ADR--GENESISDB-NODE-ID-INTERNING): the u32->id reverse
// map (`u32_to_id`) was dropped. A u32's id string is recovered from the
// canonical `nodes[u32].id` record. These tests pin the behaviors that depend on
// that resolution so the win can't silently regress fuzzy match, traversal
// direction, or snapshot reload.

use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput, NeighborInput};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let db_path = format!("G:/GenesisBlock_Dev/GenesisBlock/tests/{}", name);
    if Path::new(&db_path).exists() {
        fs::remove_dir_all(&db_path).unwrap();
    }
    db_path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
    })
    .unwrap()
}

fn node(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()), labels: vec![], props: None, embedding: None,
        lang: None, valid_from: None, caused_by: None, ttl: None, collection: None,
    })
    .unwrap();
}

fn edge(s: &Storage, eid: &str, from: &str, to: &str, rel: &str) {
    s.add_edge(EdgeInput {
        id: Some(eid.to_string()), from: from.to_string(), to: to.to_string(),
        rel: rel.to_string(), props: None, valid_from: None, supersede: None,
        impact: None, caused_by: None,
    })
    .unwrap();
}

fn hop1(s: &Storage, id: &str, dir: &str) -> Vec<String> {
    s.neighbors(
        id.to_string(),
        NeighborInput {
            depth: Some(1), rel: None, rels: None, direction: Some(dir.to_string()),
            as_of: None, include_invalid: Some(false), limit: Some(1000),
        },
        false,
    )
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .collect()
}

/// Exact id resolution is unchanged — it reads `id_to_u32`, which still holds the
/// forward entry for every interned id (nodes AND edge endpoints).
#[test]
fn exact_id_still_resolves() {
    let s = open(&fresh("test_node_intern_exact"));
    node(&s, "USER-alice");
    assert_eq!(s.find_fuzzy_id("USER-alice").as_deref(), Some("USER-alice"));
}

/// Fuzzy (non-exact) resolution still finds the nearest real node by id after the
/// reverse map is gone — candidates are resolved via `nodes[u32].id`.
#[test]
fn fuzzy_resolves_real_node_after_reverse_map_removal() {
    let s = open(&fresh("test_node_intern_fuzzy"));
    node(&s, "USER-alice");
    node(&s, "USER-bob");
    // One char off — must still snap to USER-alice (jaro_winkler > 0.85).
    assert_eq!(
        s.find_fuzzy_id("USER-alicE").as_deref(),
        Some("USER-alice"),
        "fuzzy id match must still resolve to the real node via nodes[u32].id"
    );
}

/// Traversal picks the far endpoint by u32 identity (no reverse map), so both
/// out- and in-direction hops must resolve correctly — including when an edge
/// endpoint is the current frontier.
#[test]
fn traversal_direction_resolves_without_reverse_map() {
    let s = open(&fresh("test_node_intern_traverse"));
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "e-ab", "A", "B", "LINK"); // A -> B
    edge(&s, "e-ca", "C", "A", "LINK"); // C -> A

    let out = hop1(&s, "A", "out");
    assert_eq!(out, vec!["B".to_string()], "A --out--> B");

    let inc = hop1(&s, "A", "in");
    assert_eq!(inc, vec!["C".to_string()], "C --in--> A");
}

/// Snapshot save + reopen must reconstruct everything the dropped reverse map
/// used to back: fuzzy resolution and traversal both survive an instant-load.
#[test]
fn reload_preserves_fuzzy_and_traversal() {
    let path = fresh("test_node_intern_reload");
    {
        let s = open(&path);
        node(&s, "USER-alice");
        node(&s, "USER-bob");
        edge(&s, "e-ab", "USER-alice", "USER-bob", "KNOWS");
        s.save_state().unwrap();
    }
    // Reopen the SAME path (no fresh()/delete) → instant-load from snapshot.
    let s = open(&path);
    assert_eq!(
        s.find_fuzzy_id("USER-alicE").as_deref(),
        Some("USER-alice"),
        "fuzzy must work after instant-load (trigram rebuilt, resolved via nodes)"
    );
    assert_eq!(
        hop1(&s, "USER-alice", "out"),
        vec!["USER-bob".to_string()],
        "traversal must work after instant-load"
    );
}
