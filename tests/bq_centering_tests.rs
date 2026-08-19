// P1a — BQ per-dim centering (docs/PLAN--VECTOR-QUANTIZATION-REFINEMENT.md).
//
// bge-m3 dims are positive-biased, so an uncentered `x > 0` sign bit carries
// little signal — a batch of all-positive vectors packs to the SAME all-ones
// code and BQ cannot tell them apart. Centering subtracts a per-dim mean
// (computed at compaction from the exact-f32 rerank sidecar) BEFORE the sign
// bit, rebalancing the codes. These tests pin the mechanism deterministically:
//   T1: compaction computes `bq_center` ≈ the per-dim mean of the live rows.
//   T1/T3: centering actually flips bits (packed codes are no longer all-ones).
//   T2: the center survives save_state + reopen.
//   T3: search still returns the true nearest (centering never regresses), and
//       a BQ collection WITHOUT a sidecar gets NO center (the mean needs f32).

use genesis_block_native::{ArenaStore, HybridSearchInput, NodeInput, OpenOptions, Storage};
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
        retention: None,
    })
    .unwrap()
}

fn make_bq(s: &Storage, coll: &str, rerank: bool) {
    s.create_collection(
        coll.to_string(),
        "m".to_string(),
        DIM,
        None,
        Some("bq".to_string()),
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

// Strongly positive-biased rows: every component is > 0, so the UNCENTERED sign
// code is all-ones for every row (BQ blind). The per-dim variation lives in the
// first four dims; dims 4..8 are constant (mean subtracts to exactly 0 ⇒ bit 0).
fn rows() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("r0", vec![1.4, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
        ("r1", vec![1.0, 1.4, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
        ("r2", vec![1.0, 1.0, 1.4, 1.0, 1.0, 1.0, 1.0, 1.0]),
        ("r3", vec![1.0, 1.0, 1.0, 1.4, 1.0, 1.0, 1.0, 1.0]),
    ]
}

fn seed(s: &Storage, coll: &str, rerank: bool) {
    make_bq(s, coll, rerank);
    for (id, emb) in rows() {
        add(s, id, emb, coll);
    }
    s.flush_index();
}

// Popcount over a collection's BQ arena. Uncentered all-positive data ⇒ every
// bit set (== n*dim); centering must bring this strictly below the all-ones max.
fn arena_popcount(s: &Storage, coll: &str) -> (u32, u32) {
    let c = s.collections.get(coll).unwrap();
    let arena = c.arena.read();
    match &*arena {
        ArenaStore::Binary { data, dim, n } => {
            let ones: u32 = data.iter().map(|w| w.count_ones()).sum();
            (ones, (*n * *dim) as u32)
        }
        _ => panic!("expected a Binary arena"),
    }
}

fn center_of(s: &Storage, coll: &str) -> Option<Vec<f32>> {
    s.collections.get(coll).unwrap().bq_center.read().clone()
}

// T1 — compaction computes the per-dim mean over live rows into `bq_center`.
#[test]
fn center_is_per_dim_mean_after_compaction() {
    let s = open(&fresh("test_bqc_mean"));
    seed(&s, "bq", true);
    assert!(
        center_of(&s, "bq").is_none(),
        "uncentered before compaction"
    );

    s.perform_index_compaction().unwrap();

    let center = center_of(&s, "bq").expect("compaction must compute a center");
    assert_eq!(center.len(), DIM as usize);
    // Expected per-dim mean of rows(): dims 0..4 have one 1.4 + three 1.0 ⇒ 1.1;
    // dims 4..8 are all 1.0 ⇒ 1.0.
    let expected = [1.1f32, 1.1, 1.1, 1.1, 1.0, 1.0, 1.0, 1.0];
    for (got, exp) in center.iter().zip(expected.iter()) {
        assert!((got - exp).abs() < 1e-5, "center {got} != expected {exp}");
    }
}

// T1/T3 — centering actually flips bits: the all-positive rows are all-ones when
// uncentered, and strictly fewer than all-ones once the mean is subtracted.
#[test]
fn centering_rebalances_the_codes() {
    let s = open(&fresh("test_bqc_bits"));
    seed(&s, "bq", true);

    // Before compaction: no center ⇒ every component > 0 ⇒ every bit set.
    let (ones_before, max) = arena_popcount(&s, "bq");
    assert_eq!(
        ones_before, max,
        "uncentered all-positive rows must be all-ones"
    );

    s.perform_index_compaction().unwrap();

    // After compaction: centered codes must drop below the all-ones ceiling
    // (some components now fall below their per-dim mean ⇒ bit 0) while staying
    // non-trivial (the four varying dims still set a bit on their peak row).
    let (ones_after, max_after) = arena_popcount(&s, "bq");
    assert_eq!(max_after, max, "row/dim count unchanged by compaction");
    assert!(
        ones_after < max,
        "centering must clear some bits ({ones_after} !< {max})"
    );
    assert!(ones_after > 0, "centering must not clear every bit");
}

// T2 — the computed center round-trips through save_state + reopen (bqmean_*.bin).
#[test]
fn center_survives_reopen() {
    let path = fresh("test_bqc_reopen");
    let before = {
        let s = open(&path);
        seed(&s, "bq", true);
        s.perform_index_compaction().unwrap();
        s.save_state().unwrap();
        center_of(&s, "bq").expect("center computed pre-save")
    };
    let s2 = open(&path);
    let after = center_of(&s2, "bq").expect("center must reload from bqmean_*.bin");
    assert_eq!(before, after, "center must survive save+reopen byte-exact");
    // And search still resolves the true nearest on the reopened, centered index.
    assert_eq!(top1(&s2, rows()[2].1.clone(), "bq"), Some("r2".to_string()));
}

// T3 — centering never regresses ranking: an exact-match query returns its row
// as top-1 on the centered, rerank-enabled index.
#[test]
fn centered_search_returns_true_nearest() {
    let s = open(&fresh("test_bqc_search"));
    seed(&s, "bq", true);
    s.perform_index_compaction().unwrap();
    for (id, emb) in rows() {
        assert_eq!(top1(&s, emb, "bq"), Some(id.to_string()), "top1 for {id}");
    }
}

// T3 constraint — a BQ collection WITHOUT a rerank sidecar has no f32 source for
// the mean, so compaction leaves it uncentered (documented behavior).
#[test]
fn no_sidecar_means_no_center() {
    let s = open(&fresh("test_bqc_nosidecar"));
    seed(&s, "bq_plain", false);
    s.perform_index_compaction().unwrap();
    assert!(
        center_of(&s, "bq_plain").is_none(),
        "sidecar-less BQ collection must stay uncentered"
    );
    // (No functional ranking assertion here: without a center AND without rerank,
    // these all-positive rows pack to the SAME all-ones code — indistinguishable.
    // That indistinguishability is exactly the problem centering exists to fix;
    // `centered_search_returns_true_nearest` covers the ranking win with a center.)
}
