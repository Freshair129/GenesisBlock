use axum::{
    extract::{DefaultBodyLimit, Json, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use parking_lot::RwLock;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::{
    BatchInput, CollectionInfo, EdgeInput, Event, HybridSearchInput, NodeInput, QueryInput,
    Storage, SyncPeer,
};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<RwLock<Storage>>,
    /// When set, every request must carry `Authorization: Bearer <key>`.
    /// Leave `None` (the default) for unauthenticated local-only use.
    pub api_key: Option<String>,
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
    pub merkle_root: String,
    pub peers: Vec<SyncPeer>,
}

/// Body for `POST /v1/context/retrieve` — mirrors the NAPI `retrieve_context`
/// signature (GRL tiered retrieval), not `HybridSearchInput` (which backs the
/// separate `/v1/reason/context` ranked-context route).
#[derive(serde::Deserialize)]
struct RetrieveContextInput {
    pub target_id: String,
    pub tier: String,
    pub budget: Option<u32>,
    pub fuzzy: Option<bool>,
}

#[derive(serde::Serialize)]
struct ExtendedStatus {
    pub open: bool,
    pub read_only: bool,
    pub page_cache_mb: u32,
    pub node_count: usize,
    pub edge_count: usize,
    pub memory_usage_mb: f64,
    /// Ops/credibility (P2c): engine-global async-indexing backlog
    /// (`Storage::index_lag()`), reported once at the top level. Also mirrored on
    /// each `collections` entry for convenience.
    pub index_lag: u32,
    /// Ops/credibility (P2c): per-collection quant + residency ops. Each entry is
    /// a `CollectionInfo` carrying `quant`, `count`, `sidecar_resident_bytes`
    /// (≈0 post-P0 — the sidecar is on-disk; this PROVES the RAM win),
    /// `sidecar_disk_bytes`, `arena_resident_bytes`, and `index_lag`.
    pub collections: Vec<CollectionInfo>,
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
    match storage.submit_vote(
        input.proposal_id,
        input.peer_id,
        input.approve,
        input.signature,
    ) {
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

async fn rebuild_index_handler(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.rebuild_index_parallel() {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `Storage::execute_batch` is the core primitive `/v1/bulk/nodes` and
/// `/v1/bulk/edges` are each built on top of internally (mixed nodes+edges in
/// ONE all-or-nothing WAL write / `Event::Batch`) — it previously had no direct
/// REST route (documented gotcha in CLAUDE.md). Mirrors `bulk_add_nodes_handler`'s
/// shape: same error mapping (`INTERNAL_SERVER_ERROR` on a `Result::Err`, since
/// `execute_batch` surfaces both governance and dimension-validation failures
/// through the same `Error` type as `add_node`/`bulk_add_nodes`).
async fn execute_batch_handler(
    State(state): State<AppState>,
    Json(input): Json<BatchInput>,
) -> impl IntoResponse {
    let storage = state.storage.write();
    match storage.execute_batch(input) {
        Ok(output) => (StatusCode::OK, Json(output)).into_response(),
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
    match storage.create_collection(
        input.name,
        input.model,
        input.dim,
        input.metric,
        input.quant,
        input.ef_search,
        input.rerank,
    ) {
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
    let nodes: Vec<_> = storage
        .meta_nodes
        .iter()
        .map(|e| e.value().clone())
        .collect();
    let edges: Vec<_> = storage
        .meta_edges
        .iter()
        .map(|e| e.value().clone())
        .collect();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "nodes": nodes, "edges": edges })),
    )
        .into_response()
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
    match storage
        .detect_communities()
        .and_then(|_| storage.generate_meta_graph())
    {
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
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Engine is rebuilding index...",
        )
            .into_response();
    }
    match storage.execute_hql(&query) {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /v1/query` — filter edges by `from`/`to`, bitemporal **current-view
/// by default**: retracted edges (`/v1/edge/retract`) and edges whose
/// endpoint node has been superseded (`/v1/node/supersede`) out of view are
/// excluded, same visibility rule `TRAVERSE`/`neighbors` use (see
/// `Storage::query` for the exact semantics). Two optional body fields
/// change that, backward-compatibly (absent = current view, unchanged from
/// before this endpoint enforced visibility):
///   - `as_of` (RFC3339 timestamp): time-travel — evaluate visibility at that
///     point in time instead of "now".
///   - `include_invalid: true`: escape hatch that restores the historical
///     raw-scan behavior, surfacing retracted/superseded edges too.
async fn query_handler(
    State(state): State<AppState>,
    Json(input): Json<QueryInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    if storage.is_rebuilding.load(Ordering::SeqCst) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Engine is rebuilding index...",
        )
            .into_response();
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
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Engine is rebuilding index...",
        )
            .into_response();
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
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Engine is rebuilding index...",
        )
            .into_response();
    }
    match storage.get_ranked_context(input) {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn retrieve_context_handler(
    State(state): State<AppState>,
    Json(input): Json<RetrieveContextInput>,
) -> impl IntoResponse {
    let storage = state.storage.read();
    if storage.is_rebuilding.load(Ordering::SeqCst) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Engine is rebuilding index...",
        )
            .into_response();
    }
    match storage.retrieve_context(
        &input.target_id,
        &input.tier,
        input.budget,
        input.fuzzy.unwrap_or(false),
    ) {
        Ok(pkg) => (StatusCode::OK, Json(pkg)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn swarm_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.read();
    let status = SwarmStatus {
        peer_id: storage.local_peer_id.clone(),
        logical_clock: storage.get_logical_clock(),
        merkle_root: storage.get_merkle_root(),
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
        memory_usage_mb: {
            // Vector arenas: actual bytes (f32=4B/elem, SQ8=1B/elem, BQ=1bit/elem).
            let vec_bytes: usize = storage
                .collections
                .iter()
                .map(|c| c.value().arena.read().byte_size())
                .sum();
            // Rough estimates for node/edge heap (props map + DashMap overhead).
            let node_bytes = storage.nodes.len() * 512;
            let edge_bytes = storage.edges.len() * 256;
            (vec_bytes + node_bytes + edge_bytes) as f64 / 1024.0 / 1024.0
        },
        index_lag: storage.index_lag(),
        collections: storage.list_collections(),
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
// Prometheus /metrics (Wave 1.1, ADR--GENESISDB-COMPETITIVE-SUPERIORITY)
// ---------------------------------------------------------------------------

/// Escape a label value per the Prometheus text-format spec: backslash and
/// double-quote must be backslash-escaped, and a literal newline must become
/// `\n`. Collection names are user-supplied, so this is not just defense in
/// depth — an unescaped quote in a name would break the exposition format.
fn escape_label_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
}

/// Render the engine's current state as Prometheus/OpenMetrics text-format
/// exposition. Hand-rolled (no `prometheus` crate dependency) — the format is
/// a handful of `# HELP` / `# TYPE` lines followed by `metric{labels} value`,
/// which is trivial to emit directly from `Storage`'s existing status surface
/// (the same fields `/v1/status` and `list_collections()` already expose).
fn render_metrics(storage: &Storage) -> String {
    let mut out = String::new();

    let node_count = storage.nodes.len();
    let edge_count = storage.edges.len();
    let collections = storage.list_collections();
    let index_lag = storage.index_lag();
    let is_rebuilding = storage.is_rebuilding.load(Ordering::SeqCst);
    let memory_usage_mb: f64 = {
        let vec_bytes: usize = storage
            .collections
            .iter()
            .map(|c| c.value().arena.read().byte_size())
            .sum();
        let node_bytes = storage.nodes.len() * 512;
        let edge_bytes = storage.edges.len() * 256;
        (vec_bytes + node_bytes + edge_bytes) as f64 / 1024.0 / 1024.0
    };

    out.push_str("# HELP genesisdb_nodes_total Total number of nodes currently stored.\n");
    out.push_str("# TYPE genesisdb_nodes_total gauge\n");
    out.push_str(&format!("genesisdb_nodes_total {}\n", node_count));

    out.push_str("# HELP genesisdb_edges_total Total number of edges currently stored.\n");
    out.push_str("# TYPE genesisdb_edges_total gauge\n");
    out.push_str(&format!("genesisdb_edges_total {}\n", edge_count));

    out.push_str("# HELP genesisdb_collections_total Total number of vector collections.\n");
    out.push_str("# TYPE genesisdb_collections_total gauge\n");
    out.push_str(&format!(
        "genesisdb_collections_total {}\n",
        collections.len()
    ));

    out.push_str(
        "# HELP genesisdb_index_lag Async HNSW indexing backlog (vectors staged but not yet indexed).\n",
    );
    out.push_str("# TYPE genesisdb_index_lag gauge\n");
    out.push_str(&format!("genesisdb_index_lag {}\n", index_lag));

    out.push_str(
        "# HELP genesisdb_memory_usage_mb Approximate resident memory used by vector arenas, nodes, and edges, in megabytes.\n",
    );
    out.push_str("# TYPE genesisdb_memory_usage_mb gauge\n");
    out.push_str(&format!("genesisdb_memory_usage_mb {}\n", memory_usage_mb));

    out.push_str(
        "# HELP genesisdb_is_rebuilding Whether the engine is currently rebuilding its vector index (1) or not (0).\n",
    );
    out.push_str("# TYPE genesisdb_is_rebuilding gauge\n");
    out.push_str(&format!(
        "genesisdb_is_rebuilding {}\n",
        if is_rebuilding { 1 } else { 0 }
    ));

    out.push_str("# HELP genesisdb_collection_vectors Number of vectors stored in a collection.\n");
    out.push_str("# TYPE genesisdb_collection_vectors gauge\n");
    for c in &collections {
        out.push_str(&format!(
            "genesisdb_collection_vectors{{collection=\"{}\"}} {}\n",
            escape_label_value(&c.name),
            c.count
        ));
    }

    out.push_str(
        "# HELP genesisdb_collection_sidecar_resident_bytes Resident (RAM) bytes held by a collection's exact-f32 rerank sidecar.\n",
    );
    out.push_str("# TYPE genesisdb_collection_sidecar_resident_bytes gauge\n");
    for c in &collections {
        out.push_str(&format!(
            "genesisdb_collection_sidecar_resident_bytes{{collection=\"{}\"}} {}\n",
            escape_label_value(&c.name),
            c.sidecar_resident_bytes
        ));
    }

    out.push_str(
        "# HELP genesisdb_collection_sidecar_disk_bytes On-disk bytes of a collection's exact-f32 rerank sidecar.\n",
    );
    out.push_str("# TYPE genesisdb_collection_sidecar_disk_bytes gauge\n");
    for c in &collections {
        out.push_str(&format!(
            "genesisdb_collection_sidecar_disk_bytes{{collection=\"{}\"}} {}\n",
            escape_label_value(&c.name),
            c.sidecar_disk_bytes
        ));
    }

    out.push_str(
        "# HELP genesisdb_collection_arena_resident_bytes Resident (RAM) bytes held by a collection's vector arena.\n",
    );
    out.push_str("# TYPE genesisdb_collection_arena_resident_bytes gauge\n");
    for c in &collections {
        out.push_str(&format!(
            "genesisdb_collection_arena_resident_bytes{{collection=\"{}\"}} {}\n",
            escape_label_value(&c.name),
            c.arena_resident_bytes
        ));
    }

    out
}

/// `GET /metrics` — Prometheus/OpenMetrics text-format exposition, at the
/// conventional root path (no `/v1` prefix; Prometheus scrapers default to
/// `/metrics` at root). Intentionally mounted OUTSIDE the `api_key_guard`
/// layer (see `build_router`): the payload is operational counts only, no row
/// data, matching Qdrant's default of an unauthenticated `/metrics` endpoint.
/// If you later need to gate this behind the API key (e.g. to avoid leaking
/// even node/edge counts), move its `.route(...)` above the guard layer in
/// `build_router` instead of merging it in afterward.
async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.read();
    let body = render_metrics(&storage);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Router builder — shared by main.rs and integration tests
// ---------------------------------------------------------------------------

pub fn build_router(state: AppState) -> Router {
    let guarded = Router::new()
        .route("/v1/bulk/nodes", post(bulk_add_nodes_handler))
        .route("/v1/bulk/edges", post(bulk_add_edges_handler))
        .route("/v1/bulk/rebuild", post(rebuild_index_handler))
        .route("/v1/batch", post(execute_batch_handler))
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
        .route(
            "/v1/insight/drift/:cluster_id",
            get(get_meta_history_handler),
        )
        .route("/v1/insight/communities", get(insight_communities_handler))
        .route("/v1/insight/gaps", get(insight_gaps_handler))
        .route("/v1/insight/rebuild", post(insight_rebuild_handler))
        .route("/v1/query", post(query_handler))
        .route("/v1/search/hybrid", post(hybrid_search_handler))
        .route("/v1/reason/context", post(ranked_context_handler))
        .route("/v1/context/retrieve", post(retrieve_context_handler))
        .route("/v1/status", get(status_handler))
        .route("/v1/version", get(version_handler))
        .route("/v1/swarm/status", get(swarm_status_handler))
        .route("/v1/consensus/propose", post(consensus_propose_handler))
        .route("/v1/consensus/vote", post(consensus_vote_handler))
        .route("/v1/consensus/sign-vote", post(consensus_sign_vote_handler))
        .route("/v1/consensus/verify", post(consensus_verify_handler))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
        // API key guard runs after CORS (so OPTIONS preflight is never blocked)
        // but before the body-limit layer (so we don't waste I/O on unauthorized
        // bodies).
        .layer(middleware::from_fn_with_state(state.clone(), api_key_guard))
        .with_state(state.clone());

    // `GET /metrics` (root, no `/v1` prefix — Prometheus convention). Merged in
    // AFTER the guarded router above has its `api_key_guard` layer applied, so
    // this route does NOT go through that middleware: `Router::layer` only
    // affects routes already registered at the time it's called, and `.merge`
    // combines two independent routers rather than nesting one inside the
    // other's middleware stack. Scrapers can hit `/metrics` unauthenticated —
    // matches Qdrant's default (its `/metrics` carries no row data, only
    // operational counts, so this is not a data-exposure regression).
    let metrics_router = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    guarded
        .merge(metrics_router)
        // 64 MB hard cap on all request bodies; bulk endpoints stay well under.
        .layer(RequestBodyLimitLayer::new(64 * 1024 * 1024))
}

/// Reject requests missing a valid `Authorization: Bearer <key>` header when
/// `GENESIS_API_KEY` was set at startup (stored in `AppState.api_key`).
/// No-ops when `api_key` is `None` — safe for unauthenticated local use.
async fn api_key_guard(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: Next,
) -> impl IntoResponse {
    if let Some(ref expected) = state.api_key {
        let authorized = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|token| token == expected.as_str())
            .unwrap_or(false);
        if !authorized {
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        }
    }
    next.run(req).await
}

/// Build a CORS layer.
///
/// Default: allow only localhost origins (safe for embedded / dev use).
/// Set `GENESIS_CORS_ORIGIN=*` to restore permissive mode for hosted deployments
/// where you want browser access from any origin.
fn cors_layer() -> CorsLayer {
    match std::env::var("GENESIS_CORS_ORIGIN").as_deref() {
        Ok("*") => CorsLayer::permissive(),
        Ok(origin) => {
            let allowed = origin
                .parse::<HeaderValue>()
                .expect("GENESIS_CORS_ORIGIN is not a valid header value");
            CorsLayer::new()
                .allow_origin(AllowOrigin::exact(allowed))
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
        }
        Err(_) => CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
                let b = origin.as_bytes();
                b.starts_with(b"http://localhost:") || b.starts_with(b"http://127.0.0.1:")
            }))
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any),
    }
}
