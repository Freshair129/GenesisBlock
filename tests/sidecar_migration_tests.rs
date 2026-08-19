// P0-T7 — migration / back-compat guard for pre-existing resident-era
// `fvec_<name>.bin` sidecar files (ADR--GENESISDB-ONDISK-RERANK-SIDECAR).
//
// The on-disk `SidecarReader` (src/lib.rs) reads `fvec_<name>.bin` as flat
// little-endian f32 rows, row `d_id` at byte offset `d_id * dim * 4` — this is
// IDENTICAL to the byte layout the old *resident* build wrote (and read back
// into a `Vec<f32>`) before P0. So "migration" is really "open as-is": there
// is no format change, no rewrite step, no version marker in the file itself.
//
// This test proves that by hand-writing a legacy-shaped `fvec_<name>.bin` +
// matching arena/meta/state.json fixture directly to disk (bypassing the
// current `Storage` write path entirely, so we are not just testing "the
// writer can read its own output") and then opening it through the normal
// `Storage::open` load path. A reranked search must return the exact top-1,
// proving the on-disk reader adopts a legacy-format file with zero rewrite.
//
// We build the fixture in two stages for realism and precision:
//   1. Use `Storage` normally to create a quantized+rerank collection and add
//      nodes, then `save_state()` — this exercises the real arena/meta/state
//      serializers so the fixture's non-sidecar files are authentic, not
//      hand-rolled guesses that could drift from the real format.
//   2. Overwrite the resulting `fvec_<name>.bin` with a byte buffer we
//      construct by hand (flat le-f32, no header, row d_id at d_id*dim*4) —
//      this is the actual legacy-format artifact under test, written
//      independently of `SidecarReader::write_rows` / the streaming snapshot
//      writer in `save_state()`, so a bug in either of those could not make
//      this test pass by accident.
//
// Guarded:
//   (a) A hand-written legacy fvec (row-for-row identical to what the old
//       resident build would have produced) opens transparently and reranks
//       correctly — no migration/rewrite occurs.
//   (b) The row-count guard (`len_rows() == arena_rows`) accepts a
//       byte-exact legacy file (no off-by-one on `dim*4`).
//   (c) A truncated legacy file (one short row at the tail) still degrades
//       to quantized-only search — never adopted, never panics, never empty.

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs::{self, File};
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

/// Hand-encode the legacy `fvec_<name>.bin` byte layout: flat little-endian
/// f32, row `d_id` at byte `d_id * dim * 4`, no header, no footer, no
/// version marker — exactly what the pre-P0 resident build wrote to disk
/// (and this build's `write_rows`/`save_state` still write, by design: the
/// format never changed).
fn encode_legacy_fvec(rows: &[Vec<f32>]) -> Vec<u8> {
    let mut buf = Vec::new();
    for row in rows {
        for &v in row {
            buf.extend_from_slice(&v.to_le_bytes());
        }
    }
    buf
}

/// Build a quantized+rerank collection with two known vectors via the real
/// `Storage` API and `save_state()` it to disk, producing an authentic
/// arena/meta/state.json fixture. Returns the db path. The caller then
/// overwrites `fvec_c.bin` with a hand-built legacy buffer.
fn build_fixture(path: &str) {
    let s = open(path);
    s.create_collection(
        "c".into(),
        "m".into(),
        4,
        Some("L2".into()),
        Some("bq".into()),
        None,
        Some(true), // rerank
    )
    .unwrap();
    // BIG and SMALL share sign pattern -> identical BQ codes; only exact f32
    // rerank distinguishes them. NEG has a distinct sign -> far under any
    // encoding. Same construction as tests/rerank_tests.rs::bq_rerank_distinguishes_magnitude.
    add(&s, "BIG", vec![1.0, 1.0, 1.0, 1.0], "c");
    add(&s, "SMALL", vec![0.2, 0.2, 0.2, 0.2], "c");
    add(&s, "NEG", vec![-1.0, -1.0, -1.0, -1.0], "c");
    s.flush_index();
    s.save_state().unwrap();
}

/// (a) + (b): a hand-written legacy-format `fvec_<name>.bin`, byte-exact and
/// row-count-exact, opens transparently through `Storage::open` and reranks
/// correctly — proving the on-disk reader adopts a resident-era file as-is,
/// with zero rewrite/migration step, and the `len_rows() == arena_rows`
/// guard accepts it (no off-by-one on `dim*4`).
#[test]
fn legacy_fvec_opens_and_reranks_exact() {
    let path = fresh("test_sidecar_migration_legacy");
    build_fixture(&path);

    // Arena order is insertion order: BIG=0, SMALL=1, NEG=2 (dim=4).
    let legacy_rows: Vec<Vec<f32>> = vec![
        vec![1.0, 1.0, 1.0, 1.0],     // BIG  (d_id 0)
        vec![0.2, 0.2, 0.2, 0.2],     // SMALL(d_id 1)
        vec![-1.0, -1.0, -1.0, -1.0], // NEG (d_id 2)
    ];
    let legacy_bytes = encode_legacy_fvec(&legacy_rows);
    assert_eq!(
        legacy_bytes.len(),
        3 * 4 * 4,
        "legacy fixture: 3 rows x dim 4 x 4 bytes/f32"
    );

    let fvec_path = Path::new(&path).join("fvec_c.bin");
    // Sanity: save_state() already wrote a same-shaped file; replacing it
    // with our independently hand-built buffer proves this test is not just
    // reopening the writer's own untouched output.
    assert!(
        fvec_path.exists(),
        "save_state must have written fvec_c.bin"
    );
    {
        let mut f = File::create(&fvec_path).unwrap();
        f.write_all(&legacy_bytes).unwrap();
        f.flush().unwrap();
    }
    assert_eq!(
        fs::metadata(&fvec_path).unwrap().len(),
        legacy_bytes.len() as u64,
        "hand-written legacy fvec must be exactly rows*dim*4 bytes on disk"
    );

    // Reopen: Storage::open must load the legacy file transparently (no
    // rewrite, no version negotiation) and pass the row-count guard.
    let s = open(&path);
    assert_eq!(
        top1(&s, vec![0.2, 0.2, 0.2, 0.2], "c").as_deref(),
        Some("SMALL"),
        "exact f32 rerank over a hand-written legacy-format fvec must still \
         distinguish magnitude that the BQ codes alone cannot — this is only \
         possible if the on-disk reader actually adopted the legacy file"
    );
    assert_eq!(
        top1(&s, vec![1.0, 1.0, 1.0, 1.0], "c").as_deref(),
        Some("BIG"),
        "legacy fvec reranks the BIG query correctly too"
    );
}

/// (c): a truncated legacy fvec (one short/partial row at the tail, as if a
/// resident-era snapshot were captured mid-write pre-P0) fails the
/// `len_rows() == arena_rows` guard and must degrade to quantized-only
/// search — never adopted half-written, never panics, never returns empty.
#[test]
fn truncated_legacy_fvec_degrades_not_adopted() {
    let path = fresh("test_sidecar_migration_truncated");
    build_fixture(&path);

    // Full legacy rows for BIG and SMALL, but NEG's row is short by one f32
    // (simulates a truncated/corrupt legacy snapshot) so total row count no
    // longer divides evenly into 3 whole rows of dim 4 f32s.
    let mut legacy_bytes =
        encode_legacy_fvec(&[vec![1.0, 1.0, 1.0, 1.0], vec![0.2, 0.2, 0.2, 0.2]]);
    legacy_bytes.extend_from_slice(&(-1.0f32).to_le_bytes());
    legacy_bytes.extend_from_slice(&(-1.0f32).to_le_bytes());
    legacy_bytes.extend_from_slice(&(-1.0f32).to_le_bytes());
    // (deliberately omit the 4th f32 of NEG's row -> partial trailing row)

    let fvec_path = Path::new(&path).join("fvec_c.bin");
    {
        let mut f = File::create(&fvec_path).unwrap();
        f.write_all(&legacy_bytes).unwrap();
        f.flush().unwrap();
    }
    // 2 whole rows + a partial row -> len_rows() (integer division) == 2,
    // which does not equal arena_rows == 3, so the guard must reject it.
    assert_eq!(
        legacy_bytes.len() / (4 * 4),
        2,
        "fixture sanity: byte length must floor-divide to 2 whole rows, not 3"
    );

    let s = open(&path);
    // BQ alone cannot distinguish BIG/SMALL (identical codes) -- with the
    // sidecar correctly dropped, search must still return *some* correct,
    // non-empty top-1 for a query that quantization alone resolves.
    assert_eq!(
        top1(&s, vec![-1.0, -1.0, -1.0, -1.0], "c").as_deref(),
        Some("NEG"),
        "truncated legacy fvec must degrade to quantized-only search (never \
         panic, never empty) — NEG is quantization-distinguishable so this \
         must still resolve correctly without the sidecar"
    );
}
