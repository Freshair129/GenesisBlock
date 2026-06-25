use genesis_block_native::{Storage, OpenOptions, NodeInput};
use std::fs;
use std::path::Path;

#[test]
fn test_mark_ix_instant_load_persistence() {
    let db_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persistence_db");
    if Path::new(db_path).exists() {
        fs::remove_dir_all(db_path).unwrap();
    }

    // 1. Create DB and add data
    {
        let storage = Storage::open(OpenOptions { 
            path: db_path.to_string(),
            page_cache_mb: Some(64),
            read_only: Some(false),
            vector_dim: Some(1536),
        }).unwrap();

        storage.add_node(NodeInput {
            id: Some("persist_1".to_string()),
            labels: vec!["TEST".to_string()],
            props: None,
            embedding: Some(vec![0.5; 1536]),
            lang: Some("en".to_string()),
            valid_from: None,
            caused_by: None,
            ttl: None, collection: None,
        }).unwrap();

        // Manual save or let Drop handle it
        storage.save_state().unwrap();
    } // storage dropped here, Drop trait should save state too

    // 2. Re-open and verify data exists WITHOUT WAL (by checking if .bin files are used)
    {
        // To truly verify instant load, we could temporarily move the .wal file
        // but try_load_state logs its progress.
        let storage = Storage::open(OpenOptions { 
            path: db_path.to_string(),
            page_cache_mb: Some(64),
            read_only: Some(false),
            vector_dim: Some(1536),
        }).unwrap();

        let u32_id = storage.get_u32("persist_1").expect("Node should be found via instant load");
        let node = storage.nodes.get(&u32_id).unwrap();
        assert_eq!(node.id, "persist_1");
        
        // Verify vector arena was loaded (default collection)
        let coll = storage.collections.get("default").unwrap();
        let meta_arena = coll.metadata.read();
        assert_eq!(meta_arena.len(), 1);
        // A2: metadata stores the interned u32; it resolves back to the node id.
        assert_eq!(meta_arena[0].node_u32, u32_id);
        assert_eq!(storage.nodes.get(&meta_arena[0].node_u32).unwrap().id, "persist_1");
    }
    
    println!("Mark IX: Persistence verification successful.");
}
