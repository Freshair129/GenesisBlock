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

fn make_app(capacity: usize) -> (Router, Arc<RwLock<Storage>>, TempDir) {
    let dir = TempDir::new().unwrap();
    let storage = Arc::new(RwLock::new(
        Storage::open(OpenOptions {
            path: dir.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(3),
            retention: None,
        })
        .unwrap(),
    ));
    let app = build_router(AppState {
        storage: storage.clone(),
        api_key: Some("coverage-test-key".into()),
        query_admission: Arc::new(tokio::sync::Semaphore::new(capacity)),
    });
    (app, storage, dir)
}

async fn request(
    app: &Router,
    key: Option<&str>,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("POST").uri(path);
    if let Some(key) = key {
        builder = builder.header("authorization", format!("Bearer {key}"));
    }
    let body = match body {
        Some(body) => {
            builder = builder.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn coverage_requires_authentication_and_query_admission() {
    let (app, storage, _dir) = make_app(0);
    let frontier = storage.read().stable_frontier();
    for key in [None, Some("wrong-key")] {
        let (status, _) = request(&app, key, "/v1/index/coverage/validate", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, body) = request(
        &app,
        Some("coverage-test-key"),
        "/v1/index/coverage/validate",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["code"], "QUERY_ADMISSION_REJECTED");
    assert_eq!(storage.read().stable_frontier(), frontier);
    assert!(storage
        .read()
        .list_collections()
        .iter()
        .all(|c| !c.coverage.validated));
}

#[tokio::test]
async fn coverage_drains_and_validates_all_collections_without_durable_writes() {
    let (app, storage, _dir) = make_app(1);
    let key = Some("coverage-test-key");
    assert_eq!(
        request(
            &app,
            key,
            "/v1/collection/create",
            Some(json!({
                "name":"semantic", "model":"fixture", "dim":3, "metric":"cosine"
            }))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(request(&app, key, "/v1/transaction/commit", Some(json!({
        "transaction_id":"00000000-0000-4000-8000-000000000401",
        "expected_frontier":0, "relational":[], "vectors":[],
        "graph":{"nodes":[
            {"id":"chunk-a", "labels":["chunk"], "collection":"semantic", "embedding":[1.0,0.0,0.0]},
            {"id":"chunk-b", "labels":["chunk"], "collection":"semantic", "embedding":[0.0,1.0,0.0]}
        ],"edges":[]}
    }))).await.0, StatusCode::OK);
    let frontier =
        json!({"frame":storage.read().stable_frontier(), "txn":storage.read().txn_frontier()});
    assert!(storage
        .read()
        .list_collections()
        .iter()
        .all(|c| !c.coverage.validated));
    let (status, body) = request(&app, key, "/v1/index/coverage/validate", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["frontier_before"], frontier);
    assert_eq!(body["frontier_after"], frontier);
    let reports = body["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0]["collection"], "default");
    assert_eq!(reports[0]["source_count"], 0);
    assert_eq!(reports[1]["collection"], "semantic");
    assert_eq!(reports[1]["source_count"], 2);
    for report in reports {
        assert_eq!(report["state"], "READY");
        assert_eq!(report["validated"], true);
        assert_eq!(report["source_count"], report["indexed_count"]);
        assert_eq!(report["source_frontier"], report["built_frontier"]);
        for count in ["missing_count", "extra_count", "pending_count"] {
            assert_eq!(report[count], 0);
        }
    }
    let (status, repeat) = request(&app, key, "/v1/index/coverage/validate", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(repeat, body);
    assert_eq!(
        storage.read().stable_frontier(),
        frontier["frame"].as_u64().unwrap()
    );
    assert_eq!(
        storage.read().txn_frontier(),
        frontier["txn"].as_u64().unwrap()
    );

    assert_eq!(request(&app, key, "/v1/node/add", Some(json!({
        "id":"chunk-c", "labels":["chunk"], "collection":"semantic", "embedding":[0.0,0.0,1.0]
    }))).await.0, StatusCode::OK);
    let stale = storage
        .read()
        .list_collections()
        .into_iter()
        .find(|c| c.name == "semantic")
        .unwrap();
    assert!(!stale.coverage.validated);
    let (status, updated) = request(&app, key, "/v1/index/coverage/validate", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(updated["frontier_after"], frontier);
    assert_eq!(updated["reports"][1]["source_count"], 3);
    assert_eq!(updated["reports"][1]["validated"], true);
}
