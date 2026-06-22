// Binary quantization (BQ) — ADR--GENESISDB-VECTOR-QUANTIZATION.
// Each dim is packed to one sign bit (u64 words); HNSW ranks by popcount Hamming
// (custom DistBinaryHamming, since anndists' u64 DistHamming is word-inequality).
// This is the symmetric, no-rerank first cut (matching SQ8). HNSW is approximate,
// so these pin DETERMINISTIC invariants only: (a) an exact-match query (identical
// sign pattern → Hamming 0) is top-1; (b) BQ survives save+reopen; (c) the
// vec_<name>.bin is ~32× smaller than the f32 equivalent.

use genesis_block_native::{Storage, OpenOptions, NodeInput, HybridSearchInput};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("G:/GenesisBlock_Dev/GenesisBlock/tests/{}", name);
    if Path::new(&p).exists() { fs::remove_dir_all(&p).unwrap(); }
    p
}

fn open(path: &str, dim: u32) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(), page_cache_mb: Some(32), read_only: Some(false), vector_dim: Some(dim),
    }).unwrap()
}

fn add(s: &Storage, id: &str, emb: Vec<f64>, coll: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()), labels: vec![], props: None, embedding: Some(emb), lang: None,
        valid_from: None, caused_by: None, ttl: None, collection: Some(coll.to_string()),
    }).unwrap();
}

fn top1(s: &Storage, q: Vec<f64>, coll: &str) -> Option<String> {
    s.flush_index();
    s.hybrid_search(HybridSearchInput {
        query_vector: q, k: 1, alpha: Some(0.0), lang: None, as_of: None,
        collection: Some(coll.to_string()), ef_search: None,
    }).unwrap().into_iter().map(|n| n.node.id).next()
}

const A: [f64; 8] = [1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0];
const B: [f64; 8] = [-1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0];
const C: [f64; 8] = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];

fn seed(s: &Storage, coll: &str) {
    s.create_collection(coll.to_string(), "m".to_string(), 8, None, Some("bq".to_string())).unwrap();
    add(s, "A", A.to_vec(), coll);
    add(s, "B", B.to_vec(), coll);
    add(s, "C", C.to_vec(), coll);
}

/// An exact-match query packs to the same sign code as the stored vector
/// (Hamming 0) and is returned top-1 under BQ.
#[test]
fn bq_exact_match_is_top1() {
    let s = open(&fresh("test_bq_exact"), 8);
    seed(&s, "bq");
    assert_eq!(top1(&s, A.to_vec(), "bq"), Some("A".to_string()));
    assert_eq!(top1(&s, B.to_vec(), "bq"), Some("B".to_string()));
    assert_eq!(top1(&s, C.to_vec(), "bq"), Some("C".to_string()));
}

/// BQ arena round-trips through save_state + reopen (u64 words persist, HNSW
/// rehydrates from the bit codes via the sign-preserving f32 path).
#[test]
fn bq_survives_reopen() {
    let path = fresh("test_bq_reopen");
    {
        let s = open(&path, 8);
        seed(&s, "bq");
        s.flush_index();
        s.save_state().unwrap();
    }
    let s2 = open(&path, 8);
    assert_eq!(top1(&s2, A.to_vec(), "bq"), Some("A".to_string()));
    assert_eq!(top1(&s2, C.to_vec(), "bq"), Some("C".to_string()));
}

/// The BQ vec_<name>.bin is ~32× smaller than the f32 equivalent: at dim 128 a
/// vector is 512 f32 bytes vs 2 u64 words (16 bytes).
#[test]
fn bq_disk_is_32x_smaller() {
    let path = fresh("test_bq_size");
    let dim = 128u32;
    let s = open(&path, dim); // default collection is f32 (None)
    s.create_collection("bq".to_string(), "m".to_string(), dim, None, Some("bq".to_string())).unwrap();
    let v: Vec<f64> = (0..dim).map(|i| if i % 3 == 0 { 0.5 } else { -0.5 }).collect();
    add(&s, "F", v.clone(), "default");
    add(&s, "Q", v.clone(), "bq");
    s.flush_index();
    s.save_state().unwrap();

    let full = fs::metadata(format!("{}/vec_default.bin", path)).unwrap().len();
    let bq = fs::metadata(format!("{}/vec_bq.bin", path)).unwrap().len();
    assert_eq!(full, (dim as u64) * 4, "f32 = 4 bytes/dim");
    assert_eq!(bq, 16, "bq = ceil(128/64)=2 words * 8 bytes");
    assert_eq!(full / bq, 32, "BQ is 32x smaller on disk at dim 128");
}
