use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use std::sync::Arc;
use std::time::Instant;
use rand::Rng;

fn main() {
    let db_path = ".brain/hql_bench_db";
    if std::path::Path::new(db_path).exists() {
        let _ = std::fs::remove_dir_all(db_path);
    }

    let storage = Storage::open(OpenOptions {
        path: db_path.to_string(),
        page_cache_mb: Some(512),
        read_only: Some(false),
    }).expect("Failed to open storage");

    let node_count = 1000;
    let mut nodes = Vec::with_capacity(node_count);

    println!("Ingesting {} nodes for HQL stress test...", node_count);
    for i in 0..node_count {
        let id = format!("N-{}", i);
        let node = storage.add_node(NodeInput { 
            id: Some(id.clone()),
            labels: vec!["Node".to_string()],
            props: None,
            embedding: Some(vec![i as f64; 1536]),
            lang: None,
         valid_from: None, caused_by: None, }).unwrap();
        nodes.push(node.id);
    }

    println!("Linking nodes with edges...");
    for i in 0..node_count - 1 {
        let _ = storage.add_edge(EdgeInput { 
            id: None,
            from: nodes[i].clone(),
            to: nodes[i+1].clone(),
            rel: "LINK".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
         caused_by: None, });
    }

    storage.rebuild_index_parallel().unwrap();

    let queries = [
        "TRAVERSE FROM N-0 DEPTH 5 REL LINK",
        "SEARCH Node SIMILAR TO [10.0; 1536] K 5",
        "MATCH Node SIMILAR TO [50.0; 1536] ALPHA 0.5"
    ];

    println!("Running HQL queries...");
    let storage_read = Arc::new(storage);
    let start = Instant::now();
    let iterations = 100;

    for _ in 0..iterations {
        for query in &queries {
            let _ = storage_read.execute_hql(query);
        }
    }

    let duration = start.elapsed();
    println!("HQL Stress completed in: {:.2?}", duration);
    println!("Avg Latency per query: {:.2?}", duration / (iterations * queries.len() as u32));
}
