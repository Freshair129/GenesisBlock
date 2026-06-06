use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_vector_drift_tracking() {
    let dir = tempdir().unwrap();
    let path = dir.path().to_str().unwrap().to_string();
    let storage = Arc::new(Storage::open(OpenOptions {
        path,
        page_cache_mb: Some(64),
        read_only: Some(false),
    }).unwrap());

    // 1. Add some nodes to form a cluster
    let mut v1 = vec![0.0; 1536];
    v1[0] = 1.0; 
    storage.add_node(NodeInput {
        id: Some("node-1".to_string()),
        labels: vec!["USER".to_string()],
        props: None,
        embedding: Some(v1.clone()),
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
    }).unwrap();

    storage.add_node(NodeInput {
        id: Some("node-2".to_string()),
        labels: vec!["USER".to_string()],
        props: None,
        embedding: Some(v1.clone()),
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
    }).unwrap();

    storage.add_edge(EdgeInput {
        id: None, from: "node-1".to_string(), to: "node-2".to_string(), rel: "KNOWS".to_string(),
        props: None, valid_from: None, supersede: None, impact: None, caused_by: None,
    }).unwrap();

    // 2. Initial community detection and meta-graph generation
    println!("Step 2: Starting meta-graph generation...");
    storage.detect_communities().unwrap();
    storage.generate_meta_graph().unwrap();
    println!("Step 2: Done.");

    let c_id = {
        let meta_arena = storage.metadata_arena.read();
        meta_arena[0].cluster_id // node-1's cluster
    };
    
    let history_v1 = storage.get_meta_history(c_id);
    assert_eq!(history_v1.len(), 1);

    // 3. Update nodes
    println!("Step 3: Adding node-3...");
    let mut v2 = vec![0.0; 1536];
    v2[1] = 1.0; 

    storage.add_node(NodeInput {
        id: Some("node-3".to_string()),
        labels: vec!["USER".to_string()],
        props: None,
        embedding: Some(v2.clone()),
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
    }).unwrap();

    storage.add_edge(EdgeInput {
        id: None, from: "node-2".to_string(), to: "node-3".to_string(), rel: "KNOWS".to_string(),
        props: None, valid_from: None, supersede: None, impact: None, caused_by: None,
    }).unwrap();
    println!("Step 3: Done.");

    // 4. Regenerate meta-graph
    println!("Step 4: Regenerating meta-graph...");
    storage.detect_communities().unwrap();
    storage.generate_meta_graph().unwrap();
    println!("Step 4: Done.");

    // Find node-1's cluster again (it might have changed ID but should have history)
    let c_id_new = {
        let meta_arena = storage.metadata_arena.read();
        meta_arena[0].cluster_id
    };
    let history_v2 = storage.get_meta_history(c_id_new);
    println!("History for cluster {}: {} entries", c_id_new, history_v2.len());
    
    if history_v2.len() > 1 {
        let drift = history_v2.last().unwrap().drift;
        println!("Detected Drift: {:?}", drift);
        assert!(drift.is_some(), "Drift should be recorded");
    } else {
        // If it's a new cluster ID, check if ANY cluster has history > 1
        let mut found_history = false;
        for entry in storage.meta_history.iter() {
            if entry.value().len() > 1 {
                println!("Found history for cluster {}: {} entries", entry.key(), entry.value().len());
                found_history = true;
                break;
            }
        }
        assert!(found_history, "At least one cluster should have recorded history across snapshots");
    }
}
