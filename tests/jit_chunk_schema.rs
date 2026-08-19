//! Integration tests for JIT / chunk-pointer schema readiness.
//!
//! Validates that GenesisBlockDB's generic node/edge model can faithfully
//! represent JIT chunk schemas (labels, typed props, graph edges, embeddings)
//! without any special-case engine code.

use genesis_block_native::{
    EdgeInput, HybridSearchInput, NeighborInput, NodeInput, OpenOptions, Storage,
};
use serde_json::json;
use std::fs;
use std::path::Path;

// ── helpers ─────────────────────────────────────────────────────────────────

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
        vector_dim: None,
        retention: None,
    })
    .unwrap()
}

fn open_with_dim(path: &str, dim: u32) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(dim),
        retention: None,
    })
    .unwrap()
}

// ── tests ───────────────────────────────────────────────────────────────────

/// All chunk-schema props (strings and integers) survive a save/reopen cycle
/// with correct types.
#[test]
fn chunk_node_props_preserved() {
    let path = fresh("jit_chunk_props");
    let props = json!({
        "source_type": "postgresql",
        "source_table": "documents",
        "source_id": "row-42",
        "document_id": "doc-abc",
        "chunk_index": 3,
        "chunk_strategy": "recursive_character",
        "token_count": 256,
        "content_hash": "sha256:abcdef1234567890",
        "title_path": "Engineering > Architecture > Overview"
    });

    {
        let db = open(&path);
        db.add_node(NodeInput {
            id: Some("chunk-full".into()),
            labels: vec!["Chunk".into(), "DocumentChunk".into()],
            props: Some(props.clone()),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        db.save_state().unwrap();
    }

    let db = open(&path);
    let uid = db.get_u32("chunk-full").expect("node must survive reopen");
    let node = db.node_view_u32(uid).unwrap();

    assert_eq!(node.id, "chunk-full");
    assert!(node.labels.contains(&"Chunk".to_string()));
    assert!(node.labels.contains(&"DocumentChunk".to_string()));

    let p = &node.props;
    assert_eq!(p["source_type"], "postgresql");
    assert_eq!(p["source_table"], "documents");
    assert_eq!(p["source_id"], "row-42");
    assert_eq!(p["document_id"], "doc-abc");
    assert_eq!(p["chunk_index"], 3);
    assert_eq!(p["chunk_strategy"], "recursive_character");
    assert_eq!(p["token_count"], 256);
    assert_eq!(p["content_hash"], "sha256:abcdef1234567890");
    assert_eq!(p["title_path"], "Engineering > Architecture > Overview");

    // Verify integer types are preserved (not coerced to floats/strings).
    assert!(p["chunk_index"].is_i64() || p["chunk_index"].is_u64());
    assert!(p["token_count"].is_i64() || p["token_count"].is_u64());
}

/// Source pointer to a SQL row: three key fields survive a WAL round-trip.
#[test]
fn source_pointer_sql_row() {
    let path = fresh("jit_sql_pointer");
    let props = json!({
        "source_type": "postgresql",
        "source_table": "articles",
        "source_id": "12345"
    });

    {
        let db = open(&path);
        db.add_node(NodeInput {
            id: Some("chunk-sql".into()),
            labels: vec!["Chunk".into()],
            props: Some(props.clone()),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        db.save_state().unwrap();
    }

    let db = open(&path);
    let uid = db
        .get_u32("chunk-sql")
        .expect("sql-pointer node must survive");
    let node = db.node_view_u32(uid).unwrap();
    assert_eq!(node.props["source_type"], "postgresql");
    assert_eq!(node.props["source_table"], "articles");
    assert_eq!(node.props["source_id"], "12345");
}

/// Source pointer to a filesystem path with byte offsets survives reopen.
#[test]
fn source_pointer_file_path() {
    let path = fresh("jit_file_pointer");
    let props = json!({
        "source_type": "filesystem",
        "file_path": "/data/docs/readme.md",
        "byte_offset": 1024,
        "byte_length": 512
    });

    {
        let db = open(&path);
        db.add_node(NodeInput {
            id: Some("chunk-file".into()),
            labels: vec!["Chunk".into()],
            props: Some(props.clone()),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        db.save_state().unwrap();
    }

    let db = open(&path);
    let uid = db
        .get_u32("chunk-file")
        .expect("file-pointer node must survive");
    let node = db.node_view_u32(uid).unwrap();
    assert_eq!(node.props["source_type"], "filesystem");
    assert_eq!(node.props["file_path"], "/data/docs/readme.md");
    assert_eq!(node.props["byte_offset"], 1024);
    assert_eq!(node.props["byte_length"], 512);
}

/// Document -> Section -> Chunk hierarchy with NEXT links between chunks.
/// Traversal from Document at depth=3 must reach all chunks.
#[test]
fn document_hierarchy_graph() {
    let path = fresh("jit_doc_hierarchy");
    let db = open(&path);

    // -- nodes --
    db.add_node(NodeInput {
        id: Some("doc-1".into()),
        labels: vec!["Document".into()],
        props: Some(json!({"title": "Architecture Guide"})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    db.add_node(NodeInput {
        id: Some("sec-1".into()),
        labels: vec!["Section".into()],
        props: Some(json!({"heading": "Overview"})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    for i in 1..=3 {
        db.add_node(NodeInput {
            id: Some(format!("chunk-{}", i)),
            labels: vec!["Chunk".into()],
            props: Some(json!({"chunk_index": i})),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    }

    // -- edges: doc-1 -> sec-1 -> chunk-1, chunk-1 -> chunk-2 -> chunk-3 --
    db.add_edge(EdgeInput {
        id: None,
        from: "doc-1".into(),
        to: "sec-1".into(),
        rel: "PART_OF".into(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();

    db.add_edge(EdgeInput {
        id: None,
        from: "sec-1".into(),
        to: "chunk-1".into(),
        rel: "PART_OF".into(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();

    db.add_edge(EdgeInput {
        id: None,
        from: "chunk-1".into(),
        to: "chunk-2".into(),
        rel: "NEXT".into(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();

    db.add_edge(EdgeInput {
        id: None,
        from: "chunk-2".into(),
        to: "chunk-3".into(),
        rel: "NEXT".into(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();

    // -- traverse depth=4 from doc-1 --
    // doc-1 -> sec-1 (1) -> chunk-1 (2) -> chunk-2 (3) -> chunk-3 (4)
    let results = db
        .neighbors(
            "doc-1".into(),
            NeighborInput {
                depth: Some(4),
                rel: None,
                rels: None,
                direction: Some("out".into()),
                as_of: None,
                include_invalid: None,
                limit: None,
            },
            false,
        )
        .unwrap();

    let reached_ids: Vec<String> = results.iter().map(|n| n.node.id.clone()).collect();
    assert!(
        reached_ids.contains(&"sec-1".to_string()),
        "Section must be reachable from Document"
    );
    assert!(
        reached_ids.contains(&"chunk-1".to_string()),
        "chunk-1 must be reachable at depth 2"
    );
    assert!(
        reached_ids.contains(&"chunk-2".to_string()),
        "chunk-2 must be reachable at depth 3"
    );
    assert!(
        reached_ids.contains(&"chunk-3".to_string()),
        "chunk-3 must be reachable at depth 4"
    );
}

/// A chunk node with an embedding is searchable and returns its chunk props.
#[test]
fn chunk_with_embedding() {
    let path = fresh("jit_chunk_embedding");
    let db = open_with_dim(&path, 4);

    let props = json!({
        "source_type": "postgresql",
        "chunk_index": 7,
        "content_hash": "sha256:deadbeef"
    });

    db.add_node(NodeInput {
        id: Some("embed-chunk".into()),
        labels: vec!["Chunk".into()],
        props: Some(props.clone()),
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    db.flush_index();

    let results = db
        .hybrid_search(HybridSearchInput {
            query_vector: vec![1.0, 0.0, 0.0, 0.0],
            k: 5,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: None,
            ef_search: None,
            oversample: None,
        })
        .unwrap();

    assert!(
        !results.is_empty(),
        "search must return at least one result"
    );
    let hit = results
        .iter()
        .find(|r| r.node.id == "embed-chunk")
        .expect("embed-chunk must appear in search results");

    assert_eq!(hit.node.props["source_type"], "postgresql");
    assert_eq!(hit.node.props["chunk_index"], 7);
    assert_eq!(hit.node.props["content_hash"], "sha256:deadbeef");
    assert!(hit.node.labels.contains(&"Chunk".to_string()));
}

/// Two chunk nodes with the same content_hash but different chunk_index both
/// exist — the engine does no implicit deduplication on props.
#[test]
fn duplicate_content_hash_both_stored() {
    let path = fresh("jit_dup_hash");
    let db = open(&path);

    let shared_hash = "sha256:same_hash_value";

    db.add_node(NodeInput {
        id: Some("dup-a".into()),
        labels: vec!["Chunk".into()],
        props: Some(json!({
            "content_hash": shared_hash,
            "chunk_index": 0
        })),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    db.add_node(NodeInput {
        id: Some("dup-b".into()),
        labels: vec!["Chunk".into()],
        props: Some(json!({
            "content_hash": shared_hash,
            "chunk_index": 1
        })),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    let uid_a = db.get_u32("dup-a").expect("dup-a must exist");
    let uid_b = db.get_u32("dup-b").expect("dup-b must exist");
    assert_ne!(uid_a, uid_b, "nodes must have distinct internal IDs");

    let node_a = db.node_view("dup-a").unwrap();
    let node_b = db.node_view("dup-b").unwrap();
    assert_eq!(node_a.props["content_hash"], shared_hash);
    assert_eq!(node_b.props["content_hash"], shared_hash);
    assert_eq!(node_a.props["chunk_index"], 0);
    assert_eq!(node_b.props["chunk_index"], 1);
}

/// Bulk-inserting 100 chunk nodes: all appear in the node map.
#[test]
fn chunk_node_count() {
    let path = fresh("jit_chunk_count");
    let db = open(&path);

    let before = db.nodes.len();

    for i in 0..100 {
        db.add_node(NodeInput {
            id: Some(format!("chunk-{:04}", i)),
            labels: vec!["Chunk".into()],
            props: Some(json!({
                "chunk_index": i,
                "source_type": "batch"
            })),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    }

    let after = db.nodes.len();
    assert_eq!(
        after - before,
        100,
        "all 100 chunk nodes must be present in the node map"
    );

    // Spot-check first, last, and a middle node.
    for idx in [0, 49, 99] {
        let key = format!("chunk-{:04}", idx);
        let uid = db
            .get_u32(&key)
            .unwrap_or_else(|| panic!("{} must be interned", key));
        let node = db.node_view_u32(uid).unwrap();
        assert_eq!(node.props["chunk_index"], idx);
    }
}
