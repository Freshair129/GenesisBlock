use genesis_block_native::{EdgeInput, NeighborInput, NodeInput, OpenOptions, Storage};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::json;
use std::fs;
use std::time::Instant;

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
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
    let out = std::env::var("GB_VBENCH").unwrap_or_else(|_| ".".to_string());
    let n = env_usize("GB_PROPS_N", 20_000);
    let fanout = env_usize("GB_PROPS_FANOUT", 4);
    let prop_bytes = env_usize("GB_PROPS_SIZE", 2048);
    let q = env_usize("GB_PROPS_Q", 200);
    let depth = env_usize("GB_PROPS_DEPTH", 3) as u32;
    let limit = env_usize("GB_PROPS_LIMIT", 1000) as u32;
    let chunk = env_usize("GB_PROPS_CHUNK", 10_000);

    let payload = "x".repeat(prop_bytes);
    let dbpath = format!("{out}/gdb_sqlite_props_audit");
    let _ = fs::remove_dir_all(&dbpath);
    let ts_start = chrono::Utc::now();

    let storage = Storage::open(OpenOptions {
        path: dbpath.clone(),
        page_cache_mb: Some(256),
        read_only: Some(false),
        vector_dim: Some(8),
    })
    .expect("open");

    let mut rng = StdRng::seed_from_u64(42);
    let rss_empty = rss_mb();
    println!(
        "sqlite-props-audit: N={n} fanout={fanout} prop_bytes={prop_bytes} q={q} depth={depth}"
    );
    println!("[stage 0] empty open RSS {rss_empty} MB");

    let t = Instant::now();
    let mut i0 = 0usize;
    while i0 < n {
        let i1 = (i0 + chunk).min(n);
        let nodes: Vec<NodeInput> = (i0..i1)
            .map(|i| NodeInput {
                id: Some(format!("p{i}")),
                labels: vec!["props".to_string(), "bench".to_string()],
                props: Some(json!({
                    "payload": payload,
                    "index": i,
                    "group": format!("g{}", i % 100),
                })),
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
    let rss_nodes = rss_mb();
    println!(
        "[stage 1] +{n} props-heavy nodes {:.1}s RSS {rss_nodes} MB (+{} MB)",
        node_sec,
        rss_nodes.saturating_sub(rss_empty)
    );

    let t = Instant::now();
    let mut buf: Vec<EdgeInput> = Vec::with_capacity(chunk);
    let mut total_edges = 0usize;
    for i in 0..n {
        for _ in 0..fanout {
            let to = rng.gen_range(0..n);
            buf.push(EdgeInput {
                id: None,
                from: format!("p{i}"),
                to: format!("p{to}"),
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
    let rss_edges = rss_mb();
    println!(
        "[stage 2] +{total_edges} edges {:.1}s RSS {rss_edges} MB (+{} MB vs nodes)",
        edge_sec,
        rss_edges.saturating_sub(rss_nodes)
    );

    let resident_null_props = storage
        .nodes
        .iter()
        .filter(|entry| entry.value().props.is_null())
        .count();
    let resident_non_null_props = storage.nodes.len().saturating_sub(resident_null_props);
    let resident_inline_prop_bytes: usize = storage
        .nodes
        .iter()
        .map(|entry| entry.value().props.to_string().len())
        .sum();
    let expected_inline_payload_bytes = n.saturating_mul(prop_bytes);
    let resident_payload_bytes_saved_lower_bound =
        expected_inline_payload_bytes.saturating_sub(resident_inline_prop_bytes);
    let lean_ratio = if storage.nodes.is_empty() {
        0.0
    } else {
        resident_null_props as f64 / storage.nodes.len() as f64
    };

    let sample_uid = storage.get_u32("p0").expect("sample node must exist");
    let sample_projection = storage
        .projection_props(sample_uid)
        .expect("projection query should succeed")
        .expect("sample props should exist in projection");
    assert_eq!(
        sample_projection["payload"]
            .as_str()
            .map(|s| s.len())
            .unwrap_or(0),
        prop_bytes
    );

    storage.save_state().unwrap();
    let sqlite_path = format!("{dbpath}/projection.sqlite");
    let sqlite_size = fs::metadata(&sqlite_path).map(|m| m.len()).unwrap_or(0);

    let mut lats = Vec::with_capacity(q);
    let mut total_results = 0usize;
    for _ in 0..q {
        let seed = format!("p{}", rng.gen_range(0..n));
        let t0 = Instant::now();
        let res = storage
            .neighbors(
                seed,
                NeighborInput {
                    depth: Some(depth),
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
    let mut sorted = lats.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let total_s = lats.iter().sum::<f64>() / 1_000_000.0;
    let throughput = q as f64 / total_s;
    let p50 = pct(&sorted, 50.0);
    let p95 = pct(&sorted, 95.0);
    let p99 = pct(&sorted, 99.0);
    println!(
        "[stage 3] hop{depth} traversal p50 {:.1}us p95 {:.1}us p99 {:.1}us | {:.0} trav/s | avg {} nodes",
        p50,
        p95,
        p99,
        throughput,
        total_results / q
    );

    let ts_end = chrono::Utc::now();
    let metrics = serde_json::json!({
        "benchmark_id": "sqlite_props_audit",
        "project": "GenesisBlockDB",
        "timestamp_start": ts_start.to_rfc3339(),
        "timestamp_end": ts_end.to_rfc3339(),
        "config": {
            "n": n,
            "fanout": fanout,
            "prop_bytes": prop_bytes,
            "q": q,
            "depth": depth,
            "limit": limit,
        },
        "results": {
            "pass": true,
            "rss_empty_mb": rss_empty,
            "rss_after_nodes_mb": rss_nodes,
            "rss_after_edges_mb": rss_edges,
            "node_ingest_sec": node_sec,
            "edge_ingest_sec": edge_sec,
            "resident_nodes": storage.nodes.len(),
            "resident_null_props": resident_null_props,
            "resident_non_null_props": resident_non_null_props,
            "resident_lean_ratio": lean_ratio,
            "resident_inline_prop_bytes": resident_inline_prop_bytes,
            "expected_inline_payload_bytes": expected_inline_payload_bytes,
            "resident_payload_bytes_saved_lower_bound": resident_payload_bytes_saved_lower_bound,
            "projection_sqlite_bytes": sqlite_size,
            "sample_projection_payload_bytes": sample_projection["payload"].as_str().map(|s| s.len()).unwrap_or(0),
            "traversal_depth": depth,
            "query_latency_p50_us": p50,
            "query_latency_p95_us": p95,
            "query_latency_p99_us": p99,
            "traversal_per_s": throughput,
            "avg_result_nodes": total_results / q
        }
    });

    let out_path = format!("{out}/sqlite_props_audit_metrics.json");
    fs::write(&out_path, serde_json::to_string_pretty(&metrics).unwrap()).unwrap();
    println!("metrics JSON written: {out_path}");
}
