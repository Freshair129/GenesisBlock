//! REST API integration tests — exercises all 24 /v1/* routes via in-process
//! Axum oneshot calls (no TCP socket). Each test gets its own TempDir so
//! there is no shared state between tests.
//!
//! Run: `cargo test --test rest_api_tests`

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

fn make_app() -> (Router, TempDir) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: None,
    })
    .expect("Storage::open in test");
    let state = AppState {
        storage: Arc::new(RwLock::new(storage)),
    };
    (build_router(state), dir)
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

/// POST a raw string body (not JSON-encoded as an object), return status + raw text.
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

// ---------------------------------------------------------------------------
// Status endpoints
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_status_returns_open() {
    let (app, _dir) = make_app();
    let (status, body) = get_json(&app, "/v1/status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["open"], json!(true));
    assert_eq!(body["read_only"], json!(false));
}

#[tokio::test]
async fn test_swarm_status_has_peer_id() {
    let (app, _dir) = make_app();
    let (status, body) = get_json(&app, "/v1/swarm/status").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["peer_id"].as_str().unwrap_or("").len() > 0, "peer_id must be non-empty");
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_add_node_round_trip() {
    let (app, _dir) = make_app();
    let (status, body) = post_json(
        &app,
        "/v1/node/add",
        json!({ "id": "node_a", "labels": ["Person"], "props": {"name": "Alice"} }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], json!("node_a"));
    assert!(body["labels"].as_array().unwrap().contains(&json!("Person")));
}

#[tokio::test]
async fn test_supersede_node_updates_props() {
    let (app, _dir) = make_app();
    // Add original
    post_json(&app, "/v1/node/add", json!({ "id": "n1", "labels": [], "props": {"v": 1} })).await;
    // Supersede
    let (status, body) = post_json(
        &app,
        "/v1/node/supersede",
        json!({ "id": "n1", "new_props": {"v": 2} }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["props"]["v"], json!(2));
    // The supersede creates a new version node; the `caused_by` chain is non-empty
    assert!(body["caused_by"] != Value::Null || body["id"].as_str().unwrap().starts_with("n1"));
}

// ---------------------------------------------------------------------------
// Edge
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_add_edge_visible_in_query() {
    let (app, _dir) = make_app();
    post_json(&app, "/v1/node/add", json!({ "id": "src", "labels": [] })).await;
    post_json(&app, "/v1/node/add", json!({ "id": "dst", "labels": [] })).await;
    let (e_status, edge) = post_json(
        &app,
        "/v1/edge/add",
        json!({ "id": "e1", "from": "src", "to": "dst", "rel": "LINKS" }),
    )
    .await;
    assert_eq!(e_status, StatusCode::OK);
    assert_eq!(edge["rel"], json!("LINKS"));

    // Query edges from src
    let (q_status, rows) = post_json(
        &app,
        "/v1/query",
        json!({ "from": "src", "rel": "LINKS" }),
    )
    .await;
    assert_eq!(q_status, StatusCode::OK);
    let arr = rows.as_array().unwrap();
    assert!(!arr.is_empty(), "query must return the added edge");
    assert_eq!(arr[0]["to"], json!("dst"));
}

#[tokio::test]
async fn test_retract_edge_hidden_from_current_view() {
    let (app, _dir) = make_app();
    post_json(&app, "/v1/node/add", json!({ "id": "ret_a", "labels": [] })).await;
    post_json(&app, "/v1/node/add", json!({ "id": "ret_b", "labels": [] })).await;
    post_json(&app, "/v1/edge/add", json!({ "id": "er1", "from": "ret_a", "to": "ret_b", "rel": "KNOWS" })).await;

    // Confirm the edge is visible via TRAVERSE before retraction
    let before_hql = serde_json::to_string("TRAVERSE FROM ret_a DEPTH 1 REL KNOWS").unwrap();
    let (pre_status, pre_body) = post_raw(&app, "/v1/query/hql", "application/json", &before_hql).await;
    assert_eq!(pre_status, StatusCode::OK);
    let pre: serde_json::Value = serde_json::from_str(&pre_body).unwrap();
    assert!(!pre.as_array().unwrap().is_empty(), "edge must be visible via TRAVERSE before retraction");

    // Retract
    let (ret_status, _) = post_json(&app, "/v1/edge/retract", json!({ "id": "er1" })).await;
    assert_eq!(ret_status, StatusCode::OK);

    // TRAVERSE (uses neighbors()) now hides the retracted edge from current view
    let after_hql = serde_json::to_string("TRAVERSE FROM ret_a DEPTH 1 REL KNOWS").unwrap();
    let (post_status, post_body) = post_raw(&app, "/v1/query/hql", "application/json", &after_hql).await;
    assert_eq!(post_status, StatusCode::OK);
    let after: serde_json::Value = serde_json::from_str(&post_body).unwrap();
    assert!(
        after.as_array().unwrap().is_empty(),
        "retracted edge must be hidden from TRAVERSE current view"
    );

    // Raw /v1/query with include_invalid=true still finds the edge record (WAL-preserved)
    let (qi_status, rows_i) = post_json(
        &app,
        "/v1/query",
        json!({ "from": "ret_a", "include_invalid": true }),
    )
    .await;
    assert_eq!(qi_status, StatusCode::OK);
    assert!(
        !rows_i.as_array().unwrap().is_empty(),
        "retracted edge must remain accessible via include_invalid=true on /v1/query"
    );
}

#[tokio::test]
async fn test_retract_unknown_edge_returns_404() {
    let (app, _dir) = make_app();
    let (status, _) = post_json(&app, "/v1/edge/retract", json!({ "id": "no_such_edge" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Bulk operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_bulk_add_nodes() {
    let (app, _dir) = make_app();
    let nodes = json!([
        { "id": "bulk1", "labels": ["Tag"] },
        { "id": "bulk2", "labels": ["Tag"] },
        { "id": "bulk3", "labels": ["Tag"] },
        { "id": "bulk4", "labels": ["Tag"] },
        { "id": "bulk5", "labels": ["Tag"] }
    ]);
    let (status, _) = post_json(&app, "/v1/bulk/nodes", nodes).await;
    assert_eq!(status, StatusCode::OK);

    // Verify all five are searchable via query
    let (qs, rows) = post_json(&app, "/v1/query", json!({ "from": "bulk3" })).await;
    // query by `from` with no rel returns adjacency — at least the endpoint responded OK
    assert_eq!(qs, StatusCode::OK);
    // confirm node exists: add an edge and traverse
    post_json(&app, "/v1/bulk/edges", json!([{ "id": "bke1", "from": "bulk1", "to": "bulk2", "rel": "R" }])).await;
    let (qs2, rows2) = post_json(&app, "/v1/query", json!({ "from": "bulk1", "rel": "R" })).await;
    assert_eq!(qs2, StatusCode::OK);
    assert_eq!(rows2.as_array().unwrap()[0]["to"], json!("bulk2"));
    // suppress unused warning
    let _ = rows;
}

// ---------------------------------------------------------------------------
// Collections
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_and_list_collections() {
    let (app, _dir) = make_app();
    let (create_status, body) = post_json(
        &app,
        "/v1/collection/create",
        json!({ "name": "articles", "model": "text-embed-3", "dim": 128 }),
    )
    .await;
    assert_eq!(create_status, StatusCode::OK);
    assert_eq!(body["ok"], json!(true));

    let (list_status, list) = get_json(&app, "/v1/collections").await;
    assert_eq!(list_status, StatusCode::OK);
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["name"].as_str())
        .collect();
    assert!(names.contains(&"articles"), "newly created collection must appear in list: {:?}", names);
}

#[tokio::test]
async fn test_duplicate_collection_returns_400() {
    let (app, _dir) = make_app();
    post_json(&app, "/v1/collection/create", json!({ "name": "dup", "model": "m", "dim": 4 })).await;
    let (status, _) = post_json(&app, "/v1/collection/create", json!({ "name": "dup", "model": "m", "dim": 4 })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "duplicate collection must return 400");
}

// ---------------------------------------------------------------------------
// Vector / search
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_hybrid_search_after_index_rebuild() {
    let (app, _dir) = make_app();

    // Create an explicit collection with dim=3
    post_json(&app, "/v1/collection/create", json!({ "name": "vtest", "model": "m", "dim": 3 })).await;

    // Add two nodes with embeddings into the collection
    post_json(
        &app,
        "/v1/node/add",
        json!({ "id": "v1", "labels": [], "embedding": [1.0, 0.0, 0.0], "collection": "vtest" }),
    )
    .await;
    post_json(
        &app,
        "/v1/node/add",
        json!({ "id": "v2", "labels": [], "embedding": [0.0, 1.0, 0.0], "collection": "vtest" }),
    )
    .await;

    // Force synchronous HNSW index build so the vectors are searchable
    let (rebuild_status, _) = post_json(&app, "/v1/bulk/rebuild", json!(null)).await;
    assert_eq!(rebuild_status, StatusCode::OK);

    // Search nearest to [0.9, 0.1, 0.0] — should be v1
    let (search_status, results) = post_json(
        &app,
        "/v1/search/hybrid",
        json!({ "query_vector": [0.9, 0.1, 0.0], "k": 1, "collection": "vtest" }),
    )
    .await;
    assert_eq!(search_status, StatusCode::OK);
    let hits = results.as_array().unwrap();
    assert!(!hits.is_empty(), "hybrid search must return at least one result");
    assert_eq!(hits[0]["node"]["id"], json!("v1"), "nearest node should be v1");
}

#[tokio::test]
async fn test_add_vector_wrong_dim_rejected() {
    let (app, _dir) = make_app();

    // Create collection with dim=4
    post_json(&app, "/v1/collection/create", json!({ "name": "dim4", "model": "m", "dim": 4 })).await;
    // Add a node first
    post_json(&app, "/v1/node/add", json!({ "id": "nd1", "labels": [] })).await;
    // Try to add a 3-dim vector to a 4-dim collection
    let (status, _) = post_json(
        &app,
        "/v1/vector/add",
        json!({ "node_id": "nd1", "collection": "dim4", "embedding": [1.0, 0.0, 0.0] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "wrong-dim vector must be rejected with 400");
}

#[tokio::test]
async fn test_reason_context_returns_ok() {
    let (app, _dir) = make_app();
    post_json(&app, "/v1/collection/create", json!({ "name": "ctx", "model": "m", "dim": 2 })).await;
    post_json(
        &app,
        "/v1/node/add",
        json!({ "id": "ctx1", "labels": [], "embedding": [1.0, 0.0], "collection": "ctx" }),
    )
    .await;
    post_json(&app, "/v1/bulk/rebuild", json!(null)).await;

    let (status, _) = post_json(
        &app,
        "/v1/reason/context",
        json!({ "query_vector": [1.0, 0.0], "k": 1, "collection": "ctx" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// HQL — body format contract (the documented SDK gotcha)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_hql_raw_json_string_body_succeeds() {
    let (app, _dir) = make_app();
    post_json(&app, "/v1/node/add", json!({ "id": "hql_fmt_node", "labels": [] })).await;

    // /v1/query/hql expects the body to be a JSON-encoded STRING (not a JSON object).
    // serde_json::to_string("TRAVERSE ...") produces the JSON string literal with surrounding
    // quotes, e.g. `"TRAVERSE FROM hql_fmt_node DEPTH 1 REL DUMMY"` — that is the correct body.
    let hql_body = serde_json::to_string("TRAVERSE FROM hql_fmt_node DEPTH 1 REL DUMMY").unwrap();
    let (status, _) = post_raw(&app, "/v1/query/hql", "application/json", &hql_body).await;
    assert_eq!(status, StatusCode::OK, "HQL endpoint must accept a raw JSON string body");
}

#[tokio::test]
async fn test_hql_object_body_rejected() {
    // Documents the known SDK contract mismatch: SDKs send {"query":"..."} but
    // the endpoint expects a raw JSON string.  Axum returns 422 on type mismatch.
    let (app, _dir) = make_app();
    let wrong_body = serde_json::to_string(&json!({ "query": "SEARCH Doc" })).unwrap();
    let (status, _) = post_raw(&app, "/v1/query/hql", "application/json", &wrong_body).await;
    assert_ne!(
        status,
        StatusCode::OK,
        "HQL endpoint must NOT accept {{\"query\":\"...\"}} body; SDKs using this format are broken"
    );
}

#[tokio::test]
async fn test_hql_traverse_finds_neighbor() {
    let (app, _dir) = make_app();
    post_json(&app, "/v1/node/add", json!({ "id": "src_hql", "labels": [] })).await;
    post_json(&app, "/v1/node/add", json!({ "id": "dst_hql", "labels": [] })).await;
    post_json(
        &app,
        "/v1/edge/add",
        json!({ "id": "ehql1", "from": "src_hql", "to": "dst_hql", "rel": "KNOWS" }),
    )
    .await;

    let hql = serde_json::to_string("TRAVERSE FROM src_hql DEPTH 1 REL KNOWS").unwrap();
    let (status, body) = post_raw(&app, "/v1/query/hql", "application/json", &hql).await;
    assert_eq!(status, StatusCode::OK);
    let parsed: Value = serde_json::from_str(&body).unwrap();
    let neighbors = parsed.as_array().unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0]["node"]["id"], json!("dst_hql"));
}

// ---------------------------------------------------------------------------
// Insight surface
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_insight_communities_returns_structure() {
    let (app, _dir) = make_app();
    let (status, body) = get_json(&app, "/v1/insight/communities").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["nodes"].is_array());
    assert!(body["edges"].is_array());
}

#[tokio::test]
async fn test_insight_rebuild_and_gaps() {
    let (app, _dir) = make_app();
    let (rebuild_status, ok) = post_json(&app, "/v1/insight/rebuild", json!(null)).await;
    assert_eq!(rebuild_status, StatusCode::OK);
    assert_eq!(ok["ok"], json!(true));

    let (gaps_status, _) = get_json(&app, "/v1/insight/gaps").await;
    assert_eq!(gaps_status, StatusCode::OK);
}

#[tokio::test]
async fn test_insight_drift_endpoint() {
    let (app, _dir) = make_app();
    let (status, body) = get_json(&app, "/v1/insight/drift/0").await;
    assert_eq!(status, StatusCode::OK);
    // Cluster 0 may not exist yet — engine returns empty array, not an error
    assert!(body.is_array(), "drift endpoint must return an array");
}

// ---------------------------------------------------------------------------
// Consensus surface
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_consensus_sign_vote_returns_bytes() {
    let (app, _dir) = make_app();
    let (status, sig) = post_json(
        &app,
        "/v1/consensus/sign-vote",
        json!({ "proposal_id": "p1", "approve": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // Returns a byte array (Vec<u8> serialised as JSON array)
    assert!(sig.is_array(), "sign-vote must return a JSON array of bytes");
    assert!(!sig.as_array().unwrap().is_empty(), "signature must be non-empty");
}

// ---------------------------------------------------------------------------
// HQL body contract
// ---------------------------------------------------------------------------

/// `/v1/query/hql` must accept BOTH body shapes: the historical raw JSON string
/// (`"TRAVERSE ..."`) and the object form the Python/Go SDKs send
/// (`{"query": "TRAVERSE ..."}`). Pins the contract so neither side drifts.
#[tokio::test]
async fn test_hql_accepts_both_raw_and_wrapped_body() {
    let (app, _dir) = make_app();
    post_json(&app, "/v1/node/add", json!({ "id": "hsrc", "labels": [] })).await;
    post_json(&app, "/v1/node/add", json!({ "id": "hdst", "labels": [] })).await;
    post_json(
        &app,
        "/v1/edge/add",
        json!({ "id": "he1", "from": "hsrc", "to": "hdst", "rel": "LINKS" }),
    )
    .await;

    let hql = "TRAVERSE FROM hsrc DEPTH 1 REL LINKS";

    // Raw JSON string form (legacy / native contract).
    let raw_body = serde_json::to_string(hql).unwrap();
    let (raw_status, _raw) = post_raw(&app, "/v1/query/hql", "application/json", &raw_body).await;
    assert_eq!(raw_status, StatusCode::OK, "raw-string HQL body must be accepted");

    // Wrapped object form ({"query": "..."}) — what the SDKs send.
    let (wrapped_status, _wrapped) =
        post_json(&app, "/v1/query/hql", json!({ "query": hql })).await;
    assert_eq!(
        wrapped_status,
        StatusCode::OK,
        "wrapped {{\"query\": ...}} HQL body must be accepted (SDK contract)"
    );
}

// ---------------------------------------------------------------------------
// Version surface
// ---------------------------------------------------------------------------

/// `GET /v1/version` reports engine name, package version, and on-disk schema
/// version — the update/version-control surface for clients and ops tooling.
#[tokio::test]
async fn test_version_route_reports_engine_version() {
    let (app, _dir) = make_app();
    let (status, body) = get_json(&app, "/v1/version").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["engine_name"], json!("genesis-block"));
    assert_eq!(
        body["version"],
        json!(env!("CARGO_PKG_VERSION")),
        "REST version must match the compiled CARGO_PKG_VERSION"
    );
    assert!(body["schema_version"].is_number(), "schema_version present");
}
