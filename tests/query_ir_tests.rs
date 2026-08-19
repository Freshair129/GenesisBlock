use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use serde_json::json;
use tempfile::TempDir;

fn storage(dim: u32) -> (Storage, TempDir) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(dim),
        retention: None,
    })
    .unwrap();
    (storage, dir)
}

fn node(id: &str, embedding: Option<Vec<f64>>) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

#[test]
fn query_ir_search_returns_versioned_envelope() {
    let (storage, _dir) = storage(3);
    storage
        .add_node(node("query-ir-nearest", Some(vec![1.0, 0.0, 0.0])))
        .unwrap();
    storage.flush_index();

    let response = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-search-1",
            "operation": {
                "kind": "search",
                "mode": "vector",
                "query_vector": [0.9, 0.1, 0.0],
                "k": 1
            }
        }))
        .unwrap();

    assert_eq!(response["contract_version"], "query-ir.v1");
    assert_eq!(response["request_id"], "req-search-1");
    assert_eq!(response["operation_kind"], "search");
    assert_eq!(response["data"][0]["node"]["id"], "query-ir-nearest");
}

#[test]
fn query_ir_traverse_returns_neighbor() {
    let (storage, _dir) = storage(3);
    storage.add_node(node("query-ir-src", None)).unwrap();
    storage.add_node(node("query-ir-dst", None)).unwrap();
    storage
        .add_edge(EdgeInput {
            id: Some("query-ir-edge".to_string()),
            from: "query-ir-src".to_string(),
            to: "query-ir-dst".to_string(),
            rel: "KNOWS".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    let response = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-traverse-1",
            "operation": {
                "kind": "traverse",
                "seed_id": "query-ir-src",
                "depth": 1,
                "relations": ["KNOWS"],
                "direction": "out"
            }
        }))
        .unwrap();

    assert_eq!(response["operation_kind"], "traverse");
    assert_eq!(response["data"][0]["node"]["id"], "query-ir-dst");
}

#[test]
fn query_ir_search_can_use_target_node_embedding() {
    let (storage, _dir) = storage(3);
    storage
        .add_node(node("query-ir-target", Some(vec![1.0, 0.0, 0.0])))
        .unwrap();
    storage
        .add_node(node("query-ir-other", Some(vec![0.0, 1.0, 0.0])))
        .unwrap();
    storage.flush_index();

    let response = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-target-search",
            "operation": {
                "kind": "search",
                "mode": "vector",
                "target_id": "query-ir-target",
                "k": 1
            }
        }))
        .unwrap();

    assert_eq!(response["data"][0]["node"]["id"], "query-ir-target");
}

#[test]
fn query_ir_rejects_unknown_version_and_fields() {
    let (storage, _dir) = storage(3);

    let version_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v2",
            "request_id": "req-version",
            "operation": {
                "kind": "traverse",
                "seed_id": "missing",
                "depth": 1,
                "relations": ["KNOWS"],
                "direction": "out"
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(version_error.starts_with("QUERY_IR_VERSION_UNSUPPORTED:"));

    let validation_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-unknown",
            "unexpected": true,
            "operation": {
                "kind": "traverse",
                "seed_id": "missing",
                "depth": 1,
                "relations": ["KNOWS"],
                "direction": "out"
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(validation_error.starts_with("QUERY_IR_VALIDATION_FAILED:"));
}

#[test]
fn hql_and_query_ir_preserve_search_and_traverse_results() {
    let (storage, _dir) = storage(3);
    storage
        .add_node(node("parity-src", Some(vec![1.0, 0.0, 0.0])))
        .unwrap();
    storage
        .add_node(node("parity-dst", Some(vec![0.0, 1.0, 0.0])))
        .unwrap();
    storage
        .add_edge(EdgeInput {
            id: Some("parity-edge".to_string()),
            from: "parity-src".to_string(),
            to: "parity-dst".to_string(),
            rel: "KNOWS".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
    storage.flush_index();

    let hql_search = storage
        .execute_hql("SEARCH parity-src SIMILAR TO [1,0,0] K 1")
        .unwrap();
    let ir_search = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "parity-search",
            "operation": {
                "kind": "search",
                "mode": "vector",
                "query_vector": [1.0, 0.0, 0.0],
                "k": 1
            }
        }))
        .unwrap();
    assert_eq!(
        hql_search[0]["node"]["id"],
        ir_search["data"][0]["node"]["id"]
    );

    let hql_traverse = storage
        .execute_hql("TRAVERSE FROM parity-src DEPTH 1 REL KNOWS")
        .unwrap();
    let ir_traverse = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "parity-traverse",
            "operation": {
                "kind": "traverse",
                "seed_id": "parity-src",
                "depth": 1,
                "relations": ["KNOWS"],
                "direction": "out"
            }
        }))
        .unwrap();
    assert_eq!(
        hql_traverse[0]["node"]["id"],
        ir_traverse["data"][0]["node"]["id"]
    );
}
