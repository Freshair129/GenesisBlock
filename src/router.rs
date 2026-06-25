use std::sync::Arc;
use std::sync::atomic::Ordering;
use axum::{
    extract::{DefaultBodyLimit, Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use parking_lot::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::{Storage, NodeInput, EdgeInput, QueryInput, HybridSearchInput, Event, SyncPeer};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<RwLock<Storage>>,
}

// ---------------------------------------------------------------------------
// Local input types (not exposed from the core engine)
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct VoteInput {
    pub proposal_id: String,
    pub peer_id: String,
    pub approve: bool,
    pub signature: Vec<u8>,
}

#[derive(serde::Deserialize)]
struct SignVoteInput {
    pub proposal_id: String,
    pub approve: bool,
}

#[derive(serde::Deserialize)]
struct ProposalInput {
    pub event: Event,
    pub signature: Vec<u8>,
}

#[derive(serde::Deserialize)]
struct RetractEdgeInput {
    pub id: String,
    pub at: Option<String>,
}

#[derive(serde::Deserialize)]
struct CreateCollectionInput {
    pub name: String,
    pub model: String,
    pub dim: u32,
    pub metric: Option<String>,
    pub quant: Option<String>,
    pub ef_search: Option<u32>,
    pub rerank: Option<bool>,
}

#[derive(serde::Deserialize)]
struct AddVectorInput {
    pub node_id: String,
    pub collection: String,
    pub embedding: Vec<f64>,
}

#[derive(serde::Deserialize)]
struct SupersedeInput {
    pub id: String,
    pub new_props: Option<serde_json::Value>,
    pub caused_by: Option<String>,
}

// ---------------------------------------------------------------------------
// Response shape helpers
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct SwarmStatus {
    pub peer_id: String,
    pub logical_clock: u32,
    pub peers: Vec<SyncPeer>,
}

#[derive(serde::Serialize)]
struct ExtendedStatus {
    pub open: bool,
    pub read_only: bool,
    pub page_cache_mb: u32,
    pub node_count: usize,
    pub edge_count: usize,
    pub memory_usage_mb: f64,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

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
    match storage.submit_vote(input.proposal_id, input.peer_id, input.approve, input.signature) {
        Ok(reached_quorum) => (StatusCode::OK, Json(reached_quorum)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn consensus_sign_vote_handler(
    State(state): State<AppState>,
    Json(input): Json<SignVoteInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    let sig = storage.sign_vote(input.proposal_id, input.approve);
    (StatusCode::OK, Json(sig)).into_response()
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

async fn retract_edge_handler(
    State(state): State<AppState>,
    Json(input): Json<RetractEdgeInput>,
) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.retract_edge(input.id, input.at) {
        Ok(Some(edge)) => (StatusCode::OK, Json(edge)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "edge not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_collection_handler(
    State(state): State<AppState>,
    Json(input): Json<CreateCollectionInput>,
) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.create_collection(input.name, input.model, input.dim, input.metric, input.quant, input.ef_search, input.rerank) {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn list_collections_handler(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.read();
    (StatusCode::OK, Json(storage.list_collections())).into_response()
}

async fn add_vector_handler(
    State(state): State<AppState>,
    Json(input): Json<AddVectorInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    match storage.add_vector(input.node_id, input.collection, input.embedding) {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
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

async fn insight_communities_handler(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.read();
    let nodes: Vec<_> = storage.meta_nodes.iter().map(|e| e.value().clone()).collect();
    let edges: Vec<_> = storage.meta_edges.iter().map(|e| e.value().clone()).collect();
    (StatusCode::OK, Json(serde_json::json!({ "nodes": nodes, "edges": edges }))).into_response()
}

async fn insight_gaps_handler(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.read();
    match storage.calculate_structural_gaps() {
        Ok(gaps) => (StatusCode::OK, Json(gaps)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn insight_rebuild_handler(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.detect_communities().and_then(|_| storage.generate_meta_graph()) {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// HQL request body. Accepts both the historical raw-JSON-string form
/// (`"SEARCH ..."`) and the object form the Python/Go SDKs send
/// (`{"query": "SEARCH ..."}`), so neither side has to change. Untagged: serde
/// tries each variant in order.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum HqlBody {
    Raw(String),
    Wrapped { query: String },
}

impl HqlBody {
    fn into_query(self) -> String {
        match self {
            HqlBody::Raw(q) => q,
            HqlBody::Wrapped { query } => query,
        }
    }
}

async fn execute_hql_handler(
    State(state): State<AppState>,
    Json(body): Json<HqlBody>,
) -> impl IntoResponse {
    let query = body.into_query();
    let storage = state.storage.read();
    if storage.is_rebuilding.load(Ordering::SeqCst) {
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
    if storage.is_rebuilding.load(Ordering::SeqCst) {
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
    if storage.is_rebuilding.load(Ordering::SeqCst) {
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
    if storage.is_rebuilding.load(Ordering::SeqCst) {
        return (StatusCode::SERVICE_UNAVAILABLE, "Engine is rebuilding index...").into_response();
    }
    match storage.get_ranked_context(input) {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn swarm_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.read();
    let status = SwarmStatus {
        peer_id: storage.local_peer_id.clone(),
        logical_clock: storage.get_logical_clock(),
        peers: storage.peers.iter().map(|e| e.value().clone()).collect(),
    };
    Json(status)
}

async fn status_handler(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.read();
    let base = storage.status_sync();
    let status = ExtendedStatus {
        open: base.open,
        read_only: base.read_only,
        page_cache_mb: base.page_cache_mb,
        node_count: storage.nodes.len(),
        edge_count: storage.edges.len(),
        memory_usage_mb: storage.collections.iter()
            .map(|c| c.value().arena.read().len() * 4)
            .sum::<usize>() as f64 / 1024.0 / 1024.0,
    };
    Json(status)
}

/// Engine version + schema version + stable name. Lets clients and ops tooling
/// query the running version (and on-disk schema version) to decide whether an
/// update is needed. Static — no engine state required.
async fn version_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "engine_name": crate::ENGINE_NAME,
        "version": crate::ENGINE_VERSION,
        "schema_version": crate::SCHEMA_VERSION,
    }))
}

// ---------------------------------------------------------------------------
// Router builder — shared by main.rs and integration tests
// ---------------------------------------------------------------------------

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/bulk/nodes", post(bulk_add_nodes_handler))
        .route("/v1/bulk/edges", post(bulk_add_edges_handler))
        .route("/v1/bulk/rebuild", post(rebuild_index_handler))
        // HQL is a query string; cap the body at 256 KiB so a malformed/huge
        // request can't force a large allocation (defense-in-depth — the bulk
        // routes that legitimately carry embeddings keep the default limit).
        .route(
            "/v1/query/hql",
            post(execute_hql_handler).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route("/v1/node/add", post(add_node_handler))
        .route("/v1/node/supersede", post(supersede_node_handler))
        .route("/v1/edge/add", post(add_edge_handler))
        .route("/v1/edge/retract", post(retract_edge_handler))
        .route("/v1/collection/create", post(create_collection_handler))
        .route("/v1/collections", get(list_collections_handler))
        .route("/v1/vector/add", post(add_vector_handler))
        .route("/v1/insight/drift/:cluster_id", get(get_meta_history_handler))
        .route("/v1/insight/communities", get(insight_communities_handler))
        .route("/v1/insight/gaps", get(insight_gaps_handler))
        .route("/v1/insight/rebuild", post(insight_rebuild_handler))
        .route("/v1/query", post(query_handler))
        .route("/v1/search/hybrid", post(hybrid_search_handler))
        .route("/v1/reason/context", post(ranked_context_handler))
        .route("/v1/status", get(status_handler))
        .route("/v1/version", get(version_handler))
        .route("/v1/swarm/status", get(swarm_status_handler))
        .route("/v1/consensus/propose", post(consensus_propose_handler))
        .route("/v1/consensus/vote", post(consensus_vote_handler))
        .route("/v1/consensus/sign-vote", post(consensus_sign_vote_handler))
        .route("/v1/consensus/verify", post(consensus_verify_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
