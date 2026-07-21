use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use genesis_block_native::router::{build_router, AppState};
use genesis_block_native::{
    EdgeInput, NodeInput, OpenOptions, Storage, StudioGraphSceneRequest, STUDIO_SCENE_PAGE_LIMIT,
};
use http_body_util::BodyExt;
use parking_lot::RwLock;
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

fn storage(dir: &TempDir) -> Storage {
    Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
    })
    .unwrap()
}

fn seed_graph(storage: &Storage) {
    for (id, title, labels) in [
        ("c", "Charlie", vec!["Artifact"]),
        ("a", "Alpha", vec!["Memory"]),
        ("b", "Bravo", vec!["Agent"]),
    ] {
        storage
            .add_node(NodeInput {
                id: Some(id.to_string()),
                labels: labels.into_iter().map(str::to_string).collect(),
                props: Some(json!({"title": title})),
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
    }
    for (id, from, to) in [("e1", "a", "b"), ("e2", "b", "c")] {
        storage
            .add_edge(EdgeInput {
                id: Some(id.to_string()),
                from: from.to_string(),
                to: to.to_string(),
                rel: "LINKS".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            })
            .unwrap();
    }
}

#[test]
fn bounded_scene_is_deterministic_and_paginated() {
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir);
    seed_graph(&storage);

    let first = storage
        .studio_graph_scene(StudioGraphSceneRequest {
            limit: Some(2),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        first
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert!(first.truncated);
    assert_eq!(first.continuation.as_deref(), Some("2"));
    assert_eq!(first.frontier, storage.stable_frontier());

    let second = storage
        .studio_graph_scene(StudioGraphSceneRequest {
            limit: Some(2),
            offset: Some(2),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(second.nodes[0].id, "c");
    assert!(!second.truncated);
    assert!(second.continuation.is_none());
}

#[test]
fn scene_and_entity_contracts_never_expose_embeddings() {
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir);
    seed_graph(&storage);

    let scene = storage
        .studio_graph_scene(StudioGraphSceneRequest {
            seed: Some("b".to_string()),
            limit: Some(100),
            direction: Some("both".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(scene.nodes.len(), 3);
    assert_eq!(scene.edges.len(), 2);
    assert!(!serde_json::to_string(&scene).unwrap().contains("embedding"));

    let inspection = storage.studio_inspect_entity("b").unwrap();
    assert_eq!(inspection.incident_edges.len(), 2);
    assert_eq!(inspection.availability["vector"], "not_present");
    assert!(!serde_json::to_string(&inspection)
        .unwrap()
        .contains("embedding"));
}

#[test]
fn studio_read_hql_rejects_invalid_commands_without_a_write_fallback() {
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir);
    seed_graph(&storage);

    assert!(storage
        .execute_hql_read_only("TRAVERSE FROM a DEPTH 1 REL LINKS")
        .is_ok());
    assert!(storage.execute_hql_read_only("DROP EVERYTHING").is_err());
    assert!(storage
        .studio_graph_scene(StudioGraphSceneRequest {
            limit: Some(STUDIO_SCENE_PAGE_LIMIT + 1),
            ..Default::default()
        })
        .is_err());
}

#[test]
fn seeded_scene_respects_the_requested_temporal_view() {
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir);
    storage
        .add_node(NodeInput {
            id: Some("future".to_string()),
            labels: vec!["Event".to_string()],
            props: Some(json!({"title": "Not valid yet"})),
            embedding: None,
            lang: None,
            valid_from: Some("2999-01-01T00:00:00Z".to_string()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    let global = storage
        .studio_graph_scene(StudioGraphSceneRequest::default())
        .unwrap();
    assert!(global.nodes.is_empty());
    assert!(storage
        .studio_graph_scene(StudioGraphSceneRequest {
            seed: Some("future".to_string()),
            ..Default::default()
        })
        .is_err());
}

async fn response_json(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

#[tokio::test]
async fn studio_rest_routes_are_bounded_and_api_key_guarded() {
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir);
    seed_graph(&storage);
    let app = build_router(AppState {
        storage: Arc::new(RwLock::new(storage)),
        api_key: Some("studio-secret".to_string()),
    });

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/studio/capabilities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let capabilities = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/studio/capabilities")
                .header("authorization", "Bearer studio-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, body) = response_json(capabilities).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["protocol_version"], "1");
    assert_eq!(body["write_features"], json!([]));
    assert_eq!(body["auth_features"], json!(["api-key"]));

    let oversized = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/v1/studio/graph?limit={}",
                    STUDIO_SCENE_PAGE_LIMIT + 1
                ))
                .header("authorization", "Bearer studio-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(oversized.status(), StatusCode::BAD_REQUEST);
}
