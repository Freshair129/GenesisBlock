// Self-contained vector-search benchmark for the Independent Benchmark Suite.
//
// Unlike `vbench-genesis` (which replays externally-produced bge-m3 vectors and
// therefore needs a Python + model-download step), this harness is fully
// reproducible from a clone: it deterministically generates a random unit-vector
// corpus from a fixed seed, ingests it, then measures k-NN query latency and the
// *real* recall@k against an exact brute-force ground truth computed in-process.
//
// Nothing here mutates engine behaviour — it only drives the public Storage API.
//
// Run (writes <out>/vector_bench_metrics.json):
//   GB_VEC_OUT=<dir> GB_VEC_N=50000 GB_VEC_DIM=128 GB_VEC_Q=1000 GB_VEC_K=10 \
//   GB_VEC_EF=200 GB_VEC_SEED=42 \
//   cargo run --release --no-default-features --features bins --bin vector-bench

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::fs;
use std::time::Instant;

fn env_usize(k: &str, d: usize) -> usize {
    std::env::var(k)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(d)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

fn peak_rss_mb() -> u64 {
    let mut s = sysinfo::System::new_all();
    s.refresh_all();
    sysinfo::get_current_pid()
        .ok()
        .and_then(|pid| s.process(pid).map(|p| p.memory()))
        .unwrap_or(0)
        / 1024
        / 1024
}

/// Deterministic unit vector for index `i` (cosine-friendly).
fn gen_vec(rng: &mut StdRng, dim: usize) -> Vec<f32> {
    let mut v: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0f32..1.0f32)).collect();
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
    for x in v.iter_mut() {
        *x /= norm;
    }
    v
}

fn main() {
    let out = std::env::var("GB_VEC_OUT").unwrap_or_else(|_| ".".to_string());
    let n = env_usize("GB_VEC_N", 50_000);
    let dim = env_usize("GB_VEC_DIM", 128);
    let q = env_usize("GB_VEC_Q", 1_000);
    let k = env_usize("GB_VEC_K", 10);
    let ef = env_usize("GB_VEC_EF", 200) as u32;
    let seed = env_usize("GB_VEC_SEED", 42) as u64;

    let ts_start = chrono::Utc::now();
    println!("vector-bench: N={n} dim={dim} Q={q} k={k} ef_search={ef} seed={seed}");

    // --- deterministic corpus + queries ---
    let mut rng = StdRng::seed_from_u64(seed);
    let corpus: Vec<f32> = {
        let mut buf = Vec::with_capacity(n * dim);
        for _ in 0..n {
            buf.extend_from_slice(&gen_vec(&mut rng, dim));
        }
        buf
    };
    // Queries are perturbations of random corpus rows so a true nearest neighbour
    // exists in-set (recall is meaningful), plus a little noise.
    let queries: Vec<f32> = {
        let mut buf = Vec::with_capacity(q * dim);
        for _ in 0..q {
            let src = rng.gen_range(0..n);
            let mut v: Vec<f32> = corpus[src * dim..(src + 1) * dim]
                .iter()
                .map(|&x| x + rng.gen_range(-0.02f32..0.02f32))
                .collect();
            let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
            for x in v.iter_mut() {
                *x /= norm;
            }
            buf.extend_from_slice(&v);
        }
        buf
    };

    let dbpath = format!("{out}/gdb_vector_bench");
    let _ = fs::remove_dir_all(&dbpath);
    let storage = Storage::open(OpenOptions {
        path: dbpath.clone(),
        page_cache_mb: Some(256),
        read_only: Some(false),
        vector_dim: Some(dim as u32),
    })
    .expect("open storage");
    storage.set_index_params(ef, 100);

    // --- ingest (streamed bulk) ---
    let t = Instant::now();
    let chunk = 10_000usize;
    let mut i0 = 0usize;
    while i0 < n {
        let i1 = (i0 + chunk).min(n);
        let inputs: Vec<NodeInput> = (i0..i1)
            .map(|i| NodeInput {
                id: Some(i.to_string()),
                labels: vec!["doc".to_string()],
                props: None,
                embedding: Some(
                    corpus[i * dim..(i + 1) * dim]
                        .iter()
                        .map(|&x| x as f64)
                        .collect(),
                ),
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .collect();
        storage.bulk_add_nodes(inputs).unwrap();
        i0 = i1;
    }
    storage.flush_index();
    let insert_sec = t.elapsed().as_secs_f64();
    let peak_rss = peak_rss_mb();
    println!(
        "  ingest: {n} vec in {insert_sec:.1}s ({:.0} vec/s), RSS {peak_rss} MB",
        n as f64 / insert_sec
    );

    // --- exact brute-force ground truth (cosine == dot for unit vectors) ---
    let truth: Vec<Vec<i64>> = (0..q)
        .map(|qi| {
            let qv = &queries[qi * dim..(qi + 1) * dim];
            let mut scored: Vec<(f32, i64)> = (0..n)
                .map(|c| {
                    let cv = &corpus[c * dim..(c + 1) * dim];
                    let dot: f32 = qv.iter().zip(cv).map(|(a, b)| a * b).sum();
                    (dot, c as i64)
                })
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            scored.iter().take(k).map(|&(_, id)| id).collect()
        })
        .collect();

    // --- query + recall@k ---
    let mut lats = Vec::with_capacity(q);
    let mut hits = 0usize;
    let mut total = 0usize;
    for qi in 0..q {
        let qv: Vec<f64> = queries[qi * dim..(qi + 1) * dim]
            .iter()
            .map(|&x| x as f64)
            .collect();
        let t0 = Instant::now();
        let res = storage
            .hybrid_search(HybridSearchInput {
                query_vector: qv,
                k: k as u32,
                alpha: Some(0.0), // pure vector search
                lang: None,
                as_of: None,
                collection: None,
                ef_search: Some(ef),
            })
            .unwrap();
        lats.push(t0.elapsed().as_nanos() as f64 / 1_000_000.0); // ms
        let got: std::collections::HashSet<i64> = res
            .iter()
            .take(k)
            .map(|nb| nb.node.id.parse::<i64>().unwrap_or(-1))
            .collect();
        let want = &truth[qi];
        hits += want.iter().filter(|id| got.contains(id)).count();
        total += want.len();
    }
    let recall_at_k = if total == 0 {
        0.0
    } else {
        hits as f64 / total as f64
    };
    let mut sorted = lats.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let (p50, p95, p99) = (
        percentile(&sorted, 50.0),
        percentile(&sorted, 95.0),
        percentile(&sorted, 99.0),
    );
    println!("  recall@{k}={recall_at_k:.4} | query ms p50={p50:.3} p95={p95:.3} p99={p99:.3}");

    let ts_end = chrono::Utc::now();
    let pass = recall_at_k > 0.0 && !lats.is_empty();

    let metrics = serde_json::json!({
        "benchmark_id": "vector_search",
        "project": "GenesisBlockDB",
        "timestamp_start": ts_start.to_rfc3339(),
        "timestamp_end": ts_end.to_rfc3339(),
        "duration_sec": (ts_end - ts_start).num_seconds().max(0),
        "interrupted": false,
        "config": {
            "profile": "vector_search",
            "n": n, "dim": dim, "q": q, "query_k": k, "ef_search": ef, "seed": seed,
            "metric": "cosine", "alpha": 0.0
        },
        "results": {
            // Descriptive benchmark: `pass` means "ran to completion with a valid
            // recall measurement", NOT a pass/fail threshold. recall_at_k is the
            // measured value — interpret it, do not gate on it here.
            "pass": pass,
            "total_nodes": n,
            "recall_at_k": recall_at_k,
            "peak_ram_mb": peak_rss,
            "insert_per_sec": n as f64 / insert_sec,
            "query_latency_p50_ms": p50,
            "query_latency_p95_ms": p95,
            "query_latency_p99_ms": p99
        }
    });
    let out_path = format!("{out}/vector_bench_metrics.json");
    fs::write(&out_path, serde_json::to_string_pretty(&metrics).unwrap()).unwrap();
    println!("  metrics JSON written: {out_path}");

    // Drop the storage FIRST — its Drop persists state, which would otherwise
    // re-create the scratch DB dir right after we delete it.
    drop(storage);
    let _ = fs::remove_dir_all(&dbpath);
    if !pass {
        eprintln!("vector-bench FAILED: no valid recall measurement");
        std::process::exit(1);
    }
}
