// A2 — NodeMetadata stores the interned node u32 instead of a String
// (ADR--GENESISDB-NODE-ID-INTERNING). On-disk format is gated by the manifest
// `mv` flag: mv>=1 reads the new u32 layout; absent/0 reads the pre-A2 String
// layout (`NodeMetadataV0`) and migrates it on load. These tests pin both the
// new round-trip AND backward-compat with a hand-crafted legacy snapshot.

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
    })
    .unwrap()
}

fn add(s: &Storage, id: &str, emb: Vec<f64>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: Some(emb),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

fn top1(s: &Storage, q: Vec<f64>) -> Option<String> {
    s.flush_index();
    s.hybrid_search(HybridSearchInput {
        query_vector: q,
        k: 1,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: None,
        ef_search: None,
    })
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .next()
}

/// New format (mv=1): metadata persists node_u32; save + reopen rebuilds
/// node_to_arena and search still finds the exact match.
#[test]
fn new_format_round_trips() {
    let path = fresh("test_a2_new");
    {
        let s = open(&path);
        add(&s, "A", vec![1.0, 0.0, 0.0, 0.0]);
        add(&s, "B", vec![0.0, 1.0, 0.0, 0.0]);
        s.flush_index();
        s.save_state().unwrap();
    }
    let s2 = open(&path);
    assert_eq!(top1(&s2, vec![1.0, 0.0, 0.0, 0.0]), Some("A".to_string()));
    assert_eq!(top1(&s2, vec![0.0, 1.0, 0.0, 0.0]), Some("B".to_string()));
}

/// Pre-A2 String layout of NodeMetadata, byte-compatible with `NodeMetadataV0`.
/// bincode is positional, so identical field order/types reproduce the legacy
/// bytes a pre-A2 build would have written.
#[derive(serde::Serialize)]
struct LegacyMeta {
    arena_id: u32,
    node_id: String,
    timestamp: u64,
    vector_dim: u16,
    embedding_offset: u64,
    gks_attributes: Vec<u8>,
    lang: String,
    cluster_id: u32,
}

/// A snapshot written by a pre-A2 build (String node_id, no `mv` in the
/// manifest) loads: the loader migrates the metadata to interned u32, rebuilds
/// node_to_arena, and search works.
#[test]
fn legacy_string_meta_migrates() {
    let path = fresh("test_a2_legacy");
    // 1. Produce a normal (new-format) single-vector snapshot.
    {
        let s = open(&path);
        add(&s, "L1", vec![1.0, 0.0, 0.0, 0.0]);
        s.flush_index();
        s.save_state().unwrap();
    }
    // 2. Downgrade it to the pre-A2 on-disk shape:
    //    a) rewrite meta_default.bin in the legacy String layout (the single
    //       first vector has deterministic arena_id=0 / offset=0 / dim=4 / cluster=0),
    let legacy = vec![LegacyMeta {
        arena_id: 0,
        node_id: "L1".to_string(),
        timestamp: 0,
        vector_dim: 4,
        embedding_offset: 0,
        gks_attributes: vec![],
        lang: "en".to_string(),
        cluster_id: 0,
    }];
    let bytes = bincode::serialize(&legacy).unwrap();
    fs::write(format!("{}/meta_default.bin", path), bytes).unwrap();
    //    b) remove the `mv` flag from the default collection's manifest entry.
    let sp = format!("{}/state.json", path);
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&sp).unwrap()).unwrap();
    if let Some(colls) = state["collections"].as_array_mut() {
        for c in colls.iter_mut() {
            if c["name"] == "default" {
                c.as_object_mut().unwrap().remove("mv");
            }
        }
    }
    fs::write(&sp, state.to_string()).unwrap();

    // 3. Reopen: legacy meta migrates to u32, node_to_arena rebuilds, search works.
    let s2 = open(&path);
    assert_eq!(
        top1(&s2, vec![1.0, 0.0, 0.0, 0.0]),
        Some("L1".to_string()),
        "legacy String metadata migrated and is searchable"
    );

    // 4. Re-saving upgrades the on-disk format back to mv=1.
    s2.save_state().unwrap();
    let restate: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&sp).unwrap()).unwrap();
    let dflt = restate["collections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "default")
        .unwrap();
    assert_eq!(
        dflt["mv"].as_u64(),
        Some(1),
        "re-save writes the new meta format"
    );
}
