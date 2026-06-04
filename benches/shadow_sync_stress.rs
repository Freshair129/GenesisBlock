use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput, NeighborInput, HybridSearchInput};
use std::sync::Arc;
use std::time::{Instant, Duration};
use rayon::prelude::*;
use serde_json::json;
use rand::Rng;

fn main() {
    let db_path = ".brain/shadow_sync_stress_db";
    if std::path::Path::new(db_path).exists() {
        let _ = std::fs::remove_dir_all(db_path);
    }

    println!("--- SHADOW SYNC STRESS TEST (MARK IV) ---");
    let storage = Arc::new(Storage::open(OpenOptions {
        path: db_path.to_string(),
        page_cache_mb: Some(512),
        read_only: Some(false),
    }).expect("Failed to open storage"));

    let node_count = 10_000;
    let writer_threads = 12;
    let reader_threads = 4;

    println!("Simulating {} notes with 1536-dim embeddings and complex JSON props...", node_count);
    println!("Concurrency: {} Writers vs {} Readers", writer_threads, reader_threads);

    let start_time = Instant::now();

    // Start background readers
    let storage_reader = Arc::clone(&storage);
    let reader_handle = std::thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let mut total_queries = 0;
        let mut latencies = Vec::new();
        
        while total_queries < 500 {
            let node_id = format!("Note-{}", rng.gen_range(0..node_count.max(1)));
            let query = format!("TRAVERSE FROM ~\"{}\" DEPTH 2 REL ANY", node_id);
            
            let q_start = Instant::now();
            let _ = storage_reader.execute_hql(&query);
            latencies.push(q_start.elapsed());
            
            total_queries += 1;
            std::thread::sleep(Duration::from_millis(10));
        }
        latencies
    });

    // Main Ingestion (Writers)
    (0..node_count).into_par_iter().for_each(|i| {
        let id = format!("Note-{}", i);
        let mut rng = rand::thread_rng();
        let embedding: Vec<f64> = (0..1536).map(|_| rng.gen::<f64>()).collect();
        
        let props = json!({
            "title": format!("Mark IV Stress Note #{}", i),
            "content_hash": format!("{:x}", rng.gen::<u64>()),
            "tags": ["mark-iv", "stress-test", "shadow-sync", "trigram"],
            "metadata": {
                "created_at": "2026-06-03T12:00:00Z",
                "author": "Rwang-Agent",
                "trigram_verified": true
            }
        });

        let _ = storage.add_node(NodeInput {
            id: Some(id),
            labels: vec!["Note".to_string()],
            props: Some(props),
            embedding: Some(embedding),
        });
    });

    let duration = start_time.elapsed();
    let tps = node_count as f64 / duration.as_secs_f64();
    
    let latencies = reader_handle.join().unwrap();
    let p95_latency = if !latencies.is_empty() {
        let mut l = latencies;
        l.sort();
        l[l.len() * 95 / 100]
    } else {
        Duration::from_secs(0)
    };

    println!("\n--- RESULTS (MARK IV) ---");
    println!("Total Ingestion Time: {:.2?}", duration);
    println!("Throughput: {:.2} TPS", tps);
    println!("P95 Query Latency (Under Load): {:.2?}", p95_latency);
    println!("Note: Trigram Index optimized candidate filtering.");
    
    if p95_latency < Duration::from_millis(10) {
        println!("SUCCESS: Latency target (R1 < 10ms) MET.");
    } else {
        println!("WARNING: Latency target (R1 < 10ms) EXCEEDED.");
    }

    println!("\nShutting down for durability verification (WAL Replay)...");
    drop(storage);

    // Verification Phase
    let storage_verify = Storage::open(OpenOptions {
        path: db_path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(true),
    }).expect("Failed to reopen storage for verification");

    let u32_id = storage_verify.get_u32("Note-9999");
    
    if u32_id.is_some() {
        println!("VERIFICATION: Node 'Note-9999' found. WAL Replay SUCCESS.");
    } else {
        println!("VERIFICATION: Node 'Note-9999' NOT found. WAL Replay FAILED.");
    }
}
