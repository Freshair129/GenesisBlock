use genesis_block_native::{Storage, OpenOptions, NodeInput};
use std::fs;
use std::path::Path;

#[test]
fn test_mark_ix_memory_reclamation_compaction() {
    let db_path = "G:/GenesisBlock_Dev/GenesisBlock/tests/test_compaction_db";
    if Path::new(db_path).exists() {
        fs::remove_dir_all(db_path).unwrap();
    }

    let storage = Storage::open(OpenOptions { 
        path: db_path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(3),
    }).unwrap();

    // 1. Add 100 nodes
    for i in 0..100 {
        storage.add_node(NodeInput {
            id: Some(format!("node_{}", i)),
            labels: vec!["TEMP".to_string()],
            props: None,
            embedding: Some(vec![i as f64, 0.0, 0.0]),
            lang: Some("en".to_string()),
            valid_from: None,
            caused_by: None,
            ttl: None,
        }).unwrap();
    }

    // Verify initial arena size
    {
        let vec_arena = storage.vector_arena.read();
        assert_eq!(vec_arena.len(), 100 * 3, "Initial vector arena should have 300 elements");
        let meta_arena = storage.metadata_arena.read();
        assert_eq!(meta_arena.len(), 100, "Initial metadata arena should have 100 entries");
    }

    // 2. Retract 90 nodes
    for i in 0..90 {
        storage.retract_node(&format!("node_{}", i)).unwrap();
    }

    // DashMap size is reduced, but Arenas are still large (fragmented)
    assert_eq!(storage.nodes.len(), 10);
    {
        let vec_arena = storage.vector_arena.read();
        assert_eq!(vec_arena.len(), 100 * 3, "Arenas should still be large before compaction");
    }

    // 3. Perform Compaction
    storage.perform_index_compaction().unwrap();

    // 4. Verify Reclamation
    {
        let vec_arena = storage.vector_arena.read();
        assert_eq!(vec_arena.len(), 10 * 3, "Vector arena should be reclaimed to 30 elements");
        let meta_arena = storage.metadata_arena.read();
        assert_eq!(meta_arena.len(), 10, "Metadata arena should be reclaimed to 10 entries");
    }

    // 5. Verify search still works after compaction
    let search_res = storage.hybrid_search(genesis_block_native::HybridSearchInput {
        query_vector: vec![99.0, 0.0, 0.0],
        k: 1,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
    }).unwrap();
    
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].node.id, "node_99");

    println!("Mark IX: Index Compaction (Garbage Collection) verified successfully.");
}
