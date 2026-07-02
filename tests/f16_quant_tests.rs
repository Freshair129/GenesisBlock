// P2a — F16 half-precision quantization (ADR Layer C; PLAN P2a-T3).
//
// F16 stores the arena as IEEE binary16 (u16 bits, 2 B/elem) and dequantizes to
// f32 for the HNSW (anndists has no Distance<f16>). It is near-lossless, so on
// toy data with distinct, f16-representable components the ranking is exact.
// These pin: (a) top-1 exact under F16; (b) survives save_state + reopen; (c)
// arena footprint is count*dim*2 (2× vs f32's *4); (d) no rerank sidecar is
// allocated for F16 even when rerank=true (it is near-lossless).

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

const DIM: u32 = 8;

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
        vector_dim: Some(DIM),
    })
    .unwrap()
}

fn make_f16(s: &Storage, coll: &str, rerank: bool) {
    s.create_collection(
        coll.to_string(),
        "m".to_string(),
        DIM,
        None,
        Some("f16".to_string()),
        None,
        Some(rerank),
    )
    .unwrap();
}

fn add(s: &Storage, id: &str, emb: Vec<f64>, coll: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: Some(emb),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: Some(coll.to_string()),
    })
    .unwrap();
}

fn top1(s: &Storage, q: Vec<f64>, coll: &str) -> Option<String> {
    s.flush_index();
    s.hybrid_search(HybridSearchInput {
        query_vector: q,
        k: 1,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: Some(coll.to_string()),
        ef_search: None,
        oversample: None,
    })
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .next()
}

// Distinct, exactly-f16-representable rows (small values with ≤10-bit mantissas).
fn rows() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("a", vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ("b", vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ("c", vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ("d", vec![0.5, 0.25, 0.125, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ]
}

fn seed(s: &Storage, coll: &str, rerank: bool) {
    make_f16(s, coll, rerank);
    for (id, emb) in rows() {
        add(s, id, emb, coll);
    }
    s.flush_index();
}

fn arena_bytes(s: &Storage, coll: &str) -> usize {
    s.collections.get(coll).unwrap().arena.read().byte_size()
}

// (a) F16 is lossless enough on toy data that an exact-match query is top-1.
#[test]
fn f16_exact_match_is_top1() {
    let s = open(&fresh("test_f16_exact"));
    seed(&s, "f16", false);
    for (id, emb) in rows() {
        assert_eq!(top1(&s, emb, "f16"), Some(id.to_string()), "top1 for {id}");
    }
}

// (b) F16 arena + HNSW round-trip through save_state + reopen.
#[test]
fn f16_survives_reopen() {
    let path = fresh("test_f16_reopen");
    {
        let s = open(&path);
        seed(&s, "f16", false);
        s.save_state().unwrap();
    }
    let s2 = open(&path);
    for (id, emb) in rows() {
        assert_eq!(
            top1(&s2, emb, "f16"),
            Some(id.to_string()),
            "reopen top1 {id}"
        );
    }
    assert_eq!(
        s2.collections.get("f16").unwrap().arena.read().byte_size(),
        rows().len() * DIM as usize * 2,
        "reloaded F16 arena must stay 2 B/elem"
    );
}

// (c) Footprint: F16 arena is count*dim*2 — half of f32's count*dim*4.
#[test]
fn f16_arena_is_two_bytes_per_component() {
    let s = open(&fresh("test_f16_bytes"));
    seed(&s, "f16", false);
    let n = rows().len();
    assert_eq!(arena_bytes(&s, "f16"), n * DIM as usize * 2);

    // Contrast: an f32 (Quant::None) collection of the same shape is 2× larger.
    s.create_collection(
        "plain".to_string(),
        "m".to_string(),
        DIM,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    for (id, emb) in rows() {
        add(&s, &format!("p{id}"), emb, "plain");
    }
    s.flush_index();
    assert_eq!(arena_bytes(&s, "plain"), n * DIM as usize * 4);
}

// (d) F16 is near-lossless ⇒ never allocates a rerank sidecar, even if asked.
#[test]
fn f16_never_allocates_a_sidecar() {
    let s = open(&fresh("test_f16_nosidecar"));
    seed(&s, "f16", true); // rerank=true must be ignored for F16
    let info = s
        .list_collections()
        .into_iter()
        .find(|c| c.name == "f16")
        .unwrap();
    assert_eq!(info.quant, "f16", "CollectionInfo must report f16");
    assert!(!info.rerank, "F16 must not carry a rerank sidecar");
    // And it still searches correctly.
    assert_eq!(top1(&s, rows()[2].1.clone(), "f16"), Some("c".to_string()));
}
