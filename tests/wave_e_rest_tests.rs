use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use genesis_block_native::router::{build_router, AppState};
use genesis_block_native::{NodeInput, OpenOptions, Storage};
use http_body_util::BodyExt;
use parking_lot::RwLock;
use serde_json::{json, Value};
use tempfile::tempdir;
use tokio::sync::Semaphore;
use tower::ServiceExt;

async fn call(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn app(storage: Storage) -> axum::Router {
    build_router(AppState {
        storage: Arc::new(RwLock::new(storage)),
        api_key: None,
        query_admission: Arc::new(Semaphore::new(1)),
    })
}

fn node(id: &str) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec!["ENTITY".to_string()],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

#[tokio::test]
async fn query_ir_context_and_capability_details_are_exposed() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();
    storage.add_node(node("rest-ctx")).unwrap();
    let app = app(storage);

    let (status, body) = call(
        &app,
        Request::post("/v1/query/ir")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&json!({
                    "contract_version": "query-ir.v1",
                    "request_id": "rest-ctx-1",
                    "operation": {
                        "kind": "context",
                        "target_id": "rest-ctx",
                        "tier": "H0"
                    }
                }))
                .unwrap(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["operation_kind"], "context");
    assert_eq!(body["data"]["nodes"][0]["id"], "rest-ctx");

    let capabilities = app
        .clone()
        .oneshot(
            Request::get("/v1/query/ir/capabilities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let capability_body: Value =
        serde_json::from_slice(&capabilities.into_body().collect().await.unwrap().to_bytes())
            .unwrap();
    assert_eq!(capability_body["operations"]["context"], "implemented");
    assert_eq!(
        capability_body["operation_details"]["search"]["filters"],
        "unsupported"
    );
}

#[tokio::test]
async fn query_ir_context_rejects_temporal_selector_with_typed_error() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();
    let app = app(storage);
    let (status, body) = call(
        &app,
        Request::post("/v1/query/ir")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&json!({
                    "contract_version": "query-ir.v1",
                    "request_id": "rest-ctx-temporal",
                    "temporal": { "valid_at": "2026-01-01T00:00:00Z" },
                    "operation": {
                        "kind": "context",
                        "target_id": "missing",
                        "tier": "H0"
                    }
                }))
                .unwrap(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "QUERY_CAPABILITY_UNSUPPORTED");
}
