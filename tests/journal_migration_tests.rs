//! WP-1.2 migration — legacy JSONL WAL databases open, migrate one-way into the
//! framed journal (legacy WAL sealed as segment 0, kind=legacy_jsonl), reopen
//! identically, and a downgrade fails closed (SPEC--GENESISDB-JOURNAL-FORMAT-V1 §6,
//! ADR--GENESISDB-JOURNAL-HISTORY §4).

use genesis_block_native::{NodeInput, OpenOptions, Storage};
use serde_json::json;
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/jm_{}", env!("CARGO_TARGET_TMPDIR"), name);
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

fn add(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec!["Legacy".to_string()],
        props: Some(json!({"k": id})),
        embedding: Some(vec![0.5, 0.5, 0.0, 0.0]),
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .expect("add_node");
}

/// Build a legacy-layout DB fixture by materializing a legacy JSONL WAL the way
/// pre-WP-1.2 engines did. We synthesize it through the CURRENT engine (which
/// still knows how to read legacy lines) by moving the framed journal aside is
/// not possible — so instead this helper opens a fresh DB, then rewrites the
/// on-disk layout into the legacy shape: decode the active journal's frames back
/// into JSONL lines at genesis-graph.wal and delete the framed files. The line
/// payloads ARE the original SignedEvent JSON (frames wrap, never rewrite), so
/// this reconstructs a byte-faithful legacy WAL.
fn make_legacy_db(dir: &str, ids: &[&str]) {
    let s = open(dir);
    for id in ids {
        add(&s, id);
    }
    // Capture the framed active file WHILE OPEN — Drop runs save_state, which
    // folds the frames into a base segment and resets the active file.
    let root = Path::new(dir);
    let active = root.join("wal").join("active.gwal");
    let bytes = fs::read(&active).expect("active journal exists");
    drop(s);
    // Decode frames: [u32 len][u64 seq][u32 crc][payload] after the 14-byte header.
    let mut lines: Vec<Vec<u8>> = Vec::new();
    let mut off = 14usize;
    while off + 16 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) as usize;
        let payload_start = off + 16;
        if payload_start + len > bytes.len() {
            break;
        }
        lines.push(bytes[payload_start..payload_start + len].to_vec());
        off = payload_start + len;
    }
    assert!(!lines.is_empty(), "fixture must decode at least one frame");
    let mut jsonl: Vec<u8> = Vec::new();
    for line in lines {
        jsonl.extend_from_slice(&line);
        jsonl.push(b'\n');
    }
    fs::write(root.join("genesis-graph.wal"), jsonl).unwrap();
    // Remove every new-format artifact so the layout is purely legacy.
    let _ = fs::remove_dir_all(root.join("wal"));
    let _ = fs::remove_dir_all(root.join("journal"));
    for f in ["state.json", "nodes.bin", "edges.bin"] {
        let p = root.join(f);
        if p.exists() {
            let _ = fs::remove_file(p);
        }
    }
}

/// A legacy DB opens, its data is fully readable, and the layout is migrated:
/// legacy WAL sealed as segment 0 (kind=legacy_jsonl), fresh framed active file,
/// no genesis-graph.wal left behind.
#[test]
fn legacy_db_migrates_one_way() {
    let dir = fresh("migrate");
    make_legacy_db(&dir, &["l1", "l2", "l3"]);
    let s = open(&dir);
    for id in ["l1", "l2", "l3"] {
        assert!(s.get_u32(id).is_some(), "legacy node {id} readable");
    }
    drop(s);
    let root = Path::new(&dir);
    assert!(
        !root.join("genesis-graph.wal").exists(),
        "legacy WAL is sealed away, not left in place"
    );
    assert!(
        root.join("wal").join("active.gwal").exists(),
        "framed active journal exists after migration"
    );
    let seg0: Vec<_> = fs::read_dir(root.join("journal"))
        .expect("journal dir exists")
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "gseg"))
        .collect();
    assert!(!seg0.is_empty(), "legacy WAL sealed as a segment");
}

/// Migrated DB round-trips: writes after migration land in the framed journal
/// and everything (pre + post migration) survives reopen without a snapshot.
#[test]
fn migrated_db_reopens_identically() {
    let dir = fresh("roundtrip");
    make_legacy_db(&dir, &["m1", "m2"]);
    let s = open(&dir);
    add(&s, "m3");
    drop(s);
    let s2 = open(&dir);
    for id in ["m1", "m2", "m3"] {
        assert!(s2.get_u32(id).is_some(), "{id} survives post-migration reopen");
    }
}

/// Migration must be durable before the legacy file is removed: after
/// migration, a journal-only recovery (no snapshot) still sees legacy data.
#[test]
fn migration_preserves_data_without_snapshot() {
    let dir = fresh("nosnap");
    make_legacy_db(&dir, &["p1", "p2"]);
    let s = open(&dir);
    drop(s);
    let root = Path::new(&dir);
    for f in ["state.json", "nodes.bin", "edges.bin"] {
        let p = root.join(f);
        if p.exists() {
            let _ = fs::remove_file(p);
        }
    }
    let s2 = open(&dir);
    for id in ["p1", "p2"] {
        assert!(
            s2.get_u32(id).is_some(),
            "{id} recoverable from sealed legacy segment alone"
        );
    }
}

/// Downgrade fails closed: a DB whose on-disk schema_version is NEWER than this
/// engine must refuse to open with a SCHEMA_VERSION_UNSUPPORTED error — never
/// partial-read, never rewrite.
#[test]
fn newer_schema_version_fails_closed() {
    let dir = fresh("downgrade");
    let s = open(&dir);
    add(&s, "d1");
    s.save_state().unwrap();
    drop(s);
    let state_path = Path::new(&dir).join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    state["schema_version"] = json!(9999);
    fs::write(&state_path, state.to_string()).unwrap();
    let err = Storage::open(OpenOptions {
        path: dir.clone(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
    })
    .err()
    .expect("open must fail on newer on-disk schema");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("SCHEMA_VERSION_UNSUPPORTED"),
        "error must be the versioned fail-closed marker, got: {msg}"
    );
}
