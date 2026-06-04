use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use std::time::Instant;
use rand::Rng;

fn main() {
    let db_path = ".brain/industrial_audit_db";
    if std::path::Path::new(db_path).exists() {
        let _ = std::fs::remove_dir_all(db_path);
    }

    let storage = Storage::open(OpenOptions {
        path: db_path.to_string(),
        page_cache_mb: Some(1024),
        read_only: Some(false),
    }).expect("Failed to open storage");

    let node_count = 10000;
    let mut nodes = Vec::with_capacity(node_count);

    println!("INDUSTRIAL AUDIT: Ingesting {} nodes with properties...", node_count);
    let start = Instant::now();
    for i in 0..node_count {
        let id = format!("ID-{}", i);
        let mut rng = rand::thread_rng();
        
        let node = storage.add_node(NodeInput {
            id: Some(id.clone()),
            labels: vec!["Asset".to_string()],
            props: Some(serde_json::json!({"status": "active", "value": rng.gen::<f64>()})),
            embedding: None,
            lang: None,
        }).unwrap();
        nodes.push(node.id);
    }
    let duration = start.elapsed();
    println!("Throughput: {:.2} nodes/sec", node_count as f64 / duration.as_secs_f64());

    println!("Audit Complete.");
}
