// Slice-0 durability regressions (RCA--SLICE0-DURABILITY, 2026-08-19 audit).
//
// Defect 2 — `retract_node` used to mutate memory only (no journal event):
//   a crash after the retraction ack resurrected the node and its edges on
//   replay. Now a NodeRetract frame is persisted BEFORE the memory removal.
//
// Defect 3 — a snapshot written without a journal cursor made the next open
//   skip replay entirely, silently dropping every write acked after the save.
//   Now the cursor-less branch replays the full journal on top of the load.
//
// Defect 4 — a base segment folded at a seq newer than the snapshot's cursor
//   (the fold-vs-state.json-rename crash window) used to be trusted anyway;
//   the base can only add, so deletions between the two were resurrected.
//   Now the stale snapshot is detected and skipped (journal-only recovery).
//
// A "crash" here is a byte-level copy of the DB directory taken while the
// original Storage is still open — the disk image a kill -9 would leave
// (dropping the Storage runs save_state() and would destroy the scenario).

use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
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

fn node_exists(s: &Storage, id: &str) -> bool {
    s.nodes.iter().any(|e| e.value().id == id)
}

fn edge_exists(s: &Storage, id: &str) -> bool {
    s.edges.iter().any(|e| e.value().id == id)
}

/// Byte-copy the DB directory while the source Storage may still be open —
/// the crash disk image. `genesis.lock` is exclusively locked by the live
/// process and recreated by open(); `temp_save/` is snapshot scratch.
fn crash_copy(src: &str, dst: &str) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap().flatten() {
        let name = entry.file_name();
        if name == "genesis.lock" || name == "temp_save" {
            continue;
        }
        if entry.path().is_dir() {
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

/// Defect 2, crash path: a retraction acked before a kill -9 must not be
/// resurrected by journal replay — neither the node nor its cascaded edges.
#[test]
fn retracted_node_and_edges_stay_deleted_across_crash_replay() {
    let src = fresh("slice0_retract_crash_src");
    let dst = fresh("slice0_retract_crash_dst");
    let s = open(&src);
    add_node(&s, "victim");
    add_node(&s, "survivor");
    add_edge(&s, "e1", "victim", "survivor");
    s.retract_node("victim").unwrap();

    // Crash image taken while the source is still open: no save_state, no
    // fold — recovery must come from the journal, which now carries the
    // NodeRetract frame after the victim's own upsert frames.
    crash_copy(&src, &dst);
    drop(s);

    let r = open(&dst);
    assert!(
        !node_exists(&r, "victim"),
        "retracted node resurrected by journal replay"
    );
    assert!(
        !edge_exists(&r, "e1"),
        "cascade-deleted edge resurrected by journal replay"
    );
    assert!(node_exists(&r, "survivor"), "unrelated node lost");
}

/// Defect 2, clean path: the retraction also survives a clean shutdown
/// (Drop -> save_state -> fold) and reopen.
#[test]
fn retracted_node_stays_deleted_after_clean_reopen() {
    let path = fresh("slice0_retract_clean");
    {
        let s = open(&path);
        add_node(&s, "victim");
        add_node(&s, "survivor");
        add_edge(&s, "e1", "victim", "survivor");
        s.retract_node("victim").unwrap();
    }
    let r = open(&path);
    assert!(!node_exists(&r, "victim"));
    assert!(!edge_exists(&r, "e1"));
    assert!(node_exists(&r, "survivor"));
}

/// Defect 4: a snapshot strictly older than a completed fold (the crash
/// window between `journal_fold` and the state.json rename) must not be
/// trusted — the old nodes.bin still holds a node that was retracted and
/// folded away, and base-segment replay can only add.
#[test]
fn stale_snapshot_older_than_fold_does_not_resurrect_deletes() {
    let path = fresh("slice0_stale_snapshot");
    let stash = fresh("slice0_stale_snapshot_stash");
    fs::create_dir_all(&stash).unwrap();
    {
        let s = open(&path);
        add_node(&s, "keeper");
        add_node(&s, "victim");
        s.save_state().unwrap();
        // Stash the pre-retraction snapshot (the "old state.json" half of the
        // crash window).
        for f in ["state.json", "nodes.bin", "edges.bin"] {
            fs::copy(Path::new(&path).join(f), Path::new(&stash).join(f)).unwrap();
        }
        s.retract_node("victim").unwrap();
        s.save_state().unwrap();
        // Drop folds once more; the newest base segment is now strictly newer
        // than the stashed snapshot's cursor.
    }
    // Simulate the crash-before-rename image: newest fold on disk, old
    // snapshot files restored over the new ones.
    for f in ["state.json", "nodes.bin", "edges.bin"] {
        fs::copy(Path::new(&stash).join(f), Path::new(&path).join(f)).unwrap();
    }
    let r = open(&path);
    assert!(
        !node_exists(&r, "victim"),
        "stale snapshot resurrected a node deleted before the fold"
    );
    assert!(
        node_exists(&r, "keeper"),
        "live node lost with the stale snapshot"
    );
}

/// Defect 3: a snapshot whose state.json carries NO journal cursor (the shape
/// a failed `build_compacted_wal` used to produce) must not disable replay —
/// writes acked after the snapshot live only in the journal.
#[test]
fn cursor_less_snapshot_does_not_drop_acked_writes() {
    let src = fresh("slice0_cursorless_src");
    let dst = fresh("slice0_cursorless_dst");
    let s = open(&src);
    add_node(&s, "in_snapshot");
    s.save_state().unwrap();
    add_node(&s, "post_snapshot"); // acked => journal-durable, not in snapshot
    crash_copy(&src, &dst);
    drop(s);

    // Strip the journal cursor from the crash image's state.json, recreating
    // the defect-3 snapshot shape on an otherwise-intact journal.
    let state_path = Path::new(&dst).join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert!(
        state.get("journal").is_some(),
        "fixture expectation: save_state now always writes the cursor"
    );
    state.as_object_mut().unwrap().remove("journal");
    fs::write(&state_path, state.to_string()).unwrap();

    let r = open(&dst);
    assert!(node_exists(&r, "in_snapshot"));
    assert!(
        node_exists(&r, "post_snapshot"),
        "acked post-snapshot write dropped by a cursor-less snapshot"
    );
}
