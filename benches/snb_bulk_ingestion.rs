use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use std::time::Instant;

fn main() {
    let db_path = ".brain/snb_bulk_db";
    if std::path::Path::new(db_path).exists() {
        let _ = std::fs::remove_dir_all(db_path);
    }

    let storage = Storage::open(OpenOptions { 
        path: db_path.to_string(),
        page_cache_mb: Some(1024),
        read_only: Some(false),
     vector_dim: None, }).expect("Failed to open storage");

    let batch_size = 5000;
    println!("SNB BULK INGESTION: Processing {} nodes...", batch_size);

    let mut buffer = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        buffer.push(NodeInput {  
            id: Some(format!("B-{}", i)),
            labels: vec!["Entity".to_string()],
            props: Some(serde_json::json!({"val": i})),
            embedding: None,
            lang: None,
         valid_from: None, caused_by: None,  ttl: None, });
    }

    let start = Instant::now();
    storage.bulk_add_nodes(buffer).unwrap();
    let duration = start.elapsed();

    println!("Bulk Ingestion Rate: {:.2} nodes/sec", batch_size as f64 / duration.as_secs_f64());

    let mut edge_buffer = Vec::with_capacity(batch_size);
    for i in 0..batch_size - 1 {
        edge_buffer.push(EdgeInput {  
            id: None,
            from: format!("B-{}", i),
            to: format!("B-{}", i+1),
            rel: "CHAIN".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
         caused_by: None,  });
    }
    storage.bulk_add_edges(edge_buffer).unwrap();
    println!("Bulk Chain Linking Complete.");
}
