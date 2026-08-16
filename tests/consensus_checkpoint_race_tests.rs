// A consensus commit acked under a concurrent checkpoint must survive a crash
// (RCA--PERSIST-SIGNED-CHECKPOINT-RACE).
//
// `submit_vote` was the only WAL-appending path that did not hold `commit_lock`
// — every other writer (add_node, add_edge, execute_batch, retract_edge,
// commit_transaction, reconcile_state, ...) does, and so do save_state and
// compact. It both mutates memory and appends to the WAL via `persist_signed`,
// so it could interleave inside save_state's build-payload -> checkpoint
// window:
//
//   save_state: build payload P from memory     (commit misses it)
//   save_state: collect nodes.bin from memory   (commit misses it)
//   submit_vote: mutate memory + append WAL, fsynced and ACKED -> Ok(true)
//   save_state: checkpoint truncates the WAL and rewrites it as P
//
// The acked event then exists in neither the snapshot nor the WAL. The write is
// still in RAM, so a graceful shutdown hides the loss (Drop runs save_state and
// re-snapshots it) — only a crash image exposes it. As in
// wal_tail_replay_tests, "crash" is a byte-copy of the DB dir taken while the
// Storage is still open.

use genesis_block_native::{Event, LogicalClock, NodeOutput, OpenOptions, Storage};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Proposals each test commits. Fixed so the assertions never depend on how
/// many commits happened to win the race on this machine.
const COMMITS: usize = 40;

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

fn mk_node(id: &str) -> NodeOutput {
    NodeOutput {
        id: id.to_string(),
        labels: vec!["USER".to_string()],
        props: serde_json::json!({}),
        impact: None,
        embedding: None,
        lang: None,
        valid_from: "2026-01-01T00:00:00Z".to_string(),
        valid_to: None,
        caused_by: None,
        expires_at: None,
        clock: LogicalClock {
            time: 1,
            peer_id: "p".to_string(),
        },
        collection: None,
    }
}

/// Propose + self-approve (an empty peer set means one vote is a strict
/// majority), returning true once the proposal is committed and persisted.
fn propose_and_commit(s: &Storage, id: &str) -> bool {
    let pid = match s.propose_consensus(Event::Node(mk_node(id)), vec![0u8; 64]) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let me = s.local_peer_id.clone();
    let sig = s.sign_vote(pid.clone(), true);
    s.submit_vote(pid, me, true, sig).unwrap_or(false)
}

/// Byte-copy the DB directory while the source Storage is still open: the disk
/// image a kill -9 would leave. `genesis.lock` is exclusively locked by the live
/// process (and recreated by open()); `temp_save/` is snapshot scratch.
fn crash_copy(src: &str, dst: &str) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap().flatten() {
        let name = entry.file_name();
        if name == "genesis.lock" || name == "temp_save" || entry.path().is_dir() {
            continue;
        }
        let mut attempts = 0;
        loop {
            match fs::copy(entry.path(), Path::new(dst).join(&name)) {
                Ok(_) => break,
                Err(e)
                    if attempts < 100
                        && matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33)) =>
                {
                    attempts += 1;
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(e) => panic!("crash_copy of {:?} failed: {e}", name),
            }
        }
    }
}

/// THE contract: every proposal `submit_vote` reported as committed is durable.
/// One thread commits proposals while another checkpoints; the crash image must
/// contain all of them.
#[test]
fn consensus_commits_survive_concurrent_save_state() {
    let path = fresh("test_consensus_ckpt_race");
    let crash = fresh("test_consensus_ckpt_race_img");
    let s = Arc::new(open(&path));

    // The voter does a FIXED amount of work and the checkpointer runs for
    // exactly as long as the voter is alive. Do NOT invert this (fixed
    // checkpoint count + "commit until stopped"): post-fix the voter contends
    // for commit_lock with save_state, so its throughput is machine-dependent
    // — that shape asserted on a race-derived count and failed CI at 8/10/12
    // commits while passing locally at 440.
    //
    // A committed proposal is reported durable, so record the id only once
    // submit_vote has returned true — every id in `committed` is a write the
    // engine promised to keep.
    let done = Arc::new(AtomicBool::new(false));
    let voter = {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        std::thread::spawn(move || {
            let mut committed = Vec::new();
            for i in 0..COMMITS {
                let id = format!("AXIOM{i}");
                if propose_and_commit(&s, &id) {
                    committed.push(id);
                }
            }
            done.store(true, Ordering::Release);
            committed
        })
    };

    // Checkpoint continuously underneath the voter, so every commit races a
    // checkpoint no matter how fast the machine is. Each save_state spans
    // build payload -> write snapshot -> checkpoint, and a commit landing in
    // that window is exactly the lost-write case.
    let mut checkpoints = 0;
    while !done.load(Ordering::Acquire) {
        s.save_state().unwrap();
        checkpoints += 1;
    }
    let committed = voter.join().unwrap();

    // Image the DB *before* any further save_state: the lost writes are still
    // in RAM, so another snapshot would silently heal them.
    crash_copy(&path, &crash);
    assert!(
        checkpoints > 0,
        "the checkpointer must have run at least once"
    );
    assert_eq!(
        committed.len(),
        COMMITS,
        "every proposal should reach quorum and commit"
    );

    let s2 = open(&crash);
    // Assert on `nodes`, not get_u32: projection_sync_on_open's WAL rescan
    // interns ids, so get_u32 resolves ids whose nodes were never recovered.
    let missing: Vec<&String> = committed
        .iter()
        .filter(|id| !s2.get_u32(id).is_some_and(|u| s2.nodes.contains_key(&u)))
        .collect();
    assert!(
        missing.is_empty(),
        "{} of {} consensus commits were acked (submit_vote returned true) but \
         lost on crash reopen — a checkpoint truncated the just-acked WAL append \
         away: {:?}",
        missing.len(),
        committed.len(),
        &missing[..missing.len().min(8)]
    );
}

/// The same commit path under a concurrent `compact()` (checkpoint without a
/// snapshot). Here there is no nodes.bin to accidentally carry the write, so
/// the WAL is the only thing standing between an acked commit and data loss,
/// and this asserts against the crashed WAL directly — a future loader change
/// cannot make it vacuously pass.
///
/// Both tests were verified to still detect the defect after the workload was
/// reshaped: with the `commit_lock` removed from `submit_vote` (and the
/// `persist_signed` debug_assert disabled so the durability assertion is what
/// fires), this loses 1 of 40 and the save_state test above loses 40 of 40.
#[test]
fn consensus_commits_survive_concurrent_compact() {
    let path = fresh("test_consensus_compact_race");
    let crash = fresh("test_consensus_compact_race_img");
    let s = Arc::new(open(&path));

    // Fixed voter workload + checkpointer running for the voter's lifetime;
    // see the note in the save_state test for why the inverse shape is not
    // portable across machines.
    let done = Arc::new(AtomicBool::new(false));
    let voter = {
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        std::thread::spawn(move || {
            let mut committed = Vec::new();
            for i in 0..COMMITS {
                let id = format!("CAX{i}");
                if propose_and_commit(&s, &id) {
                    committed.push(id);
                }
            }
            done.store(true, Ordering::Release);
            committed
        })
    };

    while !done.load(Ordering::Acquire) {
        s.compact().unwrap();
    }
    let committed = voter.join().unwrap();

    crash_copy(&path, &crash);
    assert_eq!(
        committed.len(),
        COMMITS,
        "every proposal should reach quorum and commit"
    );

    // Read the crash image's WAL directly: this pins the defect at the WAL
    // itself rather than at the loader, so a future loader change cannot make
    // this test vacuously pass.
    let wal = fs::read_to_string(Path::new(&crash).join("genesis-graph.wal")).unwrap_or_default();
    let absent: Vec<&String> = committed
        .iter()
        .filter(|id| !wal.contains(&format!("\"{id}\"")))
        .collect();
    assert!(
        absent.is_empty(),
        "{} of {} acked consensus commits are absent from the crashed WAL \
         (erased by a concurrent compact): {:?}",
        absent.len(),
        committed.len(),
        &absent[..absent.len().min(8)]
    );
}
