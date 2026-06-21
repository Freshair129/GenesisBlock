// Layer A (ADR--GENESISDB-EDGE-ID-INTERNING): edges use the lean interning path
// — no trigram_index pollution, no redundant u32_to_id reverse entry — while
// staying fully traversable and WAL-replay-stable.

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
        lang: None, valid_from: None, caused_by: None, ttl: None,
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

/// Edges get a forward id_to_u32 entry (for idempotency/delete) but NOT a
/// reverse u32_to_id entry (EdgeOutput.id is canonical).
#[test]
fn edges_absent_from_reverse_map() {
    let s = open(&fresh("test_edge_intern_reverse"));
    node(&s, "A");
    node(&s, "B");
    edge(&s, "edge-xyz", "A", "B", "LINK");

    let eu = s.get_u32("edge-xyz").expect("edge must be in forward id_to_u32");
    assert!(
        s.u32_to_id.get(&eu).is_none(),
        "edge u32 must NOT be in the reverse u32_to_id map"
    );
    // Nodes still have their reverse entry.
    let au = s.get_u32("A").unwrap();
    assert_eq!(s.u32_to_id.get(&au).unwrap().value(), "A");
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
