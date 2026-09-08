use genesis_block_native::{EdgeInput, NeighborInput, NodeInput, OpenOptions, Storage};
use tempfile::tempdir;

fn storage() -> (Storage, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();
    (storage, dir)
}

fn add_node(storage: &Storage, id: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.to_string()),
            labels: vec![],
            props: None,
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

#[test]
fn zero_neighbor_limit_is_rejected_before_traversal() {
    let (storage, _dir) = storage();
    add_node(&storage, "A");
    add_node(&storage, "B");
    storage
        .add_edge(EdgeInput {
            id: Some("ab".to_string()),
            from: "A".to_string(),
            to: "B".to_string(),
            rel: "LINK".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    let error = storage
        .neighbors(
            "A".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: None,
                limit: Some(0),
            },
            false,
        )
        .expect_err("zero limit must fail before traversal");
    assert!(error.to_string().starts_with("QUERY_RESOURCE_LIMIT_EXCEEDED:"));
}
