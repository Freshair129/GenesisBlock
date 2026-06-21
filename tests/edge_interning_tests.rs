// Layer A (ADR--GENESISDB-EDGE-ID-INTERNING): edges use the lean interning path
// — no trigram_index pollution, no redundant u32_to_id reverse entry — while
// staying fully traversable and WAL-replay-stable.
//
// Layer B (ADR--GENESISDB-EDGE-NUMERIC-KEYS): edges are keyed by the
// deterministic u64 hash `Storage::edge_key(id)` and carry NO `id_to_u32`
// string entry at all. Idempotency = "is this u64 already in `edges`?".

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

fn trigram_members(s: &Storage) -> usize {
    s.trigram_index.iter().map(|e| e.value().len()).sum()
}

fn hop1_out(s: &Storage, id: &str) -> usize {
    s.neighbors(
        id.to_string(),
        NeighborInput {
            depth: Some(1), rel: None, rels: None, direction: Some("out".to_string()),
            as_of: None, include_invalid: Some(false), limit: Some(1000),
        },
        false,
    )
    .unwrap()
    .len()
}

/// Edge ids must not add any members to trigram_index (find_fuzzy_id is node-only).
#[test]
fn edges_do_not_pollute_trigram() {
    let s = open(&fresh("test_edge_intern_trigram"));
    for n in ["A", "B", "C", "D"] { node(&s, n); }
    let before = trigram_members(&s);

    // UUID-shaped edge ids — the worst case (~48 trigram members each under the
    // old path).
    edge(&s, "11111111-1111-1111-1111-111111111111", "A", "B", "LINK");
    edge(&s, "22222222-2222-2222-2222-222222222222", "A", "C", "LINK");
    edge(&s, "33333333-3333-3333-3333-333333333333", "B", "D", "LINK");

    let after = trigram_members(&s);
    assert_eq!(before, after, "edge ids must contribute 0 trigram members");
}

/// Layer B: edges carry NO `id_to_u32` string entry (neither forward nor
/// reverse). They are reachable only via the deterministic `edge_key` hash in
/// the `edges` map. Nodes keep both forward and reverse entries.
#[test]
fn edges_absent_from_id_maps() {
    let s = open(&fresh("test_edge_intern_reverse"));
    node(&s, "A");
    node(&s, "B");
    edge(&s, "edge-xyz", "A", "B", "LINK");

    // Edge id is NOT in id_to_u32 at all (no 8M UUID strings — the Layer B win).
    assert!(
        s.get_u32("edge-xyz").is_none(),
        "edge id must NOT occupy a forward id_to_u32 entry under numeric keys"
    );
    // But the edge IS keyed by its u64 hash in the edges map.
    assert!(
        s.edges.get(&Storage::edge_key("edge-xyz")).is_some(),
        "edge must be reachable via its numeric key"
    );
    // Nodes still have both forward and reverse entries.
    let au = s.get_u32("A").unwrap();
    assert_eq!(s.u32_to_id.get(&au).unwrap().value(), "A");
}

/// Layer B: edge_key is a pure deterministic function of the id string.
#[test]
fn edge_key_is_deterministic() {
    assert_eq!(Storage::edge_key("e1"), Storage::edge_key("e1"));
    assert_ne!(Storage::edge_key("e1"), Storage::edge_key("e2"));
    // Stable across the whole UUID surface we actually use.
    let uuid = "aaaaaaaa-0000-0000-0000-000000000001";
    assert_eq!(Storage::edge_key(uuid), Storage::edge_key(uuid));
}

/// Layer B: re-applying the same edge id is idempotent — same u64 key, no dup,
/// adjacency stays single-membered.
#[test]
fn edge_reapply_is_idempotent() {
    let s = open(&fresh("test_edge_intern_idempotent"));
    node(&s, "A");
    node(&s, "B");
    edge(&s, "dup-edge", "A", "B", "LINK");
    edge(&s, "dup-edge", "A", "B", "LINK"); // same id again

    assert_eq!(s.edges.len(), 1, "re-applied edge must not duplicate");
    let au = s.get_u32("A").unwrap();
    let out_len = s.out_idx.get(&au).unwrap().len();
    assert_eq!(out_len, 1, "out_idx must hold the edge key once");
}

/// Layer B: delete-by-node removes the edge (by u64 key) and both adjacency
/// sides — and leaves no stray id_to_u32 entry (there was none to leak).
#[test]
fn delete_removes_numeric_edge_and_adjacency() {
    let s = open(&fresh("test_edge_intern_delete"));
    node(&s, "A");
    node(&s, "B");
    edge(&s, "del-edge", "A", "B", "LINK");
    let key = Storage::edge_key("del-edge");
    let bu = s.get_u32("B").unwrap();
    assert!(s.edges.get(&key).is_some());

    s.retract_node("A").unwrap();

    assert!(s.edges.get(&key).is_none(), "edge must be gone");
    let b_still_has_edge = s.in_idx.get(&bu).map(|s| s.contains(&key)).unwrap_or(false);
    assert!(!b_still_has_edge, "B's in-index must drop the edge key");
}

/// Layer B: snapshot round-trip keeps edges + out_idx/in_idx consistent, with
/// keys re-derived from the edge ids (not the saved tuple key).
#[test]
fn numeric_keys_survive_snapshot_roundtrip() {
    let path = fresh("test_edge_intern_numeric_reload");
    let key2 = Storage::edge_key("aaaaaaaa-0000-0000-0000-000000000002");
    {
        let s = open(&path);
        for n in ["A", "B", "C", "D"] { node(&s, n); }
        edge(&s, "aaaaaaaa-0000-0000-0000-000000000001", "A", "B", "LINK");
        edge(&s, "aaaaaaaa-0000-0000-0000-000000000002", "A", "C", "LINK");
        edge(&s, "aaaaaaaa-0000-0000-0000-000000000003", "C", "D", "LINK");
        assert!(s.edges.get(&key2).is_some());
        // drop -> snapshot flush
    }
    let s2 = open(&path);
    // Edge still keyed by the deterministic hash after reload.
    assert!(
        s2.edges.get(&key2).is_some(),
        "edge must be reachable by re-derived numeric key after reload"
    );
    // Adjacency consistent: A still reaches B and C.
    assert_eq!(hop1_out(&s2, "A"), 2, "adjacency must survive reload");
    let au = s2.get_u32("A").unwrap();
    let has_key = s2.out_idx.get(&au).unwrap().contains(&key2);
    assert!(has_key, "out_idx must hold the re-derived edge key after reload");
}

/// fuzzy node resolution must never surface an edge id as a candidate.
#[test]
fn fuzzy_id_never_resolves_to_edge() {
    let s = open(&fresh("test_edge_intern_fuzzy"));
    node(&s, "Node-Alpha");
    edge(&s, "Node-Alphaa", "Node-Alpha", "Node-Alpha", "SELF"); // near-miss edge id
    // A near-miss query should resolve to the node, never the edge.
    let hit = s.find_fuzzy_id("Node-Alpha");
    assert_eq!(hit, Some("Node-Alpha".to_string()));
}

/// Edges remain fully traversable under the lean path.
#[test]
fn edges_still_traversable() {
    let s = open(&fresh("test_edge_intern_traverse"));
    for n in ["A", "B", "C"] { node(&s, n); }
    edge(&s, "e1", "A", "B", "LINK");
    edge(&s, "e2", "A", "C", "LINK");
    assert_eq!(hop1_out(&s, "A"), 2, "A should reach B and C");
}

/// Reload reproduces identical graph state AND never carries edge ids in trigram.
/// NOTE: reopen prefers the binary snapshot (instant-load). That path now
/// rebuilds the NODE trigram index (see `fuzzy_id_survives_snapshot_reload`),
/// but still never tokenizes edge ids. The Layer-A property under test is that
/// edges contribute no trigram members on either build or reload, so we assert
/// a pollution bound, not strict equality.
#[test]
fn reload_stable_and_trigram_unpolluted() {
    let path = fresh("test_edge_intern_replay");
    let (edges_n, hop_a, tri_live);
    {
        let s = open(&path);
        for n in ["A", "B", "C", "D"] { node(&s, n); }
        edge(&s, "aaaaaaaa-0000-0000-0000-000000000001", "A", "B", "LINK");
        edge(&s, "aaaaaaaa-0000-0000-0000-000000000002", "A", "C", "LINK");
        edge(&s, "aaaaaaaa-0000-0000-0000-000000000003", "C", "D", "LINK");
        edges_n = s.edges.len();
        hop_a = hop1_out(&s, "A");
        tri_live = trigram_members(&s);
        // drop -> release lock, flush snapshot
    }
    // Live build: trigram reflects the 4 short node ids only, never ~144 (3*48).
    assert!(tri_live < 100, "live trigram ({tri_live}) must exclude edge ids");

    let s2 = open(&path); // reload (snapshot instant-load or WAL replay)
    assert_eq!(s2.edges.len(), edges_n, "edge count must survive reload");
    assert_eq!(hop1_out(&s2, "A"), hop_a, "adjacency must survive reload");
    assert!(
        trigram_members(&s2) < 100,
        "reload must not carry edge ids in trigram"
    );
}

/// Regression: the snapshot instant-load path (try_load_state) must rebuild the
/// node trigram index, or `find_fuzzy_id` is silently dead after every graceful
/// shutdown + reopen. A graceful close writes state.json + nodes.bin, so reopen
/// prefers the snapshot over WAL replay — exactly the path that used to skip
/// trigram rebuild. Nodes carry no embeddings, so the neural/vector fallback
/// can't resolve these; only the (Thai-aware) trigram lexical path can.
#[test]
fn fuzzy_id_survives_snapshot_reload() {
    let path = fresh("test_snapshot_trigram_reload");
    {
        let s = open(&path);
        node(&s, "Concept-Alpha");
        node(&s, "สวัสดีชาวโลก"); // Thai id — the trigram path's reason to exist
        node(&s, "filler-one");
        assert!(trigram_members(&s) > 0, "sanity: live build populates trigram");
        // drop -> graceful shutdown flushes the binary snapshot
    }

    let s2 = open(&path); // reopen -> snapshot instant-load
    assert!(
        trigram_members(&s2) > 0,
        "snapshot instant-load must rebuild the node trigram index (was 0 = dead fuzzy)"
    );
    // Exact match still works trivially; the real test is a NON-exact near-miss,
    // which can only resolve via the rebuilt trigram candidates.
    assert_eq!(
        s2.find_fuzzy_id("Concept-Alphaa"),
        Some("Concept-Alpha".to_string()),
        "ascii near-miss must resolve after snapshot reload"
    );
    assert_eq!(
        s2.find_fuzzy_id("สวัสดีชาวโลกก"),
        Some("สวัสดีชาวโลก".to_string()),
        "Thai near-miss must resolve after snapshot reload"
    );
}
