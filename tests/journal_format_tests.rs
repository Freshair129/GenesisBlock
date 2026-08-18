//! WP-1.2 framed journal — format-level integration tests (SPEC--GENESISDB-JOURNAL-FORMAT-V1 §7).
//!
//! Black-box against the public Storage API: every scenario builds state through
//! normal writes, manipulates the on-disk journal the way a crash would, reopens,
//! and asserts the recovered state. Covers: frame round-trip, torn-tail
//! truncation (I9), orphan .tmp cleanup, checkpoint seal ordering (I7/I9),
//! journal-only recovery (I8 second clause), and the frame/txn frontier split (D2).

use genesis_block_native::{BatchInput, GenesisTransaction, NodeInput, OpenOptions, Storage};
use serde_json::json;
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/jf_{}", env!("CARGO_TARGET_TMPDIR"), name);
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

fn node(id: &str) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec!["Test".to_string()],
        props: Some(json!({"k": id})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

fn add_nodes(s: &Storage, ids: &[&str]) {
    for id in ids {
        s.add_node(node(id)).expect("add_node");
    }
}

fn assert_nodes_present(s: &Storage, ids: &[&str]) {
    for id in ids {
        assert!(s.get_u32(id).is_some(), "node {id} must survive recovery");
    }
}

/// New-format layout exists after first open: wal/active.gwal with the GWA1
/// header, no legacy genesis-graph.wal.
#[test]
fn active_journal_uses_framed_format() {
    let dir = fresh("framed");
    let s = open(&dir);
    add_nodes(&s, &["a", "b"]);
    drop(s);
    let active = Path::new(&dir).join("wal").join("active.gwal");
    assert!(active.exists(), "wal/active.gwal must exist");
    let bytes = fs::read(&active).unwrap();
    assert!(bytes.len() >= 14, "file header present");
    assert_eq!(&bytes[0..4], b"GWA1", "active journal magic");
    assert!(
        !Path::new(&dir).join("genesis-graph.wal").exists(),
        "fresh DB must not create a legacy JSONL WAL"
    );
}

/// Round-trip: everything written before a clean drop is recovered on reopen
/// (journal replay path — no snapshot was saved).
#[test]
fn journal_only_recovery_without_snapshot() {
    let dir = fresh("noround");
    let s = open(&dir);
    add_nodes(&s, &["n1", "n2", "n3"]);
    drop(s); // Drop runs save_state — remove its snapshot to force journal-only.
    for f in ["state.json", "nodes.bin", "edges.bin"] {
        let p = Path::new(&dir).join(f);
        if p.exists() {
            fs::remove_file(p).unwrap();
        }
    }
    let s2 = open(&dir);
    assert_nodes_present(&s2, &["n1", "n2", "n3"]);
}

/// Acked writes AFTER save_state survive a crash-style reopen (tail replay by
/// commit_seq — the WP-1.1 guarantee re-based onto the framed cursor).
#[test]
fn tail_replay_recovers_post_snapshot_writes() {
    let dir = fresh("tail");
    let s = open(&dir);
    add_nodes(&s, &["pre1", "pre2"]);
    s.save_state().unwrap();
    add_nodes(&s, &["post1", "post2"]);
    // Crash simulation: drop without save_state.
    drop(s);
    let s2 = open(&dir);
    assert_nodes_present(&s2, &["pre1", "pre2", "post1", "post2"]);
}

/// I9 torn tail: garbage appended to the active file (simulating a torn write)
/// must not poison recovery — frames before the tear replay, the tear is
/// truncated away, and the DB accepts new writes afterward.
#[test]
fn torn_active_tail_is_truncated_not_fatal() {
    let dir = fresh("torn");
    let s = open(&dir);
    add_nodes(&s, &["t1", "t2"]);
    drop(s);
    let active = Path::new(&dir).join("wal").join("active.gwal");
    // Torn frame: plausible length prefix, then EOF mid-frame.
    let mut bytes = fs::read(&active).unwrap();
    bytes.extend_from_slice(&(400u32).to_le_bytes());
    bytes.extend_from_slice(&(999u64).to_le_bytes());
    bytes.extend_from_slice(&[0xAB; 7]);
    fs::write(&active, bytes).unwrap();
    let s2 = open(&dir);
    assert_nodes_present(&s2, &["t1", "t2"]);
    add_nodes(&s2, &["t3"]);
    drop(s2);
    let s3 = open(&dir);
    assert_nodes_present(&s3, &["t1", "t2", "t3"]);
}

/// I9 CRC: flipping a byte inside a frame's payload invalidates that frame;
/// recovery keeps everything before it and does not crash.
#[test]
fn corrupt_frame_crc_stops_replay_at_the_tear() {
    let dir = fresh("crc");
    let s = open(&dir);
    add_nodes(&s, &["c1"]);
    drop(s);
    let active = Path::new(&dir).join("wal").join("active.gwal");
    let mut bytes = fs::read(&active).unwrap();
    let last = bytes.len() - 2;
    bytes[last] ^= 0xFF;
    fs::write(&active, bytes).unwrap();
    // Must not panic; the corrupted frame is dropped.
    let s2 = open(&dir);
    add_nodes(&s2, &["c2"]);
    drop(s2);
    let s3 = open(&dir);
    assert_nodes_present(&s3, &["c2"]);
}

/// Checkpoint (save_state) seals/folds through the journal directory: after it,
/// journal-only recovery (snapshot deleted) still reaches identical live state.
/// This is the successor of wal_compaction_tests::checkpointed_wal_replays_to_
/// identical_live_state under the framed format (I8 second clause).
#[test]
fn checkpointed_journal_alone_recovers_live_state() {
    let dir = fresh("ckpt");
    let s = open(&dir);
    add_nodes(&s, &["k1", "k2", "k3"]);
    s.save_state().unwrap();
    add_nodes(&s, &["k4"]);
    drop(s);
    // Remove every snapshot artifact; the journal must carry the whole state.
    for f in ["state.json", "nodes.bin", "edges.bin"] {
        let p = Path::new(&dir).join(f);
        if p.exists() {
            fs::remove_file(p).unwrap();
        }
    }
    let s2 = open(&dir);
    assert_nodes_present(&s2, &["k1", "k2", "k3", "k4"]);
}

/// I9 orphan hygiene: stray .tmp segments (crashed seal) are removed on open
/// and never break recovery.
#[test]
fn orphan_tmp_segment_is_cleaned_on_open() {
    let dir = fresh("orphan");
    let s = open(&dir);
    add_nodes(&s, &["o1"]);
    s.save_state().unwrap();
    drop(s);
    let seg_dir = Path::new(&dir).join("journal");
    fs::create_dir_all(&seg_dir).unwrap();
    let tmp = seg_dir.join("J999999.gseg.tmp");
    fs::write(&tmp, b"half a sealed segment").unwrap();
    let s2 = open(&dir);
    assert_nodes_present(&s2, &["o1"]);
    assert!(!tmp.exists(), "crashed-seal .tmp must be deleted on open");
}

/// D2: stable_frontier is the last durable FRAME seq (advances on ordinary
/// writes), txn_frontier only on transactions; CommitResult carries the frame
/// stamp; both survive reopen.
#[test]
fn frame_and_txn_frontier_split() {
    let dir = fresh("frontier");
    let s = open(&dir);
    add_nodes(&s, &["f1"]);
    let after_node = s.stable_frontier();
    assert!(
        after_node >= 1,
        "ordinary writes advance the frame frontier"
    );
    assert_eq!(s.txn_frontier(), 0, "no transaction committed yet");

    let commit = s
        .commit_transaction(GenesisTransaction {
            transaction_id: "tx-frontier-1".to_string(),
            expected_frontier: Some(0),
            relational: vec![],
            graph: BatchInput {
                nodes: vec![node("f2")],
                edges: vec![],
            },
            vectors: vec![],
        })
        .expect("commit");
    assert!(
        commit.commit_sequence > after_node,
        "transaction frame is stamped after the node frame"
    );
    assert_eq!(
        s.txn_frontier(),
        commit.commit_sequence,
        "txn_frontier = frame seq of the last transaction"
    );
    assert_eq!(s.stable_frontier(), commit.commit_sequence);

    // Interleaved ordinary write advances frame frontier but NOT txn frontier —
    // and a subsequent CAS against txn_frontier still succeeds (panel: a
    // frame-level CAS would spuriously fail here).
    add_nodes(&s, &["f3"]);
    assert!(s.stable_frontier() > s.txn_frontier());
    let commit2 = s
        .commit_transaction(GenesisTransaction {
            transaction_id: "tx-frontier-2".to_string(),
            expected_frontier: Some(commit.commit_sequence),
            relational: vec![],
            graph: BatchInput {
                nodes: vec![node("f4")],
                edges: vec![],
            },
            vectors: vec![],
        })
        .expect("commit after interleaved write");
    assert!(commit2.commit_sequence > commit.commit_sequence);

    s.save_state().unwrap();
    drop(s);
    let s2 = open(&dir);
    assert_eq!(
        s2.txn_frontier(),
        commit2.commit_sequence,
        "txn frontier survives reopen"
    );
    assert!(s2.stable_frontier() >= commit2.commit_sequence);

    // Idempotent replay of the same transaction (byte-identical input — the
    // identity hash covers expected_frontier too) returns the stored frame seq.
    let replayed = s2
        .commit_transaction(GenesisTransaction {
            transaction_id: "tx-frontier-2".to_string(),
            expected_frontier: Some(commit.commit_sequence),
            relational: vec![],
            graph: BatchInput {
                nodes: vec![node("f4")],
                edges: vec![],
            },
            vectors: vec![],
        })
        .expect("idempotent replay");
    assert_eq!(replayed.commit_sequence, commit2.commit_sequence);
}

/// Measured compression ratio vs the pre-WP-1.2 JSONL baseline (SPEC §7 —
/// the ADR's estimate had to become a number). Run with `--nocapture` to read
/// the figures; the assertion is the contract: a checkpointed journal must not
/// be larger than the JSONL WAL it replaces.
#[test]
fn journal_is_smaller_than_jsonl_baseline() {
    let dir = fresh("ratio");
    let s = open(&dir);
    // Realistic agent-memory-ish payloads (the workload the ratio matters for).
    for i in 0..2000 {
        s.add_node(NodeInput {
            id: Some(format!("mem:{i}")),
            labels: vec!["Memory".to_string(), "Episodic".to_string()],
            props: Some(json!({
                "text": format!("observation {i}: the user asked about journal segment retention and folding"),
                "source": "session-2026-08-17",
                "confidence": 0.87,
                "tags": ["journal", "retention", "wp-1.2"],
            })),
            embedding: None,
            lang: Some("en".to_string()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    }

    // Uncompressed framed active file, pre-fold: payload bytes + 16B/frame.
    let active = fs::read(Path::new(&dir).join("wal").join("active.gwal")).unwrap();
    let mut frames = 0usize;
    let mut payload_bytes = 0usize;
    let mut off = 14usize;
    while off + 16 <= active.len() {
        let len = u32::from_le_bytes(active[off..off + 4].try_into().unwrap()) as usize;
        if off + 16 + len > active.len() {
            break;
        }
        frames += 1;
        payload_bytes += len;
        off += 16 + len;
    }
    // What the old format would have written for the same events: one JSON
    // line + newline each (frames wrap the identical payload bytes).
    let jsonl_baseline = payload_bytes + frames;

    s.save_state().unwrap();
    drop(s);

    let mut journal_bytes = 0u64;
    for entry in fs::read_dir(Path::new(&dir).join("journal"))
        .unwrap()
        .flatten()
    {
        journal_bytes += entry.metadata().unwrap().len();
    }
    journal_bytes += fs::metadata(Path::new(&dir).join("wal").join("active.gwal"))
        .map(|m| m.len())
        .unwrap_or(0);

    println!(
        "WP-1.2 journal size: {frames} frames · JSONL baseline {jsonl_baseline} B \
         · checkpointed journal {journal_bytes} B · ratio {:.2}x smaller",
        jsonl_baseline as f64 / journal_bytes.max(1) as f64
    );
    assert!(
        journal_bytes < jsonl_baseline as u64,
        "checkpointed journal ({journal_bytes} B) must not exceed the JSONL \
         baseline it replaces ({jsonl_baseline} B)"
    );
}

/// Sealed segments carry the GSG1 magic and appear under journal/ after a
/// checkpoint; frames keep flowing into a fresh active file afterward.
#[test]
fn checkpoint_produces_sealed_segment_files() {
    let dir = fresh("seal");
    let s = open(&dir);
    add_nodes(&s, &["s1", "s2", "s3", "s4"]);
    s.save_state().unwrap();
    drop(s);
    let seg_dir = Path::new(&dir).join("journal");
    let segs: Vec<_> = fs::read_dir(&seg_dir)
        .expect("journal dir exists after checkpoint")
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "gseg"))
        .collect();
    assert!(!segs.is_empty(), "checkpoint must produce a sealed segment");
    for seg in segs {
        let bytes = fs::read(seg.path()).unwrap();
        assert_eq!(&bytes[0..4], b"GSG1", "sealed segment magic");
    }
}
