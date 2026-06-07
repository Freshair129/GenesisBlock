use genesis_block_native::{Storage, OpenOptions, NodeInput};
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_thai_fuzzy_hardening() {
    let dir = tempdir().unwrap();
    let path = dir.path().to_str().unwrap().to_string();
    let storage = Storage::open(OpenOptions {
        path,
        page_cache_mb: Some(64),
        read_only: Some(false),
    }).unwrap();

    // 1. Add a Thai node: "บ้าน" (House)
    // Note: "บ้าน" has base 'บ', 'า', 'น' and combining mark '้' (tone)
    storage.add_node(NodeInput {
        id: Some("บ้าน".to_string()),
        labels: vec!["PLACE".to_string()],
        props: None,
        embedding: None,
        lang: Some("th".to_string()),
        valid_from: None,
        caused_by: None,
        ttl: None,
    }).unwrap();

    // 2. Search using "บาน" (Bloom/Open - very similar lexically but different meaning)
    // Lexical fuzzy should find it because we filter out the tone mark in the trigram index
    let fuzzy_id = storage.find_fuzzy_id("บาน");
    assert!(fuzzy_id.is_some());
    assert_eq!(fuzzy_id.unwrap(), "บ้าน");

    // 3. Search with a typo: "บ้น"
    let fuzzy_id_typo = storage.find_fuzzy_id("บ้น");
    assert!(fuzzy_id_typo.is_some());
    assert_eq!(fuzzy_id_typo.unwrap(), "บ้าน");
    
    println!("Thai Fuzzy Hardening verification successful.");
}
