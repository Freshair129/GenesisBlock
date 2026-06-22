// Layer B u128 edge keys (ADR--GENESISDB-EDGE-NUMERIC-KEYS): edge keys widened
// u64 -> u128 to slash birthday-collision risk (~1.7e-6 @8M -> ~9e-26). The key
// is always derived from EdgeOutput.id and never authoritative on disk, so legacy
// u64-keyed snapshots load transparently (the saved key is ignored + re-derived).

use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput, NeighborInput};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("G:/GenesisBlock_Dev/GenesisBlock/tests/{}", name);
    if Path::new(&p).exists() { fs::remove_dir_all(&p).unwrap(); }
    p
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(), page_cache_mb: Some(64), read_only: Some(false), vector_dim: None,
    }).unwrap()
}

fn node(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()), labels: vec![], props: None, embedding: None,
        lang: None, valid_from: None, caused_by: None, ttl: None, collection: None,
    }).unwrap();
}

fn edge(s: &Storage, eid: &str, from: &str, to: &str) {
    s.add_edge(EdgeInput {
        id: Some(eid.to_string()), from: from.to_string(), to: to.to_string(),
        rel: "LINK".to_string(), props: None, valid_from: None, supersede: None,
        impact: None, caused_by: None,
    }).unwrap();
}

fn hop1_out(s: &Storage, id: &str) -> usize {
    s.neighbors(id.to_string(), NeighborInput {
        depth: Some(1), rel: None, rels: None, direction: Some("out".to_string()),
        as_of: None, include_invalid: Some(false), limit: Some(1000),
    }, false).unwrap().len()
}

/// Distinct edge ids hash to distinct u128 keys, and edges stay traversable +
/// survive a snapshot save/reopen under the widened key.
#[test]
fn u128_edges_traverse_and_survive_reload() {
    let path = fresh("test_edge_u128_reload");
    {
        let s = open(&path);
        for n in ["A", "B", "C"] { node(&s, n); }
        edge(&s, "e-AB", "A", "B");
        edge(&s, "e-AC", "A", "C");
        assert_eq!(hop1_out(&s, "A"), 2);
        // Distinct ids -> distinct keys (no collision at this scale).
        assert_ne!(Storage::edge_key("e-AB"), Storage::edge_key("e-AC"));
        s.save_state().unwrap();
    }
    let s2 = open(&path);
    assert_eq!(hop1_out(&s2, "A"), 2, "u128-keyed edges survive snapshot reload");
}

/// A pre-u128 snapshot whose `edges.bin` stores a u64-range key still loads:
/// the saved key is ignored and re-derived from the edge id, so adjacency is
/// reconstructed correctly under u128.
#[test]
fn legacy_u64_keyed_snapshot_loads() {
    let path = fresh("test_edge_u128_legacy");
    {
        let s = open(&path);
        node(&s, "A"); node(&s, "B");
        edge(&s, "e-AB", "A", "B");
        s.save_state().unwrap();
    }
    // Rewrite edges.bin to a legacy u64-range key (what a pre-u128 build wrote).
    let edges_path = format!("{}/edges.bin", path);
    let mut v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&edges_path).unwrap()).unwrap();
    for tup in v.as_array_mut().unwrap() {
        tup[0] = serde_json::json!(123456789u64); // fits in u64; far from the real u128 key
    }
    fs::write(&edges_path, v.to_string()).unwrap();

    let s2 = open(&path);
    assert_eq!(hop1_out(&s2, "A"), 1, "legacy u64-keyed edges.bin loads + traverses under u128");
}
