use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use std::time::Instant;
use rand::Rng;

fn main() {
    let db_path = ".brain/scientific_audit_db";
    if std::path::Path::new(db_path).exists() {
        let _ = std::fs::remove_dir_all(db_path);
    }

    let storage = Storage::open(OpenOptions {
        path: db_path.to_string(),
        page_cache_mb: Some(512),
        read_only: Some(false),
    }).expect("Failed to open storage");

    let node_count = 5000;
    let mut nodes = Vec::with_capacity(node_count);

    println!("SCIENTIFIC AUDIT: Ingesting {} nodes with vectors...", node_count);
    let start = Instant::now();
    for i in 0..node_count {
        let id = format!("N-{}", i);
        let mut rng = rand::thread_rng();
        let embedding: Vec<f64> = (0..1536).map(|_| rng.gen::<f64>()).collect();
        
        let node = storage.add_node(NodeInput {  
            id: Some(id.clone()),
            labels: vec!["Node".to_string()],
            props: None,
            embedding: Some(embedding),
            lang: None,
         valid_from: None, caused_by: None,  ttl: None, }).unwrap();
        nodes.push(node.id);
    }
    let duration = start.elapsed();
    println!("Ingestion rate: {:.2} nodes/sec", node_count as f64 / duration.as_secs_f64());

    println!("Linking for graph structure...");
    for i in 0..node_count - 1 {
        let _ = storage.add_edge(EdgeInput {  
            id: None,
            from: nodes[i].clone(),
            to: nodes[i+1].clone(),
            rel: "LINKS".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
         caused_by: None,  });
    }

    storage.rebuild_index_parallel().unwrap();
    println!("Audit Complete.");
}
