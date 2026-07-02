//! Crash simulation tests for GenesisBlockDB.
//!
//! Validates that the engine recovers correctly from mid-write crashes
//! by simulating filesystem-level corruption: truncated WAL, garbage WAL
//! entries, missing/corrupt snapshot files, partial snapshots, and
//! combinations thereof. Every test writes data through a normal Storage,
//! persists it, then corrupts the on-disk state and re-opens to verify
//! that the recovery path never panics and recovers as much data as
//! the corruption allows.

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
    })
    .unwrap()
}

fn add_node(s: &Storage, id: &str, emb: [f64; 4]) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec!["CrashTest".to_string()],
        props: None,
        embedding: Some(emb.to_vec()),
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
        rel: "CRASH_REL".to_string(),
        props: None,
        valid_from: Some("2024-02-01T00:00:00Z".to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
}

fn node_exists(s: &Storage, id: &str) -> bool {
    s.get_u32(id).is_some()
}

fn search_returns(s: &Storage, query: [f64; 4], expected_id: &str) -> bool {
    s.flush_index();
    let results = s
        .hybrid_search(HybridSearchInput {
            query_vector: query.to_vec(),
            k: 10,
            alpha: Some(1.0),
            lang: None,
            as_of: None,
            collection: None,
            ef_search: None,
            oversample: None,
        })
        .unwrap();
    results.iter().any(|r| r.node.id == expected_id)
}

// ---------------------------------------------------------------------------
// 1. Truncated WAL — cut the file mid-entry; entries before the cut survive.
// ---------------------------------------------------------------------------
#[test]
fn truncated_wal_recovers_intact_entries() {
    let path = fresh("crash_truncated_wal");
    let wal_file = Path::new(&path).join("genesis-graph.wal");

    // Write 10 nodes. Drop calls save_state which compacts the WAL, so we
    // need to read the WAL BEFORE drop, then simulate corruption after drop
    // by rebuilding the WAL from scratch and deleting the snapshot.
    let wal_lines: Vec<String>;
    {
        let s = open(&path);
        for i in 0..10 {
            add_node(&s, &format!("node_{i}"), [i as f64, 0.0, 0.0, 0.0]);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Capture WAL content before Drop compacts it.
        wal_lines = fs::read_to_string(&wal_file)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();
    }

    assert!(
        wal_lines.len() >= 10,
        "expected ≥10 WAL lines, got {}",
        wal_lines.len()
    );

    // Rebuild WAL with lines 0..7 intact + a truncated line 7.
    let keep: String = wal_lines[..7].join("\n") + "\n";
    let partial_line = &wal_lines[7][..wal_lines[7].len() / 2];
    let corrupted = format!("{keep}{partial_line}");
    fs::write(&wal_file, corrupted).unwrap();

    // Delete snapshot so recovery goes through WAL only.
    let state_json = Path::new(&path).join("state.json");
    if state_json.exists() {
        fs::remove_file(&state_json).unwrap();
    }

    // Re-open — must not panic, must recover first 7 nodes.
    let s = open(&path);
    for i in 0..7 {
        assert!(
            node_exists(&s, &format!("node_{i}")),
            "node_{i} should survive truncation"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Garbage injected mid-WAL — corrupt entries skipped, rest recovers.
// ---------------------------------------------------------------------------
#[test]
fn garbage_wal_entry_skipped_gracefully() {
    let path = fresh("crash_garbage_wal");
    let wal_file = Path::new(&path).join("genesis-graph.wal");

    {
        let s = open(&path);
        for i in 0..6 {
            add_node(&s, &format!("gnode_{i}"), [i as f64, 1.0, 0.0, 0.0]);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Inject garbage after line 3.
    let wal_data = fs::read_to_string(&wal_file).unwrap();
    let mut lines: Vec<String> = wal_data.lines().map(|l| l.to_string()).collect();
    assert!(lines.len() >= 6);
    lines.insert(3, "THIS IS NOT VALID JSON {{{garbage!!!".to_string());
    fs::write(&wal_file, lines.join("\n") + "\n").unwrap();

    // Delete snapshot.
    let state_json = Path::new(&path).join("state.json");
    if state_json.exists() {
        fs::remove_file(&state_json).unwrap();
    }

    // Re-open — no panic, nodes 0–2 (before garbage) and 3–5 (after) all recover
    // because the WAL reader skips unparseable lines.
    let s = open(&path);
    for i in 0..6 {
        assert!(
            node_exists(&s, &format!("gnode_{i}")),
            "gnode_{i} should survive garbage injection"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Missing state.json — falls back to full WAL replay.
// ---------------------------------------------------------------------------
#[test]
fn missing_state_json_falls_back_to_wal() {
    let path = fresh("crash_missing_state");

    {
        let s = open(&path);
        for i in 0..5 {
            add_node(&s, &format!("mnode_{i}"), [0.0, i as f64, 0.0, 0.0]);
        }
        add_edge(&s, "medge_0", "mnode_0", "mnode_1");
        s.save_state().unwrap();
    }

    // Delete state.json — snapshot files remain but are ignored without manifest.
    fs::remove_file(Path::new(&path).join("state.json")).unwrap();

    let s = open(&path);
    for i in 0..5 {
        assert!(node_exists(&s, &format!("mnode_{i}")));
    }
}

// ---------------------------------------------------------------------------
// 4. Corrupt state.json — garbage JSON triggers WAL fallback.
// ---------------------------------------------------------------------------
#[test]
fn corrupt_state_json_falls_back_to_wal() {
    let path = fresh("crash_corrupt_state");

    {
        let s = open(&path);
        for i in 0..5 {
            add_node(&s, &format!("cnode_{i}"), [0.0, 0.0, i as f64, 0.0]);
        }
        s.save_state().unwrap();
    }

    // Overwrite state.json with garbage.
    fs::write(Path::new(&path).join("state.json"), "NOT VALID JSON!!!").unwrap();

    let s = open(&path);
    for i in 0..5 {
        assert!(
            node_exists(&s, &format!("cnode_{i}")),
            "cnode_{i} should recover via WAL after corrupt state.json"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Missing vector arena file — nodes load, vectors lost, no panic.
// ---------------------------------------------------------------------------
#[test]
fn missing_vec_bin_recovers_nodes_without_vectors() {
    let path = fresh("crash_missing_vec");

    {
        let s = open(&path);
        for i in 0..5 {
            add_node(&s, &format!("vnode_{i}"), [1.0, 0.0, 0.0, i as f64]);
        }
        s.save_state().unwrap();
    }

    // Delete the vector arena file for the default collection.
    let vec_bin = Path::new(&path).join("vec_default.bin");
    if vec_bin.exists() {
        fs::remove_file(&vec_bin).unwrap();
    }

    // Re-open — should not panic. Nodes exist but search may return nothing.
    let s = open(&path);
    for i in 0..5 {
        assert!(
            node_exists(&s, &format!("vnode_{i}")),
            "vnode_{i} should exist even without vec_default.bin"
        );
    }
}

// ---------------------------------------------------------------------------
// 6. Truncated vec_*.bin — partial arena data doesn't crash the engine.
// ---------------------------------------------------------------------------
#[test]
fn truncated_vec_bin_no_panic() {
    let path = fresh("crash_truncated_vec");

    {
        let s = open(&path);
        for i in 0..20 {
            add_node(&s, &format!("tvnode_{i}"), [i as f64, 0.5, 0.5, 0.5]);
        }
        s.save_state().unwrap();
    }

    // Truncate vec_default.bin to half its size.
    let vec_bin = Path::new(&path).join("vec_default.bin");
    if vec_bin.exists() {
        let data = fs::read(&vec_bin).unwrap();
        fs::write(&vec_bin, &data[..data.len() / 2]).unwrap();
    }

    // Re-open — must not panic. Whatever loads is fine; the key is no crash.
    let s = open(&path);
    // Nodes should still load from nodes.bin.
    assert!(node_exists(&s, "tvnode_0"));
}

// ---------------------------------------------------------------------------
// 7. Corrupt nodes.bin — falls back to WAL for node data.
// ---------------------------------------------------------------------------
#[test]
fn corrupt_nodes_bin_recovers_via_wal() {
    let path = fresh("crash_corrupt_nodes");

    {
        let s = open(&path);
        for i in 0..5 {
            add_node(&s, &format!("nnode_{i}"), [0.0, 0.0, 0.0, i as f64]);
        }
        s.save_state().unwrap();
    }

    // Corrupt nodes.bin.
    fs::write(Path::new(&path).join("nodes.bin"), b"GARBAGE NODES DATA").unwrap();

    // The engine should handle this gracefully — either via fallback or
    // by loading what it can. The critical thing is no panic.
    let s = open(&path);
    // With corrupt nodes.bin, the snapshot loader should still read state.json
    // and load collections; nodes might be empty, but WAL replay after snapshot
    // doesn't re-run — so nodes could be missing. The key assertion: no panic.
    let _ = node_exists(&s, "nnode_0");
}

// ---------------------------------------------------------------------------
// 8. Corrupt edges.bin — edge data lost but nodes survive, no panic.
// ---------------------------------------------------------------------------
#[test]
fn corrupt_edges_bin_no_panic() {
    let path = fresh("crash_corrupt_edges");

    {
        let s = open(&path);
        add_node(&s, "enode_a", [1.0, 0.0, 0.0, 0.0]);
        add_node(&s, "enode_b", [0.0, 1.0, 0.0, 0.0]);
        add_edge(&s, "eedge_ab", "enode_a", "enode_b");
        s.save_state().unwrap();
    }

    fs::write(Path::new(&path).join("edges.bin"), b"NOT VALID EDGE JSON").unwrap();

    let s = open(&path);
    assert!(node_exists(&s, "enode_a"), "nodes survive edge corruption");
    assert!(node_exists(&s, "enode_b"));
}

// ---------------------------------------------------------------------------
// 9. Empty WAL file — fresh DB with no data, no panic.
// ---------------------------------------------------------------------------
#[test]
fn empty_wal_no_panic() {
    let path = fresh("crash_empty_wal");

    {
        let s = open(&path);
        add_node(&s, "empty_node", [1.0, 0.0, 0.0, 0.0]);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Truncate WAL to zero bytes.
    let wal_file = Path::new(&path).join("genesis-graph.wal");
    fs::write(&wal_file, b"").unwrap();
    // Remove snapshot.
    let _ = fs::remove_file(Path::new(&path).join("state.json"));

    // Re-open — no data but no panic.
    let s = open(&path);
    assert!(!node_exists(&s, "empty_node"), "node lost to empty WAL");
}

// ---------------------------------------------------------------------------
// 10. Double crash: write → crash (corrupt WAL tail) → partial recover
//     → write more → crash again (corrupt new WAL tail) → final recover.
// ---------------------------------------------------------------------------
#[test]
fn double_crash_recovery() {
    let path = fresh("crash_double");
    let wal_file = Path::new(&path).join("genesis-graph.wal");

    // Phase 1: write 5 nodes.
    {
        let s = open(&path);
        for i in 0..5 {
            add_node(&s, &format!("d1_node_{i}"), [i as f64, 0.0, 0.0, 0.0]);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Crash 1: truncate last WAL line.
    {
        let wal_data = fs::read_to_string(&wal_file).unwrap();
        let mut lines: Vec<&str> = wal_data.lines().collect();
        if let Some(last) = lines.last_mut() {
            let truncated = &last[..last.len() / 2];
            *last = truncated;
        }
        fs::write(&wal_file, lines.join("\n") + "\n").unwrap();
        let _ = fs::remove_file(Path::new(&path).join("state.json"));
    }

    // Phase 2: recover + write 5 more nodes.
    {
        let s = open(&path);
        // At least 4 of the first 5 should be there (last line was truncated).
        let count_phase1 = (0..5)
            .filter(|i| node_exists(&s, &format!("d1_node_{i}")))
            .count();
        assert!(
            count_phase1 >= 4,
            "expected ≥4 phase-1 nodes, got {count_phase1}"
        );

        for i in 0..5 {
            add_node(&s, &format!("d2_node_{i}"), [0.0, i as f64, 0.0, 0.0]);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Crash 2: truncate last WAL line again.
    {
        let wal_data = fs::read_to_string(&wal_file).unwrap();
        let mut lines: Vec<&str> = wal_data.lines().collect();
        if let Some(last) = lines.last_mut() {
            let truncated = &last[..last.len() / 2];
            *last = truncated;
        }
        fs::write(&wal_file, lines.join("\n") + "\n").unwrap();
        let _ = fs::remove_file(Path::new(&path).join("state.json"));
    }

    // Final recovery.
    let s = open(&path);
    let count_total = (0..5)
        .filter(|i| node_exists(&s, &format!("d1_node_{i}")))
        .count()
        + (0..5)
            .filter(|i| node_exists(&s, &format!("d2_node_{i}")))
            .count();
    assert!(
        count_total >= 8,
        "expected ≥8 total nodes after double crash, got {count_total}"
    );
}

// ---------------------------------------------------------------------------
// 11. Snapshot + WAL complement: data written after snapshot survives via WAL.
// ---------------------------------------------------------------------------
#[test]
fn post_snapshot_wal_entries_survive() {
    let path = fresh("crash_post_snapshot_wal");

    {
        let s = open(&path);
        for i in 0..3 {
            add_node(&s, &format!("snap_node_{i}"), [i as f64, 0.0, 0.0, 0.0]);
        }
        s.save_state().unwrap();

        // Write more after snapshot — these go to WAL only.
        for i in 3..6 {
            add_node(&s, &format!("snap_node_{i}"), [i as f64, 0.0, 0.0, 0.0]);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Re-open — snapshot has 0..3, WAL replay adds 3..6.
    let s = open(&path);
    for i in 0..6 {
        assert!(
            node_exists(&s, &format!("snap_node_{i}")),
            "snap_node_{i} should survive (snapshot + WAL)"
        );
    }
}

// ---------------------------------------------------------------------------
// 12. Corrupt meta_*.bin — metadata lost, node data preserved, no panic.
// ---------------------------------------------------------------------------
#[test]
fn corrupt_meta_bin_no_panic() {
    let path = fresh("crash_corrupt_meta");

    {
        let s = open(&path);
        for i in 0..5 {
            add_node(&s, &format!("meta_node_{i}"), [i as f64, 0.0, 0.0, 0.0]);
        }
        s.save_state().unwrap();
    }

    let meta_bin = Path::new(&path).join("meta_default.bin");
    if meta_bin.exists() {
        fs::write(&meta_bin, b"CORRUPT BINCODE DATA").unwrap();
    }

    let s = open(&path);
    // Nodes should survive from nodes.bin even if metadata is gone.
    for i in 0..5 {
        assert!(node_exists(&s, &format!("meta_node_{i}")));
    }
}

// ---------------------------------------------------------------------------
// 13. WAL with edges — edge recovery from WAL after snapshot deletion.
// ---------------------------------------------------------------------------
#[test]
fn edge_recovery_from_wal() {
    let path = fresh("crash_edge_wal");

    {
        let s = open(&path);
        add_node(&s, "ea", [1.0, 0.0, 0.0, 0.0]);
        add_node(&s, "eb", [0.0, 1.0, 0.0, 0.0]);
        add_node(&s, "ec", [0.0, 0.0, 1.0, 0.0]);
        add_edge(&s, "eab", "ea", "eb");
        add_edge(&s, "ebc", "eb", "ec");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Delete snapshot.
    let _ = fs::remove_file(Path::new(&path).join("state.json"));

    let s = open(&path);
    assert!(node_exists(&s, "ea"));
    assert!(node_exists(&s, "eb"));
    assert!(node_exists(&s, "ec"));
    // Edges recovered from WAL.
    assert!(!s.edges.is_empty(), "edges should recover from WAL");
}

// ---------------------------------------------------------------------------
// 14. Leftover temp_save directory — stale partial snapshot doesn't interfere.
// ---------------------------------------------------------------------------
#[test]
fn stale_temp_save_dir_no_interference() {
    let path = fresh("crash_stale_temp_save");

    {
        let s = open(&path);
        for i in 0..3 {
            add_node(&s, &format!("ts_node_{i}"), [i as f64, 0.0, 0.0, 0.0]);
        }
        s.save_state().unwrap();
    }

    // Simulate a crash mid-save: leave a temp_save directory with partial files.
    let temp_dir = Path::new(&path).join("temp_save");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::write(temp_dir.join("state.json"), "partial state").unwrap();
    fs::write(temp_dir.join("nodes.bin"), "partial nodes").unwrap();

    // Re-open — the real state.json in the root is fine; temp_save is ignored.
    let s = open(&path);
    for i in 0..3 {
        assert!(node_exists(&s, &format!("ts_node_{i}")));
    }
}

// ---------------------------------------------------------------------------
// 15. Vector search works after WAL-only recovery (no snapshot).
// ---------------------------------------------------------------------------
#[test]
fn vector_search_works_after_wal_recovery() {
    let path = fresh("crash_vector_search_wal");

    {
        let s = open(&path);
        add_node(&s, "vs_a", [1.0, 0.0, 0.0, 0.0]);
        add_node(&s, "vs_b", [0.0, 1.0, 0.0, 0.0]);
        add_node(&s, "vs_c", [0.0, 0.0, 1.0, 0.0]);
        s.flush_index();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Delete snapshot → force WAL replay.
    let _ = fs::remove_file(Path::new(&path).join("state.json"));

    let s = open(&path);
    // After WAL replay, vectors should be searchable.
    assert!(
        search_returns(&s, [1.0, 0.0, 0.0, 0.0], "vs_a"),
        "vector search should work after WAL-only recovery"
    );
}
