// Per-collection vector quantization (ADR--GENESISDB-VECTOR-QUANTIZATION).
// Phase 2 = SQ8 (full resident cut): the arena + HNSW become u8; `None` stays
// byte-identical f32. These tests pin DETERMINISTIC invariants only — HNSW is
// approximate, so on tiny/degenerate sets near-tie ordering is not stable and is
// not asserted. SQ8 recall on real embeddings is validated by benchmark, not here.
//
// Guarded: (a) the exact-match query finds its own vector as top-1 under SQ8 (the
// query quantizes to the same u8 codes -> distance 0); (b) SQ8 survives
// save_state + reopen (u8 arena round-trips, index rehydrates); (c) the SQ8
// vec_<name>.bin is exactly 4x smaller than the f32 file; (d) a `None` collection
// keeps full f32 width on disk and still finds its exact match.

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
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
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

fn seed(s: &Storage, coll: &str) {
    add(s, "A", vec![1.0, 0.0, 0.0, 0.0], coll);
    add(s, "B", vec![0.0, 1.0, 0.0, 0.0], coll);
    add(s, "C", vec![0.0, 0.0, 1.0, 0.0], coll);
    add(s, "D", vec![0.0, 0.0, 0.0, 1.0], coll);
    add(s, "E", vec![0.9, 0.1, 0.0, 0.0], coll);
}

/// An exact-match query finds its own vector as top-1 under SQ8 — the query
/// quantizes to the same u8 codes as the stored vector, so its distance is 0.
#[test]
fn sq8_finds_exact_match() {
    let s = open(&fresh("test_q_sq8_exact"));
    s.create_collection(
        "sq8c".into(),
        "m".into(),
        4,
        Some("Cosine".into()),
        Some("sq8".into()),
        None,
        None,
    )
    .unwrap();
    seed(&s, "sq8c");
    assert_eq!(
        top1(&s, vec![1.0, 0.0, 0.0, 0.0], "sq8c").as_deref(),
        Some("A")
    );
    assert_eq!(
        top1(&s, vec![0.0, 0.0, 1.0, 0.0], "sq8c").as_deref(),
        Some("C")
    );
}

/// A `None` (f32) collection finds its exact match too — the control path is
/// behaviorally unchanged by the quantization plumbing.
#[test]
fn none_finds_exact_match() {
    let s = open(&fresh("test_q_none_exact"));
    s.create_collection(
        "f32c".into(),
        "m".into(),
        4,
        Some("Cosine".into()),
        None,
        None,
        None,
    )
    .unwrap();
    seed(&s, "f32c");
    assert_eq!(
        top1(&s, vec![1.0, 0.0, 0.0, 0.0], "f32c").as_deref(),
        Some("A")
    );
}

/// SQ8 survives save_state + reopen: the u8 arena round-trips on disk and the
/// index rehydrates, so the exact match is still found.
#[test]
fn sq8_survives_reload() {
    let path = fresh("test_q_sq8_reload");
    {
        let s = open(&path);
        s.create_collection(
            "sq8c".into(),
            "m".into(),
            4,
            Some("Cosine".into()),
            Some("sq8".into()),
            None,
            None,
        )
        .unwrap();
        seed(&s, "sq8c");
        assert_eq!(
            top1(&s, vec![1.0, 0.0, 0.0, 0.0], "sq8c").as_deref(),
            Some("A")
        );
        s.save_state().unwrap();
    }
    let s = open(&path); // same path, no fresh() -> instant-load from snapshot
    assert_eq!(
        top1(&s, vec![1.0, 0.0, 0.0, 0.0], "sq8c").as_deref(),
        Some("A"),
        "exact match still found after reopening the SQ8 collection"
    );
}

/// `vec_<name>.bin` is exactly 4x smaller for SQ8 than for f32 (same #vectors,
/// dim), and the `None` file is the exact f32 width — the full resident cut, on disk.
#[test]
fn sq8_disk_is_quarter_of_f32() {
    let path = fresh("test_q_disk_width");
    {
        let s = open(&path);
        s.create_collection(
            "f32c".into(),
            "m".into(),
            4,
            Some("Cosine".into()),
            None,
            None,
            None,
        )
        .unwrap();
        s.create_collection(
            "sq8c".into(),
            "m".into(),
            4,
            Some("Cosine".into()),
            Some("sq8".into()),
            None,
            None,
        )
        .unwrap();
        seed(&s, "f32c");
        seed(&s, "sq8c");
        s.flush_index();
        s.save_state().unwrap();
    }
    let f32_bytes = fs::metadata(Path::new(&path).join("vec_f32c.bin"))
        .unwrap()
        .len();
    let sq8_bytes = fs::metadata(Path::new(&path).join("vec_sq8c.bin"))
        .unwrap()
        .len();
    assert_eq!(f32_bytes, 5 * 4 * 4, "f32: 5 vectors x dim 4 x 4 bytes");
    assert_eq!(sq8_bytes, 5 * 4, "sq8: 5 vectors x dim 4 x 1 byte");
    assert_eq!(
        f32_bytes,
        sq8_bytes * 4,
        "SQ8 arena is exactly 4x smaller on disk"
    );
}
