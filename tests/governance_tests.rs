use genesis_block_native::{Storage, OpenOptions, NodeInput};
use serde_json::json;

#[test]
fn test_axiomatic_guards_enforcement() {
    let db_path = "G:/GenesisBlock_Dev/GenesisBlock/tests/test_guards_db";
    if std::path::Path::new(db_path).exists() {
        let _ = std::fs::remove_dir_all(db_path);
    }

    let storage = Storage::open(OpenOptions { 
        path: db_path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
     vector_dim: None, }).expect("Failed to open storage");

    // 1. Attempt to create a MASTER node (should fail for external agents)
    let master_res = storage.add_node(NodeInput {  
        id: Some("Axiom-1".to_string()),
        labels: vec!["MASTER".to_string()],
        props: Some(json!({"logic": "Universal truth"})),
        embedding: None,
        lang: None,
     valid_from: None, caused_by: None,  ttl: None, });

    assert!(master_res.is_err(), "MASTER node creation should be blocked for external agents");
    let err_msg = master_res.unwrap_err().to_string();
    assert!(err_msg.contains("403 Forbidden"), "Error should indicate a governance violation");

    // 2. Attempt to create a USER node (should succeed)
    let user_res = storage.add_node(NodeInput {  
        id: Some("User-Note".to_string()),
        labels: vec!["USER".to_string()],
        props: Some(json!({"content": "Hello world"})),
        embedding: None,
        lang: None,
     valid_from: None, caused_by: None,  ttl: None, });

    assert!(user_res.is_ok(), "USER node creation should be allowed");
    
    // 3. Verify that the USER node exists
    assert!(storage.get_u32("User-Note").is_some());
}
