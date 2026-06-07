use std::net::SocketAddr;
use std::sync::Arc;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use parking_lot::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Import core engine from the library
use genesis_block_native::{
    Storage, OpenOptions, NodeInput, EdgeInput, QueryInput, HybridSearchInput, Event
};

#[derive(Clone)]
struct AppState {
    storage: Arc<RwLock<Storage>>,
}

#[derive(serde::Deserialize)]
struct VoteInput {
    pub proposal_id: String,
    pub peer_id: String,
    pub approve: bool,
}

#[derive(serde::Deserialize)]
struct ProposalInput {
    pub event: Event,
    pub signature: Vec<u8>,
}

async fn consensus_propose_handler(
    State(state): State<AppState>,
    Json(input): Json<ProposalInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    match storage.propose_consensus(input.event, input.signature) {
        Ok(id) => (StatusCode::OK, Json(id)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn consensus_vote_handler(
    State(state): State<AppState>,
    Json(input): Json<VoteInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    match storage.submit_vote(input.proposal_id, input.peer_id, input.approve) {
        Ok(reached_quorum) => (StatusCode::OK, Json(reached_quorum)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn consensus_verify_handler(
    State(state): State<AppState>,
    Json(event): Json<Event>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    match storage.semantic_verify(&event) {
        Ok(is_valid) => (StatusCode::OK, Json(is_valid)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn bulk_add_nodes_handler(
    State(state): State<AppState>,
    Json(inputs): Json<Vec<NodeInput>>,
) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.bulk_add_nodes(inputs) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn bulk_add_edges_handler(
    State(state): State<AppState>,
    Json(inputs): Json<Vec<EdgeInput>>,
) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.bulk_add_edges(inputs) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn rebuild_index_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.rebuild_index_parallel() {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn add_node_handler(
    State(state): State<AppState>,
    Json(input): Json<NodeInput>,
) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.add_node(input) {
        Ok(node) => (StatusCode::OK, Json(node)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn add_edge_handler(
    State(state): State<AppState>,
    Json(input): Json<EdgeInput>,
) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.add_edge(input) {
        Ok(edge) => (StatusCode::OK, Json(edge)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct SupersedeInput {
    pub id: String,
    pub new_props: Option<serde_json::Value>,
    pub caused_by: Option<String>,
}

async fn supersede_node_handler(
    State(state): State<AppState>,
    Json(input): Json<SupersedeInput>,
) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.supersede_node(input.id, input.new_props, input.caused_by) {
        Ok(node) => (StatusCode::OK, Json(node)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_meta_history_handler(
    State(state): State<AppState>,
    axum::extract::Path(cluster_id): axum::extract::Path<u32>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    let history = storage.get_meta_history(cluster_id);
    (StatusCode::OK, Json(history)).into_response()
}

async fn execute_hql_handler(
    State(state): State<AppState>,
    Json(query): Json<String>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    if storage.is_rebuilding.load(std::sync::atomic::Ordering::SeqCst) {
        return (StatusCode::SERVICE_UNAVAILABLE, "Engine is rebuilding index...").into_response();
    }
    match storage.execute_hql(&query) {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn query_handler(
    State(state): State<AppState>,
    Json(input): Json<QueryInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    if storage.is_rebuilding.load(std::sync::atomic::Ordering::SeqCst) {
        return (StatusCode::SERVICE_UNAVAILABLE, "Engine is rebuilding index...").into_response();
    }
    match storage.query(input) {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn hybrid_search_handler(
    State(state): State<AppState>,
    Json(input): Json<HybridSearchInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    if storage.is_rebuilding.load(std::sync::atomic::Ordering::SeqCst) {
        return (StatusCode::SERVICE_UNAVAILABLE, "Engine is rebuilding index...").into_response();
    }
    match storage.hybrid_search(input) {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn ranked_context_handler(
    State(state): State<AppState>,
    Json(input): Json<HybridSearchInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    if storage.is_rebuilding.load(std::sync::atomic::Ordering::SeqCst) {
        return (StatusCode::SERVICE_UNAVAILABLE, "Engine is rebuilding index...").into_response();
    }
    match storage.get_ranked_context(input) {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn status_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    Json(storage.status_sync())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "genesis_db_server=info,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let data_dir = std::env::var("GENESIS_DATA_DIR").unwrap_or_else(|_| ".brain/gks/storage".into());
    let port: u16 = std::env::var("GENESIS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let storage = Storage::open(OpenOptions { 
        path: data_dir,
        page_cache_mb: Some(64),
        read_only: Some(false),
     vector_dim: None, })?;

    let state = AppState {
        storage: Arc::new(RwLock::new(storage)),
    };

    let app = Router::new()
        .route("/v1/bulk/nodes", post(bulk_add_nodes_handler))
        .route("/v1/bulk/edges", post(bulk_add_edges_handler))
        .route("/v1/bulk/rebuild", post(rebuild_index_handler))
        .route("/v1/query/hql", post(execute_hql_handler))
        .route("/v1/node/add", post(add_node_handler))
        .route("/v1/node/supersede", post(supersede_node_handler))
        .route("/v1/edge/add", post(add_edge_handler))
        .route("/v1/insight/drift/:cluster_id", get(get_meta_history_handler))
        .route("/v1/query", post(query_handler))
        .route("/v1/search/hybrid", post(hybrid_search_handler))
        .route("/v1/reason/context", post(ranked_context_handler))
        .route("/v1/status", get(status_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("GenesisDB Standalone Server listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
