// WAL compaction / truncation (MARK XV P3, AUDIT--P33). The WAL stores raw
// `SignedEvent` lines and used to grow without bound — at 1M nodes it hit
// ~20 GB (5–51× the snapshot) and starved the snapshot save. The old
// rename-based `compact()` silently no-op'd on Windows because the writer thread
// holds `genesis-graph.wal` open for append and Windows refuses to rename over a
// live handle. The fix checkpoints THROUGH the writer thread's own descriptor
// (`set_len(0)` + rewrite), wired into `save_state()` after the snapshot is
// durable. These tests run against a long-lived open Storage, so they exercise
// exactly the open-handle path that defeated the old implementation.

use genesis_block_native::{EdgeInput, HybridSearchInput, NodeInput, OpenOptions, Storage};
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
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

fn add_node(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: Some("en".to_string()),
        valid_from: Some("2024-01-01T00:00:00Z".to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

fn add_edge(s: &Storage, id: &str, from: &str, to: &str) {
    s.add_edge(EdgeInput {
        id: Some(id.to_string()),
        from: from.to_string(),
        to: to.to_string(),
        rel: "REL".to_string(),
        props: None,
        valid_from: Some("2024-02-01T00:00:00Z".to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
}

/// WP-1.2: total journal bytes = active file + every sealed segment. The
/// boundedness contract now covers the whole journal directory, not one file.
fn wal_len(path: &str) -> u64 {
    let mut total = fs::metadata(Path::new(path).join("wal").join("active.gwal"))
        .map(|m| m.len())
        .unwrap_or(0);
    if let Ok(entries) = fs::read_dir(Path::new(path).join("journal")) {
        for entry in entries.flatten() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

/// `save_state()` checkpoints the WAL down to live state: a workload that
/// generates lots of dead history (retracted edges) leaves the WAL far smaller
/// than the running log, even though the Storage — and thus the writer thread's
/// append handle — stays open across the call. This is the Windows regression the
/// rename-based `compact()` silently failed: the WAL never shrank.
#[test]
fn save_state_shrinks_wal_through_open_handle() {
    let path = fresh("test_wal_compaction_shrink");
    let s = open(&path);

    // 100 live nodes ...
    for i in 0..100 {
        add_node(&s, &format!("N{i}"));
    }
    // ... and 400 edges that are then all retracted. Each add + each retract is a
    // WAL line, so the running log carries ~900 events while only 100 nodes (and
    // zero live edges) survive into the checkpoint.
    for i in 0..400 {
        add_edge(
            &s,
            &format!("E{i}"),
            &format!("N{}", i % 100),
            &format!("N{}", (i + 1) % 100),
        );
    }
    for i in 0..400 {
        s.retract_edge(format!("E{i}"), Some("2025-01-01T00:00:00Z".to_string()))
            .unwrap();
    }

    let before = wal_len(&path);
    assert!(before > 0, "WAL should be non-empty before the checkpoint");

    s.save_state().unwrap(); // snapshot + WAL checkpoint, Storage still open

    let after = wal_len(&path);
    assert!(
        after < before,
        "WAL must shrink after checkpoint (dead history dropped): {before} -> {after}"
    );

    // Repeated churn + save must NOT let the WAL grow without bound: add and
    // retract another 400 edges, checkpoint again, and the WAL stays bounded by
    // the live set (~the previous checkpoint size), not the cumulative history.
    for i in 400..800 {
        add_edge(
            &s,
            &format!("E{i}"),
            &format!("N{}", i % 100),
            &format!("N{}", (i + 1) % 100),
        );
    }
    for i in 400..800 {
        s.retract_edge(format!("E{i}"), Some("2025-01-01T00:00:00Z".to_string()))
            .unwrap();
    }
    s.save_state().unwrap();
    let after2 = wal_len(&path);
    assert!(
        after2 <= after * 2,
        "WAL stays bounded across repeated checkpoints: {after} then {after2}"
    );
}

/// A checkpointed WAL is a complete, parseable recovery source independent of the
/// snapshot (compact-to-live-state, not truncate-to-empty): after a checkpoint we
/// delete the snapshot manifest, forcing reopen to replay the WAL, and the live
/// graph + searchable vectors come back intact.
#[test]
fn checkpointed_wal_replays_to_identical_live_state() {
    let path = fresh("test_wal_compaction_replay");
    {
        let s = open(&path);
        for i in 0..50 {
            add_node(&s, &format!("N{i}"));
        }
        add_edge(&s, "keep", "N0", "N1");
        add_edge(&s, "drop", "N1", "N2");
        s.retract_edge("drop".to_string(), Some("2025-01-01T00:00:00Z".to_string()))
            .unwrap();
        s.flush_index();
        s.compact().unwrap(); // truncate WAL to live state through the open handle
    } // Drop also runs save_state() (snapshot + another checkpoint).

    // Force the WAL-replay load path: state.json must be GONE so try_load_state()
    // fails and open() replays the (compacted) WAL instead of instant-loading.
    //
    // This deletion is intentionally tolerant, and that does NOT mask a durability
    // bug — the snapshot is a best-effort instant-load optimization layered ON TOP
    // of the WAL. save_state() writes state.json LAST precisely so that an absent
    // snapshot is a valid state that falls back to the WAL (see save_state's swap
    // comment in src/lib.rs), and compact() carries the live graph into the WAL
    // itself. The real recovery invariant is asserted below, against the WAL.
    //
    // Under heavy parallel load on Windows two transient, non-bug states appear:
    //   * NotFound — the `.ok()`-swallowed best-effort snapshot rename did not land
    //     under AV/IO contention, so the WAL-replay path is ALREADY forced. Done.
    //   * sharing violation (os err 5/32/33) — the just-written file is briefly
    //     held open by the AV scanner / search indexer. It releases in a few ms;
    //     retry rather than fail. (The checkpoint+snapshot are already durable: the
    //     Storage's Drop ran save_state() — which blocks on compact()'s ack — and
    //     then joined the WAL writer thread before this scope exited, so there is no
    //     missing barrier to paper over here.)
    let sj = Path::new(&path).join("state.json");
    let mut attempts = 0;
    loop {
        match fs::remove_file(&sj) {
            Ok(_) => break,
            // Snapshot never landed (or already removed) ⇒ WAL replay already forced.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            // Transient Windows sharing violation on the freshly-written file.
            Err(e)
                if attempts < 100 && matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33)) =>
            {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => {
                let listing: Vec<String> = fs::read_dir(&path)
                    .map(|rd| {
                        rd.flatten()
                            .map(|e| e.file_name().to_string_lossy().into_owned())
                            .collect()
                    })
                    .unwrap_or_default();
                panic!(
                    "could not remove state.json to force WAL replay after {attempts} retries: \
                        kind={:?} raw_os={:?} exists={} dir={:?}",
                    e.kind(),
                    e.raw_os_error(),
                    sj.exists(),
                    listing
                );
            }
        }
    }

    let s2 = open(&path);
    s2.flush_index();
    for i in 0..50 {
        assert!(
            s2.get_u32(&format!("N{i}")).is_some(),
            "N{i} survived checkpoint + WAL replay"
        );
    }
    assert_eq!(
        s2.nodes.len(),
        50,
        "exactly the 50 live nodes are reconstructed from the WAL"
    );

    let hits: Vec<String> = s2
        .hybrid_search(HybridSearchInput {
            query_vector: vec![1.0, 0.0, 0.0, 0.0],
            k: 5,
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
        .collect();
    assert!(
        !hits.is_empty(),
        "embeddings survived checkpoint + WAL replay and stay searchable"
    );
}
