//! REST API integration tests — exercises core /v1/* routes via in-process
//! Axum oneshot calls (no TCP socket). Each test gets its own TempDir so
//! there is no shared state between tests.
//!
//! Run: `cargo test --test rest_api`

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use genesis_block_native::router::{build_router, AppState};
use genesis_block_native::{OpenOptions, Storage};
use http_body_util::BodyExt;
use parking_lot::RwLock;
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a fresh in-process app with default vector_dim (1536).
fn app(name: &str) -> (Router, Arc<RwLock<Storage>>, TempDir) {
    app_with_dim(name, None)
}

/// Create a fresh in-process app with an explicit vector_dim for the default
/// collection (pass `Some(4)` when tests need tiny embeddings).
fn app_with_dim(_name: &str, dim: Option<u32>) -> (Router, Arc<RwLock<Storage>>, TempDir) {
    let dir = TempDir::new().expect("TempDir creation");
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: dim,
        retention: None,
    })
    .expect("Storage::open in test");
    let storage = Arc::new(RwLock::new(storage));
    let state = AppState {
        storage: Arc::clone(&storage),
        api_key: None,
    };
    (build_router(state), storage, dir)
}

/// POST a JSON value, return (status, parsed body).
async fn post_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

/// GET, return (status, parsed body).
async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

/// POST a raw string body (not wrapped in a JSON object), return status + raw text.
async fn post_raw(app: &Router, uri: &str, content_type: &str, raw: &str) -> (StatusCode, String) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", content_type)
        .body(Body::from(raw.to_owned()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// POST with a raw Body (for oversized payloads), return status code only.
async fn post_body(app: &Router, uri: &str, content_type: &str, body: Body) -> StatusCode {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", content_type)
        .body(body)
        .unwrap();
    app.clone().oneshot(req).await.unwrap().status()
}

// ---------------------------------------------------------------------------
// 1. version_endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn version_endpoint() {
    let (app, _storage, _dir) = app("test_rest_version");
    let (status, body) = get_json(&app, "/v1/version").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["engine_name"],
        json!("genesis-block"),
        "engine_name must be 'genesis-block'"
    );
    assert!(
        body["version"].is_string(),
        "version field must be present and a string"
    );
    assert!(
        body["schema_version"].is_number(),
        "schema_version field must be present and a number"
    );
}

// ---------------------------------------------------------------------------
// 2. status_endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn status_endpoint() {
    let (app, _storage, _dir) = app("test_rest_status");
    let (status, body) = get_json(&app, "/v1/status").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["open"], json!(true));
    assert_eq!(body["read_only"], json!(false));
    // These fields must exist (values may vary)
    assert!(body["node_count"].is_number(), "node_count present");
    assert!(body["edge_count"].is_number(), "edge_count present");
    assert!(
        body["memory_usage_mb"].is_number(),
        "memory_usage_mb present"
    );
}

// ---------------------------------------------------------------------------
// 3. add_node_and_query
// ---------------------------------------------------------------------------

#[tokio::test]
async fn add_node_and_query() {
    let (app, _storage, _dir) = app("test_rest_add_node");
    let (status, body) = post_json(
        &app,
        "/v1/node/add",
        json!({ "id": "rest-n1", "labels": ["Test"], "props": { "k": "v" } }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], json!("rest-n1"));
    assert!(body["labels"].as_array().unwrap().contains(&json!("Test")));
    assert_eq!(body["props"]["k"], json!("v"));
}

// ---------------------------------------------------------------------------
// 4. add_edge
// ---------------------------------------------------------------------------

#[tokio::test]
async fn add_edge() {
    let (app, _storage, _dir) = app("test_rest_add_edge");
    // Create both endpoints first
    post_json(&app, "/v1/node/add", json!({ "id": "a", "labels": [] })).await;
    post_json(&app, "/v1/node/add", json!({ "id": "b", "labels": [] })).await;

    let (status, body) = post_json(
        &app,
        "/v1/edge/add",
        json!({ "from": "a", "to": "b", "rel": "LINK" }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["from"], json!("a"));
    assert_eq!(body["to"], json!("b"));
    assert_eq!(body["rel"], json!("LINK"));
}

// ---------------------------------------------------------------------------
// 5. query_edges
// ---------------------------------------------------------------------------

#[tokio::test]
async fn query_edges() {
    let (app, _storage, _dir) = app("test_rest_query_edges");
    post_json(&app, "/v1/node/add", json!({ "id": "a", "labels": [] })).await;
    post_json(&app, "/v1/node/add", json!({ "id": "b", "labels": [] })).await;
    post_json(
        &app,
        "/v1/edge/add",
        json!({ "id": "e1", "from": "a", "to": "b", "rel": "LINK" }),
    )
    .await;

    let (status, rows) = post_json(&app, "/v1/query", json!({ "from": "a" })).await;

    assert_eq!(status, StatusCode::OK);
    let arr = rows.as_array().expect("/v1/query must return an array");
    assert!(!arr.is_empty(), "query must return the added edge");
    assert_eq!(arr[0]["to"], json!("b"));
    assert_eq!(arr[0]["rel"], json!("LINK"));
}

// ---------------------------------------------------------------------------
// 6. hql_raw_string
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hql_raw_string() {
    let (app, _storage, _dir) = app("test_rest_hql_raw");
    post_json(&app, "/v1/node/add", json!({ "id": "a", "labels": [] })).await;

    // /v1/query/hql accepts a JSON-encoded string: `"TRAVERSE FROM a DEPTH 1 REL ANY"`
    let hql_body = serde_json::to_string("TRAVERSE FROM a DEPTH 1 REL ANY").unwrap();
    let (status, body) = post_raw(&app, "/v1/query/hql", "application/json", &hql_body).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "HQL must accept raw JSON string body"
    );
    // Response is a JSON array of NeighborOutput (may be empty if no edges)
    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert!(parsed.is_array(), "HQL TRAVERSE must return an array");
}

// ---------------------------------------------------------------------------
// 7. hybrid_search
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hybrid_search() {
    // Use dim=4 for the default collection so we can embed tiny vectors
    let (app, storage, _dir) = app_with_dim("test_rest_hybrid", Some(4));

    // Add a node with an embedding into the default collection
    post_json(
        &app,
        "/v1/node/add",
        json!({ "id": "vec1", "labels": ["V"], "embedding": [1.0, 0.0, 0.0, 0.0] }),
    )
    .await;

    // Flush the HNSW index so the vector is searchable immediately
    {
        let s = storage.read();
        s.flush_index();
    }

    let (status, results) = post_json(
        &app,
        "/v1/search/hybrid",
        json!({ "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 1 }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let hits = results.as_array().expect("hybrid search returns an array");
    assert!(
        !hits.is_empty(),
        "hybrid search must return at least one result"
    );
    assert_eq!(hits[0]["node"]["id"], json!("vec1"));
}

// ---------------------------------------------------------------------------
// 8. create_and_list_collections
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_and_list_collections() {
    let (app, _storage, _dir) = app("test_rest_collections");

    let (create_status, create_body) = post_json(
        &app,
        "/v1/collection/create",
        json!({ "name": "test_col", "model": "test", "dim": 8 }),
    )
    .await;
    assert_eq!(create_status, StatusCode::OK);
    assert_eq!(create_body["ok"], json!(true));

    let (list_status, list) = get_json(&app, "/v1/collections").await;
    assert_eq!(list_status, StatusCode::OK);
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["name"].as_str())
        .collect();
    assert!(
        names.contains(&"test_col"),
        "newly created collection must appear in /v1/collections: {:?}",
        names
    );
}

// ---------------------------------------------------------------------------
// 9. add_vector
// ---------------------------------------------------------------------------

#[tokio::test]
async fn add_vector() {
    let (app, _storage, _dir) = app_with_dim("test_rest_add_vector", Some(4));
    // Create the node first
    post_json(&app, "/v1/node/add", json!({ "id": "n1", "labels": ["X"] })).await;

    let (status, body) = post_json(
        &app,
        "/v1/vector/add",
        json!({ "node_id": "n1", "collection": "default", "embedding": [1.0, 0.0, 0.0, 0.0] }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "add_vector to default collection must succeed"
    );
    assert_eq!(body["ok"], json!(true));
}

// ---------------------------------------------------------------------------
// 10. malformed_json_returns_error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn malformed_json_returns_error() {
    let (app, _storage, _dir) = app("test_rest_malformed");
    let (status, _) = post_raw(&app, "/v1/node/add", "application/json", "not json").await;

    assert_ne!(
        status,
        StatusCode::OK,
        "malformed JSON body must not return 200"
    );
    // Axum typically returns 400 or 422 for deserialization failures
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "expected 400 or 422, got {}",
        status
    );
}

// ---------------------------------------------------------------------------
// 11. missing_required_field
// ---------------------------------------------------------------------------

#[tokio::test]
async fn missing_required_field() {
    let (app, _storage, _dir) = app("test_rest_missing_field");
    // NodeInput requires `labels` — send an empty object
    let (status, _) = post_json(&app, "/v1/node/add", json!({})).await;

    assert_ne!(
        status,
        StatusCode::OK,
        "missing required field (labels) must not return 200"
    );
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "expected 400 or 422, got {}",
        status
    );
}

// ---------------------------------------------------------------------------
// 12. hql_body_limit
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hql_body_limit() {
    let (app, _storage, _dir) = app("test_rest_hql_limit");

    // The HQL route has DefaultBodyLimit::max(256 * 1024) = 256 KiB.
    // Send a body larger than that limit.
    let oversized = "x".repeat(300 * 1024); // 300 KiB > 256 KiB
    let body = Body::from(oversized);

    let status = post_body(&app, "/v1/query/hql", "application/json", body).await;

    assert_ne!(
        status,
        StatusCode::OK,
        "oversized HQL body must be rejected"
    );
    // tower-http / axum returns 413 Payload Too Large
    assert_eq!(
        status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "expected 413 Payload Too Large, got {}",
        status
    );
}

// ---------------------------------------------------------------------------
// 13. bulk_nodes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bulk_nodes() {
    let (app, _storage, _dir) = app("test_rest_bulk_nodes");

    let nodes: Vec<Value> = (0..10)
        .map(|i| json!({ "id": format!("bn_{}", i), "labels": ["Bulk"] }))
        .collect();

    let (status, body) = post_json(&app, "/v1/bulk/nodes", Value::Array(nodes)).await;
    assert_eq!(status, StatusCode::OK);

    // Response is { ok: true, count: 10 } or similar — verify count if present
    if let Some(count) = body["count"].as_u64() {
        assert_eq!(count, 10, "bulk response count must be 10");
    }

    // Verify the nodes actually exist by adding edges between two of them and
    // querying. If the nodes were not created this will fail.
    post_json(
        &app,
        "/v1/edge/add",
        json!({ "id": "bne", "from": "bn_0", "to": "bn_9", "rel": "SEQ" }),
    )
    .await;

    let (qs, rows) = post_json(&app, "/v1/query", json!({ "from": "bn_0", "rel": "SEQ" })).await;
    assert_eq!(qs, StatusCode::OK);
    let arr = rows.as_array().unwrap();
    assert_eq!(
        arr.len(),
        1,
        "edge between bulk-added nodes must be queryable"
    );
    assert_eq!(arr[0]["to"], json!("bn_9"));
}
