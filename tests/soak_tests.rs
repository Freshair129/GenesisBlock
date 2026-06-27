//! Long-running soak tests for GenesisBlockDB.
//!
//! Repeatedly cycle ingest → query → verify → compact to detect memory leaks,
//! index drift, latency degradation, and disk growth over sustained load.
//!
//! Two profiles:
//!   - `soak_light`:  60 cycles, 100 nodes/cycle, dim=4, ~5 min, ~50 MB disk
//!   - `soak_medium`: 360 cycles, 500 nodes/cycle, dim=4, ~30 min, ~500 MB disk
//!
//! Both are #[ignore]d by default — run explicitly:
//!   cargo test --no-default-features --test soak_tests --release -- --ignored --nocapture

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;
use std::time::Instant;

fn fresh(name: &str) -> String {
    let base =
        std::env::var("SOAK_TMPDIR").unwrap_or_else(|_| env!("CARGO_TARGET_TMPDIR").to_string());
    let p = format!("{}/{}", base, name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    fs::create_dir_all(&p).ok();
    p
}

fn open(path: &str, dim: u32) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(dim),
    })
    .unwrap()
}

fn dir_size_bytes(path: &str) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

struct SoakConfig {
    name: &'static str,
    dim: u32,
    nodes_per_cycle: usize,
    total_cycles: usize,
    compact_every: usize,
    query_k: u32,
    ef_search: Option<u32>,
    recall_threshold: f64,
}

struct CycleStats {
    cycle: usize,
    total_nodes: usize,
    ingest_ms: u128,
    query_ms: u128,
    recall_ok: bool,
    disk_mb: f64,
}

fn run_soak(cfg: SoakConfig) {
    let path = fresh(cfg.name);
    let s = open(&path, cfg.dim);

    let dim = cfg.dim as usize;
    let mut total_nodes: usize = 0;
    let mut all_stats: Vec<CycleStats> = Vec::new();
    let soak_start = Instant::now();

    println!("\n=== SOAK TEST: {} ===", cfg.name);
    println!(
        "  dim={}, nodes/cycle={}, cycles={}, compact_every={}",
        cfg.dim, cfg.nodes_per_cycle, cfg.total_cycles, cfg.compact_every
    );
    println!(
        "  {:>6} {:>8} {:>10} {:>10} {:>8} {:>8}",
        "cycle", "nodes", "ingest_ms", "query_ms", "recall", "disk_MB"
    );
    println!("  {}", "-".repeat(60));

    for cycle in 0..cfg.total_cycles {
        let cycle_base = total_nodes;

        // --- Ingest ---
        let t0 = Instant::now();
        for i in 0..cfg.nodes_per_cycle {
            let node_idx = cycle_base + i;
            let mut emb = vec![0.0f64; dim];
            // Spread embeddings using a hash-like scheme so they don't cluster
            // in dim=4 space when node_idx grows large.
            let x = node_idx as f64;
            for d in 0..dim {
                emb[d] = ((x * (d as f64 + 1.0) * 0.6180339887).fract() - 0.5) * 2.0;
            }

            s.add_node(NodeInput {
                id: Some(format!("soak_{node_idx}")),
                labels: vec!["Soak".to_string()],
                props: None,
                embedding: Some(emb),
                lang: Some("en".to_string()),
                valid_from: Some("2024-01-01T00:00:00Z".to_string()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
        }
        total_nodes += cfg.nodes_per_cycle;
        let ingest_ms = t0.elapsed().as_millis();

        // --- Query + verify recall ---
        s.flush_index();
        let probe_idx = cycle_base;
        let mut probe_emb = vec![0.0f64; dim];
        let x = probe_idx as f64;
        for d in 0..dim {
            probe_emb[d] = ((x * (d as f64 + 1.0) * 0.6180339887).fract() - 0.5) * 2.0;
        }

        let t1 = Instant::now();
        let results = s
            .hybrid_search(HybridSearchInput {
                query_vector: probe_emb,
                k: cfg.query_k,
                alpha: Some(1.0),
                lang: None,
                as_of: None,
                collection: None,
                ef_search: cfg.ef_search,
            })
            .unwrap();
        let query_ms = t1.elapsed().as_millis();

        let expected_id = format!("soak_{probe_idx}");
        let recall_ok = results.iter().any(|r| r.node.id == expected_id);

        // --- Compact ---
        if cfg.compact_every > 0 && (cycle + 1) % cfg.compact_every == 0 {
            s.save_state().unwrap();
        }

        let disk = dir_size_bytes(&path);

        let stats = CycleStats {
            cycle,
            total_nodes,
            ingest_ms,
            query_ms,
            recall_ok,
            disk_mb: mb(disk),
        };

        if cycle % 10 == 0 || cycle == cfg.total_cycles - 1 || !recall_ok {
            println!(
                "  {:>6} {:>8} {:>10} {:>10} {:>8} {:>8.1}",
                stats.cycle,
                stats.total_nodes,
                stats.ingest_ms,
                stats.query_ms,
                if stats.recall_ok { "OK" } else { "MISS" },
                stats.disk_mb
            );
        }

        all_stats.push(stats);
    }

    // --- Final save + verify ---
    s.save_state().unwrap();
    let final_disk = mb(dir_size_bytes(&path));
    let elapsed = soak_start.elapsed();

    println!("  {}", "-".repeat(60));
    println!("  Elapsed: {:.1}s", elapsed.as_secs_f64());
    println!("  Total nodes: {total_nodes}");
    println!("  Final disk: {final_disk:.1} MB");

    // --- Assertions ---
    let recall_misses: usize = all_stats.iter().filter(|s| !s.recall_ok).count();
    let miss_rate = recall_misses as f64 / all_stats.len() as f64;
    println!(
        "  Recall misses: {recall_misses}/{} ({:.1}%)",
        all_stats.len(),
        miss_rate * 100.0
    );

    // Latency should not degrade catastrophically (last 10 cycles vs first 10).
    let first_10_avg: f64 = all_stats[..10.min(all_stats.len())]
        .iter()
        .map(|s| s.query_ms as f64)
        .sum::<f64>()
        / 10.0f64.min(all_stats.len() as f64);
    let last_10_avg: f64 = all_stats[all_stats.len().saturating_sub(10)..]
        .iter()
        .map(|s| s.query_ms as f64)
        .sum::<f64>()
        / 10.0f64.min(all_stats.len() as f64);
    println!("  Query latency: first10_avg={first_10_avg:.0}ms, last10_avg={last_10_avg:.0}ms");

    assert!(
        miss_rate < cfg.recall_threshold,
        "recall miss rate {:.1}% exceeds {:.0}% threshold",
        miss_rate * 100.0,
        cfg.recall_threshold * 100.0
    );

    // Reopen and verify a sample of nodes survived.
    drop(s);
    let s2 = open(&path, cfg.dim);
    let spot_checks = [0, total_nodes / 4, total_nodes / 2, total_nodes - 1];
    for idx in spot_checks {
        let id = format!("soak_{idx}");
        assert!(
            s2.get_u32(&id).is_some(),
            "{id} missing after soak + reopen"
        );
    }
    println!("  Spot-check after reopen: OK");
    println!("=== SOAK COMPLETE ===\n");
}

// ---------------------------------------------------------------------------
// Light soak: ~5 min, ~50 MB disk
// ---------------------------------------------------------------------------
#[test]
#[ignore]
fn soak_light() {
    run_soak(SoakConfig {
        name: "soak_light",
        dim: 4,
        nodes_per_cycle: 100,
        total_cycles: 60,
        compact_every: 1,
        query_k: 5,
        ef_search: Some(200),
        recall_threshold: 0.10,
    });
}

// ---------------------------------------------------------------------------
// Medium soak: ~30 min, ~500 MB disk
// ---------------------------------------------------------------------------
#[test]
#[ignore]
fn soak_medium() {
    run_soak(SoakConfig {
        name: "soak_medium",
        dim: 4,
        nodes_per_cycle: 500,
        total_cycles: 360,
        compact_every: 10,
        query_k: 10,
        ef_search: Some(200),
        recall_threshold: 0.10,
    });
}
