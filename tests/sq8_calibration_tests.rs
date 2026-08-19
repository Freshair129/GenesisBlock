// P2b — SQ8 calibrated/quantile scale (docs/PLAN--VECTOR-QUANTIZATION-REFINEMENT.md).
//
// Fixed SQ8 maps [-1,1] -> [0,255], so UN-NORMALIZED (out-of-[-1,1]) components
// all clamp to 255 and collapse to one indistinguishable code. The opt-in
// calibrated scale (quant:"sq8c") derives a per-collection [lo,hi] quantile range
// from the rerank sidecar's exact f32 at compaction and maps THAT to [0,255],
// restoring resolution. These pin the mechanism deterministically:
//   T1: compaction installs a calibrated (scale,bias); it clears the clamp.
//   T2: the scale survives save_state + reopen (sq8scale_<name>.bin); plain "sq8"
//       (calibrate off) computes NO scale (byte-identical to pre-P2b).
//   functional: search returns the true nearest on the calibrated index.

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

// `quant` is the raw quant string: "sq8c" opts into calibration, "sq8" is fixed.
fn make(s: &Storage, coll: &str, quant: &str, rerank: bool) {
    s.create_collection(
        coll.to_string(),
        "m".to_string(),
        DIM,
        None,
        Some(quant.to_string()),
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

// UN-NORMALIZED rows: every component is in [3.0, 8.0] — all > 1, so the FIXED
// scale clamps every component to 255 (one collapsed code). The peak dim varies
// per row, which only the calibrated scale can resolve.
fn rows() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("r0", vec![8.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0]),
        ("r1", vec![3.0, 8.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0]),
        ("r2", vec![3.0, 3.0, 8.0, 3.0, 3.0, 3.0, 3.0, 3.0]),
        ("r3", vec![3.0, 3.0, 3.0, 8.0, 3.0, 3.0, 3.0, 3.0]),
    ]
}

fn seed(s: &Storage, coll: &str, quant: &str, rerank: bool) {
    make(s, coll, quant, rerank);
    for (id, emb) in rows() {
        add(s, id, emb, coll);
    }
    s.flush_index();
}

fn scale_of(s: &Storage, coll: &str) -> Option<(f32, f32)> {
    *s.collections.get(coll).unwrap().sq8_scale.read()
}

// Count how many of the arena's u8 codes are saturated at 255 (the clamp signal).
fn saturated_codes(s: &Storage, coll: &str) -> (usize, usize) {
    let c = s.collections.get(coll).unwrap();
    let arena = c.arena.read();
    match &*arena {
        ArenaStore::U8 { data, .. } => (data.iter().filter(|&&b| b == 255).count(), data.len()),
        _ => panic!("expected a U8 arena"),
    }
}

// T1 — compaction installs a calibrated scale that clears the fixed-scale clamp.
#[test]
fn calibration_clears_the_clamp() {
    let s = open(&fresh("test_sq8c_clamp"));
    seed(&s, "sq8c", "sq8c", true);

    // Before compaction: fixed scale ⇒ every out-of-range component clamps to 255.
    let (sat_before, total) = saturated_codes(&s, "sq8c");
    assert_eq!(
        sat_before, total,
        "fixed scale must clamp all >1 codes to 255"
    );
    assert!(
        scale_of(&s, "sq8c").is_none(),
        "no calibrated scale before compaction"
    );

    s.perform_index_compaction().unwrap();

    // After: a calibrated (scale,bias) is installed and the clamp is gone.
    let (scale, _bias) = scale_of(&s, "sq8c").expect("compaction must calibrate");
    assert!(scale > 0.0 && scale.is_finite());
    let (sat_after, total_after) = saturated_codes(&s, "sq8c");
    assert_eq!(total_after, total, "row/dim count unchanged");
    assert!(
        sat_after < total_after,
        "calibration must un-clamp codes ({sat_after} !< {total_after})"
    );
}

// T2 — plain "sq8" (calibrate OFF) computes NO scale: byte-identical to pre-P2b.
#[test]
fn plain_sq8_is_not_calibrated() {
    let s = open(&fresh("test_sq8_fixed"));
    seed(&s, "sq8", "sq8", true);
    s.perform_index_compaction().unwrap();
    assert!(
        scale_of(&s, "sq8").is_none(),
        "fixed SQ8 must never compute a calibrated scale"
    );
    // Fixed scale still clamps these un-normalized codes (documented limitation).
    let (sat, total) = saturated_codes(&s, "sq8");
    assert_eq!(sat, total, "fixed scale keeps clamping out-of-range codes");
}

// T2 — the calibrated scale survives save_state + reopen (sq8scale_<name>.bin),
// and search resolves the true nearest on the reopened calibrated index.
#[test]
fn calibrated_scale_survives_reopen() {
    let path = fresh("test_sq8c_reopen");
    let before = {
        let s = open(&path);
        seed(&s, "sq8c", "sq8c", true);
        s.perform_index_compaction().unwrap();
        s.save_state().unwrap();
        scale_of(&s, "sq8c").expect("calibrated pre-save")
    };
    let s2 = open(&path);
    let after = scale_of(&s2, "sq8c").expect("scale must reload from sq8scale_*.bin");
    assert_eq!(before, after, "calibrated scale must round-trip byte-exact");
    for (id, emb) in rows() {
        assert_eq!(
            top1(&s2, emb, "sq8c"),
            Some(id.to_string()),
            "reopen top1 {id}"
        );
    }
}

// functional — the calibrated, rerank-enabled index returns the true nearest.
#[test]
fn calibrated_search_returns_true_nearest() {
    let s = open(&fresh("test_sq8c_search"));
    seed(&s, "sq8c", "sq8c", true);
    s.perform_index_compaction().unwrap();
    for (id, emb) in rows() {
        assert_eq!(top1(&s, emb, "sq8c"), Some(id.to_string()), "top1 {id}");
    }
}
