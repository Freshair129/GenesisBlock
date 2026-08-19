// P22 — Graph traversal benchmark (LDBC-lite).
// Builds a directed random graph (N nodes, fanout edges each), then measures
// k-hop traversal latency (p50/p95/p99) and BFS throughput at depths 1/3/6.
// Pure graph: no embeddings. Bounded per-traversal via `limit` so hub
// neighborhoods don't explode.
//
// Run: GB_VBENCH=<dir> GB_GRAPH_N=100000 GB_GRAPH_FANOUT=8 cargo run --release --bin graph-bench

use genesis_block_native::{EdgeInput, NeighborInput, NodeInput, OpenOptions, Storage};
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

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

fn rss_mb() -> u64 {
    let mut s = sysinfo::System::new_all();
    s.refresh_all();
    sysinfo::get_current_pid()
        .ok()
        .and_then(|pid| s.process(pid).map(|p| p.memory()))
        .unwrap_or(0)
        / 1024
        / 1024
}

fn main() {
    let bench = std::env::var("GB_VBENCH").unwrap_or_else(|_| ".".to_string());
    let n = env_usize("GB_GRAPH_N", 100_000);
    let fanout = env_usize("GB_GRAPH_FANOUT", 8);
    let q = env_usize("GB_GRAPH_Q", 200);
    let limit = env_usize("GB_GRAPH_LIMIT", 1000) as u32;
    let depths: Vec<u32> = std::env::var("GB_GRAPH_DEPTHS")
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![1, 3, 6]);

    let ts_start = chrono::Utc::now();
    let dbpath = format!("{bench}/gdb_graph");
    let _ = fs::remove_dir_all(&dbpath);
    let storage = Storage::open(OpenOptions {
        path: dbpath,
        page_cache_mb: Some(256),
        read_only: Some(false),
        vector_dim: Some(8),
        retention: None,
    })
    .expect("open");

    println!(
        "P22 graph: N={n} fanout={fanout} (edges≈{}) q={q}/depth limit={limit}",
        n * fanout
    );
    let mut rng = StdRng::seed_from_u64(42);

    // --- ingest nodes (streamed) ---
    let t = Instant::now();
    let chunk = 50_000usize;
    let mut i0 = 0;
    while i0 < n {
        let i1 = (i0 + chunk).min(n);
        let nodes: Vec<NodeInput> = (i0..i1)
            .map(|i| NodeInput {
                id: Some(format!("g{i}")),
                labels: vec!["v".to_string()],
                props: None,
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .collect();
        storage.bulk_add_nodes(nodes).unwrap();
        i0 = i1;
    }
    let node_sec = t.elapsed().as_secs_f64();

    // --- ingest edges (streamed) ---
    let t = Instant::now();
    let mut buf: Vec<EdgeInput> = Vec::with_capacity(chunk);
    let mut total_edges = 0usize;
    for i in 0..n {
        for _ in 0..fanout {
            let to = rng.gen_range(0..n);
            buf.push(EdgeInput {
                id: None,
                from: format!("g{i}"),
                to: format!("g{to}"),
                rel: "LINK".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            });
            if buf.len() >= chunk {
                total_edges += buf.len();
                storage.bulk_add_edges(std::mem::take(&mut buf)).unwrap();
            }
        }
    }
    if !buf.is_empty() {
        total_edges += buf.len();
        storage.bulk_add_edges(buf).unwrap();
    }
    let edge_sec = t.elapsed().as_secs_f64();
    let rss = rss_mb();
    println!(
        "ingest: {n} nodes {:.1}s, {total_edges} edges {:.1}s, RSS {rss} MB",
        node_sec, edge_sec
    );

    // --- traversal latency per depth ---
    let mut per_depth = Vec::new();
    for &d in &depths {
        let mut lats = Vec::with_capacity(q);
        let mut total_results = 0usize;
        for _ in 0..q {
            let seed = format!("g{}", rng.gen_range(0..n));
            let t0 = Instant::now();
            let res = storage
                .neighbors(
                    seed,
                    NeighborInput {
                        depth: Some(d),
                        rel: None,
                        rels: None,
                        direction: Some("out".to_string()),
                        as_of: None,
                        include_invalid: Some(false),
                        limit: Some(limit),
                    },
                    false,
                )
                .unwrap();
            lats.push(t0.elapsed().as_nanos() as f64 / 1000.0);
            total_results += res.len();
        }
        let mut s = lats.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let total_s: f64 = lats.iter().sum::<f64>() / 1_000_000.0;
        let tput = q as f64 / total_s;
        let (p50, p95, p99) = (pct(&s, 50.0), pct(&s, 95.0), pct(&s, 99.0));
        println!(
            "  hop{d}: p50 {:.1}µs p95 {:.1}µs p99 {:.1}µs | {:.0} trav/s | avg {} nodes",
            p50,
            p95,
            p99,
            tput,
            total_results / q
        );
        per_depth.push(serde_json::json!({
            "depth": d, "p50_us": p50, "p95_us": p95, "p99_us": pct(&s, 99.0),
            "throughput_per_s": tput, "avg_result_nodes": total_results / q
        }));
    }

    let out = serde_json::json!({
        "engine": "GenesisBlockDB", "n": n, "fanout": fanout, "edges": total_edges, "limit": limit,
        "node_ingest_sec": node_sec, "edge_ingest_sec": edge_sec, "rss_mb": rss, "depths": per_depth
    });
    fs::write(
        format!("{bench}/graph_results_{n}.json"),
        serde_json::to_string_pretty(&out).unwrap(),
    )
    .unwrap();

    // Suite-format metrics for the Independent Benchmark Suite assembler. The
    // headline latency fields use the *deepest* traversal depth measured. This
    // is a descriptive benchmark: `pass` means "ran to completion", not a
    // pass/fail threshold.
    let ts_end = chrono::Utc::now();
    let deepest = per_depth.last().cloned().unwrap_or(serde_json::json!({}));
    let to_ms = |v: &serde_json::Value, key: &str| {
        v.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0) / 1000.0 // µs -> ms
    };
    let metrics = serde_json::json!({
        "benchmark_id": "graph_traversal",
        "project": "GenesisBlockDB",
        "timestamp_start": ts_start.to_rfc3339(),
        "timestamp_end": ts_end.to_rfc3339(),
        "duration_sec": (ts_end - ts_start).num_seconds().max(0),
        "interrupted": false,
        "config": {
            "profile": "graph_traversal",
            "n": n, "fanout": fanout, "edges": total_edges, "limit": limit, "depths": depths
        },
        "results": {
            "pass": true,
            "total_nodes": n,
            "peak_ram_mb": rss,
            "node_ingest_sec": node_sec,
            "edge_ingest_sec": edge_sec,
            "deepest_depth": deepest.get("depth").and_then(|x| x.as_u64()).unwrap_or(0),
            "query_latency_p50_ms": to_ms(&deepest, "p50_us"),
            "query_latency_p95_ms": to_ms(&deepest, "p95_us"),
            "query_latency_p99_ms": to_ms(&deepest, "p99_us"),
            "per_depth": per_depth
        }
    });
    fs::write(
        format!("{bench}/graph_bench_metrics.json"),
        serde_json::to_string_pretty(&metrics).unwrap(),
    )
    .unwrap();
    println!("metrics JSON written: {bench}/graph_bench_metrics.json");
}
