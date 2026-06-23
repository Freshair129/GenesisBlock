// f32-sidecar rerank for quantized collections (MARK XIV P4,
// ADR--GENESISDB-VECTOR-QUANTIZATION). A quantized collection created with
// `rerank = true` keeps the exact f32 vectors in a parallel sidecar; search
// over-fetches quantized candidates from the HNSW, then re-scores them with exact
// f32 distance. These tests pin DETERMINISTIC invariants only.
//
// Guarded: (a) SQ8 + rerank still finds the exact match (rerank doesn't break the
// quantized path); (b) BQ + rerank recovers a distinction quantization destroys —
// two same-sign vectors of different magnitude collapse to identical BQ codes, so
// only the exact f32 rerank can pick the true nearest; (c) rerank survives
// save_state + reopen (the fvec_<name>.bin sidecar round-trips) and is reported on
// CollectionInfo; (d) rerank survives compaction (the sidecar is rebuilt in
// lock-step with the arena); (e) a quantized collection WITHOUT rerank writes no
// sidecar file.

use genesis_block_native::{Storage, OpenOptions, NodeInput, HybridSearchInput, CollectionInfo};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("G:/GenesisBlock_Dev/GenesisBlock/tests/{}", name);
    if Path::new(&p).exists() { fs::remove_dir_all(&p).unwrap(); }
    p
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(), page_cache_mb: Some(64), read_only: Some(false), vector_dim: Some(4),
    }).unwrap()
}

fn add(s: &Storage, id: &str, emb: Vec<f64>, coll: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()), labels: vec![], props: None,
        embedding: Some(emb), lang: None, valid_from: None, caused_by: None, ttl: None,
        collection: Some(coll.to_string()),
    }).unwrap();
}

fn top1(s: &Storage, q: Vec<f64>, coll: &str) -> Option<String> {
    s.flush_index();
    s.hybrid_search(HybridSearchInput {
        query_vector: q, k: 1, alpha: Some(0.0), lang: None, as_of: None,
        collection: Some(coll.to_string()), ef_search: None,
    }).unwrap().into_iter().map(|n| n.node.id).next()
}

fn info(s: &Storage, name: &str) -> CollectionInfo {
    s.list_collections().into_iter().find(|c| c.name == name)
        .unwrap_or_else(|| panic!("collection '{}' not found", name))
}

/// SQ8 + rerank still finds the exact match as top-1 — the rerank stage refines,
/// never breaks, the quantized result.
#[test]
fn sq8_rerank_finds_exact_match() {
    let s = open(&fresh("test_rr_sq8"));
    s.create_collection("c".into(), "m".into(), 4, Some("Cosine".into()), Some("sq8".into()), None, Some(true)).unwrap();
    add(&s, "A", vec![1.0, 0.0, 0.0, 0.0], "c");
    add(&s, "B", vec![0.0, 1.0, 0.0, 0.0], "c");
    add(&s, "C", vec![0.0, 0.0, 1.0, 0.0], "c");
    assert_eq!(top1(&s, vec![1.0, 0.0, 0.0, 0.0], "c").as_deref(), Some("A"));
    assert_eq!(top1(&s, vec![0.0, 0.0, 1.0, 0.0], "c").as_deref(), Some("C"));
    assert!(info(&s, "c").rerank, "CollectionInfo reports rerank enabled");
}

/// BQ collapses sign-equal vectors of different magnitude to identical codes, so
/// the quantized index alone cannot tell them apart. With rerank, the exact f32
/// distance does — the true nearest (the exact-magnitude match) wins as top-1.
/// (L2 metric so magnitude is preserved; Cosine would normalize the two equal.)
#[test]
fn bq_rerank_distinguishes_magnitude() {
    let s = open(&fresh("test_rr_bq"));
    s.create_collection("c".into(), "m".into(), 4, Some("L2".into()), Some("bq".into()), None, Some(true)).unwrap();
    // BIG and SMALL share the same sign pattern -> identical BQ codes (Hamming 0).
    add(&s, "BIG",   vec![1.0, 1.0, 1.0, 1.0], "c");
    add(&s, "SMALL", vec![0.2, 0.2, 0.2, 0.2], "c");
    add(&s, "NEG",   vec![-1.0, -1.0, -1.0, -1.0], "c"); // distinct sign -> far
    // Query is exactly SMALL. Under BQ alone BIG and SMALL are tied; rerank breaks
    // the tie by exact f32 distance and must return SMALL.
    assert_eq!(top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(), Some("SMALL"),
        "exact f32 rerank distinguishes magnitude that BQ codes cannot");
}

/// Rerank survives save_state + reopen: the fvec sidecar round-trips and the
/// rerank flag is restored from the manifest.
#[test]
fn rerank_survives_reload() {
    let path = fresh("test_rr_reload");
    {
        let s = open(&path);
        s.create_collection("c".into(), "m".into(), 4, Some("L2".into()), Some("bq".into()), None, Some(true)).unwrap();
        add(&s, "BIG",   vec![1.0, 1.0, 1.0, 1.0], "c");
        add(&s, "SMALL", vec![0.2, 0.2, 0.2, 0.2], "c");
        s.flush_index();
        assert_eq!(top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(), Some("SMALL"));
        s.save_state().unwrap();
        // The sidecar file exists and holds exactly n * dim f32 values.
        let fvec = fs::metadata(Path::new(&path).join("fvec_c.bin")).unwrap().len();
        assert_eq!(fvec, 2 * 4 * 4, "fvec sidecar = 2 vectors x dim 4 x 4 bytes");
    }
    let s = open(&path); // reopen from snapshot
    assert!(info(&s, "c").rerank, "rerank flag restored from manifest");
    assert_eq!(top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(), Some("SMALL"),
        "rerank still distinguishes magnitude after reopen (sidecar round-tripped)");
}

/// Rerank survives compaction: the sidecar is rebuilt in lock-step with the arena
/// (same component offsets), so the exact re-score still works afterward.
#[test]
fn rerank_survives_compaction() {
    let s = open(&fresh("test_rr_compact"));
    s.create_collection("c".into(), "m".into(), 4, Some("L2".into()), Some("bq".into()), None, Some(true)).unwrap();
    add(&s, "BIG",   vec![1.0, 1.0, 1.0, 1.0], "c");
    add(&s, "SMALL", vec![0.2, 0.2, 0.2, 0.2], "c");
    add(&s, "NEG",   vec![-1.0, -1.0, -1.0, -1.0], "c");
    s.flush_index();
    s.perform_index_compaction().unwrap();
    assert_eq!(top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(), Some("SMALL"),
        "exact rerank still correct after the sidecar is rebuilt by compaction");
}

/// A quantized collection created WITHOUT rerank writes no sidecar file and is not
/// flagged for rerank — the control path is unchanged.
#[test]
fn no_rerank_writes_no_sidecar() {
    let path = fresh("test_rr_none");
    {
        let s = open(&path);
        s.create_collection("c".into(), "m".into(), 4, Some("Cosine".into()), Some("sq8".into()), None, None).unwrap();
        add(&s, "A", vec![1.0, 0.0, 0.0, 0.0], "c");
        s.flush_index();
        assert!(!info(&s, "c").rerank, "rerank not enabled");
        s.save_state().unwrap();
    }
    assert!(!Path::new(&path).join("fvec_c.bin").exists(), "no sidecar file for a non-rerank collection");
}
