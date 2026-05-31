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
    Storage, OpenOptions, NodeInput, EdgeInput, QueryInput, HybridSearchInput
};

#[derive(Clone)]
struct AppState {
    storage: Arc<RwLock<Storage>>,
}

async fn add_node_handler(
    State(state): State<AppState>,
    Json(input): Json<NodeInput>,
) -> impl IntoResponse {
    let mut storage = state.storage.write();
    match storage.add_node(input) {
        Ok(node) => (StatusCode::OK, Json(node)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn add_edge_handler(
    State(state): State<AppState>,
    Json(input): Json<EdgeInput>,
) -> impl IntoResponse {
    let mut storage = state.storage.write();
    match storage.add_edge(input) {
        Ok(edge) => (StatusCode::OK, Json(edge)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn query_handler(
    State(state): State<AppState>,
    Json(input): Json<QueryInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
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
    match storage.hybrid_search(input) {
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
    })?;

    let state = AppState {
        storage: Arc::new(RwLock::new(storage)),
    };

    let app = Router::new()
        .route("/v1/node/add", post(add_node_handler))
        .route("/v1/edge/add", post(add_edge_handler))
        .route("/v1/query", post(query_handler))
        .route("/v1/search/hybrid", post(hybrid_search_handler))
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
