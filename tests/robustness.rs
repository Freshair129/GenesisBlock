//! Robustness integration tests for GenesisBlockDB.
//!
//! Validates correct handling of edge-case inputs: unicode, emoji, large
//! payloads, wrong-dimension vectors, empty/missing fields, special
//! characters, and concurrent persistence.

use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

fn fresh(name: &str) -> String {
    let db_path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&db_path).exists() {
        fs::remove_dir_all(&db_path).unwrap();
    }
    db_path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(4),
    })
    .unwrap()
}

fn reopen(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(4),
    })
    .unwrap()
}

fn node_with_id(id: &str) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec!["Test".to_string()],
        props: None,
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

#[test]
fn unicode_thai_node() {
    let path = fresh("robust_thai");
    let thai_id = "à¸—à¸”à¸ªà¸­à¸š-à¹„à¸—à¸¢";
    let thai_label = "à¸‚à¹‰à¸­à¸¡à¸¹à¸¥";
    let thai_key = "à¸Šà¸·à¹ˆà¸­";
    let thai_value = "à¸„à¹ˆà¸²";

    {
        let db = open(&path);
        db.add_node(NodeInput {
            id: Some(thai_id.to_string()),
            labels: vec![thai_label.to_string()],
            props: Some(json!({ thai_key: thai_value })),
            embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        db.save_state().unwrap();
    }

    {
        let db = reopen(&path);
        let uid = db.get_u32(thai_id).expect("Thai ID should survive reopen");
        let node = db.node_view_u32(uid).expect("node should exist");
        assert_eq!(node.id, thai_id);
        assert_eq!(node.labels, vec![thai_label.to_string()]);
        assert_eq!(node.props[thai_key], thai_value);
    }
}

#[test]
fn emoji_in_props() {
    let path = fresh("robust_emoji");
    {
        let db = open(&path);
        db.add_node(NodeInput {
            id: Some("emoji-node".to_string()),
            labels: vec!["Test".to_string()],
            props: Some(json!({"mood": "ðŸ˜ŠðŸŽ‰", "flag": "ðŸ‡¹ðŸ‡­"})),
            embedding: Some(vec![0.0, 1.0, 0.0, 0.0]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        db.save_state().unwrap();
    }
    {
        let db = reopen(&path);
        let uid = db
            .get_u32("emoji-node")
            .expect("node should exist after reopen");
        let node = db.node_view_u32(uid).unwrap();
        assert_eq!(node.props["mood"], "ðŸ˜ŠðŸŽ‰");
        assert_eq!(node.props["flag"], "ðŸ‡¹ðŸ‡­");
    }
}

#[test]
fn large_props_100kb() {
    let path = fresh("robust_large_props");
    let big_value = "x".repeat(100_000);
    {
        let db = open(&path);
        let result = db.add_node(NodeInput {
            id: Some("big-props".to_string()),
            labels: vec!["Bulky".to_string()],
            props: Some(json!({"payload": big_value})),
            embedding: Some(vec![0.0, 0.0, 1.0, 0.0]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        });
        match result {
            Ok(_) => db.save_state().unwrap(),
            Err(e) => {
                eprintln!(
                    "large_props_100kb: engine rejected large props cleanly: {}",
                    e
                );
                return;
            }
        }
    }
    {
        let db = reopen(&path);
        let uid = db
            .get_u32("big-props")
            .expect("large-props node should survive");
        let node = db.node_view_u32(uid).unwrap();
        let payload = node.props["payload"].as_str().unwrap();
        assert_eq!(payload.len(), 100_000);
    }
}

#[test]
fn huge_vector_wrong_dim() {
    let db = open(&fresh("robust_wrong_dim"));
    let huge_vec = vec![0.1_f64; 10_000];
    let result = db.add_node(NodeInput {
        id: Some("wrong-dim".to_string()),
        labels: vec![],
        props: None,
        embedding: Some(huge_vec),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    });
    assert!(
        result.is_err(),
        "inserting a 10000-dim vector into a dim=4 collection should error"
    );
}

#[test]
fn zero_dim_vector() {
    let db = open(&fresh("robust_zero_dim"));
    let result = db.add_node(NodeInput {
        id: Some("zero-dim".to_string()),
        labels: vec![],
        props: None,
        embedding: Some(vec![]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    });
    assert!(result.is_err(), "empty embedding should be rejected");
}

#[test]
fn empty_node_id() {
    let db = open(&fresh("robust_empty_id"));
    let result = db.add_node(NodeInput {
        id: Some(String::new()),
        labels: vec!["Test".to_string()],
        props: None,
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    });
    match result {
        Ok(node) => assert_eq!(node.id, "", "empty id should be preserved if accepted"),
        Err(e) => eprintln!("empty_node_id: engine rejected empty id: {}", e),
    }
}

#[test]
fn empty_edge_rel() {
    let db = open(&fresh("robust_empty_rel"));
    db.add_node(node_with_id("a")).unwrap();
    db.add_node(node_with_id("b")).unwrap();
    let result = db.add_edge(EdgeInput {
        id: Some("empty-rel-edge".to_string()),
        from: "a".to_string(),
        to: "b".to_string(),
        rel: String::new(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    });
    match result {
        Ok(edge) => assert_eq!(edge.rel, "", "empty rel should be preserved if accepted"),
        Err(e) => eprintln!("empty_edge_rel: engine rejected empty rel: {}", e),
    }
}

#[test]
fn missing_labels() {
    let db = open(&fresh("robust_no_labels"));
    let node = db
        .add_node(NodeInput {
            id: Some("no-labels".to_string()),
            labels: vec![],
            props: None,
            embedding: Some(vec![0.0, 0.0, 0.0, 1.0]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    assert!(node.labels.is_empty(), "labels should be empty");
}

#[test]
fn special_chars_in_id() {
    let path = fresh("robust_special_id");
    let id = "doc:section/1 (draft)";
    {
        let db = open(&path);
        db.add_node(NodeInput {
            id: Some(id.to_string()),
            labels: vec!["Doc".to_string()],
            props: None,
            embedding: Some(vec![1.0, 1.0, 0.0, 0.0]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        db.save_state().unwrap();
    }
    {
        let db = reopen(&path);
        let uid = db
            .get_u32(id)
            .expect("special-chars ID should survive reopen");
        let node = db.node_view_u32(uid).unwrap();
        assert_eq!(node.id, id);
    }
}

#[test]
fn null_props_fields() {
    let path = fresh("robust_null_props");
    {
        let db = open(&path);
        db.add_node(NodeInput {
            id: Some("null-props".to_string()),
            labels: vec![],
            props: Some(json!({"key": null, "nested": {"a": null}})),
            embedding: Some(vec![0.0, 0.0, 1.0, 1.0]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        db.save_state().unwrap();
    }
    {
        let db = reopen(&path);
        let uid = db
            .get_u32("null-props")
            .expect("null-props node should survive reopen");
        let node = db.node_view_u32(uid).unwrap();
        assert!(
            node.props["key"].is_null(),
            "top-level null should be preserved"
        );
        assert!(
            node.props["nested"]["a"].is_null(),
            "nested null should be preserved"
        );
    }
}

#[test]
fn very_long_id() {
    let db = open(&fresh("robust_long_id"));
    let long_id = "x".repeat(10_000);
    let result = db.add_node(NodeInput {
        id: Some(long_id.clone()),
        labels: vec![],
        props: None,
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    });
    match result {
        Ok(node) => assert_eq!(node.id.len(), 10_000, "long id should be preserved"),
        Err(e) => eprintln!("very_long_id: engine rejected long id cleanly: {}", e),
    }
}

#[test]
fn concurrent_save_state() {
    let path = fresh("robust_conc_save");
    {
        let db = Arc::new(open(&path));
        for i in 0..50 {
            db.add_node(node_with_id(&format!("save-{}", i))).unwrap();
        }

        let db1 = Arc::clone(&db);
        let db2 = Arc::clone(&db);

        let t1 = thread::spawn(move || db1.save_state());
        let t2 = thread::spawn(move || db2.save_state());

        let r1 = t1.join().expect("save_state thread 1 panicked");
        let r2 = t2.join().expect("save_state thread 2 panicked");
        assert!(
            r1.is_ok() || r2.is_ok(),
            "at least one concurrent save_state should succeed"
        );
    }
    {
        let db = reopen(&path);
        assert!(
            db.nodes.len() >= 50,
            "all 50 nodes should survive concurrent save: got {}",
            db.nodes.len()
        );
        assert!(db.get_u32("save-0").is_some(), "save-0 should exist");
        assert!(db.get_u32("save-49").is_some(), "save-49 should exist");
    }
}
