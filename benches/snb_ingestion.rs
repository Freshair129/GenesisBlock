use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use std::time::Instant;

fn main() {
    let db_path = ".brain/snb_ingestion_db";
    if std::path::Path::new(db_path).exists() {
        let _ = std::fs::remove_dir_all(db_path);
    }

    let storage = Storage::open(OpenOptions {
        path: db_path.to_string(),
        page_cache_mb: Some(512),
        read_only: Some(false),
    }).expect("Failed to open storage");

    let person_count = 1000;
    println!("SNB INGESTION: Simulating LDBC-like workload...");

    let start = Instant::now();
    for i in 0..person_count {
        storage.add_node(NodeInput { 
            id: Some(format!("Person-{}", i)),
            labels: vec!["Person".to_string()],
            props: Some(serde_json::json!({"name": format!("User {}", i)})),
            embedding: None,
            lang: None,
         valid_from: None, caused_by: None, }).unwrap();
    }

    for i in 0..person_count - 1 {
        storage.add_edge(EdgeInput { 
            id: None,
            from: format!("Person-{}", i),
            to: format!("Person-{}", i + 1),
            rel: "KNOWS".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
         caused_by: None, }).unwrap();
    }
    let duration = start.elapsed();
    println!("Ingested {} Persons and relationships in {:?}", person_count, duration);
}
