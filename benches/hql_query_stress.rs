use std::time::Instant;
use rayon::prelude::*;
use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput, HybridSearchInput, NeighborInput};
use std::sync::Arc;
use parking_lot::RwLock;

fn main() {
    let path = "G:/GenesisBlock_Dev/GenesisBlock/benches/hql_bench_db";
    let mut storage = Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(512),
        read_only: Some(false),
    }).expect("Failed to open storage");

    println!("🚀 Pre-loading 10,000 Nodes for HQL Stress Test...");
    
    let mut nodes = Vec::with_capacity(10000);
    for i in 0..10000 {
        nodes.push(NodeInput {
            id: Some(format!("p{}", i)),
            labels: vec!["Person".to_string()],
            props: None,
            embedding: Some(vec![0.1, 0.2, 0.3, 0.4]),
        });
    }
    storage.bulk_add_nodes(nodes).unwrap();

    let mut edges = Vec::with_capacity(50000);
    for i in 0..50000 {
        edges.push(EdgeInput {
            id: None,
            from: format!("p{}", i % 10000),
            to: format!("p{}", (i + 1) % 10000),
            rel: "knows".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
        });
    }
    storage.bulk_add_edges(edges).unwrap();
    storage.rebuild_index_parallel().unwrap();

    println!("⚡ Starting Concurrent HQL Query Stress Test (100,000 Queries)...");
    
    // Wrap in Arc/RwLock to simulate server-side concurrent access
    let shared_storage = Arc::new(RwLock::new(storage));
    let query_count = 100_000;
    
    let start = Instant::now();
    
    (0..query_count).into_par_iter().for_each(|i| {
        let storage_read = shared_storage.read();
        let query = if i % 2 == 0 {
            "SEARCH Person SIMILAR TO [0.1, 0.2, 0.3, 0.4] K 5"
        } else {
            "TRAVERSE FROM p0 DEPTH 2 REL knows"
        };
        let _ = storage_read.execute_hql(query);
    });

    let duration = start.elapsed();
    let qps = query_count as f64 / duration.as_secs_f64();

    println!("---------------------------------");
    println!("📊 HQL QUERY PERFORMANCE REPORT");
    println!("---------------------------------");
    println!("Total Queries:  {}", query_count);
    println!("Total Time:     {:?}", duration);
    println!("Average QPS:    {:.2}", qps);
    println!("Latency/Query:  {:?}", duration / query_count as u32);
    println!("---------------------------------");
}
