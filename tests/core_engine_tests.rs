use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use std::fs;
use std::path::Path;

fn setup_test_db(name: &str) -> Storage {
    let db_path = format!("G:/GenesisBlock_Dev/GenesisBlock/tests/{}", name);
    if Path::new(&db_path).exists() {
        fs::remove_dir_all(&db_path).unwrap();
    }
    Storage::open(OpenOptions {
        path: db_path,
        page_cache_mb: Some(64),
        read_only: Some(false),
    }).unwrap()
}

#[test]
fn test_node_creation_and_interning() {
    let storage = setup_test_db("test_node_interning");
    
    let node = storage.add_node(NodeInput { 
        id: Some("user_1".to_string()),
        labels: vec!["User".to_string()],
        props: None,
        embedding: None,
        lang: None,
     valid_from: None, caused_by: None, }).unwrap();

    assert_eq!(node.id, "user_1");
    
    let u32_id = storage.get_u32("user_1").expect("ID should be interned");
    let retrieved_node = storage.nodes.get(&u32_id).unwrap();
    assert_eq!(retrieved_node.id, "user_1");
}

#[test]
fn test_edge_creation_and_graph_traversal() {
    let storage = setup_test_db("test_graph_traversal");
    
    storage.add_node(NodeInput {  id: Some("A".to_string()), labels: vec![], props: None, embedding: None, lang: None, valid_from: None, caused_by: None, }).unwrap();
    storage.add_node(NodeInput {  id: Some("B".to_string()), labels: vec![], props: None, embedding: None, lang: None, valid_from: None, caused_by: None, }).unwrap();
    
    let _edge = storage.add_edge(EdgeInput { 
        id: Some("E1".to_string()),
        from: "A".to_string(),
        to: "B".to_string(),
        rel: "knows".to_string(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
     caused_by: None, }).unwrap();

    let neighbors = storage.execute_hql("TRAVERSE FROM A DEPTH 1 REL knows").unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].node.id, "B");
}

#[test]
fn test_vector_arena_and_hybrid_search() {
    let storage = setup_test_db("test_hybrid_search");
    
    storage.add_node(NodeInput { 
        id: Some("v1".to_string()),
        labels: vec![], props: None,
        embedding: Some(vec![1.0, 0.0, 0.0]),
        lang: None,
     valid_from: None, caused_by: None, }).unwrap();
    
    storage.add_node(NodeInput { 
        id: Some("v2".to_string()),
        labels: vec![], props: None,
        embedding: Some(vec![0.0, 1.0, 0.0]),
        lang: None,
     valid_from: None, caused_by: None, }).unwrap();
    
    storage.rebuild_index_parallel().unwrap();

    let results = storage.execute_hql("SEARCH Node SIMILAR TO [0.9, 0.1, 0.0] K 1").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].node.id, "v1");
}

#[test]
fn test_wal_group_commit_durability() {
    let db_path = "G:/GenesisBlock_Dev/GenesisBlock/tests/test_wal_durability";
    if Path::new(db_path).exists() { fs::remove_dir_all(db_path).unwrap(); }

    {
        let storage = Storage::open(OpenOptions { path: db_path.to_string(), page_cache_mb: None, read_only: Some(false) }).unwrap();
        storage.add_node(NodeInput {  id: Some("durable_node".to_string()), labels: vec![], props: None, embedding: None, lang: None, valid_from: None, caused_by: None, }).unwrap();
    } 

    {
        let storage = Storage::open(OpenOptions { path: db_path.to_string(), page_cache_mb: None, read_only: Some(false) }).unwrap();
        let u32_id = storage.get_u32("durable_node").expect("Node should exist after WAL replay");
        let node = storage.nodes.get(&u32_id).unwrap();
        assert_eq!(node.id, "durable_node");
    }
}
