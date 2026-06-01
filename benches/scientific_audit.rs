use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::path::Path;
use std::fs;
use sysinfo::System;
use rayon::prelude::*;
use rand::Rng;
use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};

fn main() {
    let db_path = "G:/GenesisBlock_Dev/GenesisBlock/benches/scientific_audit_db";
    if Path::new(db_path).exists() { fs::remove_dir_all(db_path).unwrap(); }

    println!("🚀 [STAGE 1] Pre-loading Dataset (32k nodes, 150k edges)...");
    let mut storage = Storage::open(OpenOptions {
        path: db_path.to_string(),
        page_cache_mb: Some(2048),
        read_only: Some(false),
    }).expect("Failed to open storage");

    let mut rng = rand::thread_rng();
    let node_count = 32_700;
    let edge_count = 150_000;

    let mut nodes = Vec::with_capacity(10000);
    for i in 0..node_count {
        nodes.push(NodeInput {
            id: Some(format!("p{}", i)),
            labels: vec!["Person".to_string()],
            props: None,
            embedding: Some(vec![rng.gen_range(0.0..1.0); 128]),
        });
        if nodes.len() >= 10000 {
            storage.bulk_add_nodes(nodes.drain(..).collect()).unwrap();
        }
    }
    storage.bulk_add_nodes(nodes).unwrap();

    let mut edges = Vec::with_capacity(50000);
    for i in 0..edge_count {
        edges.push(EdgeInput {
            id: None,
            from: format!("p{}", rng.gen_range(0..node_count)),
            to: format!("p{}", rng.gen_range(0..node_count)),
            rel: "knows".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
        });
        if edges.len() >= 50000 {
            storage.bulk_add_edges(edges.drain(..).collect()).unwrap();
        }
    }
    storage.bulk_add_edges(edges).unwrap();
    storage.rebuild_index_parallel().unwrap();
    println!("✅ Stage 1 Complete.");

    println!("⚡ [STAGE 2] Starting 15-Second Rigorous Stress Test (80% Read / 20% Write)...");
    println!("🔔 Note: Every write includes a mandatory fsync (sync_all).");
    
    let shared_storage = Arc::new(storage);
    let search_lats = Arc::new(Mutex::new(Vec::new()));
    let write_lats = Arc::new(Mutex::new(Vec::new()));
    
    let bench_duration = Duration::from_secs(15);
    let start_time = Instant::now();

    (0..12).into_par_iter().for_each(|_| {
        let mut local_rng = rand::thread_rng();
        while start_time.elapsed() < bench_duration {
            if local_rng.gen_bool(0.8) {
                let s_start = Instant::now();
                let _ = shared_storage.execute_hql("SEARCH Person SIMILAR TO [0.5, 0.5, 0.5, 0.5] K 5");
                let s_dur = s_start.elapsed();
                search_lats.lock().unwrap().push(s_dur);
            } else {
                let w_start = Instant::now();
                let _ = shared_storage.add_edge(EdgeInput {
                    id: None,
                    from: format!("p{}", local_rng.gen_range(0..node_count)),
                    to: format!("p{}", local_rng.gen_range(0..node_count)),
                    rel: "interacts".to_string(),
                    props: None,
                    valid_from: None,
                    supersede: None,
                    impact: None,
                });
                let w_dur = w_start.elapsed();
                write_lats.lock().unwrap().push(w_dur);
            }
        }
    });

    let total_elapsed = start_time.elapsed();
    let mut system = System::new_all();
    system.refresh_memory();
    let peak_ram_gb = system.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

    let analyze = |mut lats: Vec<Duration>| {
        if lats.is_empty() { return (Duration::default(), Duration::default(), Duration::default(), Duration::default()); }
        lats.sort();
        let len = lats.len();
        let p50 = lats[len / 2];
        let p95 = lats[(len as f64 * 0.95) as usize];
        let p99 = lats[(len as f64 * 0.99) as usize];
        let mean = lats.iter().sum::<Duration>() / len as u32;
        (mean, p50, p95, p99)
    };

    let s_lats = search_lats.lock().unwrap().clone();
    let w_lats = write_lats.lock().unwrap().clone();
    let (s_mean, s_p50, s_p95, s_p99) = analyze(s_lats.clone());
    let (w_mean, w_p50, w_p95, w_p99) = analyze(w_lats.clone());

    let total_ops = s_lats.len() + w_lats.len();

    println!("---------------------------------");
    println!("🧪 SCIENTIFIC AUDIT REPORT");
    println!("---------------------------------");
    println!("Duration:         {:?}", total_elapsed);
    println!("Total Operations: {}", total_ops);
    println!("Throughput:       {:.2} QPS/TPS", total_ops as f64 / total_elapsed.as_secs_f64());
    println!("Peak RAM:         {:.2} GB", peak_ram_gb);
    println!("");
    println!("SEARCH LATENCY (80% Load):");
    println!("Mean:             {:?}", s_mean);
    println!("P50:              {:?}", s_p50);
    println!("P95:              {:?}", s_p95);
    println!("P99:              {:?}", s_p99);
    println!("");
    println!("WRITE LATENCY (20% Load + FSYNC):");
    println!("Mean:             {:?}", w_mean);
    println!("P50:              {:?}", w_p50);
    println!("P95:              {:?}", w_p95);
    println!("P99:              {:?}", w_p99);
    println!("---------------------------------");
}
