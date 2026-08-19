// On-disk rerank sidecar correctness suite (P0-T8). Companion to
// `tests/rerank_tests.rs` (NOT a replacement — those 6 tests stay as-is).
//
// P0 moved the exact-f32 rerank sidecar off-RAM: `fvec_<name>.bin` is read via
// positioned `seek_read`/`read_at` through `SidecarReader` (LRU-cached), not held
// resident. This suite pins the on-disk-specific behavioral contract:
//
//  1. Parity: on-disk rerank returns the same exact top-1 the resident-era rerank
//     did (same toy-vector oracle as `rerank_tests.rs::bq_rerank_distinguishes_magnitude`).
//  2. Reopen round-trip: close the DB (drop the `Storage`, dropping the
//     `SidecarReader`'s open file handle) and reopen — the reader re-adopts the
//     on-disk `fvec` and search stays exact.
//  3. Degraded-fvec: truncate/corrupt `fvec_<name>.bin` so `len_rows() !=
//     arena_rows` on reload — search must degrade to quantized-only (no panic,
//     no empty result), never blindly rerank against mismatched rows.

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs::{self, OpenOptions as FsOpenOptions};
use std::io::Write;
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

/// (1) Parity: BQ collapses BIG/SMALL (same sign pattern, different magnitude)
/// to identical codes under L2. The quantized index alone cannot distinguish
/// them; only the on-disk exact-f32 rerank can. This is the same oracle
/// `rerank_tests.rs` used for the resident-era sidecar — pinning that the
/// on-disk positioned-read path returns the identical exact top-1.
#[test]
fn ondisk_rerank_parity_distinguishes_magnitude() {
    let s = open(&fresh("test_sc_parity"));
    s.create_collection(
        "c".into(),
        "m".into(),
        4,
        Some("L2".into()),
        Some("bq".into()),
        None,
        Some(true),
    )
    .unwrap();
    add(&s, "BIG", vec![1.0, 1.0, 1.0, 1.0], "c");
    add(&s, "SMALL", vec![0.2, 0.2, 0.2, 0.2], "c");
    add(&s, "NEG", vec![-1.0, -1.0, -1.0, -1.0], "c");

    assert_eq!(
        top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(),
        Some("SMALL"),
        "on-disk exact f32 rerank must distinguish magnitude that BQ codes cannot"
    );
    assert_eq!(
        top1(&s, vec![1.0, 1.0, 1.0, 1.0], "c").as_deref(),
        Some("BIG"),
        "on-disk exact f32 rerank must also resolve the other magnitude exactly"
    );
}

/// (1b) Parity for SQ8: exact top-1 still correct through the on-disk rerank
/// path (mirrors `rerank_tests.rs::sq8_rerank_finds_exact_match`).
#[test]
fn ondisk_rerank_parity_sq8_exact_match() {
    let s = open(&fresh("test_sc_parity_sq8"));
    s.create_collection(
        "c".into(),
        "m".into(),
        4,
        Some("Cosine".into()),
        Some("sq8".into()),
        None,
        Some(true),
    )
    .unwrap();
    add(&s, "A", vec![1.0, 0.0, 0.0, 0.0], "c");
    add(&s, "B", vec![0.0, 1.0, 0.0, 0.0], "c");
    add(&s, "C", vec![0.0, 0.0, 1.0, 0.0], "c");

    assert_eq!(
        top1(&s, vec![1.0, 0.0, 0.0, 0.0], "c").as_deref(),
        Some("A")
    );
    assert_eq!(
        top1(&s, vec![0.0, 0.0, 1.0, 0.0], "c").as_deref(),
        Some("C")
    );
}

/// (2) Reopen round-trip: drop the `Storage` (closing the `SidecarReader`'s file
/// handle), reopen from the on-disk snapshot, and confirm search is STILL exact
/// — the reader re-adopts `fvec_<name>.bin` on load rather than silently
/// dropping rerank capability.
#[test]
fn ondisk_rerank_survives_close_and_reopen() {
    let path = fresh("test_sc_reopen");
    {
        let s = open(&path);
        s.create_collection(
            "c".into(),
            "m".into(),
            4,
            Some("L2".into()),
            Some("bq".into()),
            None,
            Some(true),
        )
        .unwrap();
        add(&s, "BIG", vec![1.0, 1.0, 1.0, 1.0], "c");
        add(&s, "SMALL", vec![0.2, 0.2, 0.2, 0.2], "c");
        s.flush_index();
        assert_eq!(
            top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(),
            Some("SMALL")
        );
        s.save_state().unwrap();
        // `s` drops here -> the SidecarReader's positioned-read file handle closes.
    }

    // Reopen from the snapshot: a brand-new SidecarReader must be constructed
    // against the on-disk fvec file, not reuse any in-process state.
    let s = open(&path);
    assert_eq!(
        top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(),
        Some("SMALL"),
        "on-disk rerank still exact after full close+reopen (reader re-adopted fvec)"
    );

    // A second reopen (reopen-of-a-reopen) exercises the same path again, ruling
    // out state that only survives exactly one round-trip.
    drop(s);
    let s2 = open(&path);
    assert_eq!(
        top1(&s2, vec![1.0, 1.0, 1.0, 1.0], "c").as_deref(),
        Some("BIG"),
        "on-disk rerank exact after a second close+reopen"
    );
}

/// (2b) Reopen round-trip after compaction: compaction rewrites the sidecar
/// in lock-step with the arena; verify that survives a subsequent close+reopen
/// too (not just an in-process compaction check, which `rerank_tests.rs`
/// already covers).
#[test]
fn ondisk_rerank_survives_compaction_then_reopen() {
    let path = fresh("test_sc_compact_reopen");
    {
        let s = open(&path);
        s.create_collection(
            "c".into(),
            "m".into(),
            4,
            Some("L2".into()),
            Some("bq".into()),
            None,
            Some(true),
        )
        .unwrap();
        add(&s, "BIG", vec![1.0, 1.0, 1.0, 1.0], "c");
        add(&s, "SMALL", vec![0.2, 0.2, 0.2, 0.2], "c");
        add(&s, "NEG", vec![-1.0, -1.0, -1.0, -1.0], "c");
        s.flush_index();
        s.perform_index_compaction().unwrap();
        assert_eq!(
            top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(),
            Some("SMALL")
        );
        s.save_state().unwrap();
    }
    let s = open(&path);
    assert_eq!(
        top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(),
        Some("SMALL"),
        "rerank exact after compaction + close + reopen"
    );
}

/// (3) Degraded-fvec: truncate `fvec_<name>.bin` (so `len_rows() < arena_rows`)
/// before reopening. The load guard must detect the row-count mismatch and drop
/// the sidecar (degrade to quantized-only), NOT panic and NOT return empty
/// results. SQ8's exact-match query still quantizes to the correct code, so
/// top-1 stays correct even without the sidecar.
#[test]
fn truncated_fvec_degrades_to_quantized_not_panic_not_empty() {
    let path = fresh("test_sc_truncated");
    {
        let s = open(&path);
        s.create_collection(
            "c".into(),
            "m".into(),
            4,
            Some("Cosine".into()),
            Some("sq8".into()),
            None,
            Some(true),
        )
        .unwrap();
        add(&s, "A", vec![1.0, 0.0, 0.0, 0.0], "c");
        add(&s, "B", vec![0.0, 1.0, 0.0, 0.0], "c");
        add(&s, "C", vec![0.0, 0.0, 1.0, 0.0], "c");
        s.flush_index();
        s.save_state().unwrap();
    }

    // Truncate the sidecar to less than one full row's worth of bytes (dim=4 ->
    // 16 bytes/row). This forces len_rows() to disagree with the 3-row arena
    // without deleting the file outright (a distinct corruption mode from the
    // existing `missing_sidecar_degrades_not_empties` test in rerank_tests.rs).
    let fvec_path = Path::new(&path).join("fvec_c.bin");
    let full_len = fs::metadata(&fvec_path).unwrap().len();
    assert_eq!(full_len, 3 * 4 * 4, "sanity: 3 rows x dim 4 x 4 bytes");
    {
        let f = FsOpenOptions::new().write(true).open(&fvec_path).unwrap();
        f.set_len(20).unwrap(); // 1 row + 4 stray bytes: not a whole-row count
    }

    let s = open(&path);
    // Must not panic getting here, and must not silently return zero hits.
    let hit = top1(&s, vec![1.0, 0.0, 0.0, 0.0], "c");
    assert_eq!(
        hit.as_deref(),
        Some("A"),
        "degraded (quantized-only) search still finds the correct exact-match top-1, not empty"
    );

    let hit_c = top1(&s, vec![0.0, 0.0, 1.0, 0.0], "c");
    assert_eq!(
        hit_c.as_deref(),
        Some("C"),
        "degraded search returns non-empty, correct results for a second query"
    );
}

/// (3b) Degraded-fvec via corruption (garbage bytes, correct length): even if
/// the byte length still matches `arena_rows * dim * 4` but the content is
/// corrupt, the on-disk BQ-magnitude oracle at least must not panic. This does
/// not assert exact top-1 (corrupt rerank data can legitimately change the
/// ranking) — only that the engine degrades gracefully (no panic, non-empty).
#[test]
fn corrupted_fvec_same_length_does_not_panic_or_empty() {
    let path = fresh("test_sc_corrupt");
    {
        let s = open(&path);
        s.create_collection(
            "c".into(),
            "m".into(),
            4,
            Some("Cosine".into()),
            Some("sq8".into()),
            None,
            Some(true),
        )
        .unwrap();
        add(&s, "A", vec![1.0, 0.0, 0.0, 0.0], "c");
        add(&s, "B", vec![0.0, 1.0, 0.0, 0.0], "c");
        s.flush_index();
        s.save_state().unwrap();
    }

    let fvec_path = Path::new(&path).join("fvec_c.bin");
    let len = fs::metadata(&fvec_path).unwrap().len();
    // Overwrite with garbage of the SAME length (row count matches, so the
    // guard would normally adopt it) to prove the engine still returns a
    // sane, non-empty, non-panicking result even against bad row content.
    let garbage: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    {
        let mut f = FsOpenOptions::new().write(true).open(&fvec_path).unwrap();
        f.write_all(&garbage).unwrap();
    }

    let s = open(&path);
    let hits = s
        .hybrid_search(HybridSearchInput {
            query_vector: vec![1.0, 0.0, 0.0, 0.0],
            k: 2,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: Some("c".to_string()),
            ef_search: None,
            oversample: None,
        })
        .unwrap();
    assert!(
        !hits.is_empty(),
        "corrupted (but correctly-sized) sidecar must not produce empty results"
    );
}
