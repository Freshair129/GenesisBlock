//! Long-running soak tests for GenesisBlockDB.
//!
//! Repeatedly cycle ingest → query → verify → compact to detect memory leaks,
//! index drift, latency degradation, and disk growth over sustained load.
//!
//! Profiles:
//!   - `soak_light`:  60 cycles, 100 nodes/cycle, dim=4, ~5 min, ~50 MB disk
//!   - `soak_medium`: 360 cycles, 500 nodes/cycle, dim=4, ~30 min, ~500 MB disk
//!   - `soak_heavy`:  duration-bounded, fully env-configurable. Used by the
//!     Independent Benchmark Suite (`benchmark/`) for the smoke / 1h / 12h soaks.
//!     It loops by wall-clock until `SOAK_DURATION_SEC` and, when
//!     `SOAK_RESULT_JSON` is set, writes a machine-readable metrics file that the
//!     `benchmark/assemble_result.py` step folds into the public `result.json`.
//!
//! All are #[ignore]d by default — run explicitly:
//!   cargo test --no-default-features --test soak_tests --release -- --ignored --nocapture
//!
//! `soak_heavy` configuration (all optional; defaults shown):
//!   SOAK_DURATION_SEC   = 43200   wall-clock target (12h); loop stops after this
//!   SOAK_MAX_CYCLES     = 0       hard cycle cap (0 = unlimited); marks `interrupted`
//!   SOAK_NODES_PER_CYCLE= 500
//!   SOAK_COMPACT_EVERY  = 20
//!   SOAK_QUERY_K        = 10
//!   SOAK_EF_SEARCH      = 200
//!   SOAK_DIM            = 16
//!   SOAK_RECALL_THRESH  = 0.10    max tolerated recall-miss rate before `pass=false`
//!   SOAK_BENCHMARK_ID   = soak_heavy
//!   SOAK_PROFILE_LABEL  = (defaults to SOAK_BENCHMARK_ID)
//!   SOAK_RESULT_JSON    = (unset) path to write the metrics JSON; no file if unset
//!
//! The metrics JSON is intentionally *partial*: it carries only what the engine
//! itself can observe (cycles, nodes, disk, latency percentiles, recall, reopen
//! timing, timestamps). Host/env/commit/peak-RAM fields are filled by the
//! wrapper so the benchmark code never self-reports its own environment.

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

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

/// Linear-interpolation-free nearest-rank percentile over an already-sorted slice.
fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

fn env_str(k: &str, d: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| d.to_string())
}

fn env_usize(k: &str, d: usize) -> usize {
    std::env::var(k)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(d)
}

fn env_f64(k: &str, d: f64) -> f64 {
    std::env::var(k)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(d)
}

struct SoakConfig {
    name: String,
    /// Stable id recorded in the metrics JSON (e.g. `soak_heavy_12h`).
    benchmark_id: String,
    dim: u32,
    nodes_per_cycle: usize,
    /// Cycle cap. With `duration_target` set, this is a *safety* upper bound
    /// (0 = unlimited); otherwise it is the exact number of cycles to run.
    total_cycles: usize,
    /// When `Some`, loop by wall-clock until this many seconds elapse instead of
    /// running a fixed cycle count.
    duration_target: Option<Duration>,
    compact_every: usize,
    query_k: u32,
    ef_search: Option<u32>,
    recall_threshold: f64,
    /// Optional path for the machine-readable metrics JSON.
    result_json: Option<String>,
}

impl SoakConfig {
    /// Fixed-cycle profile (legacy light/medium).
    #[allow(clippy::too_many_arguments)]
    fn fixed(
        name: &str,
        dim: u32,
        nodes_per_cycle: usize,
        total_cycles: usize,
        compact_every: usize,
        query_k: u32,
        ef_search: Option<u32>,
        recall_threshold: f64,
    ) -> Self {
        SoakConfig {
            name: name.to_string(),
            benchmark_id: name.to_string(),
            dim,
            nodes_per_cycle,
            total_cycles,
            duration_target: None,
            compact_every,
            query_k,
            ef_search,
            recall_threshold,
            result_json: None,
        }
    }
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
    let path = fresh(&cfg.name);
    let s = open(&path, cfg.dim);

    let dim = cfg.dim as usize;
    let mut total_nodes: usize = 0;
    let mut all_stats: Vec<CycleStats> = Vec::new();
    // ISO-8601 start stamp for the machine-readable report. chrono is a normal
    // (non-optional) dep so it is available even under --no-default-features.
    let ts_start = chrono::Utc::now();
    let soak_start = Instant::now();

    let duration_label = cfg
        .duration_target
        .map(|d| format!("{}s wall-clock", d.as_secs()))
        .unwrap_or_else(|| format!("{} cycles", cfg.total_cycles));

    println!("\n=== SOAK TEST: {} ({}) ===", cfg.name, cfg.benchmark_id);
    println!(
        "  dim={}, nodes/cycle={}, target={}, compact_every={}",
        cfg.dim, cfg.nodes_per_cycle, duration_label, cfg.compact_every
    );
    println!(
        "  {:>6} {:>8} {:>10} {:>10} {:>8} {:>8}",
        "cycle", "nodes", "ingest_ms", "query_ms", "recall", "disk_MB"
    );
    println!("  {}", "-".repeat(60));

    // Either fixed-cycle or duration-bounded. `interrupted` is set when a hard
    // cycle cap (SOAK_MAX_CYCLES) cut a duration run short before its target.
    let mut interrupted = false;
    let mut cycle = 0usize;
    loop {
        // --- Stop conditions ---
        match cfg.duration_target {
            Some(target) => {
                if soak_start.elapsed() >= target {
                    break;
                }
                if cfg.total_cycles > 0 && cycle >= cfg.total_cycles {
                    interrupted = true;
                    break;
                }
            }
            None => {
                if cycle >= cfg.total_cycles {
                    break;
                }
            }
        }

        let cycle_base = total_nodes;

        // --- Ingest ---
        let t0 = Instant::now();
        for i in 0..cfg.nodes_per_cycle {
            let node_idx = cycle_base + i;
            let mut emb = vec![0.0f64; dim];
            // Spread embeddings using a hash-like scheme so they don't cluster
            // in dim=4 space when node_idx grows large.
            let x = node_idx as f64;
            for (d, val) in emb.iter_mut().enumerate() {
                *val = ((x * (d as f64 + 1.0) * 0.6180339887).fract() - 0.5) * 2.0;
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
        for (d, val) in probe_emb.iter_mut().enumerate() {
            *val = ((x * (d as f64 + 1.0) * 0.6180339887).fract() - 0.5) * 2.0;
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
        if cfg.compact_every > 0 && (cycle + 1).is_multiple_of(cfg.compact_every) {
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

        // Heartbeat: every 10 cycles, on a recall miss, and (for fixed runs) the
        // last cycle. A 12h run prints ~hundreds of lines — enough to show
        // liveness in the raw log without flooding it.
        let last_fixed = cfg.duration_target.is_none() && cycle == cfg.total_cycles - 1;
        if cycle.is_multiple_of(10) || last_fixed || !recall_ok {
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
        cycle += 1;
    }

    // --- Final save ---
    s.save_state().unwrap();
    let final_disk = mb(dir_size_bytes(&path));
    let elapsed = soak_start.elapsed();

    println!("  {}", "-".repeat(60));
    println!("  Elapsed: {:.1}s", elapsed.as_secs_f64());
    println!("  Total nodes: {total_nodes}");
    println!("  Final disk: {final_disk:.1} MB");

    // --- Aggregate metrics ---
    let recall_misses: usize = all_stats.iter().filter(|s| !s.recall_ok).count();
    let miss_rate = if all_stats.is_empty() {
        1.0
    } else {
        recall_misses as f64 / all_stats.len() as f64
    };
    println!(
        "  Recall misses: {recall_misses}/{} ({:.1}%)",
        all_stats.len(),
        miss_rate * 100.0
    );

    let mut q_sorted: Vec<f64> = all_stats.iter().map(|s| s.query_ms as f64).collect();
    q_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut i_sorted: Vec<f64> = all_stats.iter().map(|s| s.ingest_ms as f64).collect();
    i_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let (q_p50, q_p95, q_p99) = (
        pct(&q_sorted, 50.0),
        pct(&q_sorted, 95.0),
        pct(&q_sorted, 99.0),
    );
    let (i_p50, i_p95) = (pct(&i_sorted, 50.0), pct(&i_sorted, 95.0));
    println!(
        "  Query latency ms: p50={q_p50:.0} p95={q_p95:.0} p99={q_p99:.0} | Ingest/cycle ms: p50={i_p50:.0} p95={i_p95:.0}"
    );

    // --- Reopen verification (timed) ---
    drop(s);
    let reopen_t = Instant::now();
    let s2 = open(&path, cfg.dim);
    let reopen_load_sec = reopen_t.elapsed().as_secs_f64();
    let spot_checks = [
        0,
        total_nodes / 4,
        total_nodes / 2,
        total_nodes.saturating_sub(1),
    ];
    let reopen_ok = !all_stats.is_empty()
        && spot_checks
            .iter()
            .all(|&idx| s2.get_u32(&format!("soak_{idx}")).is_some());
    println!(
        "  Reopen: load {reopen_load_sec:.3}s, spot-check {}",
        if reopen_ok { "OK" } else { "FAIL" }
    );

    let recall_ok = miss_rate < cfg.recall_threshold;
    let pass = recall_ok && reopen_ok && !all_stats.is_empty();
    let ts_end = chrono::Utc::now();

    // --- Machine-readable metrics (partial; env/commit/RAM filled by wrapper) ---
    if let Some(out) = cfg.result_json.as_ref() {
        let metrics = serde_json::json!({
            "benchmark_id": cfg.benchmark_id,
            "project": "GenesisBlockDB",
            "timestamp_start": ts_start.to_rfc3339(),
            "timestamp_end": ts_end.to_rfc3339(),
            "duration_sec": elapsed.as_secs(),
            "interrupted": interrupted,
            "config": {
                "profile": cfg.name,
                "duration_target_sec": cfg.duration_target.map(|d| d.as_secs()),
                "dim": cfg.dim,
                "nodes_per_cycle": cfg.nodes_per_cycle,
                "compact_every": cfg.compact_every,
                "query_k": cfg.query_k,
                "ef_search": cfg.ef_search,
                "recall_threshold": cfg.recall_threshold,
            },
            "results": {
                "pass": pass,
                "cycles": all_stats.len(),
                "total_nodes": total_nodes,
                "final_disk_mb": final_disk,
                // peak_ram_mb is observed by the wrapper (sysinfo is bins-only and
                // not linked under --no-default-features); null until merged.
                "peak_ram_mb": serde_json::Value::Null,
                "recall_miss_rate": miss_rate,
                "query_latency_p50_ms": q_p50,
                "query_latency_p95_ms": q_p95,
                "query_latency_p99_ms": q_p99,
                // Per-cycle batch ingest wall time (nodes_per_cycle nodes/cycle).
                "ingest_latency_p50_ms": i_p50,
                "ingest_latency_p95_ms": i_p95,
                "reopen_ok": reopen_ok,
                "reopen_load_sec": reopen_load_sec,
            }
        });
        if let Some(parent) = Path::new(out).parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(out, serde_json::to_string_pretty(&metrics).unwrap())
            .unwrap_or_else(|e| panic!("write metrics json {out}: {e}"));
        println!("  Metrics JSON written: {out}");
    }

    println!(
        "=== SOAK {} ({}) ===\n",
        cfg.benchmark_id,
        if pass { "PASS" } else { "FAIL" }
    );

    // Assert LAST so the metrics artifact is always written first — a failed 12h
    // soak still leaves a `pass=false` result.json for the verifier to flag,
    // rather than vanishing with the panic.
    assert!(
        recall_ok,
        "recall miss rate {:.1}% exceeds {:.0}% threshold",
        miss_rate * 100.0,
        cfg.recall_threshold * 100.0
    );
    assert!(reopen_ok, "reopen verification failed");
    assert!(!all_stats.is_empty(), "soak ran zero cycles");
}

// ---------------------------------------------------------------------------
// Light soak: ~5 min, ~50 MB disk
// ---------------------------------------------------------------------------
#[test]
#[ignore]
fn soak_light() {
    run_soak(SoakConfig::fixed(
        "soak_light",
        4,
        100,
        60,
        1,
        5,
        Some(200),
        0.10,
    ));
}

// ---------------------------------------------------------------------------
// Medium soak: ~30 min, ~500 MB disk
// ---------------------------------------------------------------------------
#[test]
#[ignore]
fn soak_medium() {
    run_soak(SoakConfig::fixed(
        "soak_medium",
        4,
        500,
        360,
        10,
        10,
        Some(200),
        0.10,
    ));
}

// ---------------------------------------------------------------------------
// Heavy soak: duration-bounded, fully env-configurable. This is the profile the
// Independent Benchmark Suite drives for the smoke / 1h / 12h soaks. With no env
// set it would attempt a 12h run, so it stays #[ignore]d and the wrapper scripts
// always pin SOAK_DURATION_SEC. See the module docs for every SOAK_* knob.
// ---------------------------------------------------------------------------
#[test]
#[ignore]
fn soak_heavy() {
    let duration = Duration::from_secs(env_usize("SOAK_DURATION_SEC", 43_200) as u64);
    let benchmark_id = env_str("SOAK_BENCHMARK_ID", "soak_heavy");
    let profile = env_str("SOAK_PROFILE_LABEL", &benchmark_id);
    let ef = env_usize("SOAK_EF_SEARCH", 200) as u32;

    run_soak(SoakConfig {
        name: profile,
        benchmark_id,
        dim: env_usize("SOAK_DIM", 16) as u32,
        nodes_per_cycle: env_usize("SOAK_NODES_PER_CYCLE", 500),
        total_cycles: env_usize("SOAK_MAX_CYCLES", 0), // 0 = unlimited (duration-bounded)
        duration_target: Some(duration),
        compact_every: env_usize("SOAK_COMPACT_EVERY", 20),
        query_k: env_usize("SOAK_QUERY_K", 10) as u32,
        ef_search: Some(ef),
        recall_threshold: env_f64("SOAK_RECALL_THRESH", 0.10),
        result_json: std::env::var("SOAK_RESULT_JSON").ok(),
    });
}
