//! ADR--GENESISDB-BINCODE-EXIT — metadata snapshot format migration.
//!
//! `meta_<name>.bin` (per-collection vector metadata: arena_id/node_u32
//! mapping, timestamp, embedding_offset, gks_attributes, lang, cluster_id)
//! used to be a bare `bincode::serialize(&Vec<NodeMetadata>)` blob: no magic,
//! not self-describing, and bincode 1.3 is unmaintained (RUSTSEC-2025-0141).
//!
//! The magic-tagged format prepends a 4-byte magic and encodes the body with
//! `postcard` (maintained, serde-based, alloc-only): `GBP2` = the current
//! layout with epoch stamps (SPEC--EPOCH-HNSW §3.1), `GBP1` = the pre-epoch
//! layout, migrated on load with zeroed stamps. The read path sniffs the
//! magic first; its absence falls back to the existing (unchanged) legacy
//! bincode try-chain, so old snapshots still open. This file guards three
//! things:
//!   1. A DB saved by the current engine round-trips through postcard and is
//!      byte-tagged `GBP2` on disk.
//!   2. A hand-built *legacy* bincode blob (no magic, v1 field layout) still
//!      loads correctly, and gets rewritten as postcard on the very next
//!      `save_state()` — i.e. legacy snapshots migrate for free, with no
//!      dedicated migration step. (The GBP1 → GBP2 postcard migration is
//!      covered in tests/epoch_e2_tests.rs.)
//!   3. A blob that *has* the magic but a corrupt postcard body fails
//!      loudly (panics) instead of being silently misread as bincode garbage
//!      or silently dropped.

use genesis_block_native::{HybridSearchInput, NodeInput, NodeMetadata, OpenOptions, Storage};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const META_MAGIC: &[u8; 4] = b"GBP2";

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

fn add(s: &Storage, id: &str, emb: Vec<f64>, lang: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec!["Doc".to_string()],
        props: None,
        embedding: Some(emb),
        lang: Some(lang.to_string()),
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None, // -> "default"
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
        oversample: None,
    })
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .next()
}

fn meta_path(dir: &str) -> std::path::PathBuf {
    Path::new(dir).join("meta_default.bin")
}

/// (1) A snapshot saved by the current engine writes `meta_default.bin` in
/// the new format (`GBP1` magic prefix), and the metadata it carries (the
/// arena_id <-> node_u32 mapping that lets hybrid_search resolve a matched
/// embedding back to the right node) survives a full close/reopen round trip.
#[test]
fn postcard_roundtrip_survives_save_and_reload() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap().to_string();

    {
        let s = open(&path);
        add(&s, "BIG", vec![1.0, 1.0, 1.0, 1.0], "en");
        add(&s, "SMALL", vec![0.2, 0.2, 0.2, 0.2], "en");
        add(&s, "NEG", vec![-1.0, -1.0, -1.0, -1.0], "en");
        s.flush_index();
        s.save_state().unwrap();
    } // Drop -> also saves, but state is already durable above.

    let bytes = fs::read(meta_path(&path)).unwrap();
    assert!(
        bytes.len() > META_MAGIC.len(),
        "meta_default.bin must contain more than just the magic"
    );
    assert_eq!(
        &bytes[..META_MAGIC.len()],
        META_MAGIC,
        "a snapshot written by the current engine must be GBP2-tagged"
    );

    // Reopen fresh and confirm the arena_id/node_u32 mapping (the payload of
    // meta_default.bin) still resolves searches to the right nodes.
    let s2 = open(&path);
    assert_eq!(
        top1(&s2, vec![0.2, 0.2, 0.2, 0.2]).as_deref(),
        Some("SMALL"),
        "postcard-roundtripped metadata must still resolve the right node"
    );
    assert_eq!(
        top1(&s2, vec![1.0, 1.0, 1.0, 1.0]).as_deref(),
        Some("BIG"),
        "postcard-roundtripped metadata must still resolve the right node"
    );
    assert_eq!(
        top1(&s2, vec![-1.0, -1.0, -1.0, -1.0]).as_deref(),
        Some("NEG"),
        "postcard-roundtripped metadata must still resolve the right node"
    );
}

/// (2) Hand-build a *legacy* (no-magic) bincode blob out of the real
/// `Vec<NodeMetadata>` the engine just wrote (decoded from the postcard body
/// it produced), so the fixture is byte-accurate rather than a guess. Write
/// it over `meta_default.bin` (bypassing the magic prefix entirely) and
/// confirm: (a) it still loads and searches correctly through the unchanged
/// legacy bincode try-chain, and (b) the very next `save_state()` rewrites
/// the file back into the current `GBP1` + postcard format — legacy
/// snapshots migrate for free, no dedicated migration step required.
#[test]
fn legacy_bincode_blob_loads_and_rewrites_as_postcard() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap().to_string();

    {
        let s = open(&path);
        add(&s, "BIG", vec![1.0, 1.0, 1.0, 1.0], "en");
        add(&s, "SMALL", vec![0.2, 0.2, 0.2, 0.2], "en");
        s.flush_index();
        s.save_state().unwrap();
    }

    // Decode the real GBP2+postcard body the engine just wrote, so the
    // legacy fixture we build carries the actual arena_id/node_u32 mapping
    // (not hand-guessed values that could drift from reality).
    let gbp2_bytes = fs::read(meta_path(&path)).unwrap();
    assert_eq!(&gbp2_bytes[..META_MAGIC.len()], META_MAGIC);
    let real_meta: Vec<NodeMetadata> =
        postcard::from_bytes(&gbp2_bytes[META_MAGIC.len()..]).unwrap();
    assert_eq!(real_meta.len(), 2);

    // Re-encode that same data as a bare (no magic) bincode 1.x blob in the
    // v1 field layout (no epoch stamps) — this is exactly what every
    // `meta_<name>.bin` on disk looked like before the BINCODE-EXIT ADR
    // shipped. bincode encodes struct fields positionally, so a tuple in the
    // same order is byte-identical to the old struct.
    #[allow(clippy::type_complexity)]
    let v1_meta: Vec<(u32, u32, u64, u16, u64, Vec<u8>, String, u32)> = real_meta
        .iter()
        .map(|m| {
            (
                m.arena_id,
                m.node_u32,
                m.timestamp,
                m.vector_dim,
                m.embedding_offset,
                m.gks_attributes.clone(),
                m.lang.clone(),
                m.cluster_id,
            )
        })
        .collect();
    let legacy_bytes = bincode::serialize(&v1_meta).unwrap();
    assert_ne!(
        &legacy_bytes[..META_MAGIC.len().min(legacy_bytes.len())],
        META_MAGIC,
        "sanity: a real bincode blob must not accidentally start with GBP1"
    );
    fs::write(meta_path(&path), &legacy_bytes).unwrap();

    // Reopen: must load fine via the (unchanged) legacy bincode arm.
    let s2 = open(&path);
    assert_eq!(
        top1(&s2, vec![0.2, 0.2, 0.2, 0.2]).as_deref(),
        Some("SMALL"),
        "a legacy (no-magic) bincode meta blob must still load correctly"
    );
    assert_eq!(
        top1(&s2, vec![1.0, 1.0, 1.0, 1.0]).as_deref(),
        Some("BIG"),
        "a legacy (no-magic) bincode meta blob must still load correctly"
    );

    // Saving again must rewrite the file in the current format.
    s2.save_state().unwrap();
    let rewritten = fs::read(meta_path(&path)).unwrap();
    assert_eq!(
        &rewritten[..META_MAGIC.len()],
        META_MAGIC,
        "a legacy-loaded snapshot must be rewritten as GBP1+postcard on the \
         very next save_state(), with no dedicated migration step"
    );
}

/// (3) A blob that DOES carry the `GBP1` magic but whose body is not valid
/// postcard is corruption, not "an older format we don't recognize yet" —
/// the loader must fail loudly (panic) rather than either (a) silently
/// falling through to the legacy bincode arms, which could misparse the
/// bytes into bogus-but-structurally-valid garbage, or (b) silently
/// swallowing the error and loading empty/wrong metadata.
#[test]
#[should_panic(expected = "GBP1")]
fn corrupted_magic_fails_loudly_not_silently() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap().to_string();

    {
        let s = open(&path);
        add(&s, "n1", vec![1.0, 0.0, 0.0, 0.0], "en");
        s.flush_index();
        s.save_state().unwrap();
    }

    // GBP1 magic + a run of high-bit-set bytes: postcard's varint-prefixed
    // Vec<T> encoding can never terminate its length varint against this
    // buffer, so `postcard::from_bytes` is guaranteed to error, not to
    // "successfully" misparse it into a valid-looking result.
    let mut corrupt = META_MAGIC.to_vec();
    corrupt.extend(std::iter::repeat_n(0xFFu8, 32));
    fs::write(meta_path(&path), &corrupt).unwrap();

    let _ = Storage::open(OpenOptions {
        path,
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    });
}
