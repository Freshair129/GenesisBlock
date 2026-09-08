use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use genesis_block_native::router::{build_router, AppState};
use genesis_block_native::{OpenOptions, Storage};
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

#[tokio::test]
async fn query_admission_rejection_does_not_block_status() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: None,
    })
    .unwrap();
    let admission = Arc::new(Semaphore::new(1));
    let app = build_router(AppState {
        storage: Arc::new(RwLock::new(storage)),
        api_key: None,
        query_admission: admission.clone(),
    });
    let held = admission.try_acquire_owned().unwrap();

    let (query_status, query_body) = call(
        &app,
        Request::post("/v1/query/ir")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&json!({
                    "contract_version": "query-ir.v1",
                    "request_id": "admission-rejected",
                    "operation": {
                        "kind": "traverse",
                        "seed_id": "missing",
                        "depth": 1,
                        "relations": ["LINK"],
                        "direction": "out"
                    }
                }))
                .unwrap(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(query_status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(query_body["code"], "QUERY_ADMISSION_REJECTED");

    let (status, _) = call(
        &app,
        Request::get("/v1/status").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    drop(held);
}

#[tokio::test]
async fn hql_budget_envelope_returns_typed_exhaustion() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();
    let app = build_router(AppState {
        storage: Arc::new(RwLock::new(storage)),
        api_key: None,
        query_admission: Arc::new(Semaphore::new(1)),
    });

    let (status, body) = call(
        &app,
        Request::post("/v1/query/hql")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&json!({
                    "query": "TRAVERSE FROM missing DEPTH 1 REL LINK",
                    "budget": { "max_serialized_bytes": 1 }
                }))
                .unwrap(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "QUERY_BUDGET_EXCEEDED");
    assert_eq!(body["message"], "QUERY_BUDGET_EXCEEDED: reason=bytes");
}
