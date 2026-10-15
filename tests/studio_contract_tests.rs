use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use genesis_block_native::router::{build_router, AppState};
use genesis_block_native::{
    EdgeInput, NamedQueryDefinition, NamedQueryRequest, NodeInput, OpenOptions, RelationalColumn,
    RelationalColumnType, RelationalMutationBatch, RelationalMutationKind, RelationalQuery,
    RelationalRowMutation, RelationalSchemaPackage, RelationalTable, Storage,
    StudioGraphSceneRequest, STUDIO_SCENE_PAGE_LIMIT,
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
        retention: None,
    })
    .unwrap()
}

fn read_only_storage(dir: &TempDir) -> Storage {
    Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(true),
        vector_dim: Some(4),
        retention: None,
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

fn seed_relational_fixture(storage: &Storage) {
    storage
        .register_relational_schema(RelationalSchemaPackage {
            namespace: "studio_fixture".to_string(),
            schema_version: 1,
            previous_version: None,
            package_id: "00000000-0000-4000-8000-000000000301".to_string(),
            schema_hash: String::new(),
            tables: vec![RelationalTable {
                name: "projects".to_string(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::EntityId),
                    RelationalColumn::required("name", RelationalColumnType::Text),
                ],
                primary_key: vec!["id".to_string()],
                foreign_keys: vec![],
                indexes: vec![],
            }],
            named_queries: vec![NamedQueryDefinition {
                name: "list_projects".to_string(),
                parameters: vec![],
                query: RelationalQuery {
                    namespace: "studio_fixture".to_string(),
                    table: "projects".to_string(),
                    columns: vec!["projects.id".to_string(), "projects.name".to_string()],
                    joins: vec![],
                    filters: vec![],
                    limit: None,
                    offset: None,
                },
                default_limit: 10,
                max_limit: 20,
            }],
        })
        .unwrap();
    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: "00000000-0000-4000-8000-000000000302".to_string(),
            namespace: "studio_fixture".to_string(),
            schema_version: 1,
            operations: vec![RelationalRowMutation {
                table: "projects".to_string(),
                kind: RelationalMutationKind::Insert,
                values: json!({"id":"project-a","name":"Studio fixture project"}),
                key: None,
            }],
        })
        .unwrap();
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

    let view = Some("2026-08-01T00:00:00Z".to_string());
    let global = storage
        .studio_graph_scene(StudioGraphSceneRequest {
            as_of: view.clone(),
            ..Default::default()
        })
        .unwrap();
    assert!(global.nodes.is_empty());
    assert!(storage
        .studio_graph_scene(StudioGraphSceneRequest {
            seed: Some("future".to_string()),
            as_of: view,
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

async fn get_json(app: &axum::Router, uri: &str) -> Value {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (status, body) = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    body
}

async fn post_json(app: &axum::Router, uri: &str, request: Value) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, body) = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    body
}

#[tokio::test]
async fn local_and_remote_studio_reads_match_on_the_same_persisted_fixture() {
    let dir = tempfile::Builder::new()
        .prefix("Genesis Studio parity - 雪 ")
        .tempdir()
        .unwrap();
    let writer = storage(&dir);
    seed_graph(&writer);
    seed_relational_fixture(&writer);
    drop(writer);

    // The Tauri commands open Storage read-only. The REST server cannot own the
    // same data root at the same time, so exercise the two boundaries
    // sequentially against this one persisted fixture.
    let local_storage = read_only_storage(&dir);
    let local_capabilities =
        local_storage.studio_capabilities("local", vec!["os-user".to_string()]);
    let local_collections = serde_json::to_value(local_storage.list_collections()).unwrap();
    let local_schemas =
        serde_json::to_value(local_storage.list_relational_schemas().unwrap()).unwrap();
    let local_graph = serde_json::to_value(
        local_storage
            .studio_graph_scene(StudioGraphSceneRequest {
                limit: Some(240),
                ..Default::default()
            })
            .unwrap(),
    )
    .unwrap();
    let local_inspection =
        serde_json::to_value(local_storage.studio_inspect_entity("b").unwrap()).unwrap();
    let local_hql = local_storage
        .execute_hql_read_only("TRAVERSE FROM a DEPTH 1 REL LINKS")
        .unwrap();
    let named_query = NamedQueryRequest {
        namespace: "studio_fixture".to_string(),
        schema_version: 1,
        query_name: "list_projects".to_string(),
        parameters: json!({}),
        limit: None,
    };
    let local_named_query = local_storage
        .execute_named_query(named_query.clone())
        .unwrap();
    let local_status = json!({
        "node_count": local_storage.nodes.len(),
        "edge_count": local_storage.edges.len(),
        "collection_count": local_storage.collections.len(),
        "index_lag": local_storage.index_lag(),
        "frontier": local_storage.stable_frontier(),
    });
    drop(local_storage);

    let remote_storage = read_only_storage(&dir);
    let remote_frontier = remote_storage.stable_frontier();
    let app = build_router(AppState {
        storage: Arc::new(RwLock::new(remote_storage)),
        api_key: None,
        query_admission: Arc::new(tokio::sync::Semaphore::new(8)),
    });

    let remote_capabilities = get_json(&app, "/v1/studio/capabilities").await;
    assert_eq!(remote_capabilities["mode"], "remote");
    assert_eq!(local_capabilities.mode, "local");
    assert_eq!(
        remote_capabilities["read_features"],
        json!(local_capabilities.read_features)
    );
    assert_eq!(
        remote_capabilities["write_features"],
        json!(local_capabilities.write_features)
    );
    assert_eq!(
        remote_capabilities["limits"],
        json!(local_capabilities.limits)
    );
    assert_eq!(
        remote_capabilities["consistency"],
        local_capabilities.consistency
    );

    let remote_collections = get_json(&app, "/v1/collections").await;
    assert_eq!(remote_collections, local_collections);
    let remote_schemas = get_json(&app, "/v1/studio/relational/schemas").await;
    assert_eq!(remote_schemas, local_schemas);

    let mut local_graph = local_graph;
    let mut remote_graph = get_json(&app, "/v1/studio/graph?limit=240").await;
    // Scene IDs are request-local correlation IDs and intentionally differ.
    local_graph.as_object_mut().unwrap().remove("scene_id");
    remote_graph.as_object_mut().unwrap().remove("scene_id");
    assert_eq!(remote_graph, local_graph);

    let remote_inspection = get_json(&app, "/v1/studio/entity/b").await;
    assert_eq!(remote_inspection, local_inspection);
    let remote_hql = post_json(
        &app,
        "/v1/studio/query/read",
        json!({"query":"TRAVERSE FROM a DEPTH 1 REL LINKS"}),
    )
    .await;
    assert_eq!(remote_hql, local_hql);
    let remote_named_query = post_json(
        &app,
        "/v1/relational/query",
        serde_json::to_value(named_query).unwrap(),
    )
    .await;
    assert_eq!(remote_named_query, json!(local_named_query));

    let remote_status = get_json(&app, "/v1/status").await;
    assert_eq!(remote_status["node_count"], local_status["node_count"]);
    assert_eq!(remote_status["edge_count"], local_status["edge_count"]);
    assert_eq!(
        remote_status["collections"].as_array().unwrap().len(),
        local_status["collection_count"]
    );
    assert_eq!(remote_status["index_lag"], local_status["index_lag"]);
    assert_eq!(
        get_json(&app, "/v1/frontier").await["frame"],
        remote_frontier
    );
    assert_eq!(remote_frontier, local_status["frontier"]);
}

#[tokio::test]
async fn studio_rest_routes_are_bounded_and_api_key_guarded() {
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir);
    seed_graph(&storage);
    let app = build_router(AppState {
        storage: Arc::new(RwLock::new(storage)),
        api_key: Some("studio-secret".to_string()),
        query_admission: Arc::new(tokio::sync::Semaphore::new(8)),
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
