// WAL tail replay after an instant snapshot load (RCA--WAL-TAIL-REPLAY).
//
// Recovery used to be either/or: if `state.json` parsed, `try_load_state()`
// short-circuited and `genesis-graph.wal` was NEVER read — so any write acked
// AFTER the last `save_state()` (each append is flushed + fsynced + acked by
// the writer thread before `add_node` returns) was durably on disk yet
// silently dropped on the next open after a crash. That violates the "WAL is
// the only authority" durability contract.
//
// The fix records a `wal_frontier` (byte length + sha256 of the compacted WAL
// payload the snapshot was saved with) in `state.json`; open validates the
// on-disk WAL still starts with exactly that payload and replays only the tail
// beyond it. A mismatched prefix (crash between the state.json rename and the
// WAL checkpoint) means snapshot and WAL disagree — the WAL wins: the snapshot
// is skipped and the whole (compact-to-live-state) WAL is replayed.
//
// A "crash" here is a byte-level copy of the DB directory taken while the
// original Storage is still open: exactly the disk image a kill -9 would
// leave. (Dropping the Storage would run save_state() and fold the tail into
// a fresh snapshot, destroying the scenario.)

use genesis_block_native::{
    EdgeInput, HybridSearchInput, NeighborInput, NodeInput, OpenOptions, Storage,
};
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

/// Byte-copy the DB directory while the source Storage may still be open —
/// the crash disk image. `genesis.lock` is skipped (it is exclusively locked
/// by the live process and recreated by open()); `temp_save/` is a snapshot
/// scratch dir, never part of recovery. Reads retry briefly: on Windows a
/// freshly-written file can be held a few ms by the AV scanner / indexer.
fn crash_copy(src: &str, dst: &str) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap().flatten() {
        let name = entry.file_name();
        if name == "genesis.lock" || name == "temp_save" {
            continue;
        }
        if entry.path().is_dir() {
            // WP-1.2: the journal lives in wal/ and journal/ — a crash image
            // must include them (one level deep is the whole layout).
            let sub_dst = Path::new(dst).join(&name);
            fs::create_dir_all(&sub_dst).unwrap();
            for sub in fs::read_dir(entry.path()).unwrap().flatten() {
                if sub.path().is_file() {
                    copy_with_retry(&sub.path(), &sub_dst.join(sub.file_name()));
                }
            }
            continue;
        }
        copy_with_retry(&entry.path(), &Path::new(dst).join(&name));
    }
}

fn copy_with_retry(src: &Path, dst: &Path) {
    let mut attempts = 0;
    loop {
        match fs::copy(src, dst) {
            Ok(_) => break,
            Err(e)
                if attempts < 100 && matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33)) =>
            {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => panic!("crash_copy of {:?} failed: {e}", src),
        }
    }
}

fn default_arena_rows(s: &Storage) -> usize {
    s.collections
        .get("default")
        .map(|c| c.metadata.read().len())
        .unwrap_or(0)
}

fn search_ids(s: &Storage, k: u32) -> Vec<String> {
    s.hybrid_search(HybridSearchInput {
        query_vector: vec![1.0, 0.0, 0.0, 0.0],
        k,
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
    .collect()
}

/// THE contract: a write acked after the last snapshot survives a crash.
/// Snapshot 10 nodes, then add 5 more nodes + 1 edge (each acked, i.e. WAL
/// fsynced), crash, reopen — everything acked must be there.
#[test]
fn acked_writes_after_snapshot_survive_crash_reopen() {
    let path = fresh("test_wal_tail_replay_crash");
    let crash = fresh("test_wal_tail_replay_crash_img");

    let s = open(&path);
    for i in 0..10 {
        add_node(&s, &format!("PRE{i}"));
    }
    s.save_state().unwrap(); // snapshot + WAL checkpoint

    // Acked post-snapshot writes: add_node/add_edge return only after the WAL
    // writer thread has flushed + fsynced + acked the append.
    for i in 0..5 {
        add_node(&s, &format!("POST{i}"));
    }
    add_edge(&s, "POST-EDGE", "POST0", "POST1");

    crash_copy(&path, &crash);
    drop(s); // graceful shutdown of the ORIGINAL only; `crash` is the image

    let s2 = open(&crash);
    s2.flush_index();

    // NB: assert on the `nodes` map, not `get_u32` — the SQLite projection
    // rescan interns WAL ids on open, so `get_u32` can resolve an id whose
    // node was never actually recovered into the graph.
    for i in 0..10 {
        let u = s2.get_u32(&format!("PRE{i}"));
        assert!(
            u.is_some_and(|u| s2.nodes.contains_key(&u)),
            "snapshot node PRE{i} must load"
        );
    }
    for i in 0..5 {
        let u = s2.get_u32(&format!("POST{i}"));
        assert!(
            u.is_some_and(|u| s2.nodes.contains_key(&u)),
            "acked post-snapshot write POST{i} was durably fsynced to the WAL \
             but lost on crash reopen (either/or recovery skipped the WAL tail)"
        );
    }
    assert_eq!(
        s2.nodes.len(),
        15,
        "exactly PRE(10) + POST(5) — tail replay must not duplicate snapshot nodes"
    );
    let n = s2
        .neighbors(
            "POST0".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: None,
                limit: None,
            },
            false,
        )
        .unwrap();
    assert!(
        n.iter().any(|nb| nb.node.id == "POST1"),
        "acked post-snapshot edge POST-EDGE survives crash reopen"
    );
    // Vector staged exactly once per node: 10 snapshot rows + 5 tail rows.
    assert_eq!(
        default_arena_rows(&s2),
        15,
        "tail replay stages only post-snapshot vectors (no arena duplication)"
    );
    assert!(
        search_ids(&s2, 15).iter().any(|id| id.starts_with("POST")),
        "post-snapshot embeddings are searchable after crash reopen"
    );
}

/// Crash around the fold/snapshot boundary: the image carries the NEW
/// state.json (frontier F) but the OLD pre-fold active file (frames ≤ F) on
/// top of the folded base segment. WP-1.2's seq-filtered cursor makes this
/// safe by construction — frames ≤ F are skipped, nothing duplicates, no
/// stale-detection needed (SPEC §8.1: idempotent replay supersedes the old
/// prefix-hash SnapshotStale fallback).
#[test]
fn stale_wal_prefix_falls_back_to_full_replay() {
    let path = fresh("test_wal_tail_replay_stale");
    let crash = fresh("test_wal_tail_replay_stale_img");

    let s = open(&path);
    for i in 0..10 {
        add_node(&s, &format!("N{i}"));
    }
    // The active journal as it stood BEFORE the fold — what a crash right
    // before the checkpoint would leave alongside the new snapshot.
    let old_wal = fs::read(Path::new(&path).join("wal").join("active.gwal")).unwrap();
    s.save_state().unwrap();

    crash_copy(&path, &crash);
    drop(s);
    fs::write(Path::new(&crash).join("wal").join("active.gwal"), &old_wal).unwrap();

    let s2 = open(&crash);
    s2.flush_index();
    for i in 0..10 {
        assert!(
            s2.get_u32(&format!("N{i}")).is_some(),
            "N{i} recovered despite the pre-fold active file"
        );
    }
    assert_eq!(
        s2.nodes.len(),
        10,
        "seq-filtered recovery reconstructs exactly the live set"
    );
    assert_eq!(
        default_arena_rows(&s2),
        10,
        "frames at-or-below the frontier are skipped: vectors staged once, not twice"
    );
    assert!(
        !search_ids(&s2, 5).is_empty(),
        "embeddings searchable after crash recovery"
    );
}

/// Pre-upgrade snapshots carry no `wal_frontier` field. They still load — and
/// since RCA--SLICE0-DURABILITY defect 3, a cursor-less snapshot additionally
/// gets a full journal replay on top: without a cursor, tail replay is
/// impossible, and the journal may hold writes acked after the snapshot (the
/// exact shape a failed checkpoint used to produce). Graph state is LWW-
/// idempotent; the replay re-stages vector arena rows once (same documented
/// one-time tradeoff as the legacy `wal_frontier` branch — reclaimed by the
/// next index compaction).
#[test]
fn snapshot_without_frontier_still_loads() {
    let path = fresh("test_wal_tail_replay_legacy");
    {
        let s = open(&path);
        for i in 0..10 {
            add_node(&s, &format!("N{i}"));
        }
    } // Drop runs save_state(): snapshot + checkpoint.

    let sj = Path::new(&path).join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&sj).unwrap()).unwrap();
    if let Some(obj) = state.as_object_mut() {
        // Pre-frontier snapshot: neither the WP-1.2 `journal` cursor nor the
        // legacy `wal_frontier` byte cursor.
        obj.remove("journal");
        obj.remove("wal_frontier");
    }
    fs::write(&sj, state.to_string()).unwrap();

    let s2 = open(&path);
    s2.flush_index();
    assert_eq!(
        s2.nodes.len(),
        10,
        "legacy snapshot loads intact (LWW replay)"
    );
    assert_eq!(
        default_arena_rows(&s2),
        20,
        "cursor-less snapshot replays the journal on top: arena rows double once \
         (10 snapshot + 10 replayed), reclaimed by the next index compaction"
    );
}

/// Clean-shutdown reopen: the checkpoint ran, so the WAL tail is empty and the
/// frontier covers the whole file. Tail replay must be a no-op — no duplicated
/// nodes, no duplicated arena rows.
#[test]
fn graceful_reopen_replays_nothing() {
    let path = fresh("test_wal_tail_replay_graceful");
    {
        let s = open(&path);
        for i in 0..10 {
            add_node(&s, &format!("N{i}"));
        }
    } // Drop runs save_state().

    let s2 = open(&path);
    s2.flush_index();
    assert_eq!(s2.nodes.len(), 10);
    assert_eq!(
        default_arena_rows(&s2),
        10,
        "empty tail: nothing re-staged on a graceful reopen"
    );
}
