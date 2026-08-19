// WP-1.3 retention profiles (ADR--GENESISDB-JOURNAL-HISTORY D3).
//
// `frontier_only` (default): fold at every checkpoint — the WP-1.2 interim
//   behavior, unchanged. Forfeits tx-time history; capabilities discloses it.
// `full`: never fold at a checkpoint — history accumulates in the journal
//   (sealed segments + active file) and journal-only recovery replays it.
// `budget:<bytes>`: fold only when sealed history exceeds the budget — the
//   bounded-disk contract, retaining up to that much history between folds.
//   The active-file seal threshold derives from the budget so small budgets
//   actually seal (and therefore actually trip).
//
// Also under test: unknown profiles fail open() loudly (no silent-default
// trap), and the horizon/retention disclosure on query_ir_capabilities
// (ADR I6 "horizon honesty" — previously computed but unreachable from any
// surface).

use genesis_block_native::{NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn opts(path: &str, retention: Option<&str>) -> OpenOptions {
    OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: retention.map(|r| r.to_string()),
    }
}

fn open_with(path: &str, retention: Option<&str>) -> Storage {
    Storage::open(opts(path, retention)).unwrap()
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

fn node_exists(s: &Storage, id: &str) -> bool {
    s.nodes.iter().any(|e| e.value().id == id)
}

/// Journal segment counts by kind, from filenames: `B*` = base (fold output),
/// `J*` = sealed history.
fn seg_counts(path: &str) -> (usize, usize) {
    let dir = Path::new(path).join("journal");
    let (mut base, mut history) = (0, 0);
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.ends_with(".gseg") {
                continue;
            }
            if name.starts_with('B') {
                base += 1;
            } else {
                history += 1;
            }
        }
    }
    (base, history)
}

fn delete_snapshot(path: &str) {
    for f in ["state.json", "nodes.bin", "edges.bin"] {
        let p = Path::new(path).join(f);
        if p.exists() {
            fs::remove_file(p).unwrap();
        }
    }
}

#[test]
fn unknown_retention_profile_fails_open_loudly() {
    let path = fresh("wp13_bad_profile");
    let err = Storage::open(opts(&path, Some("budgett:123")))
        .err()
        .expect(
            "a typo'd retention profile must fail open, not silently degrade to a different policy",
        );
    assert!(format!("{err}").contains("retention profile"));
    let path2 = fresh("wp13_bad_budget");
    let err2 = Storage::open(opts(&path2, Some("budget:0")))
        .err()
        .expect("budget:0 is not a usable budget");
    assert!(format!("{err2}").contains("budget"));
}

#[test]
fn frontier_only_default_folds_every_checkpoint() {
    let path = fresh("wp13_default");
    let s = open_with(&path, None);
    add_node(&s, "n1");
    s.save_state().unwrap();
    let (base, history) = seg_counts(&path);
    assert!(base >= 1, "default profile must fold at the checkpoint");
    assert_eq!(history, 0, "default profile retains no history segments");
    // Horizon honesty: after a fold the horizon is the fold frontier (> 0)
    // and capabilities discloses the forfeit.
    assert!(s.history_horizon() > 0);
    let caps = s.query_ir_capabilities();
    assert_eq!(caps["temporal"]["retention_profile"], "frontier_only");
    assert_eq!(caps["temporal"]["tx_time_retention"], "none");
    assert_eq!(
        caps["temporal"]["history_horizon"].as_u64().unwrap(),
        s.history_horizon()
    );
}

#[test]
fn full_profile_never_folds_and_recovers_journal_only() {
    let path = fresh("wp13_full");
    {
        let s = open_with(&path, Some("full"));
        for i in 0..20 {
            add_node(&s, &format!("N{i}"));
        }
        s.save_state().unwrap();
        let (base, _) = seg_counts(&path);
        assert_eq!(base, 0, "full profile must not fold at a checkpoint");
        assert_eq!(s.history_horizon(), 0, "no fold ⇒ no history discarded");
        let caps = s.query_ir_capabilities();
        assert_eq!(caps["temporal"]["retention_profile"], "full");
        assert_eq!(caps["temporal"]["tx_time_retention"], "full");
    } // Drop -> save_state: still no fold.
    let (base, _) = seg_counts(&path);
    assert_eq!(base, 0, "clean shutdown under full must not fold either");

    // The journal alone (active file, no base segment) is a complete
    // recovery source — I8 with history retained.
    delete_snapshot(&path);
    let s = open_with(&path, Some("full"));
    for i in 0..20 {
        assert!(
            node_exists(&s, &format!("N{i}")),
            "N{i} lost on journal-only recovery under full retention"
        );
    }
}

#[test]
fn budget_profile_folds_only_when_exceeded() {
    let path = fresh("wp13_budget");
    let budget: u64 = 64 * 1024;
    let spec = format!("budget:{budget}");
    let s = open_with(&path, Some(&spec));

    // Below budget: checkpoints must NOT fold.
    add_node(&s, "early");
    s.save_state().unwrap();
    let (base, _) = seg_counts(&path);
    assert_eq!(base, 0, "budget profile folded below its budget");

    // Churn until sealed history exceeds the budget (the derived seal
    // threshold guarantees segments appear long before the 64 MiB default).
    let mut i = 0u32;
    while s.sealed_history_bytes() <= budget {
        add_node(&s, &format!("churn-{i}"));
        i += 1;
        assert!(i < 200_000, "budget never tripped — sealing is broken");
    }
    let (_, history) = seg_counts(&path);
    assert!(
        history >= 1,
        "sealed history segments must exist before the fold (that IS the retained history)"
    );

    // Now over budget: the checkpoint folds and disk is reclaimed.
    s.save_state().unwrap();
    let (base, _) = seg_counts(&path);
    assert!(base >= 1, "budget exceeded but the checkpoint did not fold");
    assert!(
        s.sealed_history_bytes() <= budget,
        "fold must bring sealed history back under the budget (got {} > {budget})",
        s.sealed_history_bytes()
    );
    assert!(s.history_horizon() > 0);
    assert_eq!(
        s.query_ir_capabilities()["temporal"]["tx_time_retention"],
        "windowed"
    );

    // Everything (incl. pre-fold writes) still recoverable journal-only.
    drop(s);
    delete_snapshot(&path);
    let s = open_with(&path, Some(&spec));
    assert!(node_exists(&s, "early"));
    assert!(node_exists(&s, "churn-0"));
}
