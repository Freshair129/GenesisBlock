//! Genesis Block — high-performance hybrid semantic-graph engine.
//!
//! Mark VI: Collective Intelligence & Autonomic Substrate

#![deny(clippy::all)]

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File, OpenOptions as FileOpenOptions};
use std::io::{Read, Write};
// Positioned (offset) reads for the on-disk rerank sidecar. Both are gated by
// `#[cfg]` so the crate builds on either platform; NEVER mmap (Windows rename /
// atomic-swap safety — see ADR--GENESISDB-ONDISK-RERANK-SIDECAR).
#[cfg(unix)]
use std::os::unix::fs::FileExt;
#[cfg(windows)]
use std::os::windows::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use dashmap::DashMap;
use half::f16;
use hnsw_rs::prelude::*;
#[cfg(feature = "napi-bindings")]
use napi::bindgen_prelude::*;
#[cfg(feature = "napi-bindings")]
use napi_derive::napi;
use roaring::RoaringBitmap;
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{params, params_from_iter, Connection, OpenFlags, OptionalExtension};

// When the napi bindings are disabled (REST server / Linux integration-test
// build) the storage core uses a minimal Error/Result that mirror the only napi
// API it relies on — `Error::from_reason`. This is what lets the core link as a
// plain native binary with no napi_* symbols (core/napi split, #161). The cdylib
// build (default features) is unchanged: it still uses napi's Error/Result.
#[cfg(not(feature = "napi-bindings"))]
mod core_error {
    #[derive(Debug)]
    pub struct Error {
        pub reason: String,
    }
    impl Error {
        pub fn from_reason<T: Into<String>>(reason: T) -> Self {
            Error {
                reason: reason.into(),
            }
        }
    }
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.reason)
        }
    }
    impl std::error::Error for Error {}
    pub type Result<T> = std::result::Result<T, Error>;
}
#[cfg(not(feature = "napi-bindings"))]
use core_error::{Error, Result};
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[cfg(feature = "ffi")]
mod ffi;
#[cfg(feature = "android-jni")]
mod jni;
pub mod query;
pub mod router;
use query::HqlCommand;

// v3: Event::NodeRetract journal frames (RCA--SLICE0-DURABILITY defect 2).
// Older engines silently skip unknown event variants on replay, which would
// resurrect deleted nodes — the bump makes downgrade fail closed instead.
pub const SCHEMA_VERSION: u32 = 3;
const PROJECTION_SCHEMA_VERSION: u32 = 2;
const PROJECTION_DB_FILE: &str = "projection.sqlite";
/// Stable engine identifier (independent of package version).
pub const ENGINE_NAME: &str = "genesis-block";
/// Engine package version (semver x.y.z[-prerelease]), baked in from Cargo.toml
/// at compile time. Single source of truth for the running version — surfaced
/// via `version_sync()` (NAPI) and `GET /v1/version` (REST).
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

// --- Types (PROTOCOL §3) ---

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OpenOptions {
    pub path: String,
    pub page_cache_mb: Option<u32>,
    pub read_only: Option<bool>,
    pub vector_dim: Option<u32>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeInput {
    pub id: Option<String>,
    pub labels: Vec<String>,
    pub props: Option<serde_json::Value>,
    pub embedding: Option<Vec<f64>>,
    pub lang: Option<String>,
    pub valid_from: Option<String>,
    pub caused_by: Option<String>,
    pub ttl: Option<u32>,
    /// Vector collection to route `embedding` into. Defaults to `default`.
    pub collection: Option<String>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalClock {
    pub time: u32,
    pub peer_id: String,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeOutput {
    pub id: String,
    pub labels: Vec<String>,
    pub props: serde_json::Value,
    pub impact: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f64>>,
    pub lang: Option<String>,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub caused_by: Option<String>,
    pub expires_at: Option<String>,
    pub clock: LogicalClock,
    /// Which vector collection this node's embedding lives in (None = default).
    /// Persisted in the WAL `Event::Node` so replay rebuilds the right space.
    #[serde(default)]
    pub collection: Option<String>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EdgeInput {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub rel: String,
    pub props: Option<serde_json::Value>,
    pub valid_from: Option<String>,
    pub supersede: Option<bool>,
    pub impact: Option<f64>,
    pub caused_by: Option<String>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EdgeOutput {
    pub id: String,
    pub from: String,
    pub to: String,
    pub rel: String,
    pub props: serde_json::Value,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub recorded_at: String,
    pub superseded_by: Option<String>,
    pub impact: Option<f64>,
    pub caused_by: Option<String>,
    pub clock: LogicalClock,
}

#[cfg_attr(feature = "napi-bindings", napi)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalingTier {
    H0 = 0,
    H1 = 1,
    H2 = 2,
    H3 = 3,
    H4 = 4,
    H5 = 5,
    /// 6 hops — the maximum retrieval radius the engine resolves. This is a
    /// hop count and nothing more: what a caller does on reaching the ceiling
    /// is the caller's policy, not the engine's.
    H6 = 6,
}

impl ScalingTier {
    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "H0" => ScalingTier::H0,
            "H1" => ScalingTier::H1,
            "H2" => ScalingTier::H2,
            "H3" => ScalingTier::H3,
            "H4" => ScalingTier::H4,
            "H5" => ScalingTier::H5,
            "H6" => ScalingTier::H6,
            _ => ScalingTier::H1,
        }
    }
    pub fn hops(&self) -> u32 {
        match self {
            ScalingTier::H0 => 0,
            ScalingTier::H1 => 1,
            ScalingTier::H2 => 2,
            ScalingTier::H3 => 3,
            ScalingTier::H4 => 4,
            ScalingTier::H5 => 5,
            ScalingTier::H6 => 6,
        }
    }
}

/// Factual retrieval-coverage report for a `ContextPackage`: how much of the
/// requested radius the engine actually served, whether it stopped at the tier
/// boundary with graph still beyond it, and whether budget compression replaced
/// atoms. Every field is a measurement. The engine states facts and never
/// policy — what a consumer does about incomplete coverage (compact, delegate,
/// decompose, re-query) is that consumer's decision, made above the database.
///
/// Nested as its own object so additional coverage measurements can be added
/// here without changing the `ContextPackage` surface.
#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoverageReport {
    /// The requested tier's hop radius (`tier.hops()`).
    pub hops_requested: u32,
    /// The deepest BFS depth actually reached among nodes included in this package.
    pub hops_served: u32,
    /// True if traversal stopped at the requested radius while at least one
    /// boundary node still had an incident edge leading outside the package —
    /// i.e. reachable graph genuinely exists beyond the tier. False when the
    /// reachable subgraph was exhausted at or before the tier limit, including
    /// the isolated-node and boundary-leaf cases.
    pub ceiling_hit: bool,
    /// True if budget/SuperNode compression replaced atoms in this package.
    pub truncated: bool,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContextPackage {
    pub nodes: Vec<NodeOutput>,
    pub edges: Vec<EdgeOutput>,
    pub super_nodes: Vec<SuperNode>,
    pub token_estimate: u32,
    pub reasoning_path: String,
    /// Factual retrieval-coverage signal. See `CoverageReport`.
    pub coverage: CoverageReport,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryInput {
    pub from: Option<String>,
    pub to: Option<String>,
    pub rel: Option<String>,
    pub as_of: Option<String>,
    pub include_invalid: Option<bool>,
    pub limit: Option<u32>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Debug)]
pub struct NeighborInput {
    pub depth: Option<u32>,
    pub rel: Option<String>,
    pub rels: Option<Vec<String>>,
    pub direction: Option<String>,
    pub as_of: Option<String>,
    pub include_invalid: Option<bool>,
    pub limit: Option<u32>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NeighborOutput {
    pub node: NodeOutput,
    pub path: Vec<EdgeOutput>,
    pub depth: u32,
    /// Relevance score for ranked results (hybrid_search): the blended
    /// `similarity*(1-alpha) + impact*alpha`. `None` for graph traversal
    /// (`neighbors`), which is not relevance-ranked. Kept separate from
    /// `node.impact` so the caller still sees the node's true graph-authority
    /// signal and can fuse it itself (see ADR--GENESISDB-KIMPACT-AS-SIGNAL).
    pub score: Option<f64>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Debug)]
pub struct HybridSearchInput {
    pub query_vector: Vec<f64>,
    pub k: u32,
    pub alpha: Option<f64>,
    pub lang: Option<String>,
    pub as_of: Option<String>,
    /// Vector collection to search. Defaults to `default`. Query dim is
    /// validated against the collection dim (closes the cross-space bug).
    pub collection: Option<String>,
    /// Per-query HNSW `ef_search` override. When `None`, falls back to the
    /// engine-global value (`set_index_params`). Higher = better recall, higher
    /// latency. Lets a single index serve both high-recall and low-latency
    /// callers (the global value can't satisfy both as N grows — see the
    /// Recall@500k frontier).
    pub ef_search: Option<u32>,
    /// Per-query rerank over-fetch multiplier. When `None`, falls back to the
    /// `RERANK_OVERFETCH` default. Only affects collections that carry a rerank
    /// sidecar (quantized + rerank); ignored otherwise. Higher = a wider exact
    /// re-score pool = better recall on quantized collections, at the cost of
    /// more positioned sidecar reads per query. Mirrors the `ef_search` knob.
    pub oversample: Option<u32>,
}

pub const QUERY_IR_V1: &str = "query-ir.v1";
const QUERY_IR_MAX_K: u32 = 1_000;
const QUERY_IR_MAX_DEPTH: u32 = 32;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct QueryIrRequest {
    pub contract_version: String,
    pub request_id: String,
    pub namespace: Option<String>,
    pub temporal: Option<QueryIrTemporal>,
    pub consistency: Option<QueryIrConsistency>,
    pub operation: QueryIrOperation,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct QueryIrTemporal {
    pub valid_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct QueryIrConsistency {
    pub index: QueryIrIndexConsistency,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryIrIndexConsistency {
    Eventual,
    ReadYourWrite,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryIrSearchMode {
    Vector,
    Hybrid,
    Lexical,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryIrDirection {
    Out,
    In,
    Both,
}

impl QueryIrDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Out => "out",
            Self::In => "in",
            Self::Both => "both",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum QueryIrOperation {
    Search {
        mode: QueryIrSearchMode,
        target_id: Option<String>,
        query_vector: Option<Vec<f64>>,
        collection: Option<String>,
        k: u32,
        alpha: Option<f64>,
        language: Option<String>,
        ef_search: Option<u32>,
        oversample: Option<u32>,
    },
    Traverse {
        seed_id: String,
        depth: u32,
        relations: Vec<String>,
        direction: QueryIrDirection,
        limit: Option<u32>,
    },
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DatabaseStatus {
    pub open: bool,
    pub read_only: bool,
    pub page_cache_mb: u32,
}

/// One opaque, engine-owned backup file. Callers may encrypt or transport this
/// file, but must not copy Genesis projection files individually.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupExportRequest {
    pub destination: PathBuf,
}

/// Restores only to a target path that does not yet exist.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupRestoreRequest {
    pub bundle_path: PathBuf,
    pub target_root: PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BackupBundleInfo {
    pub format_version: u32,
    pub engine_name: String,
    pub engine_version: String,
    pub schema_version: u32,
    pub stable_frontier: u64,
    pub logical_clock: u32,
    pub created_at: String,
    pub bundle_path: PathBuf,
    pub byte_count: u64,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BackupArtifact {
    path: String,
    byte_count: u64,
    sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BackupManifest {
    format_version: u32,
    engine_name: String,
    engine_version: String,
    schema_version: u32,
    stable_frontier: u64,
    logical_clock: u32,
    created_at: String,
    artifacts: Vec<BackupArtifact>,
}

const BACKUP_FORMAT_VERSION: u32 = 1;
const BACKUP_MAGIC: &[u8] = b"GENESIS-BACKUP-V1\0";
const BACKUP_MANIFEST_NAME: &[u8] = b"manifest.json";

pub const STUDIO_PROTOCOL_VERSION: &str = "1";
pub const STUDIO_SCENE_NODE_CEILING: usize = 1000;
pub const STUDIO_SCENE_EDGE_CEILING: usize = 3000;
pub const STUDIO_SCENE_PAGE_LIMIT: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StudioCapabilities {
    pub protocol_version: String,
    pub engine_version: String,
    pub mode: String,
    pub read_features: Vec<String>,
    pub write_features: Vec<String>,
    pub auth_features: Vec<String>,
    pub limits: StudioLimits,
    pub consistency: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StudioLimits {
    pub initial_scene_nodes: usize,
    pub scene_node_ceiling: usize,
    pub scene_edge_ceiling: usize,
    pub expansion_nodes: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StudioGraphSceneRequest {
    pub seed: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub direction: Option<String>,
    pub as_of: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StudioGraphNode {
    pub id: String,
    pub label: String,
    pub labels: Vec<String>,
    pub collection: Option<String>,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub caused_by: Option<String>,
    pub impact: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StudioGraphEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: String,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub caused_by: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StudioGraphScene {
    pub scene_id: String,
    pub frontier: u64,
    pub nodes: Vec<StudioGraphNode>,
    pub edges: Vec<StudioGraphEdge>,
    pub groups: Vec<String>,
    pub truncated: bool,
    pub continuation: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StudioEntityInspection {
    pub entity_id: String,
    pub frontier: u64,
    pub node: StudioGraphNode,
    pub properties: Value,
    pub incident_edges: Vec<StudioGraphEdge>,
    pub vector_collection: Option<String>,
    pub vector_present: bool,
    pub index_lag: u32,
    pub availability: HashMap<String, String>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CollectionInfo {
    pub name: String,
    pub model: String,
    pub dim: u32,
    pub metric: String,
    pub quant: String,
    pub count: u32,
    /// Per-collection default HNSW `ef_search`. `None` ⇒ uses the engine-global default.
    pub ef_search: Option<u32>,
    /// Whether this (quantized) collection keeps an f32 sidecar for exact rerank.
    pub rerank: bool,
    /// Ops/credibility (P2c): resident (RAM) bytes held by the exact-f32 rerank
    /// sidecar. **≈0 post-P0**: the sidecar is on-disk (`fvec_<name>.bin`) behind a
    /// positioned-read `SidecarReader`, not a resident `Vec<f32>`, so its vector
    /// data consumes no heap. This field exists to PROVE the P0 RAM win (see
    /// docs/RCA--VECTOR-QUANTIZATION.md); a non-zero value here would mean the
    /// resident-sidecar regression came back. (`i64` not `u64`: byte counts fit a
    /// JS-safe integer and NAPI maps `i64`→JS number; `u64` would force BigInt.)
    pub sidecar_resident_bytes: i64,
    /// Ops/credibility (P2c): on-disk bytes of the rerank sidecar
    /// (`len_rows * dim * 4`), i.e. where the exact-f32 vectors actually live now.
    /// `0` when the collection has no sidecar. Shows RAM → disk is where the bytes
    /// went, not that they vanished.
    pub sidecar_disk_bytes: i64,
    /// Ops/credibility (P2c): resident (RAM) bytes of this collection's vector
    /// arena (`arena.byte_size()` — f32=4B/elem, SQ8=1B/elem, BQ=1bit/elem).
    /// Context for the residency story: this is the RAM the quantized arena costs.
    pub arena_resident_bytes: i64,
    /// Ops/credibility (P2c): engine-global async-indexing backlog
    /// (`Storage::index_lag()`) — vectors staged into the arena but not yet
    /// inserted into HNSW. Not per-collection (one indexing thread serves all
    /// collections); the SAME value is repeated on every entry for convenience.
    pub index_lag: u32,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncPeer {
    pub id: String,
    pub addr: String,
    pub last_seen: u32,
    pub verifying_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedEvent {
    pub event: Event,
    pub signature: Vec<u8>,
    pub signer_peer_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GossipMessage {
    Heartbeat {
        peer_id: String,
        merkle_root: String,
        logical_time: u32,
        port: u16,
        verifying_key: Vec<u8>,
    },
    PullRequest {
        from_clock: u32,
        target_peer_id: String,
        /// WP-1.2 (ADR D4): commit_seq/segment cursor alongside the Lamport
        /// cursor. When present, the responder serves frames with a LOCAL
        /// commit_seq greater than this value (the requester tracks the
        /// responder-domain cursor; replica sequences are never comparable
        /// across peers). `default` keeps pre-WP-1.2 peers deserializing.
        #[serde(default)]
        from_commit_seq: Option<u64>,
    },
    PushDelta {
        events: Vec<SignedEvent>,
    },
    /// WP-1.2 (ADR D4): the requester's commit_seq cursor predates this
    /// responder's history horizon — delta pull cannot serve it. The requester
    /// must abandon delta-pull for this peer and re-bootstrap (transfer channel
    /// lands in WP-1.3; until retention folds can advance the horizon past a
    /// live cursor this variant is wire-defined but not triggered in practice).
    BeyondHorizon {
        horizon: u64,
    },
    ConsensusPropose {
        proposal: Box<ConsensusProposal>,
    },
    ConsensusVote {
        proposal_id: String,
        voter_peer_id: String,
        approve: bool,
        signature: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SyncEvent {
    ProposeMutation(Box<Event>),
    AcknowledgeMutation(String),
    RequestFragment(String),
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BatchInput {
    pub nodes: Vec<NodeInput>,
    pub edges: Vec<EdgeInput>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Debug)]
pub struct BatchOutput {
    pub nodes: Vec<NodeOutput>,
    pub edges: Vec<EdgeOutput>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelationalMutationGroup {
    pub namespace: String,
    pub mutations: Vec<RelationalRowMutation>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VectorUpsertInput {
    pub node_id: String,
    pub collection: String,
    pub embedding: Vec<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GenesisTransaction {
    pub transaction_id: String,
    pub expected_frontier: Option<u64>,
    pub relational: Vec<RelationalMutationGroup>,
    pub graph: BatchInput,
    pub vectors: Vec<VectorUpsertInput>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GenesisTransactionEvent {
    pub transaction_id: String,
    /// ADR--GENESISDB-JOURNAL-HISTORY D2.2: demoted from "the" commit sequence
    /// to origin metadata. The authoritative transaction sequence is the LOCAL
    /// frame stamp (`commit_seq` in the unsigned frame header) — never this
    /// signed payload field, which a receiving replica must NOT merge into its
    /// own counter. Locally-created events write 0; ingested peer events keep
    /// whatever the origin wrote. `alias` keeps pre-WP-1.2 WAL lines parsing.
    #[serde(alias = "commit_sequence")]
    pub origin_commit_seq: u64,
    pub payload_hash: String,
    pub relational: Vec<RelationalMutationGroup>,
    pub nodes: Vec<NodeOutput>,
    pub edges: Vec<EdgeOutput>,
    pub vectors: Vec<VectorEvent>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommitResult {
    pub transaction_id: String,
    pub commit_sequence: u64,
    pub stable: bool,
}

// --- Internal Storage ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    Node(NodeOutput),
    Edge(EdgeOutput),
    Batch(Vec<Event>),
    /// Attach an additional vector to an existing node in a named collection
    /// (a node may hold one vector per collection — e.g. code + text embeddings).
    /// Carries the f64 embedding for WAL replay, mirroring `Event::Node`.
    Vector(VectorEvent),
    /// Versioned application relational schema, durably ordered by Genesis WAL.
    RelationalSchema(RelationalSchemaPackage),
    /// Typed application row mutations applied atomically to the SQLite projection.
    RelationalRows {
        namespace: String,
        #[serde(default)]
        schema_version: u32,
        #[serde(default)]
        mutation_id: String,
        #[serde(default)]
        payload_hash: String,
        #[serde(default)]
        affected_rows: u32,
        mutations: Vec<RelationalRowMutation>,
    },
    /// One canonical cross-domain commit with a single durable sequence.
    Transaction(GenesisTransactionEvent),
    /// Hard node retraction (RCA--SLICE0-DURABILITY defect 2): removes the node,
    /// its incident edges, and its vector mappings from the live view. Journaled
    /// so deletions survive crash replay (previously the delete existed only in
    /// RAM until the next fold — replay and CRDT peers resurrected it). Carries
    /// the retraction's logical clock so `events_since` replicates it and
    /// `reconcile_state` can apply node-style LWW. Older engines silently skip
    /// unknown variants on replay, which would resurrect deletions on downgrade
    /// — hence the SCHEMA_VERSION 2 → 3 fail-closed bump that ships with this
    /// variant.
    NodeRetract {
        id: String,
        #[serde(default)]
        clock: LogicalClock,
    },
}

/// Payload of `Event::Vector`: a standalone vector attached to a node, routed to
/// `collection` (None = default). `lang` defaults to "en" on replay if absent.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VectorEvent {
    pub node_id: String,
    pub collection: Option<String>,
    pub embedding: Vec<f64>,
    pub lang: Option<String>,
    /// Logical clock at the time the vector was attached. Lets `events_since`
    /// time-filter secondary embeddings into anti-entropy pull deltas (they used
    /// to be excluded for lack of a clock). `#[serde(default)]` ⇒ pre-clock WAL
    /// entries deserialize with a zero clock and still replay (they never block
    /// LWW, since vectors are append-applied).
    #[serde(default)]
    pub clock: LogicalClock,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RelationalColumnType {
    Text,
    Integer,
    Real,
    Boolean,
    Json,
    Blob,
    Timestamp,
    EntityId,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RelationalColumn {
    pub name: String,
    pub column_type: RelationalColumnType,
    pub nullable: bool,
    #[serde(default)]
    pub default: Option<Value>,
}

impl RelationalColumn {
    pub fn required(name: &str, column_type: RelationalColumnType) -> Self {
        Self {
            name: name.to_string(),
            column_type,
            nullable: false,
            default: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RelationalForeignKey {
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RelationalIndex {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RelationalTable {
    pub name: String,
    pub columns: Vec<RelationalColumn>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<RelationalForeignKey>,
    pub indexes: Vec<RelationalIndex>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RelationalSchemaPackage {
    pub namespace: String,
    pub schema_version: u32,
    #[serde(default)]
    pub previous_version: Option<u32>,
    #[serde(default)]
    pub package_id: String,
    #[serde(default)]
    pub schema_hash: String,
    pub tables: Vec<RelationalTable>,
    #[serde(default)]
    pub named_queries: Vec<NamedQueryDefinition>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RelationalMutationKind {
    Insert,
    Upsert,
    Update,
    Delete,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RelationalRowMutation {
    pub table: String,
    pub kind: RelationalMutationKind,
    pub values: Value,
    pub key: Option<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RelationalMutationBatch {
    pub mutation_id: String,
    pub namespace: String,
    pub schema_version: u32,
    pub operations: Vec<RelationalRowMutation>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RelationalMutationResult {
    pub mutation_id: String,
    pub namespace: String,
    pub schema_version: u32,
    pub wal_committed: bool,
    pub projection_applied: bool,
    pub affected_rows: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RelationalFilter {
    pub column: String,
    pub value: Value,
}

impl RelationalFilter {
    pub fn equal(column: &str, value: Value) -> Self {
        Self {
            column: column.to_string(),
            value,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RelationalJoin {
    pub table: String,
    pub left_column: String,
    pub right_column: String,
    #[serde(default)]
    pub kind: RelationalJoinKind,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum RelationalJoinKind {
    #[default]
    Inner,
    Left,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RelationalQuery {
    pub namespace: String,
    pub table: String,
    pub columns: Vec<String>,
    pub joins: Vec<RelationalJoin>,
    pub filters: Vec<RelationalFilter>,
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RelationalQueryParameter {
    pub name: String,
    pub column_type: RelationalColumnType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NamedQueryDefinition {
    pub name: String,
    pub parameters: Vec<RelationalQueryParameter>,
    pub query: RelationalQuery,
    pub default_limit: u32,
    pub max_limit: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NamedQueryRequest {
    pub namespace: String,
    pub schema_version: u32,
    pub query_name: String,
    pub parameters: Value,
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeMetadata {
    pub arena_id: u32,
    /// Interned node id (u32), not the String form — A2 of
    /// `ADR--GENESISDB-NODE-ID-INTERNING`: drops a per-vector × per-collection
    /// String copy. The String is recoverable via `nodes[node_u32].id`. Pre-A2
    /// snapshots stored a String here and are migrated on load via
    /// `NodeMetadataV0` (selected by the manifest `mv` flag).
    pub node_u32: u32,
    pub timestamp: u64,
    pub vector_dim: u16,
    pub embedding_offset: u64,
    pub gks_attributes: Vec<u8>,
    pub lang: String,
    pub cluster_id: u32,
}

/// Pre-A2 on-disk layout of `NodeMetadata` (node id as a String). Used ONLY to
/// read legacy `meta_*.bin` / `meta.bin` snapshots, which are migrated to the
/// interned-u32 `NodeMetadata` on load. bincode is not self-describing, so the
/// format is chosen by the manifest `mv` flag (absent ⇒ legacy), never by trial
/// deserialization (which would silently misread).
#[derive(Deserialize)]
struct NodeMetadataV0 {
    arena_id: u32,
    node_id: String,
    timestamp: u64,
    vector_dim: u16,
    embedding_offset: u64,
    gks_attributes: Vec<u8>,
    lang: String,
    cluster_id: u32,
}

/// 4-byte magic prefixing the on-disk `meta_<name>.bin` blob written by the
/// current code (ADR--GENESISDB-BINCODE-EXIT). Its presence/absence is the
/// format discriminator: bincode 1.3 is unmaintained (RUSTSEC-2025-0141) and
/// non-self-describing (there was never a marker for it), so `GBP1` is what
/// makes the *new* postcard container sniffable going forward. Absent ⇒ a
/// pre-existing bincode blob, whose V1-vs-V0 shape is still resolved by the
/// manifest `mv` flag, never by trial deserialization (see `NodeMetadataV0`
/// doc comment above).
const META_MAGIC: &[u8; 4] = b"GBP1";

/// Encode a metadata snapshot in the current on-disk format: `GBP1` magic +
/// `postcard::to_allocvec(&Vec<NodeMetadata>)`. Every `save_state()` call
/// writes this format, so a legacy (bincode) snapshot loaded via
/// `decode_metadata_snapshot`'s fallback path is transparently rewritten as
/// postcard the next time the DB saves — no separate migration step needed.
fn encode_metadata_snapshot(meta: &[NodeMetadata]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(4 + meta.len() * 32);
    out.extend_from_slice(META_MAGIC);
    let body = postcard::to_allocvec(meta).map_err(|e| Error::from_reason(e.to_string()))?;
    out.extend_from_slice(&body);
    Ok(out)
}

/// Sniff-first decode of a `meta_<name>.bin` / `meta.bin` blob:
///   - `GBP1` magic present -> `Some(postcard::from_bytes(..))`. A magic hit
///     whose body fails to decode is corruption (not "unrecognized format"),
///     so the caller must treat `Some(Err(_))` as a loud failure rather than
///     falling through to the legacy bincode arms — bincode is non-self-
///     describing, so silently trying it against postcard bytes risks
///     "successfully" misparsing garbage into a bogus-but-valid result.
///   - no magic -> `None`, meaning "fall back to the existing legacy bincode
///     try-chain" (unchanged: V1 vs V0 is still resolved by the manifest `mv`
///     flag, never by trial deserialization).
fn decode_metadata_snapshot(data: &[u8]) -> Option<std::result::Result<Vec<NodeMetadata>, String>> {
    if data.len() >= META_MAGIC.len() && &data[..META_MAGIC.len()] == META_MAGIC {
        Some(
            postcard::from_bytes::<Vec<NodeMetadata>>(&data[META_MAGIC.len()..])
                .map_err(|e| e.to_string()),
        )
    } else {
        None
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsensusProposal {
    pub proposal_id: String,
    pub signed_event: SignedEvent,
    pub votes: HashMap<String, bool>, // PeerID -> Vote
    pub quorum_signatures: HashMap<String, Vec<u8>>, // PeerID -> Signature
    /// Set once the proposal has crossed quorum and been applied. Guards against
    /// re-applying (and re-persisting) the event on every approving vote that
    /// arrives after quorum. `#[serde(default)]` so proposals gossiped by peers
    /// running an older build (no field) deserialize as not-yet-committed.
    #[serde(default)]
    pub committed: bool,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SuperNode {
    pub cluster_id: u32,
    pub theme: String,
    pub member_count: u32,
    pub impact: f64,
    pub centroid: Vec<f64>,
    pub timestamp: String,
    pub drift: Option<f64>,
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MetaEdge {
    pub from_cluster: u32,
    pub to_cluster: u32,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationTask {
    ConsolidateStagnant,
    PruneEntropy,
    RebuildMetaGraph,
}

/// Distance/ranking metric for a vector collection. Cosine is implemented as
/// L2 over L2-normalized vectors (SPEC §10) so a single `DistL2` HNSW index
/// type serves both — vectors are normalized on insert and at query time.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Metric {
    L2,
    Cosine,
}

impl Metric {
    fn parse(s: &str) -> Self {
        if s.eq_ignore_ascii_case("cosine") {
            Metric::Cosine
        } else {
            Metric::L2
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            Metric::L2 => "L2",
            Metric::Cosine => "Cosine",
        }
    }
}

/// Per-collection vector quantization (ADR--GENESISDB-VECTOR-QUANTIZATION).
/// `None` keeps lossless f32 end-to-end — byte-identical to pre-quant DBs.
/// `ScalarU8` (SQ8) stores BOTH the resident arena and the HNSW as u8 (the
/// "full resident cut", ~4× vector RAM); symmetric quant.
/// `Binary` (BQ) packs each dim to one sign bit (u64 words) with a popcount-
/// Hamming HNSW — ~32× vector RAM, lossy.
/// `F16` stores the arena as IEEE half-precision (`u16` bits, 2×/elem on the
/// arena + disk snapshot) and dequantizes to f32 for the HNSW (`anndists` has no
/// `Distance<f16>`), so the HNSW copy stays f32 — near-lossless recall, arena/disk
/// 2×, total resident ~1.33× at 1536-d. No sidecar (see ADR Layer C / P2a-T0).
/// Either quantized mode may opt into an **f32-sidecar rerank** (per-collection
/// `rerank` flag): the exact f32 vectors are kept in a `fvec_<name>.bin` sidecar
/// and used to re-score an over-fetched candidate set, recovering recall lost to
/// quantization (ADR--GENESISDB-VECTOR-QUANTIZATION). `None` collections never
/// allocate a sidecar — the arena is already exact f32.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Quant {
    None,
    ScalarU8,
    Binary,
    F16,
}

impl Quant {
    fn parse(s: &str) -> Self {
        // "sq8c"/"sq8-calibrated" are SQ8 with the calibrated-scale opt-in — same
        // storage element type; the calibrate flag is read via `sq8_calibrate_requested`.
        if s.eq_ignore_ascii_case("sq8")
            || s.eq_ignore_ascii_case("scalaru8")
            || Self::sq8_calibrate_requested(s)
        {
            Quant::ScalarU8
        } else if s.eq_ignore_ascii_case("bq") || s.eq_ignore_ascii_case("binary") {
            Quant::Binary
        } else if s.eq_ignore_ascii_case("f16") || s.eq_ignore_ascii_case("half") {
            Quant::F16
        } else {
            Quant::None
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            Quant::None => "none",
            Quant::ScalarU8 => "sq8",
            Quant::Binary => "bq",
            Quant::F16 => "f16",
        }
    }
    /// Whether a `quant` string requests the SQ8 **calibrated** scale (P2b opt-in).
    /// `"sq8"` is fixed-scale (default); `"sq8c"`/`"sq8-calibrated"`/`"sq8_calibrated"`
    /// opt into the per-collection quantile scale computed at compaction.
    fn sq8_calibrate_requested(s: &str) -> bool {
        s.eq_ignore_ascii_case("sq8c")
            || s.eq_ignore_ascii_case("sq8-calibrated")
            || s.eq_ignore_ascii_case("sq8_calibrated")
    }
}

// SQ8 maps a value range -> [0,255] with an affine `q = v*scale + bias`. The
// DEFAULT (fixed) scale assumes [-1,1] so concurrent async inserts agree without
// any of them having seen the whole data distribution. Cosine collections are
// unit-normalized into [-1,1] (clean); L2 values outside [-1,1] clamp. The
// per-collection **calibrated** scale (P2b, opt-in via `quant:"sq8c"`) replaces
// this default with a quantile range computed at compaction — see
// `sq8_snapshot`/`perform_index_compaction`.
const SQ8_SCALE: f32 = 127.5;
const SQ8_BIAS: f32 = 127.5;
/// The fixed (scale, bias) — used when a collection has no calibrated scale.
const SQ8_FIXED: (f32, f32) = (SQ8_SCALE, SQ8_BIAS);

#[inline]
fn sq8_q(v: f32, scale: f32, bias: f32) -> u8 {
    let q = (v * scale + bias).round();
    if q <= 0.0 {
        0
    } else if q >= 255.0 {
        255
    } else {
        q as u8
    }
}
#[inline]
fn sq8_dq(q: u8, scale: f32, bias: f32) -> f32 {
    (q as f32 - bias) / scale
}

// Binary quantization (BQ): one sign bit per dim, packed into u64 words. Distance
// is bit Hamming via popcount.
#[inline]
fn bq_words(dim: usize) -> usize {
    dim.div_ceil(64)
}

/// Pack a prepared f32 vector to sign-bit codes (bit set iff component > 0).
#[inline]
fn bq_pack(emb: &[f32]) -> Vec<u64> {
    let mut w = vec![0u64; bq_words(emb.len())];
    for (i, &x) in emb.iter().enumerate() {
        if x > 0.0 {
            w[i >> 6] |= 1u64 << (i & 63);
        }
    }
    w
}

/// Pack with optional per-dim centering: the bit is set iff `x - center[i] > 0`.
/// `center == None` is byte-identical to `bq_pack` (the pre-P1a behavior). The
/// SAME center MUST be applied on every insert path and on the query, or the
/// codes live in different spaces and Hamming distance is meaningless. When
/// `center` is shorter than `emb` (should not happen — both are `dim`), the
/// unmatched tail falls back to the uncentered sign so the loop never panics.
#[inline]
fn bq_pack_centered(emb: &[f32], center: Option<&[f32]>) -> Vec<u64> {
    let Some(c) = center else {
        return bq_pack(emb);
    };
    let mut w = vec![0u64; bq_words(emb.len())];
    for (i, &x) in emb.iter().enumerate() {
        let m = c.get(i).copied().unwrap_or(0.0);
        if x - m > 0.0 {
            w[i >> 6] |= 1u64 << (i & 63);
        }
    }
    w
}

/// Expand `dim` sign bits back to ±1.0 f32 (for the heuristic f32 readers only —
/// meta-graph / clustering; never for search). Lossless on sign, not magnitude.
#[inline]
fn bq_unpack(words: &[u64], dim: usize) -> Vec<f32> {
    (0..dim)
        .map(|i| {
            if words[i >> 6] & (1u64 << (i & 63)) != 0 {
                1.0
            } else {
                -1.0
            }
        })
        .collect()
}

/// Default over-fetch multiplier for f32-sidecar rerank: pull `k * this` quantized
/// candidates from the HNSW, then re-score them exactly and keep the best `k*2`.
/// Bigger = better recall, more f32 distance work. BQ (1 bit/dim) benefits most.
const RERANK_OVERFETCH: usize = 8;

/// Slot-count ceiling for the exact-scan recall floor in `hybrid_search`: when
/// the HNSW returns fewer candidates than the collection holds and the
/// collection is no larger than this, the candidate set is rebuilt by exhaustive
/// scan instead (RCA--HNSW-UNDER-RETURN-SMALL-GRAPH). Sized so the fallback stays
/// cheap — a scan of this many vectors is a few ms even at large dims — while
/// leaving real corpora, where a short return can be legitimate and a scan would
/// be ruinous, entirely on the graph path.
const EXACT_SCAN_MAX_SLOTS: usize = 4096;

/// Exact Euclidean distance between two prepared f32 vectors. Used by the rerank
/// stage; reuses the same geometry as the F32 HNSW (`DistL2`). For Cosine
/// collections both the query and the sidecar vector are unit-normalized, so this
/// ranks by cosine (L2 on the unit sphere is monotonic in 1-cos). Exact match ⇒ 0.
#[inline]
fn exact_l2(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum::<f32>()
        .sqrt()
}

/// Bit-Hamming distance over BQ-packed u64 codes. anndists' `DistHamming<u64>`
/// counts WORD inequality (wrong for bit codes), so HNSW uses this popcount
/// variant. Distance is the raw bit-difference count (normalized to [0,1] by the
/// caller via the dim).
#[derive(Clone)]
struct DistBinaryHamming;
impl Distance<u64> for DistBinaryHamming {
    fn eval(&self, a: &[u64], b: &[u64]) -> f32 {
        let mut d = 0u32;
        for i in 0..a.len() {
            d += (a[i] ^ b[i]).count_ones();
        }
        d as f32
    }
}

/// The resident vector arena, element type chosen by the collection's `Quant`.
/// Offsets/lengths are in scalar-component units (== dim), identical across
/// variants — so `NodeMetadata.embedding_offset`/`vector_dim` mean the same in
/// every mode; only the on-disk byte width differs (4 for f32, 1 for u8).
pub enum ArenaStore {
    F32(Vec<f32>),
    /// SQ8: one u8 per component with an affine `(scale, bias)`. The scale is
    /// carried IN the variant so `f32_at`/`push_f32` dequantize/quantize without
    /// an external param — `SQ8_FIXED` by default, or a per-collection calibrated
    /// range (P2b) installed at compaction / load. All rows in one arena share it.
    U8 {
        data: Vec<u8>,
        scale: f32,
        bias: f32,
    },
    /// BQ: bit-packed sign codes — `n` vectors × `bq_words(dim)` u64 words.
    /// `len()` still reports logical components (`n*dim`), so `embedding_offset`
    /// stays in component units exactly like the f32/u8 variants and the shared
    /// `start + len <= arena.len()` bounds checks remain valid.
    Binary {
        data: Vec<u64>,
        dim: usize,
        n: usize,
    },
    /// F16: one IEEE half (`f16::to_bits()` → `u16`) per component, so `len()` is
    /// the component count exactly like F32/U8. 2 B/elem on the arena and in the
    /// `vec_<name>.bin` snapshot. Dequantized to f32 for the HNSW (no native f16
    /// distance) and for the heuristic f32 readers.
    F16(Vec<u16>),
}

impl ArenaStore {
    fn new(q: Quant, dim: usize) -> Self {
        match q {
            Quant::None => ArenaStore::F32(Vec::new()),
            Quant::ScalarU8 => ArenaStore::U8 {
                data: Vec::new(),
                scale: SQ8_FIXED.0,
                bias: SQ8_FIXED.1,
            },
            Quant::Binary => ArenaStore::Binary {
                data: Vec::new(),
                dim,
                n: 0,
            },
            Quant::F16 => ArenaStore::F16(Vec::new()),
        }
    }
    /// Number of scalar components stored (NOT bytes).
    pub fn len(&self) -> usize {
        match self {
            ArenaStore::F32(v) => v.len(),
            ArenaStore::U8 { data, .. } => data.len(),
            ArenaStore::Binary { n, dim, .. } => n * dim,
            ArenaStore::F16(v) => v.len(),
        }
    }
    /// Actual heap bytes consumed by the vector data.
    pub fn byte_size(&self) -> usize {
        match self {
            ArenaStore::F32(v) => v.len() * 4,
            ArenaStore::U8 { data, .. } => data.len(),
            ArenaStore::Binary { data, .. } => data.len() * 8,
            ArenaStore::F16(v) => v.len() * 2,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Overwrite the SQ8 (scale, bias). No-op on non-U8 arenas. Used by load to
    /// adopt a persisted calibrated scale and by compaction's re-pack.
    fn set_sq8_scale(&mut self, s: f32, b: f32) {
        if let ArenaStore::U8 { scale, bias, .. } = self {
            *scale = s;
            *bias = b;
        }
    }
    /// This arena's SQ8 (scale, bias), or `SQ8_FIXED` for non-U8 arenas.
    fn sq8_scale(&self) -> (f32, f32) {
        match self {
            ArenaStore::U8 { scale, bias, .. } => (*scale, *bias),
            _ => SQ8_FIXED,
        }
    }
    /// Append a prepared f32 vector, quantizing per mode. `center` is the BQ
    /// per-dim centering vector (`None` ⇒ uncentered); it only affects the
    /// `Binary` arm — F32/U8 ignore it.
    fn push_f32(&mut self, emb: &[f32], center: Option<&[f32]>) {
        match self {
            ArenaStore::F32(v) => v.extend_from_slice(emb),
            ArenaStore::U8 { data, scale, bias } => {
                let (s, b) = (*scale, *bias);
                data.extend(emb.iter().map(|&x| sq8_q(x, s, b)));
            }
            ArenaStore::Binary { data, n, .. } => {
                data.extend(bq_pack_centered(emb, center));
                *n += 1;
            }
            ArenaStore::F16(v) => v.extend(emb.iter().map(|&x| f16::from_f32(x).to_bits())),
        }
    }
    /// Read a vector back as f32 (dequantizing per mode). For the heuristic
    /// f32-value readers (meta-graph / clustering) only — never for search.
    fn f32_at(&self, start: usize, len: usize) -> Vec<f32> {
        match self {
            ArenaStore::F32(v) => v[start..start + len].to_vec(),
            ArenaStore::U8 { data, scale, bias } => data[start..start + len]
                .iter()
                .map(|&q| sq8_dq(q, *scale, *bias))
                .collect(),
            ArenaStore::Binary { data, dim, .. } => {
                let wpv = bq_words(*dim);
                let ws = (start / *dim) * wpv;
                bq_unpack(&data[ws..ws + wpv], len)
            }
            ArenaStore::F16(v) => v[start..start + len]
                .iter()
                .map(|&b| f16::from_bits(b).to_f32())
                .collect(),
        }
    }
    /// Append elements [start, start+len) from `src` (same variant) — compaction.
    fn append_range(&mut self, src: &ArenaStore, start: usize, len: usize) {
        match (self, src) {
            (ArenaStore::F32(d), ArenaStore::F32(s)) => d.extend_from_slice(&s[start..start + len]),
            (
                ArenaStore::U8 {
                    data: d,
                    scale: ds,
                    bias: db,
                },
                ArenaStore::U8 {
                    data: s,
                    scale: ss,
                    bias: sb,
                },
            ) => {
                // Codes are only comparable under one scale; inherit the source's
                // so a copied arena keeps interpreting its u8s correctly.
                d.extend_from_slice(&s[start..start + len]);
                *ds = *ss;
                *db = *sb;
            }
            (ArenaStore::Binary { data: d, n, .. }, ArenaStore::Binary { data: s, dim, .. }) => {
                let wpv = bq_words(*dim);
                let ws = (start / *dim) * wpv;
                d.extend_from_slice(&s[ws..ws + wpv]);
                *n += 1;
            }
            (ArenaStore::F16(d), ArenaStore::F16(s)) => d.extend_from_slice(&s[start..start + len]),
            _ => {} // a collection never mixes variants
        }
    }
    /// Raw little-endian bytes for the `vec_<name>.bin` snapshot.
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            ArenaStore::F32(v) => {
                unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }.to_vec()
            }
            ArenaStore::U8 { data, .. } => data.clone(),
            ArenaStore::Binary { data, .. } => data.iter().flat_map(|w| w.to_le_bytes()).collect(),
            ArenaStore::F16(v) => v.iter().flat_map(|b| b.to_le_bytes()).collect(),
        }
    }
    fn from_bytes(data: &[u8], q: Quant, dim: usize) -> Self {
        match q {
            Quant::None => ArenaStore::F32(
                data.chunks_exact(4)
                    .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                    .collect(),
            ),
            // Scale defaults to fixed; load() injects a persisted calibrated scale
            // via `set_sq8_scale` after this (the u8 codes are scale-agnostic bytes).
            Quant::ScalarU8 => ArenaStore::U8 {
                data: data.to_vec(),
                scale: SQ8_FIXED.0,
                bias: SQ8_FIXED.1,
            },
            Quant::Binary => {
                let words: Vec<u64> = data
                    .chunks_exact(8)
                    .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                    .collect();
                let wpv = bq_words(dim).max(1);
                ArenaStore::Binary {
                    n: words.len() / wpv,
                    data: words,
                    dim,
                }
            }
            Quant::F16 => ArenaStore::F16(
                data.chunks_exact(2)
                    .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
                    .collect(),
            ),
        }
    }
}

/// HNSW index, element type chosen by `Quant`. `search` distances are always f32
/// (the `Distance::eval` contract), so callers stay mode-agnostic.
enum VecIndex {
    F32(Hnsw<'static, f32, DistL2>),
    U8(Hnsw<'static, u8, DistL2>),
    Binary(Hnsw<'static, u64, DistBinaryHamming>),
    /// F16 has no native `Distance<f16>` (anndists), so it indexes f32: rows are
    /// dequantized f16→f32 on insert. The graph copy is therefore f32-sized; the
    /// F16 win is on the arena/snapshot only (ADR Layer C).
    F16(Hnsw<'static, f32, DistL2>),
}

impl VecIndex {
    fn build(q: Quant, ef_c: usize, cap: usize) -> Self {
        let cap = cap.max(VectorCollection::HNSW_MIN_CAP);
        match q {
            Quant::None => VecIndex::F32(Hnsw::new(16, cap, 16, ef_c, DistL2 {})),
            Quant::ScalarU8 => VecIndex::U8(Hnsw::new(16, cap, 16, ef_c, DistL2 {})),
            Quant::Binary => VecIndex::Binary(Hnsw::new(16, cap, 16, ef_c, DistBinaryHamming {})),
            Quant::F16 => VecIndex::F16(Hnsw::new(16, cap, 16, ef_c, DistL2 {})),
        }
    }
    /// Insert one prepared f32 vector. `center` is the BQ per-dim centering
    /// vector (`None` ⇒ uncentered) and only affects the `Binary` arm. Callers
    /// packing a RAW embedding pass the collection's center; `rehydrate` — which
    /// feeds already-centered arena codes as ±1 — passes `None`.
    fn insert_f32(&self, emb: &[f32], id: usize, center: Option<&[f32]>, sq8: (f32, f32)) {
        match self {
            VecIndex::F32(h) => h.insert((emb, id)),
            VecIndex::U8(h) => {
                let q: Vec<u8> = emb.iter().map(|&x| sq8_q(x, sq8.0, sq8.1)).collect();
                h.insert((&q, id));
            }
            VecIndex::Binary(h) => {
                let c = bq_pack_centered(emb, center);
                h.insert((&c, id));
            }
            VecIndex::F16(h) => {
                // Index the f16-rounded value (not raw emb) so a fresh insert and a
                // rehydrate-from-arena produce byte-identical HNSW inputs.
                let q: Vec<f32> = emb.iter().map(|&x| f16::from_f32(x).to_f32()).collect();
                h.insert((&q, id));
            }
        }
    }
    fn parallel_insert_f32(
        &self,
        items: &[(Vec<f32>, u32)],
        center: Option<&[f32]>,
        sq8: (f32, f32),
    ) {
        match self {
            VecIndex::F32(h) => {
                let refs: Vec<(&Vec<f32>, usize)> =
                    items.iter().map(|(v, id)| (v, *id as usize)).collect();
                h.parallel_insert(&refs);
            }
            VecIndex::U8(h) => {
                let q: Vec<(Vec<u8>, usize)> = items
                    .iter()
                    .map(|(v, id)| {
                        (
                            v.iter().map(|&x| sq8_q(x, sq8.0, sq8.1)).collect(),
                            *id as usize,
                        )
                    })
                    .collect();
                let refs: Vec<(&Vec<u8>, usize)> = q.iter().map(|(v, id)| (v, *id)).collect();
                h.parallel_insert(&refs);
            }
            VecIndex::Binary(h) => {
                let q: Vec<(Vec<u64>, usize)> = items
                    .iter()
                    .map(|(v, id)| (bq_pack_centered(v, center), *id as usize))
                    .collect();
                let refs: Vec<(&Vec<u64>, usize)> = q.iter().map(|(v, id)| (v, *id)).collect();
                h.parallel_insert(&refs);
            }
            VecIndex::F16(h) => {
                let q: Vec<(Vec<f32>, usize)> = items
                    .iter()
                    .map(|(v, id)| {
                        (
                            v.iter().map(|&x| f16::from_f32(x).to_f32()).collect(),
                            *id as usize,
                        )
                    })
                    .collect();
                let refs: Vec<(&Vec<f32>, usize)> = q.iter().map(|(v, id)| (v, *id)).collect();
                h.parallel_insert(&refs);
            }
        }
    }
    /// Search, returning `(arena_id, distance_f32)` so callers never name the
    /// hnsw_rs `Neighbour` type or branch on element type.
    /// `center` (BQ only, `None` ⇒ uncentered) must be the SAME vector the insert
    /// paths used, or the query code and the indexed codes live in different
    /// spaces. It centers a local copy of the query for packing only — the
    /// caller's `query` slice is untouched, so the exact-f32 rerank stage still
    /// sees the raw (uncentered) query.
    fn search_f32(
        &self,
        query: &[f32],
        k: usize,
        ef: usize,
        center: Option<&[f32]>,
        sq8: (f32, f32),
    ) -> Vec<(usize, f32)> {
        match self {
            VecIndex::F32(h) => h
                .search(query, k, ef)
                .into_iter()
                .map(|n| (n.d_id, n.distance))
                .collect(),
            VecIndex::U8(h) => {
                let q: Vec<u8> = query.iter().map(|&x| sq8_q(x, sq8.0, sq8.1)).collect();
                h.search(&q, k, ef)
                    .into_iter()
                    .map(|n| (n.d_id, n.distance))
                    .collect()
            }
            VecIndex::Binary(h) => {
                // Normalize Hamming (0..dim) to [0,1] so `1 - distance` stays a
                // sane similarity for the hybrid score blend.
                let c = bq_pack_centered(query, center);
                let dim = query.len().max(1) as f32;
                h.search(&c, k, ef)
                    .into_iter()
                    .map(|n| (n.d_id, n.distance / dim))
                    .collect()
            }
            VecIndex::F16(h) => h
                .search(query, k, ef)
                .into_iter()
                .map(|n| (n.d_id, n.distance))
                .collect(),
        }
    }
}

/// Fixed capacity (in rows) of the per-`SidecarReader` LRU decode cache. Bounds
/// the resident bytes at `SIDECAR_CACHE_ROWS * dim * 4` regardless of collection
/// size, so the cache can never re-create the resident-RAM problem the on-disk
/// sidecar exists to solve. 4096 rows × 1024 dim × 4 B ≈ 16 MiB worst case.
pub const SIDECAR_CACHE_ROWS: usize = 4096;

/// Positioned-read view over `fvec_<name>.bin` — the exact-f32 rerank sidecar,
/// kept ON DISK (post-P0) instead of resident in a `Vec<f32>`. Row `d_id` lives
/// at byte offset `d_id * dim * 4`, `dim` little-endian f32s, matching the
/// resident-era layout exactly (no migration; legacy files reused as-is). Reads
/// use `seek_read` (Windows) / `read_at` (Unix) behind `#[cfg]` — NEVER mmap
/// (see ADR--GENESISDB-ONDISK-RERANK-SIDECAR for the Windows-safety rationale).
///
/// A small bounded LRU (`SIDECAR_CACHE_ROWS`) fronts the disk so hot rerank rows
/// don't re-hit the file. The cache `Mutex` is strictly nested: `row()` takes it
/// only to look up / insert a decoded row and releases it before/after the disk
/// read — it is never held across, and never wraps, the arena or `meta` locks,
/// so the `meta → arena → sidecar` order is preserved.
pub struct SidecarReader {
    file: File,
    dim: usize,
    /// Bounded LRU: `(d_id -> decoded row)` newest-last. Hand-rolled over a
    /// `Vec` (no new crate dep); capacity == `SIDECAR_CACHE_ROWS`.
    cache: parking_lot::Mutex<SidecarCache>,
}

/// Hand-rolled bounded LRU keyed by `d_id`. `order` is a recency list, front =
/// least-recently-used, back = most-recently-used; `map` holds the decoded rows.
/// On a hit the key is moved to the back; on insert past capacity the front key
/// is evicted. Kept tiny/local so it needs no external LRU crate.
struct SidecarCache {
    map: HashMap<usize, Vec<f32>>,
    order: VecDeque<usize>,
}

impl SidecarCache {
    fn new() -> Self {
        SidecarCache {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// Move `d_id` to the most-recently-used position. Linear scan of `order`
    /// bounded by `SIDECAR_CACHE_ROWS`, so O(cap) worst case.
    fn touch(&mut self, d_id: usize) {
        if let Some(pos) = self.order.iter().position(|&k| k == d_id) {
            self.order.remove(pos);
        }
        self.order.push_back(d_id);
    }

    fn get(&mut self, d_id: usize) -> Option<Vec<f32>> {
        if let Some(row) = self.map.get(&d_id).cloned() {
            self.touch(d_id);
            Some(row)
        } else {
            None
        }
    }

    fn put(&mut self, d_id: usize, row: Vec<f32>) {
        if self.map.insert(d_id, row).is_some() {
            // Key already resident: the value is overwritten above; just bump recency.
            self.touch(d_id);
            return;
        }
        while self.order.len() >= SIDECAR_CACHE_ROWS {
            if let Some(evict) = self.order.pop_front() {
                self.map.remove(&evict);
            } else {
                break;
            }
        }
        // Row was inserted above; the new key just needs its recency slot.
        self.order.push_back(d_id);
    }
}

impl SidecarReader {
    /// `pub` (not just `pub(crate)`) so `tests/*.rs` integration crates — which
    /// link against this crate as an external dependency — can construct a
    /// reader directly against a hand-written `fvec_<name>.bin` fixture. Query
    /// API surface is unaffected: this type is not reachable from any HQL/NAPI
    /// entry point.
    pub fn new(file: File, dim: usize) -> Self {
        SidecarReader {
            file,
            dim,
            cache: parking_lot::Mutex::new(SidecarCache::new()),
        }
    }

    /// Read exactly `dim` little-endian f32s for arena row `d_id` at byte offset
    /// `d_id * dim * 4`. Returns `None` on a short read / out-of-range `d_id`
    /// (never panics, never reads past EOF). Checks the LRU cache first; a cache
    /// hit is byte-identical to the disk read.
    pub fn row(&self, d_id: usize) -> Option<Vec<f32>> {
        if self.dim == 0 {
            return None;
        }
        // Fast path: cached decode.
        if let Some(row) = self.cache.lock().get(d_id) {
            return Some(row);
        }

        let row_bytes = self.dim.checked_mul(4)?; // symmetry with the offset overflow guard below
        let offset = (d_id as u64).checked_mul(row_bytes as u64)?;
        let mut buf = vec![0u8; row_bytes];
        // Loop to tolerate short positioned reads (fill the whole row or bail).
        let mut filled = 0usize;
        while filled < row_bytes {
            let cur = offset + filled as u64;
            #[cfg(windows)]
            let n = self.file.seek_read(&mut buf[filled..], cur).ok()?;
            #[cfg(unix)]
            let n = self.file.read_at(&mut buf[filled..], cur).ok()?;
            if n == 0 {
                // EOF before a full row — out of range / truncated.
                return None;
            }
            filled += n;
        }

        let row: Vec<f32> = buf
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        self.cache.lock().put(d_id, row.clone());
        Some(row)
    }

    /// Truncate the backing file to zero length and drop the LRU cache. Used by
    /// compaction to rewrite the sidecar in place (truncate, then re-append the
    /// surviving rows via `write_rows`). Clearing the cache is REQUIRED: after a
    /// truncate+rewrite, a given row index `d_id` may hold a different vector than
    /// before, so any cached decode keyed by that index is stale and must go.
    fn truncate_empty(&self) -> std::io::Result<()> {
        self.file.set_len(0)?;
        let mut c = self.cache.lock();
        c.map.clear();
        c.order.clear();
        Ok(())
    }

    /// Number of complete `dim`-wide rows on disk (`file_len / (dim*4)`).
    pub fn len_rows(&self) -> usize {
        if self.dim == 0 {
            return 0;
        }
        let row_bytes = (self.dim * 4) as u64;
        match self.file.metadata() {
            Ok(m) => (m.len() / row_bytes) as usize,
            Err(_) => 0,
        }
    }

    /// Append helper: write flat f32 `rows` to the end of the file as
    /// little-endian bytes. Used by `stage()` (append one row per add), and by
    /// save / compaction to (re)materialize the sidecar. Does not touch the LRU:
    /// a freshly-appended `d_id` has never been read so it can't be cached stale,
    /// and re-materialize paths open a fresh reader with an empty cache.
    fn write_rows(&self, rows: &[f32]) -> std::io::Result<()> {
        let mut buf = Vec::with_capacity(rows.len() * 4);
        for &v in rows {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        // Positioned append at current EOF so this stays symmetric with the
        // positioned-read path (no reliance on a shared file cursor).
        let end = self.file.metadata()?.len();
        #[cfg(windows)]
        {
            let mut written = 0usize;
            while written < buf.len() {
                let n = self
                    .file
                    .seek_write(&buf[written..], end + written as u64)?;
                if n == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "sidecar write_rows wrote 0 bytes",
                    ));
                }
                written += n;
            }
        }
        #[cfg(unix)]
        {
            let mut written = 0usize;
            while written < buf.len() {
                let n = self.file.write_at(&buf[written..], end + written as u64)?;
                if n == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "sidecar write_rows wrote 0 bytes",
                    ));
                }
                written += n;
            }
        }
        Ok(())
    }
}

/// One isolated vector space (ADR--GENESISDB-MULTI-COLLECTION / SPEC §3). All
/// vectors in a collection come from ONE embedding model and share one dim +
/// metric + arena + metadata + HNSW index. Cross-model distances are
/// meaningless, so models must never share a collection.
pub struct VectorCollection {
    pub name: String,
    pub model: String,
    pub dim: u16,
    pub metric: Metric,
    pub quant: Quant,
    pub arena: RwLock<ArenaStore>, // element type per `quant`; offsets in components
    pub metadata: RwLock<Vec<NodeMetadata>>,
    hnsw: RwLock<Option<VecIndex>>,
    pub node_to_arena: DashMap<u32, u32>, // node u32 -> arena_id (this collection)
    pub count: AtomicUsize,
    /// Per-collection default HNSW `ef_search`. Set at creation, immutable.
    /// `None` ⇒ fall back to the engine-global default. Resolution order in
    /// `hybrid_search`: per-query override → this → engine-global.
    pub ef_search: Option<u32>,
    /// Optional exact-f32 sidecar for rerank, kept ON DISK (post-P0) in
    /// `fvec_<name>.bin` behind a positioned-read `SidecarReader` — NOT resident
    /// in RAM (the old `Vec<f32>` re-created the very residency cost rerank was
    /// meant to avoid; see docs/RCA--VECTOR-QUANTIZATION.md). Row `arena_id` lives
    /// at byte offset `arena_id * dim * 4` (fixed dim ⇒ `arena_id*dim ==
    /// embedding_offset`), so the on-disk row index stays lock-step with the
    /// arena. `Some` only for a quantized collection created with `rerank = true`;
    /// `None` collections and non-rerank quantized collections leave it `None`.
    /// The `RwLock` serializes stage-time appends (`write_rows`) and the
    /// compaction rewrite/reopen against readers.
    pub f32_sidecar: Option<RwLock<SidecarReader>>,
    /// Per-dim BQ centering vector (`len == dim`), computed at compaction as the
    /// mean of the live rows' exact f32 (read from `f32_sidecar`). bge-m3 dims are
    /// positive-biased, so an uncentered `x > 0` sign bit carries little signal;
    /// subtracting this mean before the sign bit rebalances the codes and lifts
    /// BQ ranking quality (BQ-with-centering; Faiss/Qdrant `ubinary`). `Some` only
    /// for a `Quant::Binary` collection that carries a sidecar (the f32 source the
    /// mean needs) AND has been compacted at least once; `None` everywhere else ⇒
    /// uncentered `bq_pack` (byte-identical to pre-P1a behavior). Persisted as
    /// `bqmean_<name>.bin`. Query and EVERY insert path subtract the SAME center;
    /// rehydrate feeds already-centered arena codes (±1) and does NOT re-center.
    /// `RwLock<Option<…>>` (not `Option<RwLock<…>>`) so compaction can install a
    /// freshly-computed center through the shared `Arc<VectorCollection>` without
    /// a `&mut`. `None` inside ⇒ uncentered.
    pub bq_center: RwLock<Option<Vec<f32>>>,
    /// Whether this SQ8 collection opts into a **calibrated** scale (P2b). Set at
    /// creation from a `quant:"sq8c"`/`"sq8-calibrated"` request and persisted in
    /// the manifest. `false` ⇒ the fixed `SQ8_FIXED` scale (byte-identical to
    /// pre-P2b). Calibration also needs an f32 source, so it only fires for an
    /// SQ8 collection that ALSO has a rerank sidecar.
    pub sq8_calibrate: bool,
    /// Per-collection calibrated SQ8 `(scale, bias)`, computed at compaction from
    /// the quantile range of the sidecar f32. `None` ⇒ `SQ8_FIXED`. Kept lock-step
    /// with the arena's own `(scale, bias)` (set together at compaction/load) so
    /// the HNSW (threaded this value) and the arena agree on one scale.
    pub sq8_scale: RwLock<Option<(f32, f32)>>,
}

impl VectorCollection {
    /// `sidecar_path` is the on-disk backing for the rerank sidecar. It is only
    /// consulted when `rerank && quant != None`; callers pass
    /// `Some(<path>/fvec_<name>.bin)` in that case and `None` otherwise. The file
    /// is opened read+write here (created if absent). `truncate` controls whether
    /// a pre-existing file is emptied: `true` for a freshly-created collection
    /// (starts empty), `false` on load (adopt the on-disk rows, then the caller
    /// validates `len_rows()` against the arena and drops the sidecar on
    /// mismatch). If the open fails we degrade to `None` (quantized-only search)
    /// rather than abort collection creation.
    #[allow(clippy::too_many_arguments)]
    fn new(
        name: String,
        model: String,
        dim: u16,
        metric: Metric,
        quant: Quant,
        ef_search: Option<u32>,
        rerank: bool,
        sidecar_path: Option<PathBuf>,
        truncate: bool,
        sq8_calibrate: bool,
    ) -> Self {
        // Rerank only makes sense for a lossy (quantized) arena; a `None`
        // collection already stores exact f32, so never open a sidecar there.
        let f32_sidecar = if rerank && matches!(quant, Quant::ScalarU8 | Quant::Binary) {
            sidecar_path.and_then(|p| {
                FileOpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(truncate)
                    .open(&p)
                    .ok()
                    .map(|f| RwLock::new(SidecarReader::new(f, dim as usize)))
            })
        } else {
            None
        };
        Self {
            name,
            model,
            dim,
            metric,
            quant,
            ef_search,
            f32_sidecar,
            arena: RwLock::new(ArenaStore::new(quant, dim as usize)),
            metadata: RwLock::new(Vec::new()),
            hnsw: RwLock::new(None),
            node_to_arena: DashMap::new(),
            count: AtomicUsize::new(0),
            // Uncentered until a compaction computes the mean from the sidecar;
            // load() overwrites this from `bqmean_<name>.bin` when present.
            bq_center: RwLock::new(None),
            sq8_calibrate,
            // Fixed scale until a compaction calibrates (opt-in SQ8 only);
            // load() overwrites this from `sq8scale_<name>.bin` when present.
            sq8_scale: RwLock::new(None),
        }
    }

    /// Snapshot this collection's BQ centering vector for a pack/search call.
    /// Returns `None` (⇒ uncentered) unless the collection has a computed center
    /// AND is BQ — so non-BQ collections are always uncentered. The clone keeps
    /// the (short) `bq_center` read lock off the hot pack loop.
    fn center_snapshot(&self) -> Option<Vec<f32>> {
        if self.quant != Quant::Binary {
            return None;
        }
        self.bq_center.read().clone()
    }

    /// The SQ8 `(scale, bias)` to pack/unpack with — the calibrated scale if one
    /// has been computed, else `SQ8_FIXED`. Always `SQ8_FIXED` for non-SQ8
    /// collections. Must match the arena's own `(scale, bias)`; they are set
    /// together at compaction and load.
    fn sq8_snapshot(&self) -> (f32, f32) {
        if self.quant != Quant::ScalarU8 {
            return SQ8_FIXED;
        }
        self.sq8_scale.read().unwrap_or(SQ8_FIXED)
    }

    /// Build an HNSW index reserving capacity for ~`cap` elements. hnsw_rs uses
    /// this only as a `Vec::with_capacity` hint per graph layer and grows via
    /// plain `Vec::push` on overflow (hnsw.rs:511), so a small `cap` is safe —
    /// it just reallocates (amortized) past the hint. The old hardcoded
    /// `1_000_000` was NOT ~8 MB as once assumed: the layer-fraction reservation
    /// compounds across layers to >100 MB *per index*, so every freshly-created
    /// collection index eagerly committed that much. With many collections (or
    /// many DBs open at once, e.g. parallel tests) those reservations stacked and
    /// aborted on OOM. Size to the data instead (ADR--GENESISDB-HNSW-CAPACITY).
    const HNSW_MIN_CAP: usize = 1024;

    /// Exhaustive exact-distance scan over every slot, nearest-first, capped at
    /// `k`. This is the recall floor beneath the approximate index — see
    /// `EXACT_SCAN_MAX_SLOTS` and RCA--HNSW-UNDER-RETURN-SMALL-GRAPH.
    ///
    /// Distances come from `ArenaStore::f32_at`, which dequantizes for every
    /// `Quant` mode, so a caller must REPLACE the candidate set with this rather
    /// than merge into it — mixing these with raw graph distances (Hamming for
    /// `Binary`) would rank two different scales against each other.
    fn exact_candidates(&self, query: &[f32], k: usize) -> Vec<(usize, f32)> {
        let meta = self.metadata.read();
        let arena = self.arena.read();
        let mut out: Vec<(usize, f32)> = Vec::with_capacity(meta.len());
        for (i, m) in meta.iter().enumerate() {
            let (start, len) = (m.embedding_offset as usize, m.vector_dim as usize);
            // Skip a slot the arena can't serve (dim mismatch / truncated
            // arena) rather than panicking on the slice.
            if len == 0 || len != query.len() || start + len > arena.len() {
                continue;
            }
            out.push((i, exact_l2(query, &arena.f32_at(start, len))));
        }
        out.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        out.truncate(k);
        out
    }

    fn ensure_hnsw(&self, ef_construction: usize) {
        if self.hnsw.read().is_none() {
            let mut w = self.hnsw.write();
            // Lazy create: final element count is unknown here, so reserve the
            // floor and let inserts grow it. Rehydrate (count known) sizes exactly.
            if w.is_none() {
                *w = Some(VecIndex::build(
                    self.quant,
                    ef_construction,
                    Self::HNSW_MIN_CAP,
                ));
            }
        }
    }

    /// f64 -> f32, normalizing for Cosine collections so DistL2 ranks by cosine.
    fn prep(&self, emb_64: Vec<f64>) -> Vec<f32> {
        let mut emb: Vec<f32> = emb_64.into_iter().map(|v| v as f32).collect();
        if self.metric == Metric::Cosine {
            let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in emb.iter_mut() {
                    *x /= norm;
                }
            }
        }
        emb
    }

    /// Push a prepared vector into the arena/metadata, return its arena_id.
    /// Does NOT touch HNSW. Short critical section: only the Vec pushes.
    fn stage(&self, node_u32: u32, emb: &[f32], lang: String) -> u32 {
        // BQ centering: the arena stores centered sign codes (`sign(x-center)`),
        // but the sidecar below keeps the RAW f32 (rerank re-scores exactly and
        // the next compaction recomputes the mean from raw). `None` ⇒ uncentered.
        let center = self.center_snapshot();
        let arena_id = {
            let mut meta = self.metadata.write();
            let mut arena = self.arena.write();
            let off = arena.len();
            arena.push_f32(emb, center.as_deref());
            // Rerank sidecar: persist the exact f32 row to disk in lock-step with
            // the arena. `write_rows` positioned-appends exactly `dim*4` bytes at
            // file-EOF = `arena_id*dim*4`, so the on-disk row index stays lock-step
            // with `embedding_offset/dim`. We hold arena.write() (exclusive) across
            // this call, so appends are serialized — no shared-cursor race. Lock
            // order stays meta → arena → sidecar everywhere (stage / compaction).
            // A write failure must not silently corrupt the sidecar; on error we
            // give up on rerank for this collection by dropping to quantized-only
            // rather than leave the file half-written (a short row would desync the
            // whole tail). Load/compaction/save all guard len_rows == arena_rows.
            if let Some(sidecar) = &self.f32_sidecar {
                let _ = sidecar.write().write_rows(emb);
            }
            let arena_id = meta.len() as u32;
            meta.push(NodeMetadata {
                arena_id,
                node_u32,
                timestamp: Utc::now().timestamp() as u64,
                vector_dim: self.dim,
                embedding_offset: off as u64,
                gks_attributes: Vec::new(),
                lang,
                cluster_id: arena_id,
            });
            arena_id
        };
        self.node_to_arena.insert(node_u32, arena_id);
        self.count.fetch_add(1, Ordering::Relaxed);
        arena_id
    }

    /// Rebuild this collection's HNSW from its arena (the source of truth).
    fn rehydrate(&self, ef_c: usize) {
        let meta = self.metadata.read();
        if meta.is_empty() {
            return;
        }
        let index = VecIndex::build(self.quant, ef_c, meta.len());
        let arena = self.arena.read();
        // SQ8: re-quantize with the ARENA's own scale (what f32_at just dequantized
        // with), so the rehydrated HNSW codes match the arena exactly.
        let sq8 = arena.sq8_scale();
        for m in meta.iter() {
            let start = m.embedding_offset as usize;
            let len = m.vector_dim as usize;
            if start + len <= arena.len() {
                let v = arena.f32_at(start, len);
                // The arena already holds centered BQ codes; `f32_at` returns them
                // as ±1, so re-packing must NOT center again (pass `None`) — the
                // sign of ±1 reproduces the stored centered code exactly.
                index.insert_f32(&v, m.arena_id as usize, None, sq8);
            }
        }
        *self.hnsw.write() = Some(index);
    }

    /// `index_lag` is engine-global (`Storage::index_lag()`), passed in by
    /// `Storage::list_collections()` so every entry carries it — `info()` lives on
    /// the collection and has no view of the Storage-wide indexing backlog.
    fn info(&self, index_lag: u32) -> CollectionInfo {
        // Sidecar is ON DISK post-P0: its vector data is not resident, so resident
        // bytes are 0. On-disk bytes = live rows * dim * 4 (fixed dim).
        let (sidecar_resident_bytes, sidecar_disk_bytes) = match &self.f32_sidecar {
            Some(s) => {
                let rows = s.read().len_rows() as i64;
                (0i64, rows * (self.dim as i64) * 4)
            }
            None => (0i64, 0i64),
        };
        CollectionInfo {
            name: self.name.clone(),
            model: self.model.clone(),
            dim: self.dim as u32,
            metric: self.metric.as_str().to_string(),
            quant: self.quant.as_str().to_string(),
            count: self.count.load(Ordering::Relaxed) as u32,
            ef_search: self.ef_search,
            rerank: self.f32_sidecar.is_some(),
            sidecar_resident_bytes,
            sidecar_disk_bytes,
            arena_resident_bytes: self.arena.read().byte_size() as i64,
            index_lag,
        }
    }
}

/// A unit of deferred HNSW indexing, processed off the write hot path by the
/// per-Storage indexing thread (ADR--GENESISDB-ASYNC-INDEXING). The vector is
/// already staged in the collection's arena (durable) before the job is sent;
/// only the HNSW graph insert is deferred.
enum IndexJob {
    One {
        coll: Arc<VectorCollection>,
        arena_id: u32,
        emb: Vec<f32>,
        ef_c: usize,
    },
    Batch {
        coll: Arc<VectorCollection>,
        items: Vec<(Vec<f32>, u32)>,
        ef_c: usize,
    },
    Flush(Sender<()>),
}

/// A message to the WAL-writer thread. Every WAL mutation is serialized through
/// this single channel so appends and checkpoints can never race the file.
///
/// `Checkpoint` exists because the writer thread holds `genesis-graph.wal` open
/// for append for the whole `Storage` lifetime, and Windows refuses to rename
/// over a file with a live handle — so the old rename-based `compact()` silently
/// no-op'd there and the WAL grew unbounded (AUDIT--P33: ~20 GB at 1M nodes).
/// Only the thread that owns the handle can replace the file safely, so the
/// checkpoint payload (already-serialized live-state `SignedEvent` lines) is
/// handed to it and the file is truncated + rewritten there (see `wal_checkpoint`).
pub enum WalMsg {
    /// Append one signed event; `ack` fires with the stamped `commit_seq` once
    /// the frame is flushed + fsynced (WP-1.2, ADR D2.4: the ack returns the
    /// stamp so `CommitResult` and the projection consume the local frame seq).
    /// `None` = append failed (read-only, or writer error).
    Append(Box<SignedEvent>, Sender<Option<u64>>),
    /// Fold-to-base checkpoint (ADR D3 `frontier_only` profile — the WP-1.2
    /// interim default until WP-1.3 lands budget profiles): frame each
    /// `payload_lines` JSONL line at `frontier_seq` into a base segment
    /// (kind=base, derived/materialized), then drop all older journal files and
    /// start a fresh active file. The journal stays a complete recovery source
    /// at every instant (I8): the base segment IS the folded history's live
    /// effect. Executed on the writer thread — only the handle owner can rotate
    /// the active file (Windows; same constraint PR #28 hit).
    Checkpoint {
        payload_lines: Vec<u8>,
        frontier_seq: u64,
        ack: Sender<bool>,
    },
}

// === WP-1.2 framed journal codec (SPEC--GENESISDB-JOURNAL-FORMAT-V1) ===
// Frame:   [u32 len | u64 commit_seq | u32 crc32c(seq||payload) | payload]
//          payload = the ORIGINAL serde_json SignedEvent bytes (I4: frames wrap,
//          never rewrite — signatures verify over stored bytes).
// Active:  wal/active.gwal = GWA1 header (magic4 + ver2 + first_seq8), then
//          uncompressed frames (append + group-commit fsync).
// Segment: journal/<J|B><min_seq>.gseg = GSG1 header (magic4 + ver2 + kind1 +
//          codec1 + min8 + max8 + count4), codec-compressed frame body, footer
//          (sha256(body)32 + body_len8 + crc32c(header||sha||len)4).

const ACTIVE_MAGIC: [u8; 4] = *b"GWA1";
const SEG_MAGIC: [u8; 4] = *b"GSG1";
const JOURNAL_FORMAT_VERSION: u16 = 1;
const ACTIVE_HEADER_LEN: usize = 14;
const FRAME_HEADER_LEN: usize = 16;
const SEG_HEADER_LEN: usize = 28;
const SEG_FOOTER_LEN: usize = 44;
/// Active-file seal threshold (spec §2; mobile profile tightens this in WP-1.3).
const ACTIVE_SEAL_THRESHOLD: u64 = 64 * 1024 * 1024;

const SEG_KIND_HISTORY: u8 = 1;
const SEG_KIND_BASE: u8 = 2;
const SEG_KIND_LEGACY_JSONL: u8 = 3;

const CODEC_NONE: u8 = 0;
// 1 was zstd — dropped in this same PR (SPEC §8): zstd-sys's vendored C
// fails to link for aarch64-apple-ios ("Undefined symbols... ___chkstk_darwin",
// mobile-build.yml run 31982222256). Reserved/unreadable — `read_segment_body`
// falls through to its `_ => None` arm for byte 1, so an old segment
// (none exist outside this branch) fails closed rather than silently misreading.
const CODEC_LZ4: u8 = 2;

/// LZ4 frame format (not the size-prepended block API) so the SAME encoder
/// type serves both a fully-in-memory body (`write_segment`) and a true
/// bounded-memory stream from a `Read` source (`migrate_legacy_wal` — the
/// legacy WAL can be tens of GB and must not be materialized whole).
fn lz4_frame_compress(body: &[u8]) -> std::io::Result<Vec<u8>> {
    use std::io::Write;
    let mut out = Vec::new();
    {
        let mut enc = lz4_flex::frame::FrameEncoder::new(&mut out);
        enc.write_all(body)?;
        enc.finish()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
    }
    Ok(out)
}

fn lz4_frame_decompress(compressed: &[u8]) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut dec = lz4_flex::frame::FrameDecoder::new(compressed);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)?;
    Ok(out)
}

fn frame_crc(seq: u64, payload: &[u8]) -> u32 {
    crc32c::crc32c_append(crc32c::crc32c(&seq.to_le_bytes()), payload)
}

fn encode_frame(out: &mut Vec<u8>, seq: u64, payload: &[u8]) {
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&seq.to_le_bytes());
    out.extend_from_slice(&frame_crc(seq, payload).to_le_bytes());
    out.extend_from_slice(payload);
}

fn active_header(first_seq: u64) -> [u8; ACTIVE_HEADER_LEN] {
    let mut h = [0u8; ACTIVE_HEADER_LEN];
    h[0..4].copy_from_slice(&ACTIVE_MAGIC);
    h[4..6].copy_from_slice(&JOURNAL_FORMAT_VERSION.to_le_bytes());
    h[6..14].copy_from_slice(&first_seq.to_le_bytes());
    h
}

/// Walk frames in `bytes` starting at `start`; call `f(seq, payload)` for each
/// valid frame. Returns the byte offset of the first torn/corrupt frame
/// (== `bytes.len()` when everything parsed clean) so callers can truncate the
/// tear away (I9: a crash leaves at most a torn tail, never a poisoned file).
fn walk_frames(bytes: &[u8], start: usize, mut f: impl FnMut(u64, &[u8])) -> usize {
    let mut off = start;
    loop {
        if off + FRAME_HEADER_LEN > bytes.len() {
            return off;
        }
        let len = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) as usize;
        let seq = u64::from_le_bytes(bytes[off + 4..off + 12].try_into().unwrap());
        let crc = u32::from_le_bytes(bytes[off + 12..off + 16].try_into().unwrap());
        let ps = off + FRAME_HEADER_LEN;
        if ps + len > bytes.len() {
            return off;
        }
        let payload = &bytes[ps..ps + len];
        if frame_crc(seq, payload) != crc {
            return off;
        }
        f(seq, payload);
        off = ps + len;
    }
}

/// Sealed-segment identity read off the fixed-size header (no body decode).
struct SegmentInfo {
    path: PathBuf,
    kind: u8,
    min_seq: u64,
    max_seq: u64,
}

fn read_segment_header(path: &std::path::Path) -> Option<SegmentInfo> {
    let mut file = File::open(path).ok()?;
    let mut h = [0u8; SEG_HEADER_LEN];
    file.read_exact(&mut h).ok()?;
    if h[0..4] != SEG_MAGIC {
        return None;
    }
    Some(SegmentInfo {
        path: path.to_path_buf(),
        kind: h[6],
        min_seq: u64::from_le_bytes(h[8..16].try_into().unwrap()),
        max_seq: u64::from_le_bytes(h[16..24].try_into().unwrap()),
    })
}

/// Read + verify a sealed segment; returns (kind, uncompressed body). `None`
/// on any integrity failure (magic/sha/crc/codec) — the caller treats the
/// segment as unreadable and recovery degrades explicitly, never silently.
fn read_segment_body(path: &std::path::Path) -> Option<(u8, Vec<u8>)> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() < SEG_HEADER_LEN + SEG_FOOTER_LEN || bytes[0..4] != SEG_MAGIC {
        return None;
    }
    let kind = bytes[6];
    let codec = bytes[7];
    let footer_start = bytes.len() - SEG_FOOTER_LEN;
    let sha_expected = &bytes[footer_start..footer_start + 32];
    let body_len = u64::from_le_bytes(
        bytes[footer_start + 32..footer_start + 40]
            .try_into()
            .unwrap(),
    );
    let crc_expected = u32::from_le_bytes(
        bytes[footer_start + 40..footer_start + 44]
            .try_into()
            .unwrap(),
    );
    let mut crc_input = Vec::with_capacity(SEG_HEADER_LEN + 40);
    crc_input.extend_from_slice(&bytes[0..SEG_HEADER_LEN]);
    crc_input.extend_from_slice(sha_expected);
    crc_input.extend_from_slice(&body_len.to_le_bytes());
    if crc32c::crc32c(&crc_input) != crc_expected {
        return None;
    }
    let compressed = &bytes[SEG_HEADER_LEN..footer_start];
    let body = match codec {
        CODEC_LZ4 => lz4_frame_decompress(compressed).ok()?,
        CODEC_NONE => compressed.to_vec(),
        _ => return None,
    };
    if body.len() as u64 != body_len || Sha256::digest(&body)[..] != sha_expected[..] {
        return None;
    }
    Some((kind, body))
}

/// Seal `body` (uncompressed frame bytes, or raw JSONL for legacy segments)
/// into `final_path` with I9 atomicity: tmp → fsync → rename → best-effort
/// directory fsync (Windows has no directory fsync; rename durability there is
/// best-effort, per ADR D1).
fn write_segment(
    final_path: &std::path::Path,
    kind: u8,
    codec: u8,
    min_seq: u64,
    max_seq: u64,
    frame_count: u32,
    body: &[u8],
) -> std::io::Result<()> {
    let compressed = match codec {
        CODEC_LZ4 => lz4_frame_compress(body)?,
        _ => body.to_vec(),
    };
    let mut header = Vec::with_capacity(SEG_HEADER_LEN);
    header.extend_from_slice(&SEG_MAGIC);
    header.extend_from_slice(&JOURNAL_FORMAT_VERSION.to_le_bytes());
    header.push(kind);
    header.push(codec);
    header.extend_from_slice(&min_seq.to_le_bytes());
    header.extend_from_slice(&max_seq.to_le_bytes());
    header.extend_from_slice(&frame_count.to_le_bytes());
    let sha = Sha256::digest(body);
    let mut crc_input = Vec::with_capacity(SEG_HEADER_LEN + 40);
    crc_input.extend_from_slice(&header);
    crc_input.extend_from_slice(&sha);
    crc_input.extend_from_slice(&(body.len() as u64).to_le_bytes());
    let crc = crc32c::crc32c(&crc_input);

    let tmp_path = final_path.with_extension("gseg.tmp");
    {
        let mut f = File::create(&tmp_path)?;
        f.write_all(&header)?;
        f.write_all(&compressed)?;
        f.write_all(&sha)?;
        f.write_all(&(body.len() as u64).to_le_bytes())?;
        f.write_all(&crc.to_le_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp_path, final_path)?;
    #[cfg(unix)]
    if let Some(dir) = final_path.parent() {
        if let Ok(d) = File::open(dir) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

pub struct Storage {
    pub path: PathBuf,
    pub read_only: bool,
    projection_path: PathBuf,
    projection_db: Mutex<Connection>,
    commit_lock: Mutex<()>,
    /// WP-1.2 (ADR D2.3): the FRAME frontier — commit_seq of the last durable
    /// journal frame, advancing on every mutation. Replica-local; never
    /// comparable across peers. `stable_frontier()` reports this.
    commit_sequence: AtomicU64,
    /// Frame seq of the last `Event::Transaction` frame (the transaction-lineage
    /// frontier `expected_frontier` CASes against — a frame-level CAS would
    /// spuriously fail on any interleaved ordinary write).
    txn_frontier: AtomicU64,
    /// Pre-WP-1.2 JSONL WAL path (`genesis-graph.wal`). Exists only until the
    /// one-way migration seals it as segment 0, or indefinitely in read-only
    /// opens of a legacy database.
    legacy_log_path: PathBuf,
    pub nodes: DashMap<u32, NodeOutput>,
    pub edges: DashMap<u128, EdgeOutput>,
    pub out_idx: DashMap<u32, HashSet<u128>>,
    pub in_idx: DashMap<u32, HashSet<u128>>,
    /// Per-model/per-dim isolated vector spaces, keyed by collection name.
    /// Replaces the former single global arena/metadata/hnsw/u32_to_arena_id.
    pub collections: DashMap<String, Arc<VectorCollection>>,
    pub default_collection: String,
    pub log_path: PathBuf,
    pub bin_path: PathBuf,
    pub _lock_file: Option<File>,
    pub id_to_u32: DashMap<String, u32>,
    // Node interning Layer A (ADR--GENESISDB-NODE-ID-INTERNING): the u32->id
    // reverse map was dropped. A u32's id string is read from `nodes[u32].id`
    // (the canonical copy) on the rare paths that need it — mirroring how edges
    // resolve via `EdgeOutput.id` (ADR--GENESISDB-EDGE-ID-INTERNING). Saves one
    // full id-string copy per interned id.
    pub next_u32: AtomicU32,
    pub is_rebuilding: AtomicBool,
    // Posting lists are roaring bitmaps, not HashSet<u32>: far denser per-node
    // overhead at scale, union/iter stay fast (ADR--GENESISDB-NODE-ID-INTERNING,
    // A3). Only `find_fuzzy_id` reads this; nodes only (edges skip trigram).
    pub trigram_index: DashMap<String, RoaringBitmap>,
    pub lang_centroids: DashMap<String, Vec<f32>>,
    pub peers: DashMap<String, SyncPeer>,
    pub proposals: DashMap<String, ConsensusProposal>,
    pub meta_nodes: DashMap<u32, SuperNode>,
    pub meta_edges: DashMap<String, MetaEdge>,
    pub meta_history: DashMap<u32, Vec<SuperNode>>,
    pub wal_sender: Sender<WalMsg>,
    /// Deferred-indexing queue: live HNSW inserts run off the write hot path on
    /// a dedicated thread (ADR--GENESISDB-ASYNC-INDEXING). Internal — drive via
    /// add/flush, not directly (the job type is private).
    index_tx: Sender<IndexJob>,
    /// Vectors staged in an arena but not yet inserted into HNSW (observability;
    /// see `index_lag` / `flush_index`).
    index_pending: Arc<AtomicUsize>,
    /// Join handles for the WAL-writer and deferred-indexing threads. Held so
    /// `Drop` can close their senders + join them — letting any in-flight WAL
    /// flush / HNSW insert finish before the process tears down, rather than
    /// leaving detached threads running into static teardown. `Option` so `Drop`
    /// can `take()`.
    wal_handle: Option<std::thread::JoinHandle<()>>,
    index_handle: Option<std::thread::JoinHandle<()>>,
    pub local_peer_id: String,
    pub logical_clock: AtomicU32,
    pub gossip_port: AtomicU32,
    pub ef_construction: AtomicUsize,
    pub ef_search: AtomicUsize,
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

// --- Governance (AXIOMATIC GUARDS §2) ---

#[derive(Debug, PartialEq, PartialOrd)]
pub enum Tier {
    MASTER = 0,
    SPEC = 1,
    ADR = 2,
    USER = 3,
}

impl Tier {
    pub fn from_labels(labels: &[String]) -> Self {
        if labels.iter().any(|l| l.to_uppercase() == "MASTER") {
            Tier::MASTER
        } else if labels.iter().any(|l| l.to_uppercase() == "SPEC") {
            Tier::SPEC
        } else if labels.iter().any(|l| l.to_uppercase() == "ADR") {
            Tier::ADR
        } else {
            Tier::USER
        }
    }
}

/// Batch-insert entry: (node_u32, node_id, embedding, lang).
type CollVecBatch = Vec<(u32, String, Vec<f64>, String)>;

impl Storage {
    pub fn validate_governance(&self, labels: &[String], is_system: bool) -> Result<()> {
        let tier = Tier::from_labels(labels);
        if tier == Tier::MASTER && !is_system {
            return Err(Error::from_reason(
                "403 Forbidden: MASTER tier is immutable for external agents",
            ));
        }
        Ok(())
    }

    fn tokenize_id(id: &str) -> Vec<String> {
        let base_chars: String = id
            .chars()
            .filter(|c| {
                let cat = unicode_general_category::get_general_category(*c);
                !matches!(
                    cat,
                    unicode_general_category::GeneralCategory::NonspacingMark
                        | unicode_general_category::GeneralCategory::SpacingMark
                        | unicode_general_category::GeneralCategory::EnclosingMark
                )
            })
            .collect();

        let mut tokens: Vec<String> = id.chars().map(|c| c.to_lowercase().to_string()).collect();
        if id != base_chars {
            tokens.extend(base_chars.chars().map(|c| c.to_lowercase().to_string()));
        }
        tokens.extend(
            id.chars()
                .collect::<Vec<_>>()
                .windows(2)
                .map(|w| w.iter().collect::<String>().to_lowercase()),
        );
        tokens
    }

    pub fn get_or_intern_id(&self, id: &str) -> u32 {
        if let Some(existing) = self.id_to_u32.get(id) {
            return *existing;
        }
        let new_id = self.next_u32.fetch_add(1, Ordering::SeqCst);
        self.id_to_u32.insert(id.to_string(), new_id);
        // No reverse-map insert: id string is recoverable via `nodes[u32].id`
        // (ADR--GENESISDB-NODE-ID-INTERNING, Layer A).

        for trigram in Self::tokenize_id(id) {
            self.trigram_index
                .entry(trigram)
                .or_default()
                .insert(new_id);
        }
        new_id
    }

    /// Derive an EDGE's internal u64 key from its string id — `SHA256(id)`
    /// truncated to the first 8 bytes (big-endian). Deterministic and
    /// allocation-free: edges store **no** string in `id_to_u32`, no reverse
    /// map, no counter. Re-deriving on WAL replay / snapshot reload reproduces
    /// the same key, so `edges` + `out_idx`/`in_idx` stay consistent without
    /// stored coordination. `EdgeOutput.id` remains the canonical reverse
    /// lookup. Idempotency = "is this u64 already in `edges`?".
    /// See ADR--GENESISDB-EDGE-NUMERIC-KEYS (Layer B) / RCA--EDGE-ID-INTERNING-RAM.
    pub fn edge_key(id: &str) -> u128 {
        // 128-bit truncation of SHA256(id). Widened from u64 (Layer B) to slash
        // the birthday-collision risk: ~1.7e-6 at 8M edges (u64) -> ~9e-26 (u128).
        // The key is always derived from EdgeOutput.id, never stored, so this is a
        // pure in-memory width change — legacy u64 snapshots re-key transparently.
        let digest = Sha256::digest(id.as_bytes());
        u128::from_be_bytes(digest[..16].try_into().unwrap())
    }

    pub fn get_u32(&self, id: &str) -> Option<u32> {
        self.id_to_u32.get(id).map(|v| *v)
    }

    /// Tune HNSW build/search effort. Call before bulk load to trade recall for
    /// speed (e.g. 100/100 = fast, 200/100 = quality default). Affects future
    /// inserts and the next rebuild; not a retroactive re-index. Applies to all
    /// collections (the tunables are global).
    pub fn set_index_params(&self, ef_construction: u32, ef_search: u32) {
        self.ef_construction
            .store(ef_construction as usize, Ordering::Relaxed);
        self.ef_search.store(ef_search as usize, Ordering::Relaxed);
    }

    // --- Collection resolution (multi-collection vector space) ---

    /// The default vector collection (always present — created at open).
    fn default_coll(&self) -> Arc<VectorCollection> {
        self.collections
            .get(&self.default_collection)
            .map(|r| Arc::clone(r.value()))
            .expect("default collection must always exist")
    }

    /// Resolve a collection by optional name (None -> default).
    fn resolve_collection(&self, name: &Option<String>) -> Result<Arc<VectorCollection>> {
        let n = name
            .clone()
            .unwrap_or_else(|| self.default_collection.clone());
        self.collections
            .get(&n)
            .map(|r| Arc::clone(r.value()))
            .ok_or_else(|| Error::from_reason(format!("collection '{}' not found", n)))
    }

    /// Create an isolated vector collection. Idempotent-erroring: fails if a
    /// collection with this name already exists.
    #[allow(clippy::too_many_arguments)]
    pub fn create_collection(
        &self,
        name: String,
        model: String,
        dim: u32,
        metric: Option<String>,
        quant: Option<String>,
        ef_search: Option<u32>,
        rerank: Option<bool>,
    ) -> Result<()> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        if self.collections.contains_key(&name) {
            return Err(Error::from_reason(format!(
                "collection '{}' already exists",
                name
            )));
        }
        let m = metric.as_deref().map(Metric::parse).unwrap_or(Metric::L2);
        let q = quant.as_deref().map(Quant::parse).unwrap_or(Quant::None);
        // SQ8 calibrated-scale opt-in, encoded in the quant string ("sq8c") so it
        // needs no new create_collection arg (automatic NAPI+REST parity).
        let sq8_calibrate =
            q == Quant::ScalarU8 && quant.as_deref().is_some_and(Quant::sq8_calibrate_requested);
        let rerank = rerank.unwrap_or(false);
        // Fresh collection: truncate-create an empty fvec so on-disk rows start
        // lock-step with the (empty) arena. Path only matters when rerank+quant.
        let sidecar_path = if rerank && q != Quant::None {
            Some(self.path.join(format!("fvec_{}.bin", name)))
        } else {
            None
        };
        self.collections.insert(
            name.clone(),
            Arc::new(VectorCollection::new(
                name,
                model,
                dim as u16,
                m,
                q,
                ef_search,
                rerank,
                sidecar_path,
                true, // fresh collection: start empty
                sq8_calibrate,
            )),
        );
        Ok(())
    }

    pub fn list_collections(&self) -> Vec<CollectionInfo> {
        // `index_lag` is engine-global; snapshot it once and stamp every entry.
        let lag = self.index_lag();
        self.collections
            .iter()
            .map(|c| c.value().info(lag))
            .collect()
    }

    /// Insert one vector into the named (or default) collection. Validates the
    /// embedding length against the collection dim, stages it into the arena
    /// (durable, immediately in-memory), and defers the HNSW insert to the
    /// indexing thread (ADR--GENESISDB-ASYNC-INDEXING).
    fn add_vector_internal(
        &self,
        collection: &Option<String>,
        node_id: &str,
        emb_64: Vec<f64>,
        lang: String,
    ) -> Result<()> {
        let coll = self.resolve_collection(collection)?;
        if emb_64.len() != coll.dim as usize {
            return Err(Error::from_reason(format!(
                "embedding dim {} != collection '{}' dim {}",
                emb_64.len(),
                coll.name,
                coll.dim
            )));
        }
        let node_u32 = self.get_or_intern_id(node_id);
        let emb = coll.prep(emb_64);
        let arena_id = coll.stage(node_u32, &emb, lang);
        self.enqueue_one(&coll, arena_id, emb);
        Ok(())
    }

    /// Attach an additional vector to an existing node, in `collection`. Lets one
    /// node carry vectors from different models/spaces (e.g. a `code` embedding
    /// and a `text` embedding). The node must already exist; the embedding dim is
    /// validated against the collection. The vector is staged into the arena
    /// (durable), the HNSW insert is deferred (eventually searchable), and an
    /// `Event::Vector` is persisted for replay — same durability as a node's
    /// primary embedding. A node holds at most one vector per collection;
    /// re-adding to a collection it already has supersedes the prior mapping
    /// (the old arena slot is reclaimed on the next compaction).
    pub fn add_vector(
        &self,
        node_id: String,
        collection: String,
        embedding: Vec<f64>,
    ) -> Result<()> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        // A vector attaches to a node — the node must exist.
        let exists = self
            .get_u32(&node_id)
            .is_some_and(|u| self.nodes.contains_key(&u));
        if !exists {
            return Err(Error::from_reason(format!("node '{}' not found", node_id)));
        }
        let lang = "en".to_string();
        let coll = Some(collection);
        // Validates dim, stages into the arena, enqueues the deferred HNSW insert.
        self.add_vector_internal(&coll, &node_id, embedding.clone(), lang.clone())?;
        // Stamp a logical clock so the vector is time-orderable for anti-entropy
        // (events_since) — secondary embeddings now sync across peers like nodes.
        let clock = self.next_clock();
        // Durability: replayed by the WAL `Event::Vector` arm.
        self.persist(&Event::Vector(VectorEvent {
            node_id,
            collection: coll,
            embedding,
            lang: Some(lang),
            clock,
        }))?;
        Ok(())
    }

    fn enqueue_one(&self, coll: &Arc<VectorCollection>, arena_id: u32, emb: Vec<f32>) {
        self.index_pending.fetch_add(1, Ordering::Relaxed);
        let _ = self.index_tx.send(IndexJob::One {
            coll: Arc::clone(coll),
            arena_id,
            emb,
            ef_c: self.ef_construction.load(Ordering::Relaxed),
        });
    }

    fn enqueue_batch(&self, coll: &Arc<VectorCollection>, items: Vec<(Vec<f32>, u32)>) {
        if items.is_empty() {
            return;
        }
        self.index_pending.fetch_add(items.len(), Ordering::Relaxed);
        let _ = self.index_tx.send(IndexJob::Batch {
            coll: Arc::clone(coll),
            items,
            ef_c: self.ef_construction.load(Ordering::Relaxed),
        });
    }

    /// Block until every queued vector has been inserted into its HNSW index.
    /// Use before asserting searchability, and before any operation that
    /// reassigns arena ids (compaction / index rebuild) so a pending insert
    /// never targets a stale arena id.
    pub fn flush_index(&self) {
        let (tx, rx) = bounded(1);
        if self.index_tx.send(IndexJob::Flush(tx)).is_ok() {
            let _ = rx.recv();
        }
    }

    /// Vectors staged but not yet inserted into HNSW (eventually-searchable lag).
    pub fn index_lag(&self) -> u32 {
        self.index_pending.load(Ordering::Relaxed) as u32
    }

    /// WAL-replay / CRDT-sync vector insert: tolerant of a not-yet-created
    /// collection. `create_collection` is an in-memory op (durable only via the
    /// snapshot manifest), so on pure WAL replay — or a remote node referencing
    /// a collection we lack — auto-provision it from the embedding's dim (L2,
    /// model "recovered"). A subsequent save_state records the true model/metric.
    /// The live `add_node` path stays strict; only recovery/sync auto-provisions.
    /// `index = false` (startup WAL replay): stage into the arena only — the
    /// post-load `rehydrate_hnsw_index` builds every index once, so enqueuing
    /// here would double-insert. `index = true` (runtime CRDT sync): stage AND
    /// enqueue the deferred HNSW insert, since no rehydrate follows.
    fn replay_vector(
        &self,
        collection: &Option<String>,
        node_id: &str,
        emb: Vec<f64>,
        lang: String,
        index: bool,
    ) {
        let name = collection
            .clone()
            .unwrap_or_else(|| self.default_collection.clone());
        if !self.collections.contains_key(&name) {
            self.collections.insert(
                name.clone(),
                Arc::new(VectorCollection::new(
                    name.clone(),
                    "recovered".to_string(),
                    emb.len() as u16,
                    Metric::L2,
                    Quant::None,
                    None,
                    false,
                    None, // recovered collections are Quant::None + no rerank
                    true,
                    false, // Quant::None ⇒ no SQ8 calibration
                )),
            );
        }
        if let Ok(coll) = self.resolve_collection(&Some(name)) {
            if emb.len() != coll.dim as usize {
                return;
            }
            let node_u32 = self.get_or_intern_id(node_id);
            let e = coll.prep(emb);
            let arena_id = coll.stage(node_u32, &e, lang);
            if index {
                self.enqueue_one(&coll, arena_id, e);
            }
        }
    }

    /// Rebuild every collection's HNSW from its arena (both load paths).
    fn rehydrate_hnsw_index(&self) {
        let ef_c = self.ef_construction.load(Ordering::Relaxed);
        for c in self.collections.iter() {
            c.value().rehydrate(ef_c);
        }
    }

    /// Read the on-disk schema version from the snapshot manifest (state.json).
    /// `Some(0)` for a pre-versioned snapshot lacking the field; `None` when no
    /// snapshot exists yet (fresh database). Used by the open-time compatibility
    /// gate to refuse databases written by a newer engine.
    fn read_ondisk_schema_version(root: &std::path::Path) -> Option<u32> {
        let state_path = root.join("state.json");
        if !state_path.exists() {
            return None;
        }
        let txt = fs::read_to_string(&state_path).ok()?;
        let val: serde_json::Value = serde_json::from_str(&txt).ok()?;
        Some(val["schema_version"].as_u64().unwrap_or(0) as u32)
    }

    fn open_projection(path: &std::path::Path, read_only: bool) -> Result<Connection> {
        let flags = if read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
        };
        Connection::open_with_flags(path, flags).map_err(|e| Error::from_reason(e.to_string()))
    }

    fn init_projection_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS props (
                node_u32 INTEGER PRIMARY KEY,
                payload TEXT NOT NULL,
                clock_time INTEGER NOT NULL,
                clock_peer TEXT NOT NULL DEFAULT '',
                valid_from TEXT NOT NULL,
                valid_to TEXT
            );
            CREATE TABLE IF NOT EXISTS node_labels (
                node_u32 INTEGER NOT NULL,
                label TEXT NOT NULL,
                PRIMARY KEY (node_u32, label)
            );
            CREATE TABLE IF NOT EXISTS projection_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS relational_schema_registry (
                namespace TEXT PRIMARY KEY,
                schema_version INTEGER NOT NULL,
                package_id TEXT NOT NULL DEFAULT '',
                schema_hash TEXT NOT NULL DEFAULT '',
                package_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS applied_relational_mutations (
                mutation_id TEXT PRIMARY KEY,
                namespace TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                payload_hash TEXT NOT NULL,
                affected_rows INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS applied_transactions (
                transaction_id TEXT PRIMARY KEY,
                commit_sequence INTEGER NOT NULL,
                payload_hash TEXT NOT NULL,
                frame_seq INTEGER
            );
            ",
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;
        // WP-1.2 (ADR D2.2): uniqueness re-keys from the origin commit_sequence
        // to the LOCAL frame seq. Two replicas each committing their own
        // transaction N and then syncing used to collide on the old UNIQUE
        // constraint and abort the whole reconcile batch. SQLite cannot drop a
        // constraint, so pre-WP-1.2 tables are rebuilt in place.
        let txn_needs_rebuild = conn
            .prepare("PRAGMA table_info(applied_transactions)")
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
                let mut has_frame_seq = false;
                for name in rows {
                    if name? == "frame_seq" {
                        has_frame_seq = true;
                    }
                }
                Ok(!has_frame_seq)
            })
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if txn_needs_rebuild {
            conn.execute_batch(
                "
                CREATE TABLE applied_transactions_v2 (
                    transaction_id TEXT PRIMARY KEY,
                    commit_sequence INTEGER NOT NULL,
                    payload_hash TEXT NOT NULL,
                    frame_seq INTEGER
                );
                INSERT INTO applied_transactions_v2(transaction_id, commit_sequence, payload_hash)
                    SELECT transaction_id, commit_sequence, payload_hash FROM applied_transactions;
                DROP TABLE applied_transactions;
                ALTER TABLE applied_transactions_v2 RENAME TO applied_transactions;
                ",
            )
            .map_err(|e| Error::from_reason(e.to_string()))?;
        }
        let has_clock_peer = conn
            .prepare("PRAGMA table_info(props)")
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
                for name in rows {
                    if name? == "clock_peer" {
                        return Ok(true);
                    }
                }
                Ok(false)
            })
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if !has_clock_peer {
            conn.execute(
                "ALTER TABLE props ADD COLUMN clock_peer TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| Error::from_reason(e.to_string()))?;
        }
        let registry_columns = conn
            .prepare("PRAGMA table_info(relational_schema_registry)")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| row.get::<_, String>(1))?
                    .collect::<std::result::Result<HashSet<_>, _>>()
            })
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if !registry_columns.contains("package_id") {
            conn.execute(
                "ALTER TABLE relational_schema_registry ADD COLUMN package_id TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| Error::from_reason(e.to_string()))?;
        }
        if !registry_columns.contains("schema_hash") {
            conn.execute(
                "ALTER TABLE relational_schema_registry ADD COLUMN schema_hash TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| Error::from_reason(e.to_string()))?;
        }
        conn.execute(
            "INSERT INTO projection_state(key, value) VALUES('schema_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [PROJECTION_SCHEMA_VERSION.to_string()],
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;
        conn.execute(
            "INSERT INTO projection_state(key, value) VALUES('node_clock', '0')
             ON CONFLICT(key) DO NOTHING",
            [],
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;
        conn.execute(
            "INSERT INTO projection_state(key, value) VALUES('stable_frontier', '0')
             ON CONFLICT(key) DO NOTHING",
            [],
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    fn validate_identifier(value: &str) -> Result<()> {
        let mut chars = value.chars();
        let valid_first = chars
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic());
        if value.len() > 64
            || !valid_first
            || !chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            return Err(Error::from_reason(format!(
                "invalid relational identifier: {value}"
            )));
        }
        Ok(())
    }

    fn physical_table(namespace: &str, table: &str) -> Result<String> {
        Self::validate_identifier(namespace)?;
        Self::validate_identifier(table)?;
        Ok(format!("app_{namespace}__{table}"))
    }

    fn validate_namespace(value: &str) -> Result<()> {
        let mut chars = value.chars();
        let valid_first = chars.next().is_some_and(|ch| ch.is_ascii_lowercase());
        let reserved = ["genesis", "sqlite", "internal"];
        if value.len() > 63
            || !valid_first
            || !chars.all(|ch| ch == '_' || ch.is_ascii_lowercase() || ch.is_ascii_digit())
            || value.starts_with("_gb_")
            || reserved.iter().any(|prefix| value.starts_with(prefix))
        {
            return Err(Error::from_reason(
                "REL_NAMESPACE_INVALID: invalid or reserved relational namespace",
            ));
        }
        Ok(())
    }

    fn relational_value_matches(column_type: &RelationalColumnType, value: &Value) -> bool {
        match column_type {
            RelationalColumnType::Text => value.is_string(),
            RelationalColumnType::Integer => value.as_i64().is_some(),
            RelationalColumnType::Real => value.as_f64().is_some_and(f64::is_finite),
            RelationalColumnType::Boolean => value.is_boolean(),
            RelationalColumnType::Json => true,
            RelationalColumnType::Blob => value.as_array().is_some_and(|bytes| {
                bytes
                    .iter()
                    .all(|byte| byte.as_u64().is_some_and(|value| value <= u8::MAX as u64))
            }),
            RelationalColumnType::Timestamp => value
                .as_str()
                .is_some_and(|value| chrono::DateTime::parse_from_rfc3339(value).is_ok()),
            RelationalColumnType::EntityId => value
                .as_str()
                .is_some_and(|value| !value.is_empty() && value.len() <= 256),
        }
    }

    fn schema_hash(package: &RelationalSchemaPackage) -> Result<String> {
        let mut canonical = package.clone();
        canonical.schema_hash.clear();
        let bytes =
            serde_json::to_vec(&canonical).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(hex::encode(Sha256::digest(bytes)))
    }

    fn normalize_schema_package(
        mut package: RelationalSchemaPackage,
    ) -> Result<RelationalSchemaPackage> {
        if Uuid::parse_str(&package.package_id).is_err() {
            return Err(Error::from_reason(
                "REL_SCHEMA_VALIDATION_FAILED: package_id must be a UUID",
            ));
        }
        let calculated = Self::schema_hash(&package)?;
        if !package.schema_hash.is_empty() && package.schema_hash != calculated {
            return Err(Error::from_reason(
                "REL_SCHEMA_VALIDATION_FAILED: schema_hash mismatch",
            ));
        }
        package.schema_hash = calculated;
        Ok(package)
    }

    fn validate_relational_schema(package: &RelationalSchemaPackage) -> Result<()> {
        Self::validate_namespace(&package.namespace)?;
        if package.schema_version == 0 || package.tables.is_empty() {
            return Err(Error::from_reason(
                "relational schema requires a positive version and at least one table",
            ));
        }
        if package.tables.len() > 64 || package.named_queries.len() > 128 {
            return Err(Error::from_reason(
                "REL_SCHEMA_VALIDATION_FAILED: schema resource limit exceeded",
            ));
        }
        let mut table_names = HashSet::new();
        for table in &package.tables {
            Self::validate_identifier(&table.name)?;
            if !table_names.insert(table.name.as_str()) || table.columns.is_empty() {
                return Err(Error::from_reason(format!(
                    "duplicate or empty relational table: {}",
                    table.name
                )));
            }
            if table.columns.len() > 128
                || table.primary_key.is_empty()
                || table.primary_key.len() > 4
                || table.indexes.len() > 64
            {
                return Err(Error::from_reason(
                    "REL_SCHEMA_VALIDATION_FAILED: table resource limit exceeded",
                ));
            }
            let mut column_names = HashSet::new();
            for column in &table.columns {
                Self::validate_identifier(&column.name)?;
                if !column_names.insert(column.name.as_str()) {
                    return Err(Error::from_reason(format!(
                        "duplicate relational column: {}.{}",
                        table.name, column.name
                    )));
                }
                if let Some(default) = &column.default {
                    if !Self::relational_value_matches(&column.column_type, default) {
                        return Err(Error::from_reason(
                            "REL_SCHEMA_VALIDATION_FAILED: column default type mismatch",
                        ));
                    }
                }
            }
            if table.primary_key.is_empty()
                || table
                    .primary_key
                    .iter()
                    .any(|column| !column_names.contains(column.as_str()))
            {
                return Err(Error::from_reason(format!(
                    "invalid primary key for relational table: {}",
                    table.name
                )));
            }
            for foreign_key in &table.foreign_keys {
                if foreign_key.columns.is_empty()
                    || foreign_key.columns.len() != foreign_key.referenced_columns.len()
                    || foreign_key
                        .columns
                        .iter()
                        .any(|column| !column_names.contains(column.as_str()))
                {
                    return Err(Error::from_reason(format!(
                        "invalid foreign key for relational table: {}",
                        table.name
                    )));
                }
                Self::validate_identifier(&foreign_key.referenced_table)?;
                for column in &foreign_key.referenced_columns {
                    Self::validate_identifier(column)?;
                }
            }
            for index in &table.indexes {
                Self::validate_identifier(&index.name)?;
                if index.columns.is_empty()
                    || index
                        .columns
                        .iter()
                        .any(|column| !column_names.contains(column.as_str()))
                {
                    return Err(Error::from_reason(format!(
                        "invalid index for relational table: {}",
                        table.name
                    )));
                }
            }
        }
        let mut query_names = HashSet::new();
        for named in &package.named_queries {
            Self::validate_identifier(&named.name)?;
            if !query_names.insert(named.name.as_str())
                || named.default_limit == 0
                || named.default_limit > named.max_limit
                || named.max_limit > 1000
                || named.parameters.len() > 128
                || named.query.namespace != package.namespace
            {
                return Err(Error::from_reason(
                    "REL_SCHEMA_VALIDATION_FAILED: invalid named query",
                ));
            }
            let mut parameter_names = HashSet::new();
            for parameter in &named.parameters {
                Self::validate_identifier(&parameter.name)?;
                if !parameter_names.insert(parameter.name.as_str()) {
                    return Err(Error::from_reason(
                        "REL_SCHEMA_VALIDATION_FAILED: duplicate query parameter",
                    ));
                }
            }
            for filter in &named.query.filters {
                if let Some(parameter) = filter
                    .value
                    .as_object()
                    .and_then(|marker| marker.get("$param"))
                    .and_then(Value::as_str)
                {
                    if !parameter_names.contains(parameter) {
                        return Err(Error::from_reason(
                            "REL_SCHEMA_VALIDATION_FAILED: unknown named-query parameter",
                        ));
                    }
                }
            }
            if named.query.columns.is_empty()
                || named.query.joins.len() > 8
                || named.query.filters.len() > 128
                || !package
                    .tables
                    .iter()
                    .any(|table| table.name == named.query.table)
            {
                return Err(Error::from_reason(
                    "REL_SCHEMA_VALIDATION_FAILED: invalid named-query shape",
                ));
            }
            let mut available = HashSet::from([named.query.table.clone()]);
            for join in &named.query.joins {
                if !package.tables.iter().any(|table| table.name == join.table)
                    || !available.insert(join.table.clone())
                {
                    return Err(Error::from_reason(
                        "REL_SCHEMA_VALIDATION_FAILED: invalid named-query join",
                    ));
                }
            }
            for column in named
                .query
                .columns
                .iter()
                .chain(named.query.filters.iter().map(|filter| &filter.column))
                .chain(
                    named
                        .query
                        .joins
                        .iter()
                        .flat_map(|join| [&join.left_column, &join.right_column]),
                )
            {
                Self::resolve_query_column(package, &available, column)?;
            }
            for filter in &named.query.filters {
                let (_, column_name) =
                    Self::resolve_query_column(package, &available, &filter.column)?;
                let table_name = filter
                    .column
                    .split_once('.')
                    .expect("validated qualified column")
                    .0;
                let column = package
                    .tables
                    .iter()
                    .find(|table| table.name == table_name)
                    .and_then(|table| {
                        table
                            .columns
                            .iter()
                            .find(|column| column.name == column_name)
                    })
                    .expect("validated named-query column");
                if let Some(parameter) = filter
                    .value
                    .as_object()
                    .and_then(|marker| marker.get("$param"))
                    .and_then(Value::as_str)
                {
                    let definition = named
                        .parameters
                        .iter()
                        .find(|definition| definition.name == parameter)
                        .expect("validated named-query parameter");
                    if definition.column_type != column.column_type {
                        return Err(Error::from_reason(
                            "REL_SCHEMA_VALIDATION_FAILED: query parameter type does not match filter column",
                        ));
                    }
                } else {
                    Self::json_to_sql_typed(column, &filter.value)?;
                }
            }
        }
        for table in &package.tables {
            for foreign_key in &table.foreign_keys {
                let referenced = package
                    .tables
                    .iter()
                    .find(|candidate| candidate.name == foreign_key.referenced_table)
                    .ok_or_else(|| Error::from_reason("foreign key references unknown table"))?;
                if foreign_key.referenced_columns.iter().any(|column| {
                    !referenced
                        .columns
                        .iter()
                        .any(|candidate| candidate.name == *column)
                }) {
                    return Err(Error::from_reason("foreign key references unknown column"));
                }
            }
        }
        Ok(())
    }

    fn load_relational_schema_conn(
        conn: &Connection,
        namespace: &str,
    ) -> Result<Option<RelationalSchemaPackage>> {
        let package_json: Option<String> = conn
            .query_row(
                "SELECT package_json FROM relational_schema_registry WHERE namespace = ?1",
                [namespace],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        package_json
            .map(|json| serde_json::from_str(&json).map_err(|e| Error::from_reason(e.to_string())))
            .transpose()
    }

    fn validate_schema_upgrade(
        previous: &RelationalSchemaPackage,
        next: &RelationalSchemaPackage,
    ) -> Result<()> {
        if next.schema_version != previous.schema_version + 1
            || next.previous_version != Some(previous.schema_version)
        {
            return Err(Error::from_reason(
                "REL_SCHEMA_VERSION_CONFLICT: schema version must advance exactly once",
            ));
        }
        for old_table in &previous.tables {
            let new_table = next
                .tables
                .iter()
                .find(|table| table.name == old_table.name)
                .ok_or_else(|| Error::from_reason("relational schema cannot remove a table"))?;
            if new_table.primary_key != old_table.primary_key {
                return Err(Error::from_reason(
                    "relational schema cannot change a primary key",
                ));
            }
            for old_column in &old_table.columns {
                let new_column = new_table
                    .columns
                    .iter()
                    .find(|column| column.name == old_column.name)
                    .ok_or_else(|| {
                        Error::from_reason("relational schema cannot remove a column")
                    })?;
                if new_column.column_type != old_column.column_type
                    || new_column.nullable != old_column.nullable
                    || new_column.default != old_column.default
                {
                    return Err(Error::from_reason(
                        "relational schema cannot change a column incompatibly",
                    ));
                }
            }
            if new_table.foreign_keys != old_table.foreign_keys {
                return Err(Error::from_reason(
                    "relational schema cannot change foreign keys on an existing table",
                ));
            }
            if old_table.indexes.iter().any(|old_index| {
                !new_table
                    .indexes
                    .iter()
                    .any(|new_index| new_index == old_index)
            }) {
                return Err(Error::from_reason(
                    "relational schema cannot remove or redefine an existing index",
                ));
            }
            if new_table.columns.iter().any(|column| {
                !old_table.columns.iter().any(|old| old.name == column.name)
                    && !column.nullable
                    && column.default.is_none()
            }) {
                return Err(Error::from_reason(
                    "new relational columns must be nullable or have a literal default",
                ));
            }
        }
        Ok(())
    }

    fn sqlite_type(column_type: &RelationalColumnType) -> &'static str {
        match column_type {
            RelationalColumnType::Text
            | RelationalColumnType::Json
            | RelationalColumnType::Timestamp
            | RelationalColumnType::EntityId => "TEXT",
            RelationalColumnType::Integer | RelationalColumnType::Boolean => "INTEGER",
            RelationalColumnType::Real => "REAL",
            RelationalColumnType::Blob => "BLOB",
        }
    }

    fn sqlite_literal(column: &RelationalColumn, value: &Value) -> Result<String> {
        match Self::json_to_sql_typed(column, value)? {
            SqlValue::Null => Ok("NULL".to_string()),
            SqlValue::Integer(value) => Ok(value.to_string()),
            SqlValue::Real(value) => Ok(value.to_string()),
            SqlValue::Text(value) => Ok(format!("'{}'", value.replace('\'', "''"))),
            SqlValue::Blob(value) => Ok(format!("X'{}'", hex::encode(value))),
        }
    }

    fn projection_apply_schema_conn(
        conn: &Connection,
        package: &RelationalSchemaPackage,
    ) -> Result<()> {
        Self::validate_relational_schema(package)?;
        let previous = Self::load_relational_schema_conn(conn, &package.namespace)?;
        if let Some(previous) = &previous {
            if previous == package {
                return Ok(());
            }
            Self::validate_schema_upgrade(previous, package)?;
        } else if package.schema_version != 1 || package.previous_version.is_some() {
            return Err(Error::from_reason(
                "REL_SCHEMA_VERSION_CONFLICT: initial schema must be version 1",
            ));
        }
        for table in &package.tables {
            let physical = Self::physical_table(&package.namespace, &table.name)?;
            let definitions = table
                .columns
                .iter()
                .map(|column| {
                    let default = column
                        .default
                        .as_ref()
                        .map(|value| Self::sqlite_literal(column, value))
                        .transpose()?
                        .map(|value| format!(" DEFAULT {value}"))
                        .unwrap_or_default();
                    Ok(format!(
                        "\"{}\" {}{}{}",
                        column.name,
                        Self::sqlite_type(&column.column_type),
                        if column.nullable { "" } else { " NOT NULL" },
                        default
                    ))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .chain(std::iter::once(format!(
                    "PRIMARY KEY ({})",
                    table
                        .primary_key
                        .iter()
                        .map(|column| format!("\"{column}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                )))
                .chain(table.foreign_keys.iter().map(|foreign_key| {
                    let referenced =
                        Self::physical_table(&package.namespace, &foreign_key.referenced_table)
                            .expect("validated relational schema");
                    format!(
                        "FOREIGN KEY ({}) REFERENCES \"{}\" ({})",
                        foreign_key
                            .columns
                            .iter()
                            .map(|column| format!("\"{column}\""))
                            .collect::<Vec<_>>()
                            .join(", "),
                        referenced,
                        foreign_key
                            .referenced_columns
                            .iter()
                            .map(|column| format!("\"{column}\""))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }))
                .collect::<Vec<_>>();
            conn.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS \"{physical}\" ({})",
                definitions.join(", ")
            ))
            .map_err(|e| Error::from_reason(e.to_string()))?;

            if let Some(old_table) = previous
                .as_ref()
                .and_then(|old| old.tables.iter().find(|old| old.name == table.name))
            {
                for column in table
                    .columns
                    .iter()
                    .filter(|column| !old_table.columns.iter().any(|old| old.name == column.name))
                {
                    conn.execute_batch(&format!(
                        "ALTER TABLE \"{physical}\" ADD COLUMN \"{}\" {}{}{}",
                        column.name,
                        Self::sqlite_type(&column.column_type),
                        if column.nullable { "" } else { " NOT NULL" },
                        column
                            .default
                            .as_ref()
                            .map(|value| Self::sqlite_literal(column, value))
                            .transpose()?
                            .map(|value| format!(" DEFAULT {value}"))
                            .unwrap_or_default()
                    ))
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                }
            }
            for index in &table.indexes {
                let index_name =
                    format!("idx_{}__{}__{}", package.namespace, table.name, index.name);
                conn.execute_batch(&format!(
                    "CREATE {} INDEX IF NOT EXISTS \"{}\" ON \"{}\" ({})",
                    if index.unique { "UNIQUE" } else { "" },
                    index_name,
                    physical,
                    index
                        .columns
                        .iter()
                        .map(|column| format!("\"{column}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .map_err(|e| Error::from_reason(e.to_string()))?;
            }
        }
        let package_json =
            serde_json::to_string(package).map_err(|e| Error::from_reason(e.to_string()))?;
        conn.execute(
            "INSERT INTO relational_schema_registry(namespace, schema_version, package_id, schema_hash, package_json)
             VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(namespace) DO UPDATE SET
                schema_version = excluded.schema_version,
                package_id = excluded.package_id,
                schema_hash = excluded.schema_hash,
                package_json = excluded.package_json",
            params![
                package.namespace,
                package.schema_version,
                package.package_id,
                package.schema_hash,
                package_json
            ],
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    fn json_to_sql(value: &Value) -> Result<SqlValue> {
        match value {
            Value::Null => Ok(SqlValue::Null),
            Value::Bool(value) => Ok(SqlValue::Integer(i64::from(*value))),
            Value::Number(value) => value
                .as_i64()
                .map(SqlValue::Integer)
                .or_else(|| value.as_f64().map(SqlValue::Real))
                .ok_or_else(|| Error::from_reason("unsupported relational number")),
            Value::String(value) => Ok(SqlValue::Text(value.clone())),
            Value::Array(_) | Value::Object(_) => serde_json::to_string(value)
                .map(SqlValue::Text)
                .map_err(|e| Error::from_reason(e.to_string())),
        }
    }

    fn json_to_sql_typed(column: &RelationalColumn, value: &Value) -> Result<SqlValue> {
        if value.is_null() {
            return if column.nullable {
                Ok(SqlValue::Null)
            } else {
                Err(Error::from_reason(
                    "REL_TYPE_MISMATCH: null supplied for required column",
                ))
            };
        }
        if !Self::relational_value_matches(&column.column_type, value) {
            return Err(Error::from_reason(
                "REL_TYPE_MISMATCH: relational value does not match column type",
            ));
        }
        match column.column_type {
            RelationalColumnType::Blob => Ok(SqlValue::Blob(
                value
                    .as_array()
                    .expect("validated blob")
                    .iter()
                    .map(|byte| byte.as_u64().expect("validated byte") as u8)
                    .collect(),
            )),
            RelationalColumnType::Json => serde_json::to_string(value)
                .map(SqlValue::Text)
                .map_err(|e| Error::from_reason(e.to_string())),
            _ => Self::json_to_sql(value),
        }
    }

    fn validate_row_mutation(
        schema: &RelationalSchemaPackage,
        mutation: &RelationalRowMutation,
    ) -> Result<()> {
        let table = schema
            .tables
            .iter()
            .find(|table| table.name == mutation.table)
            .ok_or_else(|| Error::from_reason("relational mutation references unknown table"))?;
        let object = match mutation.kind {
            RelationalMutationKind::Insert
            | RelationalMutationKind::Upsert
            | RelationalMutationKind::Update => mutation.values.as_object(),
            RelationalMutationKind::Delete => mutation.key.as_ref().and_then(Value::as_object),
        }
        .ok_or_else(|| Error::from_reason("relational mutation requires an object payload"))?;
        if object
            .keys()
            .any(|key| !table.columns.iter().any(|column| column.name == *key))
        {
            return Err(Error::from_reason(
                "relational mutation contains unknown column",
            ));
        }
        match mutation.kind {
            RelationalMutationKind::Insert | RelationalMutationKind::Upsert => {
                if table.columns.iter().any(|column| {
                    !column.nullable
                        && column.default.is_none()
                        && !object.contains_key(&column.name)
                }) {
                    return Err(Error::from_reason(
                        "relational upsert omits a required column",
                    ));
                }
            }
            RelationalMutationKind::Update => {
                let key = mutation
                    .key
                    .as_ref()
                    .and_then(Value::as_object)
                    .ok_or_else(|| {
                        Error::from_reason("relational update requires a primary-key object")
                    })?;
                if object.is_empty()
                    || table
                        .primary_key
                        .iter()
                        .any(|column| !key.contains_key(column))
                    || object
                        .keys()
                        .any(|column| table.primary_key.contains(column))
                {
                    return Err(Error::from_reason(
                        "relational update requires a complete key and cannot change it",
                    ));
                }
            }
            RelationalMutationKind::Delete => {
                if table
                    .primary_key
                    .iter()
                    .any(|column| !object.contains_key(column))
                {
                    return Err(Error::from_reason(
                        "relational delete requires every primary-key column",
                    ));
                }
            }
        }
        for (name, value) in object {
            let column = table
                .columns
                .iter()
                .find(|column| column.name == *name)
                .expect("validated column");
            Self::json_to_sql_typed(column, value)?;
        }
        if let Some(key) = mutation.key.as_ref().and_then(Value::as_object) {
            for name in &table.primary_key {
                if let Some(value) = key.get(name) {
                    let column = table
                        .columns
                        .iter()
                        .find(|column| column.name == *name)
                        .expect("validated primary key");
                    Self::json_to_sql_typed(column, value)?;
                }
            }
        }
        Ok(())
    }

    fn projection_apply_rows_tx(
        tx: &rusqlite::Transaction<'_>,
        schema: &RelationalSchemaPackage,
        mutations: &[RelationalRowMutation],
    ) -> Result<u32> {
        let mut affected = 0u32;
        for mutation in mutations {
            Self::validate_row_mutation(schema, mutation)?;
            let table = schema
                .tables
                .iter()
                .find(|table| table.name == mutation.table)
                .expect("validated relational mutation");
            let physical = Self::physical_table(&schema.namespace, &table.name)?;
            match mutation.kind {
                RelationalMutationKind::Insert | RelationalMutationKind::Upsert => {
                    let object = mutation.values.as_object().expect("validated object");
                    let columns: Vec<&String> = table
                        .columns
                        .iter()
                        .filter_map(|column| {
                            object.contains_key(&column.name).then_some(&column.name)
                        })
                        .collect();
                    let values = columns
                        .iter()
                        .map(|column| {
                            let definition = table
                                .columns
                                .iter()
                                .find(|definition| definition.name == column.as_str())
                                .expect("validated column");
                            Self::json_to_sql_typed(definition, &object[*column])
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let updates: Vec<String> = columns
                        .iter()
                        .filter(|column| !table.primary_key.contains(column))
                        .map(|column| format!("\"{column}\" = excluded.\"{column}\""))
                        .collect();
                    let conflict = if matches!(mutation.kind, RelationalMutationKind::Insert) {
                        String::new()
                    } else if updates.is_empty() {
                        " ON CONFLICT DO NOTHING".to_string()
                    } else {
                        format!(
                            " ON CONFLICT ({}) DO UPDATE SET {}",
                            table
                                .primary_key
                                .iter()
                                .map(|column| format!("\"{column}\""))
                                .collect::<Vec<_>>()
                                .join(", "),
                            updates.join(", ")
                        )
                    };
                    let sql = format!(
                        "INSERT INTO \"{physical}\" ({}) VALUES ({}){conflict}",
                        columns
                            .iter()
                            .map(|column| format!("\"{column}\""))
                            .collect::<Vec<_>>()
                            .join(", "),
                        vec!["?"; columns.len()].join(", ")
                    );
                    affected = affected.saturating_add(
                        tx.execute(&sql, params_from_iter(values))
                            .map_err(|e| Error::from_reason(e.to_string()))?
                            as u32,
                    );
                }
                RelationalMutationKind::Update => {
                    let object = mutation.values.as_object().expect("validated object");
                    let key = mutation
                        .key
                        .as_ref()
                        .and_then(Value::as_object)
                        .expect("validated key");
                    let set_columns = object.keys().collect::<Vec<_>>();
                    let mut values = set_columns
                        .iter()
                        .map(|name| {
                            let column = table
                                .columns
                                .iter()
                                .find(|column| column.name == name.as_str())
                                .expect("validated column");
                            Self::json_to_sql_typed(column, &object[name.as_str()])
                        })
                        .collect::<Result<Vec<_>>>()?;
                    values.extend(
                        table
                            .primary_key
                            .iter()
                            .map(|name| {
                                let column = table
                                    .columns
                                    .iter()
                                    .find(|column| column.name == *name)
                                    .expect("validated key column");
                                Self::json_to_sql_typed(column, &key[name])
                            })
                            .collect::<Result<Vec<_>>>()?,
                    );
                    let sql = format!(
                        "UPDATE \"{physical}\" SET {} WHERE {}",
                        set_columns
                            .iter()
                            .map(|column| format!("\"{column}\" = ?"))
                            .collect::<Vec<_>>()
                            .join(", "),
                        table
                            .primary_key
                            .iter()
                            .map(|column| format!("\"{column}\" = ?"))
                            .collect::<Vec<_>>()
                            .join(" AND ")
                    );
                    affected = affected.saturating_add(
                        tx.execute(&sql, params_from_iter(values))
                            .map_err(|e| Error::from_reason(e.to_string()))?
                            as u32,
                    );
                }
                RelationalMutationKind::Delete => {
                    let object = mutation
                        .key
                        .as_ref()
                        .and_then(Value::as_object)
                        .expect("validated object");
                    let values = table
                        .primary_key
                        .iter()
                        .map(|name| {
                            let column = table
                                .columns
                                .iter()
                                .find(|column| column.name == *name)
                                .expect("validated key column");
                            Self::json_to_sql_typed(column, &object[name])
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let sql = format!(
                        "DELETE FROM \"{physical}\" WHERE {}",
                        table
                            .primary_key
                            .iter()
                            .map(|column| format!("\"{column}\" = ?"))
                            .collect::<Vec<_>>()
                            .join(" AND ")
                    );
                    affected = affected.saturating_add(
                        tx.execute(&sql, params_from_iter(values))
                            .map_err(|e| Error::from_reason(e.to_string()))?
                            as u32,
                    );
                }
            }
        }
        Ok(affected)
    }

    fn projection_state_set<C>(conn: &C, key: &str, value: &str) -> Result<()>
    where
        C: std::ops::Deref<Target = Connection>,
    {
        conn.execute(
            "INSERT INTO projection_state(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    fn projection_apply_node_conn(
        conn: &mut Connection,
        node_u32: u32,
        node: &NodeOutput,
    ) -> Result<()> {
        let tx = conn
            .transaction()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Self::projection_apply_node_tx(&tx, node_u32, node)?;
        Self::projection_state_set(&tx, "node_clock", &node.clock.time.to_string())?;
        tx.commit().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    fn projection_apply_node_tx(
        tx: &rusqlite::Transaction<'_>,
        node_u32: u32,
        node: &NodeOutput,
    ) -> Result<()> {
        let current_clock: Option<(u32, String)> = tx
            .query_row(
                "SELECT clock_time, clock_peer FROM props WHERE node_u32 = ?1",
                [node_u32],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if current_clock.is_some_and(|seen| seen > (node.clock.time, node.clock.peer_id.clone())) {
            return Ok(());
        }

        let payload =
            serde_json::to_string(&node.props).map_err(|e| Error::from_reason(e.to_string()))?;
        tx.execute(
            "INSERT INTO props(node_u32, payload, clock_time, clock_peer, valid_from, valid_to)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(node_u32) DO UPDATE SET
                 payload = excluded.payload,
                 clock_time = excluded.clock_time,
                 clock_peer = excluded.clock_peer,
                 valid_from = excluded.valid_from,
                 valid_to = excluded.valid_to
             WHERE excluded.clock_time > props.clock_time
                OR (excluded.clock_time = props.clock_time
                    AND excluded.clock_peer >= props.clock_peer)",
            params![
                node_u32,
                payload,
                node.clock.time,
                node.clock.peer_id,
                node.valid_from,
                node.valid_to
            ],
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;
        tx.execute("DELETE FROM node_labels WHERE node_u32 = ?1", [node_u32])
            .map_err(|e| Error::from_reason(e.to_string()))?;
        for label in &node.labels {
            tx.execute(
                "INSERT OR REPLACE INTO node_labels(node_u32, label) VALUES(?1, ?2)",
                params![node_u32, label],
            )
            .map_err(|e| Error::from_reason(e.to_string()))?;
        }
        Ok(())
    }

    /// Apply one journal event to the SQLite projection. `frame_seq` is the
    /// event's LOCAL frame stamp (WP-1.2) — the authoritative sequence for
    /// transaction identity and the frontier keys.
    fn projection_apply_event(&self, event: &Event, frame_seq: u64) -> Result<()> {
        if self.read_only {
            return Ok(());
        }
        let mut conn = self.projection_db.lock();
        match event {
            Event::Node(node) => {
                let node_u32 = self.get_or_intern_id(&node.id);
                Self::projection_apply_node_conn(&mut conn, node_u32, node)?;
            }
            Event::RelationalSchema(package) => {
                let tx = conn
                    .transaction()
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                Self::projection_apply_schema_conn(&tx, package)?;
                tx.commit().map_err(|e| Error::from_reason(e.to_string()))?;
            }
            Event::RelationalRows {
                namespace,
                schema_version,
                mutation_id,
                payload_hash,
                affected_rows,
                mutations,
            } => {
                let schema = Self::load_relational_schema_conn(&conn, namespace)?
                    .ok_or_else(|| Error::from_reason("relational schema is not registered"))?;
                if *schema_version != 0 && *schema_version != schema.schema_version {
                    return Err(Error::from_reason(
                        "REL_SCHEMA_VERSION_CONFLICT: mutation schema version is not current",
                    ));
                }
                if !mutation_id.is_empty() {
                    let existing: Option<String> = conn
                        .query_row(
                            "SELECT payload_hash FROM applied_relational_mutations WHERE mutation_id = ?1",
                            [mutation_id],
                            |row| row.get(0),
                        )
                        .optional()
                        .map_err(|e| Error::from_reason(e.to_string()))?;
                    if let Some(existing) = existing {
                        return if existing == *payload_hash {
                            Ok(())
                        } else {
                            Err(Error::from_reason(
                                "REL_MUTATION_CONFLICT: mutation identity reused with different payload",
                            ))
                        };
                    }
                }
                let tx = conn
                    .transaction()
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                let applied = Self::projection_apply_rows_tx(&tx, &schema, mutations)?;
                let recorded_affected = if mutations.is_empty() {
                    *affected_rows
                } else {
                    applied
                };
                if !mutation_id.is_empty() {
                    tx.execute(
                        "INSERT INTO applied_relational_mutations(mutation_id, namespace, schema_version, payload_hash, affected_rows) VALUES(?1, ?2, ?3, ?4, ?5)",
                        params![mutation_id, namespace, schema.schema_version, payload_hash, recorded_affected],
                    )
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                }
                tx.commit().map_err(|e| Error::from_reason(e.to_string()))?;
            }
            Event::Transaction(transaction) => {
                // Identity = (transaction_id, payload_hash). The origin sequence
                // is metadata (ADR D2.2) and is deliberately NOT part of the
                // identity check — two replicas legitimately assign different
                // sequences to the same transaction.
                let existing: Option<String> = conn
                    .query_row(
                        "SELECT payload_hash FROM applied_transactions WHERE transaction_id = ?1",
                        [&transaction.transaction_id],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                if let Some(hash) = existing {
                    if hash == transaction.payload_hash {
                        return Ok(());
                    }
                    return Err(Error::from_reason("transaction identity conflict"));
                }
                let tx = conn
                    .transaction()
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                for node in &transaction.nodes {
                    let node_u32 = self.get_or_intern_id(&node.id);
                    Self::projection_apply_node_tx(&tx, node_u32, node)?;
                }
                for group in &transaction.relational {
                    let schema = Self::load_relational_schema_conn(&tx, &group.namespace)?
                        .ok_or_else(|| Error::from_reason("relational schema is not registered"))?;
                    let _ = Self::projection_apply_rows_tx(&tx, &schema, &group.mutations)?;
                }
                tx.execute(
                    "INSERT INTO applied_transactions(transaction_id, commit_sequence, payload_hash, frame_seq) VALUES(?1, ?2, ?3, ?4)",
                    params![transaction.transaction_id, transaction.origin_commit_seq, transaction.payload_hash, frame_seq],
                )
                .map_err(|e| Error::from_reason(e.to_string()))?;
                // Informational only since WP-1.2 — the counters re-seed from
                // the journal itself on open, never from the projection.
                Self::projection_state_set(&tx, "stable_frontier", &frame_seq.to_string())?;
                Self::projection_state_set(&tx, "txn_frontier", &frame_seq.to_string())?;
                tx.commit().map_err(|e| Error::from_reason(e.to_string()))?;
            }
            Event::Batch(events) => {
                let tx = conn
                    .transaction()
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                let mut max_clock = 0;
                for inner in events {
                    if let Event::Node(node) = inner {
                        let node_u32 = self.get_or_intern_id(&node.id);
                        if node.clock.time > max_clock {
                            max_clock = node.clock.time;
                        }
                        Self::projection_apply_node_tx(&tx, node_u32, node)?;
                    }
                }
                if max_clock > 0 {
                    Self::projection_state_set(&tx, "node_clock", &max_clock.to_string())?;
                }
                tx.commit().map_err(|e| Error::from_reason(e.to_string()))?;
            }
            // Drop the retracted node's projection rows. On the live path this
            // runs from `persist` while `id` still resolves (memory removal
            // happens after); on projection rebuild the preceding Node frame's
            // re-intern makes the id resolve again, so the delete still lands.
            Event::NodeRetract { id, .. } => {
                if let Some(node_u32) = self.get_u32(id) {
                    conn.execute("DELETE FROM props WHERE node_u32 = ?1", params![node_u32])
                        .map_err(|e| Error::from_reason(e.to_string()))?;
                    conn.execute(
                        "DELETE FROM node_labels WHERE node_u32 = ?1",
                        params![node_u32],
                    )
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn projection_snapshot(&self, target: &std::path::Path) -> Result<()> {
        if self.read_only {
            return Ok(());
        }
        if target.exists() {
            let _ = fs::remove_file(target);
        }
        let escaped = target.to_string_lossy().replace('\'', "''");
        let conn = self.projection_db.lock();
        conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
            .map_err(|e| Error::from_reason(e.to_string()))?;
        conn.execute_batch(&format!("VACUUM INTO '{}';", escaped))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    fn projection_reopen_fresh(&self) -> Result<()> {
        if self.read_only {
            return Ok(());
        }
        if self.projection_path.exists() {
            let _ = fs::remove_file(&self.projection_path);
        }
        let conn = Self::open_projection(&self.projection_path, false)?;
        Self::init_projection_schema(&conn)?;
        *self.projection_db.lock() = conn;
        Ok(())
    }

    fn projection_replay_wal(&self) -> Result<()> {
        let journal_present = self.log_path.exists()
            || self.legacy_log_path.exists()
            || !Self::journal_list_segments(&self.path).is_empty();
        if !journal_present {
            let projection_complete = self
                .nodes
                .iter()
                .all(|entry| self.projection_props(*entry.key()).ok().flatten().is_some());
            if projection_complete {
                return Ok(());
            }
            return Err(Error::from_reason(
                "projection recovery requires the journal when node props are missing",
            ));
        }
        let mut first_err: Option<Error> = None;
        self.scan_journal(None, true, &mut |seq, signed_event| {
            if first_err.is_some() {
                return;
            }
            let mut apply = |event: Event| {
                if let Err(e) = self.projection_apply_event(&event, seq) {
                    first_err = Some(e);
                }
            };
            match signed_event.event {
                Event::Node(node) => apply(Event::Node(node)),
                Event::RelationalSchema(package) => apply(Event::RelationalSchema(package)),
                rows @ Event::RelationalRows { .. } => apply(rows),
                Event::Transaction(transaction) => apply(Event::Transaction(transaction)),
                // A rebuild must re-delete the rows the retracted node's own
                // earlier Node frame just re-inserted.
                retract @ Event::NodeRetract { .. } => apply(retract),
                Event::Batch(events) => {
                    for event in events {
                        match event {
                            Event::Node(_)
                            | Event::RelationalSchema(_)
                            | Event::RelationalRows { .. }
                            | Event::Transaction(_)
                            | Event::NodeRetract { .. } => apply(event),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        });
        if let Some(e) = first_err {
            return Err(e);
        }
        let projection_complete = self
            .nodes
            .iter()
            .all(|entry| self.projection_props(*entry.key()).ok().flatten().is_some());
        if !projection_complete {
            return Err(Error::from_reason(
                "journal replay did not recover props for every resident node",
            ));
        }
        Ok(())
    }

    fn projection_sync_on_open(&self) -> Result<()> {
        if self.read_only {
            return Ok(());
        }
        // Lamport time is not a WAL cursor: distinct peers may emit equal clocks,
        // and compaction rewrites the WAL. Scan the authoritative WAL on open and
        // rely on full-clock LWW upserts for idempotence. A durable sequence cursor
        // belongs to the unified transaction protocol, not this S0/S1 projection.
        if self.projection_replay_wal().is_err() {
            self.projection_reopen_fresh()?;
            self.projection_replay_wal()?;
        }
        Ok(())
    }

    pub fn projection_props(&self, node_u32: u32) -> Result<Option<Value>> {
        let conn = self.projection_db.lock();
        let payload: Option<String> = conn
            .query_row(
                "SELECT payload FROM props WHERE node_u32 = ?1",
                [node_u32],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        match payload {
            Some(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|e| Error::from_reason(e.to_string())),
            None => Ok(None),
        }
    }

    pub fn register_relational_schema(&self, package: RelationalSchemaPackage) -> Result<u32> {
        self.ensure_writable()?;
        let package = Self::normalize_schema_package(package)?;
        Self::validate_relational_schema(&package)?;
        let _commit_guard = self.commit_lock.lock();
        let mut conn = self.projection_db.lock();
        if let Some(previous) = Self::load_relational_schema_conn(&conn, &package.namespace)? {
            if previous == package {
                return Ok(package.schema_version);
            }
            Self::validate_schema_upgrade(&previous, &package)?;
        } else if package.schema_version != 1 || package.previous_version.is_some() {
            return Err(Error::from_reason(
                "REL_SCHEMA_VERSION_CONFLICT: initial schema must be version 1",
            ));
        }
        let tx = conn
            .transaction()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Self::projection_apply_schema_conn(&tx, &package)?;
        self.append_wal_event(&Event::RelationalSchema(package.clone()))?;
        tx.commit().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(package.schema_version)
    }

    pub fn get_relational_schema(
        &self,
        namespace: &str,
    ) -> Result<Option<RelationalSchemaPackage>> {
        Self::validate_namespace(namespace)?;
        let conn = self.projection_db.lock();
        Self::load_relational_schema_conn(&conn, namespace)
    }

    pub fn list_relational_schemas(&self) -> Result<Vec<RelationalSchemaPackage>> {
        let conn = self.projection_db.lock();
        let mut statement = conn
            .prepare("SELECT package_json FROM relational_schema_registry ORDER BY namespace")
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let schemas = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| Error::from_reason(e.to_string()))?
            .map(|row| {
                let raw = row.map_err(|e| Error::from_reason(e.to_string()))?;
                serde_json::from_str(&raw).map_err(|e| Error::from_reason(e.to_string()))
            })
            .collect();
        schemas
    }

    pub fn apply_relational_batch(
        &self,
        batch: RelationalMutationBatch,
    ) -> Result<RelationalMutationResult> {
        self.ensure_writable()?;
        Self::validate_namespace(&batch.namespace)?;
        if Uuid::parse_str(&batch.mutation_id).is_err()
            || batch.operations.is_empty()
            || batch.operations.len() > 1000
        {
            return Err(Error::from_reason(
                "REL_SCHEMA_VALIDATION_FAILED: invalid relational mutation batch",
            ));
        }
        let encoded = serde_json::to_vec(&batch).map_err(|e| Error::from_reason(e.to_string()))?;
        if encoded.len() > 8 * 1024 * 1024 {
            return Err(Error::from_reason(
                "REL_QUERY_LIMIT_EXCEEDED: mutation batch exceeds 8 MiB",
            ));
        }
        let payload_hash = hex::encode(Sha256::digest(&encoded));
        let _commit_guard = self.commit_lock.lock();
        let mut conn = self.projection_db.lock();
        let schema = Self::load_relational_schema_conn(&conn, &batch.namespace)?
            .ok_or_else(|| Error::from_reason("REL_SCHEMA_NOT_FOUND"))?;
        if schema.schema_version != batch.schema_version {
            return Err(Error::from_reason(
                "REL_SCHEMA_VERSION_CONFLICT: mutation schema version is not current",
            ));
        }
        for mutation in &batch.operations {
            Self::validate_row_mutation(&schema, mutation)?;
        }
        let existing: Option<(String, u32)> = conn
            .query_row(
                "SELECT payload_hash, affected_rows FROM applied_relational_mutations WHERE mutation_id = ?1",
                [&batch.mutation_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if let Some((existing_hash, affected_rows)) = existing {
            if existing_hash != payload_hash {
                return Err(Error::from_reason(
                    "REL_MUTATION_CONFLICT: mutation identity reused with different payload",
                ));
            }
            return Ok(RelationalMutationResult {
                mutation_id: batch.mutation_id,
                namespace: batch.namespace,
                schema_version: batch.schema_version,
                wal_committed: true,
                projection_applied: true,
                affected_rows,
            });
        }
        let tx = conn
            .transaction()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let affected_rows = Self::projection_apply_rows_tx(&tx, &schema, &batch.operations)?;
        self.append_wal_event(&Event::RelationalRows {
            namespace: batch.namespace.clone(),
            schema_version: batch.schema_version,
            mutation_id: batch.mutation_id.clone(),
            payload_hash: payload_hash.clone(),
            affected_rows,
            mutations: batch.operations.clone(),
        })?;
        tx.execute(
            "INSERT INTO applied_relational_mutations(mutation_id, namespace, schema_version, payload_hash, affected_rows) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                batch.mutation_id,
                batch.namespace,
                batch.schema_version,
                payload_hash,
                affected_rows
            ],
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;
        tx.commit().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(RelationalMutationResult {
            mutation_id: batch.mutation_id,
            namespace: batch.namespace,
            schema_version: batch.schema_version,
            wal_committed: true,
            projection_applied: true,
            affected_rows,
        })
    }

    pub fn apply_relational_rows(
        &self,
        namespace: &str,
        mutations: Vec<RelationalRowMutation>,
    ) -> Result<()> {
        self.ensure_writable()?;
        Self::validate_namespace(namespace)?;
        if mutations.is_empty() {
            return Ok(());
        }
        {
            let conn = self.projection_db.lock();
            let schema = Self::load_relational_schema_conn(&conn, namespace)?
                .ok_or_else(|| Error::from_reason("relational schema is not registered"))?;
            for mutation in &mutations {
                Self::validate_row_mutation(&schema, mutation)?;
            }
        }
        let schema_version = {
            let conn = self.projection_db.lock();
            Self::load_relational_schema_conn(&conn, namespace)?
                .ok_or_else(|| Error::from_reason("REL_SCHEMA_NOT_FOUND"))?
                .schema_version
        };
        self.apply_relational_batch(RelationalMutationBatch {
            mutation_id: Uuid::new_v4().to_string(),
            namespace: namespace.to_string(),
            schema_version,
            operations: mutations,
        })
        .map(|_| ())
    }

    fn resolve_query_column(
        schema: &RelationalSchemaPackage,
        available_tables: &HashSet<String>,
        column: &str,
    ) -> Result<(String, String)> {
        let parts: Vec<&str> = column.split('.').collect();
        if parts.len() != 2 {
            return Err(Error::from_reason(
                "relational query columns must be table-qualified",
            ));
        }
        let table_name = parts[0];
        let column_name = parts[1];
        Self::validate_identifier(table_name)?;
        Self::validate_identifier(column_name)?;
        if !available_tables.contains(table_name) {
            return Err(Error::from_reason(
                "relational query references unavailable table",
            ));
        }
        let table = schema
            .tables
            .iter()
            .find(|table| table.name == table_name)
            .ok_or_else(|| Error::from_reason("relational query references unknown table"))?;
        if !table
            .columns
            .iter()
            .any(|column| column.name == column_name)
        {
            return Err(Error::from_reason(
                "relational query references unknown column",
            ));
        }
        Ok((
            Self::physical_table(&schema.namespace, table_name)?,
            column_name.to_string(),
        ))
    }

    fn query_column_definition<'a>(
        schema: &'a RelationalSchemaPackage,
        available_tables: &HashSet<String>,
        column: &str,
    ) -> Result<&'a RelationalColumn> {
        Self::resolve_query_column(schema, available_tables, column)?;
        let (table_name, column_name) = column
            .split_once('.')
            .expect("validated qualified relational column");
        schema
            .tables
            .iter()
            .find(|table| table.name == table_name)
            .and_then(|table| {
                table
                    .columns
                    .iter()
                    .find(|candidate| candidate.name == column_name)
            })
            .ok_or_else(|| Error::from_reason("relational query references unknown column"))
    }

    pub fn query_relational(&self, query: RelationalQuery) -> Result<Vec<Value>> {
        Self::validate_namespace(&query.namespace)?;
        Self::validate_identifier(&query.table)?;
        if query.columns.is_empty() {
            return Err(Error::from_reason(
                "relational query requires at least one selected column",
            ));
        }
        if query.joins.len() > 8 || query.filters.len() > 128 {
            return Err(Error::from_reason(
                "REL_QUERY_LIMIT_EXCEEDED: query shape exceeds U2 bounds",
            ));
        }
        let limit = query.limit.unwrap_or(100);
        if limit == 0 || limit > 1000 {
            return Err(Error::from_reason(
                "REL_QUERY_LIMIT_EXCEEDED: query row limit must be 1..1000",
            ));
        }
        let conn = self.projection_db.lock();
        let schema = Self::load_relational_schema_conn(&conn, &query.namespace)?
            .ok_or_else(|| Error::from_reason("relational schema is not registered"))?;
        if !schema.tables.iter().any(|table| table.name == query.table) {
            return Err(Error::from_reason(
                "relational query references unknown table",
            ));
        }
        let mut available_tables = HashSet::from([query.table.clone()]);
        for join in &query.joins {
            Self::validate_identifier(&join.table)?;
            if !schema.tables.iter().any(|table| table.name == join.table)
                || !available_tables.insert(join.table.clone())
            {
                return Err(Error::from_reason("invalid relational join table"));
            }
        }
        let selected = query
            .columns
            .iter()
            .map(|column| Self::resolve_query_column(&schema, &available_tables, column))
            .collect::<Result<Vec<_>>>()?;
        let selected_types = query
            .columns
            .iter()
            .map(|column| {
                Self::query_column_definition(&schema, &available_tables, column)
                    .map(|definition| definition.column_type.clone())
            })
            .collect::<Result<Vec<_>>>()?;
        let base = Self::physical_table(&query.namespace, &query.table)?;
        let mut sql = format!(
            "SELECT {} FROM \"{}\"",
            selected
                .iter()
                .map(|(table, column)| format!("\"{table}\".\"{column}\""))
                .collect::<Vec<_>>()
                .join(", "),
            base
        );
        for join in &query.joins {
            let physical = Self::physical_table(&query.namespace, &join.table)?;
            let (left_table, left_column) =
                Self::resolve_query_column(&schema, &available_tables, &join.left_column)?;
            let (right_table, right_column) =
                Self::resolve_query_column(&schema, &available_tables, &join.right_column)?;
            if physical != left_table && physical != right_table {
                return Err(Error::from_reason(
                    "relational join condition must reference the joined table",
                ));
            }
            sql.push_str(&format!(
                " {} JOIN \"{physical}\" ON \"{left_table}\".\"{left_column}\" = \"{right_table}\".\"{right_column}\"",
                match join.kind {
                    RelationalJoinKind::Inner => "INNER",
                    RelationalJoinKind::Left => "LEFT",
                }
            ));
        }
        let mut parameters = Vec::new();
        if !query.filters.is_empty() {
            let predicates = query
                .filters
                .iter()
                .map(|filter| {
                    let (table, column) =
                        Self::resolve_query_column(&schema, &available_tables, &filter.column)?;
                    let definition =
                        Self::query_column_definition(&schema, &available_tables, &filter.column)?;
                    parameters.push(Self::json_to_sql_typed(definition, &filter.value)?);
                    Ok(format!("\"{table}\".\"{column}\" = ?"))
                })
                .collect::<Result<Vec<_>>>()?;
            sql.push_str(&format!(" WHERE {}", predicates.join(" AND ")));
        }
        sql.push_str(&format!(" LIMIT {limit}"));
        let mut statement = conn
            .prepare(&sql)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let rows = statement
            .query_map(params_from_iter(parameters), |row| {
                let mut object = serde_json::Map::new();
                for (index, name) in query.columns.iter().enumerate() {
                    let value = match (&selected_types[index], row.get_ref(index)?) {
                        (RelationalColumnType::Boolean, ValueRef::Integer(value)) => {
                            Value::Bool(value != 0)
                        }
                        (RelationalColumnType::Json, ValueRef::Text(value)) => {
                            serde_json::from_slice(value).unwrap_or_else(|_| {
                                Value::String(String::from_utf8_lossy(value).into_owned())
                            })
                        }
                        (_, ValueRef::Null) => Value::Null,
                        (_, ValueRef::Integer(value)) => Value::from(value),
                        (_, ValueRef::Real(value)) => Value::from(value),
                        (_, ValueRef::Text(value)) => {
                            Value::String(String::from_utf8_lossy(value).into_owned())
                        }
                        (_, ValueRef::Blob(value)) => {
                            Value::Array(value.iter().copied().map(Value::from).collect())
                        }
                    };
                    object.insert(name.clone(), value);
                }
                Ok(Value::Object(object))
            })
            .map_err(|e| Error::from_reason(e.to_string()))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    pub fn execute_named_query(&self, request: NamedQueryRequest) -> Result<Vec<Value>> {
        Self::validate_namespace(&request.namespace)?;
        Self::validate_identifier(&request.query_name)?;
        let definition = {
            let conn = self.projection_db.lock();
            let schema = Self::load_relational_schema_conn(&conn, &request.namespace)?
                .ok_or_else(|| Error::from_reason("REL_SCHEMA_NOT_FOUND"))?;
            if schema.schema_version != request.schema_version {
                return Err(Error::from_reason(
                    "REL_SCHEMA_VERSION_CONFLICT: query schema version is not current",
                ));
            }
            schema
                .named_queries
                .iter()
                .find(|query| query.name == request.query_name)
                .cloned()
                .ok_or_else(|| Error::from_reason("REL_QUERY_NOT_FOUND"))?
        };
        let parameters = request.parameters.as_object().ok_or_else(|| {
            Error::from_reason("REL_TYPE_MISMATCH: named-query parameters must be an object")
        })?;
        if parameters.len() != definition.parameters.len() {
            return Err(Error::from_reason(
                "REL_TYPE_MISMATCH: missing or extra named-query parameter",
            ));
        }
        for parameter in &definition.parameters {
            let value = parameters.get(&parameter.name).ok_or_else(|| {
                Error::from_reason("REL_TYPE_MISMATCH: missing named-query parameter")
            })?;
            if !Self::relational_value_matches(&parameter.column_type, value) {
                return Err(Error::from_reason(
                    "REL_TYPE_MISMATCH: named-query parameter type mismatch",
                ));
            }
        }
        if parameters
            .keys()
            .any(|name| !definition.parameters.iter().any(|item| item.name == *name))
        {
            return Err(Error::from_reason(
                "REL_TYPE_MISMATCH: unknown named-query parameter",
            ));
        }
        let mut query = definition.query;
        for filter in &mut query.filters {
            if let Some(parameter) = filter
                .value
                .as_object()
                .and_then(|marker| marker.get("$param"))
                .and_then(Value::as_str)
            {
                filter.value = parameters
                    .get(parameter)
                    .expect("validated named-query parameter")
                    .clone();
            }
        }
        let limit = request.limit.unwrap_or(definition.default_limit);
        if limit == 0 || limit > definition.max_limit {
            return Err(Error::from_reason(
                "REL_QUERY_LIMIT_EXCEEDED: named-query limit exceeds definition",
            ));
        }
        query.limit = Some(limit);
        self.query_relational(query)
    }

    fn relational_snapshot_events(&self) -> Result<Vec<Event>> {
        let conn = self.projection_db.lock();
        let mut registry = conn
            .prepare("SELECT package_json FROM relational_schema_registry ORDER BY namespace")
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let packages = registry
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| Error::from_reason(e.to_string()))?
            .map(|row| {
                let json = row.map_err(|e| Error::from_reason(e.to_string()))?;
                serde_json::from_str::<RelationalSchemaPackage>(&json)
                    .map_err(|e| Error::from_reason(e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        drop(registry);

        let mut events = Vec::new();
        for package in packages {
            events.push(Event::RelationalSchema(package.clone()));
            for table in &package.tables {
                let physical = Self::physical_table(&package.namespace, &table.name)?;
                let sql = format!(
                    "SELECT {} FROM \"{physical}\"",
                    table
                        .columns
                        .iter()
                        .map(|column| format!("\"{}\"", column.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let mut statement = conn
                    .prepare(&sql)
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                let rows = statement
                    .query_map([], |row| {
                        let mut object = serde_json::Map::new();
                        for (index, column) in table.columns.iter().enumerate() {
                            let value = match (&column.column_type, row.get_ref(index)?) {
                                (RelationalColumnType::Boolean, ValueRef::Integer(value)) => {
                                    Value::Bool(value != 0)
                                }
                                (RelationalColumnType::Json, ValueRef::Text(value)) => {
                                    serde_json::from_slice(value).unwrap_or_else(|_| {
                                        Value::String(String::from_utf8_lossy(value).into_owned())
                                    })
                                }
                                (_, ValueRef::Null) => Value::Null,
                                (_, ValueRef::Integer(value)) => Value::from(value),
                                (_, ValueRef::Real(value)) => Value::from(value),
                                (_, ValueRef::Text(value)) => {
                                    Value::String(String::from_utf8_lossy(value).into_owned())
                                }
                                (_, ValueRef::Blob(value)) => {
                                    Value::Array(value.iter().copied().map(Value::from).collect())
                                }
                            };
                            object.insert(column.name.clone(), value);
                        }
                        Ok(RelationalRowMutation {
                            table: table.name.clone(),
                            kind: RelationalMutationKind::Upsert,
                            values: Value::Object(object),
                            key: None,
                        })
                    })
                    .map_err(|e| Error::from_reason(e.to_string()))?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                if !rows.is_empty() {
                    events.push(Event::RelationalRows {
                        namespace: package.namespace.clone(),
                        schema_version: package.schema_version,
                        mutation_id: String::new(),
                        payload_hash: String::new(),
                        affected_rows: 0,
                        mutations: rows,
                    });
                }
            }
        }
        let mut applied = conn
            .prepare(
                "SELECT mutation_id, namespace, schema_version, payload_hash, affected_rows FROM applied_relational_mutations ORDER BY mutation_id",
            )
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let markers = applied
            .query_map([], |row| {
                Ok(Event::RelationalRows {
                    mutation_id: row.get(0)?,
                    namespace: row.get(1)?,
                    schema_version: row.get(2)?,
                    payload_hash: row.get(3)?,
                    affected_rows: row.get(4)?,
                    mutations: vec![],
                })
            })
            .map_err(|e| Error::from_reason(e.to_string()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        events.extend(markers);
        Ok(events)
    }

    fn transaction_checkpoint_events(&self) -> Result<Vec<Event>> {
        let conn = self.projection_db.lock();
        let mut statement = conn
            .prepare(
                "SELECT transaction_id, commit_sequence, payload_hash FROM applied_transactions ORDER BY COALESCE(frame_seq, commit_sequence)",
            )
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let events = statement
            .query_map([], |row| {
                Ok(Event::Transaction(GenesisTransactionEvent {
                    transaction_id: row.get(0)?,
                    origin_commit_seq: row.get(1)?,
                    payload_hash: row.get(2)?,
                    relational: vec![],
                    nodes: vec![],
                    edges: vec![],
                    vectors: vec![],
                }))
            })
            .map_err(|e| Error::from_reason(e.to_string()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(events)
    }

    fn hydrated_props(&self, node_u32: u32, node: &NodeOutput) -> Value {
        if !node.props.is_null() {
            return node.props.clone();
        }
        self.projection_props(node_u32)
            .ok()
            .flatten()
            .unwrap_or_else(|| Value::Object(Default::default()))
    }

    fn hydrated_node(&self, node_u32: u32, node: &NodeOutput) -> NodeOutput {
        if !node.props.is_null() {
            return node.clone();
        }
        let mut hydrated = node.clone();
        hydrated.props = self.hydrated_props(node_u32, node);
        hydrated
    }

    pub fn node_view(&self, id: &str) -> Option<NodeOutput> {
        let u32_id = self.get_u32(id)?;
        self.node_view_u32(u32_id)
    }

    pub fn node_view_u32(&self, u32_id: u32) -> Option<NodeOutput> {
        let node = self.nodes.get(&u32_id)?;
        Some(self.hydrated_node(u32_id, node.value()))
    }

    pub fn open(opts: OpenOptions) -> Result<Self> {
        let root = PathBuf::from(opts.path.clone());
        if !root.exists() {
            fs::create_dir_all(&root).ok();
        }
        let lock_path = root.join("genesis.lock");
        let lock_file = FileOpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| {
                Error::from_reason(format!(
                    "failed to open database ownership lock {}: {}",
                    lock_path.display(),
                    e
                ))
            })?;
        fs2::FileExt::try_lock_exclusive(&lock_file).map_err(|_| {
            Error::from_reason(format!(
                "database is already open by another GenesisBlockDB process: {}",
                root.display()
            ))
        })?;
        let read_only = opts.read_only.unwrap_or(false);
        let vector_dim = opts.vector_dim.unwrap_or(1536) as u16;

        // --- Schema-version compatibility gate (forward-incompat protection) ---
        // A database written by a NEWER engine must not be silently misread.
        // Read the on-disk schema version from the snapshot manifest and refuse
        // to open if it exceeds what this engine understands. Older snapshots
        // fall through to the existing on-load migrations (legacy meta/edge
        // formats) and are rewritten at the current SCHEMA_VERSION on the next
        // save_state().
        if let Some(on_disk) = Self::read_ondisk_schema_version(&root) {
            if on_disk > SCHEMA_VERSION {
                return Err(Error::from_reason(format!(
                    "SCHEMA_VERSION_UNSUPPORTED: database schema v{} was written by a newer engine; this engine supports up to v{}. Upgrade the engine to open this database (never partial-read, never rewrite — ADR--GENESISDB-JOURNAL-HISTORY §4).",
                    on_disk, SCHEMA_VERSION
                )));
            }
            if on_disk < SCHEMA_VERSION {
                println!(
                    "Schema: migrating on-disk format v{} -> v{} (applied on load, persisted on next save).",
                    on_disk, SCHEMA_VERSION
                );
            }
        }

        // --- Cryptographic Identity (Mark X) ---
        let identity_path = root.join("identity.bin");
        let signing_key = if identity_path.exists() {
            let bytes = fs::read(&identity_path).map_err(|e| Error::from_reason(e.to_string()))?;
            SigningKey::from_bytes(
                bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| Error::from_reason("invalid identity key length"))?,
            )
        } else {
            let key = SigningKey::from_bytes(&OsRng.gen::<[u8; 32]>());
            if !read_only {
                fs::write(&identity_path, key.to_bytes())
                    .map_err(|e| Error::from_reason(e.to_string()))?;
            }
            key
        };
        let verifying_key = signing_key.verifying_key();
        let local_peer_id = hex::encode(Sha256::digest(verifying_key.as_bytes()))[..16].to_string();

        // WP-1.2 framed journal layout: wal/active.gwal + journal/*.gseg.
        let legacy_log_path = root.join("genesis-graph.wal");
        let log_path = root.join("wal").join("active.gwal");
        if !read_only {
            fs::create_dir_all(root.join("wal")).ok();
            fs::create_dir_all(root.join("journal")).ok();
            // I9 hygiene: crashed seals leave *.tmp — delete them before
            // scanning. Skipped on read-only opens (which must not mutate the
            // database): a stray .tmp is never listed as a segment anyway.
            Self::journal_cleanup(&root);
        }
        // One-way migration (ADR §4): seal the legacy JSONL WAL as segment 0
        // (kind=legacy_jsonl, recovery-only, not tx-addressable) and remove it.
        // Lossless — the segment stores the exact legacy bytes — so it needs no
        // prior snapshot (the snapshot-first rule guards FOLDS, which discard;
        // seals discard nothing). Read-only opens skip migration and read the
        // legacy file in place.
        if !read_only && legacy_log_path.exists() {
            Self::migrate_legacy_wal(&root, &legacy_log_path)?;
        }
        // I9: truncate a torn active-file tail (crash mid-append) BEFORE the
        // writer thread opens it for append, then seed the frame counter from
        // the journal itself (ADR D2.4 — never from the projection). Read-only
        // opens skip the truncation (no mutation) — replay stops at the tear
        // either way, since `walk_frames` refuses the first invalid frame.
        if !read_only {
            Self::journal_truncate_torn_active(&log_path);
        }
        let initial_next_seq = Self::journal_max_seq(&root, &log_path) + 1;
        let projection_path = root.join(PROJECTION_DB_FILE);
        let projection_conn = if read_only {
            Self::open_projection(&projection_path, true)?
        } else {
            match Self::open_projection(&projection_path, false) {
                Ok(conn) if Self::init_projection_schema(&conn).is_ok() => conn,
                Ok(conn) => {
                    drop(conn);
                    if projection_path.exists() {
                        fs::remove_file(&projection_path)
                            .map_err(|e| Error::from_reason(e.to_string()))?;
                    }
                    let conn = Self::open_projection(&projection_path, false)?;
                    Self::init_projection_schema(&conn)?;
                    conn
                }
                Err(_) => {
                    if projection_path.exists() {
                        fs::remove_file(&projection_path)
                            .map_err(|e| Error::from_reason(e.to_string()))?;
                    }
                    let conn = Self::open_projection(&projection_path, false)?;
                    Self::init_projection_schema(&conn)?;
                    conn
                }
            }
        };
        let (wal_sender, wal_receiver): (Sender<WalMsg>, Receiver<WalMsg>) = unbounded();
        let log_path_clone = log_path.clone();

        let wal_handle = if read_only {
            std::thread::spawn(move || {
                while let Ok(message) = wal_receiver.recv() {
                    match message {
                        WalMsg::Append(_, ack) => {
                            let _ = ack.send(None);
                        }
                        WalMsg::Checkpoint { ack, .. } => {
                            let _ = ack.send(false);
                        }
                    }
                }
            })
        } else {
            let seg_root = root.clone();
            std::thread::spawn(move || {
                // The writer thread owns the active-file handle for the whole
                // Storage lifetime — every rotation (seal/fold) happens HERE,
                // because only the handle owner can replace the file on Windows
                // (the PR #28 lesson; see wal_checkpoint history in git).
                let Some(mut writer) = Self::journal_open_active(&log_path_clone, initial_next_seq)
                else {
                    // Can't open the journal: refuse every append so no caller
                    // ever gets a false durability ack.
                    while let Ok(message) = wal_receiver.recv() {
                        match message {
                            WalMsg::Append(_, ack) => {
                                let _ = ack.send(None);
                            }
                            WalMsg::Checkpoint { ack, .. } => {
                                let _ = ack.send(false);
                            }
                        }
                    }
                    return;
                };
                let mut next_seq = initial_next_seq;
                let mut frame_buf: Vec<u8> = Vec::with_capacity(4096);
                let mut batch: Vec<(crossbeam_channel::Sender<Option<u64>>, u64)> =
                    Vec::with_capacity(1024);
                // A fold pulled out of the micro-batch drain is stashed here and
                // applied only after the in-flight append batch is flushed +
                // acked, so the folded journal never loses a just-acked write.
                let mut pending_ckpt: Option<(Vec<u8>, u64, crossbeam_channel::Sender<bool>)> =
                    None;
                // I1 (RCA--SLICE0-DURABILITY defect 1): once ANY frame write,
                // flush, or fsync fails, the active file may hold a torn frame —
                // replay stops at the tear, so even a later successfully-synced
                // append would be unrecoverable. Poison the writer: every append
                // is acked None (no false durability ack) until a successful
                // fold rebuilds a clean active file.
                let mut poisoned = false;
                loop {
                    match wal_receiver.recv() {
                        Ok(WalMsg::Append(signed_event, ack_tx)) => {
                            // Any I/O failure in this batch fails the WHOLE batch:
                            // frames after a torn write are unrecoverable too.
                            let mut io_ok = !poisoned;
                            if poisoned {
                                let _ = ack_tx.send(None);
                            } else if let Ok(json) = serde_json::to_vec(&signed_event) {
                                let seq = next_seq;
                                next_seq += 1;
                                frame_buf.clear();
                                encode_frame(&mut frame_buf, seq, &json);
                                io_ok &= writer.write_all(&frame_buf).is_ok();
                                batch.push((ack_tx, seq));
                            } else {
                                let _ = ack_tx.send(None);
                            }
                            let timeout = Duration::from_millis(5);
                            let start = Instant::now();
                            while batch.len() < 1024 && start.elapsed() < timeout {
                                match wal_receiver.try_recv() {
                                    Ok(WalMsg::Append(se, tx)) => {
                                        if !io_ok {
                                            // Torn batch: refuse instead of appending
                                            // past the tear.
                                            let _ = tx.send(None);
                                        } else if let Ok(j) = serde_json::to_vec(&se) {
                                            let seq = next_seq;
                                            next_seq += 1;
                                            frame_buf.clear();
                                            encode_frame(&mut frame_buf, seq, &j);
                                            io_ok &= writer.write_all(&frame_buf).is_ok();
                                            batch.push((tx, seq));
                                        } else {
                                            let _ = tx.send(None);
                                        }
                                    }
                                    // Defer the fold: drain ends so the current
                                    // append batch is durably flushed + acked first.
                                    Ok(WalMsg::Checkpoint {
                                        payload_lines,
                                        frontier_seq,
                                        ack,
                                    }) => {
                                        pending_ckpt = Some((payload_lines, frontier_seq, ack));
                                        break;
                                    }
                                    Err(_) => break,
                                }
                            }
                            // I1: the ack is Some(seq) only if write_all AND
                            // flush AND sync_all all succeeded for the batch.
                            let ok = io_ok
                                && writer.flush().is_ok()
                                && writer.get_mut().sync_all().is_ok();
                            if !ok && !batch.is_empty() {
                                poisoned = true;
                                println!(
                                    "Mark IX: journal append I/O failure — refusing further appends until a successful checkpoint rebuilds the active file."
                                );
                            }
                            for (ack, seq) in batch.drain(..) {
                                let _ = ack.send(if ok { Some(seq) } else { None });
                            }
                            if let Some((lines, frontier, ack)) = pending_ckpt.take() {
                                let ok = Self::journal_fold(
                                    &mut writer,
                                    &seg_root,
                                    &log_path_clone,
                                    &lines,
                                    frontier,
                                    next_seq,
                                );
                                if ok {
                                    // A successful fold wrote + fsynced a fresh
                                    // base segment and reset the active file —
                                    // the torn tail (if any) is gone.
                                    poisoned = false;
                                }
                                let _ = ack.send(ok);
                            } else if Self::journal_active_len(&writer) > ACTIVE_SEAL_THRESHOLD {
                                Self::journal_seal_active(
                                    &mut writer,
                                    &seg_root,
                                    &log_path_clone,
                                    next_seq,
                                );
                            }
                        }
                        Ok(WalMsg::Checkpoint {
                            payload_lines,
                            frontier_seq,
                            ack,
                        }) => {
                            let ok = Self::journal_fold(
                                &mut writer,
                                &seg_root,
                                &log_path_clone,
                                &payload_lines,
                                frontier_seq,
                                next_seq,
                            );
                            if ok {
                                poisoned = false;
                            }
                            let _ = ack.send(ok);
                        }
                        Err(_) => break,
                    }
                }
            })
        };

        // --- Deferred-indexing thread (ADR--GENESISDB-ASYNC-INDEXING) ---
        // Drains HNSW insert jobs off the write hot path. Bounded for
        // backpressure: a sustained bulk load blocks the writer rather than
        // growing an unbounded queue of vector copies.
        let (index_tx, index_rx): (Sender<IndexJob>, Receiver<IndexJob>) = bounded(4096);
        let index_pending = Arc::new(AtomicUsize::new(0));
        let index_pending_thread = Arc::clone(&index_pending);
        let index_handle = std::thread::spawn(move || {
            while let Ok(job) = index_rx.recv() {
                match job {
                    IndexJob::One {
                        coll,
                        arena_id,
                        emb,
                        ef_c,
                    } => {
                        coll.ensure_hnsw(ef_c);
                        // insert(&self): hnsw_rs is internally synchronized -> read lock.
                        // The job ships f32; VecIndex quantizes per mode before insert.
                        // BQ centering: apply the SAME center `stage` used for the
                        // arena. Compaction flushes the queue before recomputing the
                        // center, so this read matches the stage-time value (no skew).
                        let center = coll.center_snapshot();
                        let sq8 = coll.sq8_snapshot();
                        if let Some(ref idx) = *coll.hnsw.read() {
                            idx.insert_f32(&emb, arena_id as usize, center.as_deref(), sq8);
                        }
                        index_pending_thread.fetch_sub(1, Ordering::Relaxed);
                    }
                    IndexJob::Batch { coll, items, ef_c } => {
                        coll.ensure_hnsw(ef_c);
                        let center = coll.center_snapshot();
                        let sq8 = coll.sq8_snapshot();
                        if let Some(ref idx) = *coll.hnsw.read() {
                            // hnsw_rs `parallel_insert` can leave nodes UNREACHABLE on
                            // small / near-degenerate graphs — concurrent insertion races
                            // graph linkage, so a vector lands in the arena but never gets
                            // wired into the navigable graph and search can never find it.
                            // RCA (collinear 30-pt loads): parallel_insert 97/300 loads had
                            // an unsearchable vector; sequential insert 0/300. Parallelism
                            // only pays off past a few hundred vectors anyway (rayon spawn
                            // overhead), so insert small batches sequentially for correct
                            // connectivity and keep the parallel path for large bulk loads
                            // (real high-dim data indexes fine there — recall benchmarks
                            // 0.98+). See ADR--GENESISDB-ASYNC-INDEXING.
                            const PARALLEL_INSERT_MIN: usize = 1024;
                            if items.len() < PARALLEL_INSERT_MIN {
                                for (emb, id) in &items {
                                    idx.insert_f32(emb, *id as usize, center.as_deref(), sq8);
                                }
                            } else {
                                idx.parallel_insert_f32(&items, center.as_deref(), sq8);
                            }
                        }
                        index_pending_thread.fetch_sub(items.len(), Ordering::Relaxed);
                    }
                    IndexJob::Flush(ack) => {
                        let _ = ack.send(());
                    }
                }
            }
        });

        // The `default` collection always exists; legacy single-space data and
        // any node added without an explicit collection routes here. Its dim is
        // the OpenOptions vector_dim (back-compat with the old global space).
        let collections: DashMap<String, Arc<VectorCollection>> = DashMap::new();
        collections.insert(
            "default".to_string(),
            Arc::new(VectorCollection::new(
                "default".to_string(),
                "default".to_string(),
                vector_dim,
                Metric::L2,
                Quant::None,
                None,
                false,
                None, // default is Quant::None + no rerank
                true,
                false, // Quant::None ⇒ no SQ8 calibration
            )),
        );

        let storage = Self {
            path: root,
            read_only,
            projection_path,
            projection_db: Mutex::new(projection_conn),
            commit_lock: Mutex::new(()),
            commit_sequence: AtomicU64::new(0),
            nodes: DashMap::new(),
            edges: DashMap::new(),
            out_idx: DashMap::new(),
            in_idx: DashMap::new(),
            collections,
            default_collection: "default".to_string(),
            log_path,
            bin_path: PathBuf::from(""),
            _lock_file: Some(lock_file),
            id_to_u32: DashMap::new(),
            next_u32: AtomicU32::new(0),
            is_rebuilding: AtomicBool::new(false),
            trigram_index: DashMap::new(),
            lang_centroids: DashMap::new(),
            peers: DashMap::new(),
            proposals: DashMap::new(),
            meta_nodes: DashMap::new(),
            meta_edges: DashMap::new(),
            meta_history: DashMap::new(),
            wal_sender,
            index_tx,
            index_pending,
            wal_handle: Some(wal_handle),
            index_handle: Some(index_handle),
            txn_frontier: AtomicU64::new(0),
            legacy_log_path,
            local_peer_id,
            logical_clock: AtomicU32::new(0),
            gossip_port: AtomicU32::new(0),
            // HNSW tunables — quality-first defaults; override via set_index_params
            // before bulk load to trade recall for build/query speed.
            ef_construction: AtomicUsize::new(200),
            ef_search: AtomicUsize::new(100),
            signing_key,
            verifying_key,
        };

        // --- Recovery (ADR--GENESISDB-JOURNAL-HISTORY D5): snapshot@frontier +
        // replay journal frames > frontier. The frame stamps make the cursor
        // position-independent, which retires PR #100's byte-positional
        // prefix-hash: replay is seq-filtered and idempotent (LWW upserts), so
        // a crash between the state.json rename and the fold merely re-applies
        // frames the snapshot already contains — correct either way, no
        // SnapshotStale machinery needed. Snapshots without a journal frontier
        // (pre-WP-1.2, or never saved) get a full journal replay, which is also
        // idempotent over whatever try_load_state loaded.
        storage
            .commit_sequence
            .store(initial_next_seq.saturating_sub(1), Ordering::SeqCst);
        let journal_frontier = fs::read_to_string(storage.path.join("state.json"))
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .and_then(|v| v["journal"]["frontier_seq"].as_u64());
        let saved_txn_frontier = fs::read_to_string(storage.path.join("state.json"))
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .and_then(|v| v["journal"]["txn_frontier"].as_u64())
            .unwrap_or(0);
        storage
            .txn_frontier
            .store(saved_txn_frontier, Ordering::SeqCst);
        let legacy_byte_frontier = fs::read_to_string(storage.path.join("state.json"))
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .is_some_and(|v| v.get("wal_frontier").is_some());
        // RCA--SLICE0-DURABILITY defect 4: a base segment folded at a seq
        // STRICTLY NEWER than the snapshot's cursor means the snapshot predates
        // a completed checkpoint (the fold-vs-state.json-rename crash window).
        // That fold already erased the history between the two cursors —
        // including any NodeRetract frames — and a base-segment replay can only
        // add, so trusting the stale snapshot would resurrect state deleted
        // between them. The base segment is a complete recovery source on its
        // own (I8): skip the snapshot and recover from the journal alone.
        let fold_horizon = storage.history_horizon();
        let snapshot_stale_vs_fold =
            journal_frontier.is_some_and(|frontier| fold_horizon > frontier);
        if snapshot_stale_vs_fold {
            println!(
                "Mark IX: snapshot cursor {} predates journal fold @{} — ignoring stale snapshot, recovering from the journal.",
                journal_frontier.unwrap_or(0),
                fold_horizon
            );
        }
        let snapshot_loaded = !snapshot_stale_vs_fold && storage.try_load_state();
        match (snapshot_loaded, journal_frontier) {
            // Framed snapshot: replay only frames past the seq frontier.
            (true, Some(frontier)) => storage.replay_journal(Some(frontier), false),
            // Pre-WP-1.2 snapshot carrying the byte-positional cursor: the
            // cursor is meaningless against the migrated journal, and the
            // legacy WAL tail may hold acked writes newer than the snapshot —
            // full replay, once (idempotent for graph state; duplicate vector
            // arena rows are reclaimed by the next index compaction).
            (true, None) if legacy_byte_frontier => storage.replay_journal(None, true),
            // Pre-frontier snapshot (no cursor of either kind): tail replay is
            // impossible, and the journal may hold acked writes newer than the
            // snapshot (RCA--SLICE0-DURABILITY defect 3) — full replay on top
            // of the instant load, exactly like the legacy-cursor branch above
            // (idempotent LWW; same one-time duplicate-arena-rows tradeoff).
            (true, None) => storage.replay_journal(None, true),
            // No snapshot (or a stale one skipped above): the journal alone is
            // the recovery source.
            (false, _) => storage.replay_journal(None, true),
        }
        // Rebuild the HNSW index for BOTH load paths: journal replay and the
        // instant snapshot load (try_load_state populates the vector/metadata
        // arenas but never rehydrates HNSW, leaving semantic search broken
        // until a manual rebuild).
        storage.rehydrate_hnsw_index();
        storage.projection_sync_on_open()?;
        Ok(storage)
    }

    pub fn start_autonomic_loop(storage: Arc<Self>) {
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(3600));
            if !storage.read_only {
                let _ = storage.perform_autonomic_optimization();
                let _ = storage.save_state();
            }
        });
    }

    pub fn ensure_writable(&self) -> Result<()> {
        if self.read_only {
            return Err(Error::from_reason("read-only"));
        }
        Ok(())
    }

    /// Sign an event with this node's key, producing a WAL/gossip-ready
    /// `SignedEvent`. The WAL is a log of `SignedEvent` lines — `persist`,
    /// `reconcile_state`, and `compact` all write through this so the format is
    /// uniform and every reader (`try_load_state` replay, `events_since`) can
    /// parse it back.
    fn sign_event(&self, event: &Event) -> SignedEvent {
        let event_data = serde_json::to_vec(event).unwrap_or_default();
        let signature = self.signing_key.sign(&event_data).to_bytes().to_vec();
        SignedEvent {
            event: event.clone(),
            signature,
            signer_peer_id: self.local_peer_id.clone(),
        }
    }

    /// Append + fsync one event; returns the stamped frame `commit_seq`
    /// (WP-1.2, ADR D2.4). The frame frontier (`stable_frontier`) advances here.
    fn append_wal_event(&self, event: &Event) -> Result<u64> {
        let (ack_tx, ack_rx) = unbounded();
        let signed_event = self.sign_event(event);
        self.wal_sender
            .send(WalMsg::Append(Box::new(signed_event), ack_tx))
            .map_err(|_| Error::from_reason("wal disconnected"))?;
        let seq = ack_rx
            .recv()
            .ok()
            .flatten()
            .ok_or_else(|| Error::from_reason("wal append failed"))?;
        self.commit_sequence.fetch_max(seq, Ordering::SeqCst);
        Ok(seq)
    }

    pub fn persist(&self, event: &Event) -> Result<u64> {
        let seq = self.append_wal_event(event)?;
        self.projection_apply_event(event, seq)?;
        Ok(seq)
    }

    fn apply_transaction_memory(&self, transaction: &GenesisTransactionEvent, index: bool) {
        for node in &transaction.nodes {
            let node_u32 = self.get_or_intern_id(&node.id);
            if let Some(embedding) = &node.embedding {
                self.replay_vector(
                    &node.collection,
                    &node.id,
                    embedding.clone(),
                    node.lang.clone().unwrap_or_else(|| "en".to_string()),
                    index,
                );
            }
            self.insert_node_lean(node_u32, node.clone());
        }
        for edge in &transaction.edges {
            let edge_key = self.index_edge_internal(&edge.id, &edge.from, &edge.to);
            self.edges.insert(edge_key, edge.clone());
        }
        for vector in &transaction.vectors {
            self.replay_vector(
                &vector.collection,
                &vector.node_id,
                vector.embedding.clone(),
                vector.lang.clone().unwrap_or_else(|| "en".to_string()),
                index,
            );
        }
    }

    /// The FRAME frontier: commit_seq of the last durable journal frame
    /// (WP-1.2 REDEFINITION, ADR D2.3 — previously "last transaction
    /// commit_sequence", which is now `txn_frontier()`). Advances on every
    /// mutation; replica-local.
    pub fn stable_frontier(&self) -> u64 {
        self.commit_sequence.load(Ordering::SeqCst)
    }

    /// Frame seq of the last `Event::Transaction` frame — the value
    /// `GenesisTransaction.expected_frontier` CASes against.
    pub fn txn_frontier(&self) -> u64 {
        self.txn_frontier.load(Ordering::SeqCst)
    }

    /// ADR D4: oldest history boundary — the max seq folded into a base
    /// segment (0 = no fold has ever discarded history). tx-time queries below
    /// this fail `beyond_horizon`; delta-pull cursors below it get
    /// `GossipMessage::BeyondHorizon`.
    pub fn history_horizon(&self) -> u64 {
        Self::journal_list_segments(&self.path)
            .into_iter()
            .filter(|s| s.kind == SEG_KIND_BASE)
            .map(|s| s.max_seq)
            .max()
            .unwrap_or(0)
    }

    pub fn commit_transaction(&self, input: GenesisTransaction) -> Result<CommitResult> {
        self.ensure_writable()?;
        if input.transaction_id.is_empty() || input.transaction_id.len() > 128 {
            return Err(Error::from_reason("invalid transaction id"));
        }
        let payload = serde_json::to_vec(&input).map_err(|e| Error::from_reason(e.to_string()))?;
        let payload_hash = hex::encode(Sha256::digest(&payload));
        let _commit_guard = self.commit_lock.lock();
        {
            let conn = self.projection_db.lock();
            // Idempotent replay returns the stored LOCAL frame seq; legacy rows
            // (pre-WP-1.2, frame_seq NULL) fall back to their origin sequence.
            let existing: Option<(u64, String)> = conn
                .query_row(
                    "SELECT COALESCE(frame_seq, commit_sequence), payload_hash FROM applied_transactions WHERE transaction_id = ?1",
                    [&input.transaction_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| Error::from_reason(e.to_string()))?;
            if let Some((commit_sequence, existing_hash)) = existing {
                if existing_hash != payload_hash {
                    return Err(Error::from_reason("transaction identity conflict"));
                }
                return Ok(CommitResult {
                    transaction_id: input.transaction_id,
                    commit_sequence,
                    stable: true,
                });
            }
        }
        // WP-1.2 (ADR D2.3): the CAS is re-based to the TRANSACTION-lineage
        // frontier. stable_frontier() now advances on every mutation, so a
        // frame-level CAS would spuriously fail on any interleaved add_node.
        let frontier = self.txn_frontier();
        if input
            .expected_frontier
            .is_some_and(|expected| expected != frontier)
        {
            return Err(Error::from_reason("expected frontier conflict"));
        }
        for group in &input.relational {
            Self::validate_identifier(&group.namespace)?;
            let conn = self.projection_db.lock();
            let schema = Self::load_relational_schema_conn(&conn, &group.namespace)?
                .ok_or_else(|| Error::from_reason("relational schema is not registered"))?;
            for mutation in &group.mutations {
                Self::validate_row_mutation(&schema, mutation)?;
            }
        }
        for node in &input.graph.nodes {
            self.validate_governance(&node.labels, false)?;
            if let Some(embedding) = &node.embedding {
                let collection = self.resolve_collection(&node.collection)?;
                if embedding.len() != collection.dim as usize {
                    return Err(Error::from_reason("embedding dimension mismatch"));
                }
            }
        }
        for vector in &input.vectors {
            let collection = self.resolve_collection(&Some(vector.collection.clone()))?;
            if vector.embedding.len() != collection.dim as usize {
                return Err(Error::from_reason("embedding dimension mismatch"));
            }
        }

        let now = Utc::now();
        let nodes = input
            .graph
            .nodes
            .into_iter()
            .map(|node| {
                let collection = node
                    .embedding
                    .as_ref()
                    .and_then(|_| self.resolve_collection(&node.collection).ok())
                    .map(|collection| collection.name.clone());
                NodeOutput {
                    id: node.id.unwrap_or_else(|| format!("N-{}", Uuid::new_v4())),
                    labels: node.labels,
                    props: node.props.unwrap_or(Value::Object(Default::default())),
                    impact: Some(0.7),
                    embedding: node.embedding,
                    lang: Some(node.lang.unwrap_or_else(|| "en".to_string())),
                    valid_from: node.valid_from.unwrap_or_else(|| now.to_rfc3339()),
                    valid_to: None,
                    caused_by: node.caused_by,
                    expires_at: node.ttl.map(|seconds| {
                        (now + chrono::Duration::seconds(seconds as i64)).to_rfc3339()
                    }),
                    clock: self.next_clock(),
                    collection,
                }
            })
            .collect::<Vec<_>>();
        let edges = input
            .graph
            .edges
            .into_iter()
            .map(|edge| EdgeOutput {
                id: edge.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                from: edge.from,
                to: edge.to,
                rel: edge.rel,
                props: edge.props.unwrap_or(Value::Object(Default::default())),
                valid_from: edge.valid_from.unwrap_or_else(|| now.to_rfc3339()),
                valid_to: None,
                recorded_at: now.to_rfc3339(),
                superseded_by: None,
                impact: edge.impact,
                caused_by: edge.caused_by,
                clock: self.next_clock(),
            })
            .collect::<Vec<_>>();
        let vectors = input
            .vectors
            .into_iter()
            .map(|vector| VectorEvent {
                node_id: vector.node_id,
                collection: Some(vector.collection),
                embedding: vector.embedding,
                lang: None,
                clock: self.next_clock(),
            })
            .collect::<Vec<_>>();
        let event = GenesisTransactionEvent {
            transaction_id: input.transaction_id.clone(),
            // ADR D2.2: origin metadata only. 0 for locally-created events; the
            // authoritative sequence is the unsigned frame stamp below (it
            // cannot live in this signed payload — peer frames keep original
            // signatures, I4).
            origin_commit_seq: 0,
            payload_hash,
            relational: input.relational,
            nodes,
            edges,
            vectors,
        };
        let commit_sequence = self.persist(&Event::Transaction(event.clone()))?;
        self.apply_transaction_memory(&event, true);
        self.txn_frontier.store(commit_sequence, Ordering::SeqCst);
        Ok(CommitResult {
            transaction_id: input.transaction_id,
            commit_sequence,
            stable: true,
        })
    }

    pub fn find_fuzzy_id(&self, id: &str) -> Option<String> {
        // 1. Exact Match
        if self.get_u32(id).is_some() {
            return Some(id.to_string());
        }

        // 2. Lexical Fuzzy (Thai-aware Trigrams)
        let mut candidates = HashSet::new();
        let tokens = Self::tokenize_id(id);

        for trigram in tokens {
            if let Some(nodes) = self.trigram_index.get(&trigram) {
                candidates.extend(nodes.value().iter());
            }
        }

        let mut best_lexical_id = None;
        let mut max_lexical_sim = 0.0;

        for u32_id in &candidates {
            // Resolve the candidate's id from the canonical `nodes` record
            // (the reverse map was dropped — ADR--GENESISDB-NODE-ID-INTERNING).
            // Trigram candidates that intern only as edge endpoints (no node
            // record) are skipped: fuzzy id resolution targets real nodes.
            if let Some(node_ref) = self.nodes.get(u32_id) {
                let candidate_id = &node_ref.value().id;
                let sim = strsim::jaro_winkler(id, candidate_id);
                if sim > max_lexical_sim {
                    max_lexical_sim = sim;
                    best_lexical_id = Some(candidate_id.clone());
                }
            }
        }

        if max_lexical_sim > 0.85 {
            return best_lexical_id;
        }

        // 3. Neural Fuzzy (Vector Fallback)
        // Relaxed threshold for Thai characters.
        if max_lexical_sim > 0.20 {
            return best_lexical_id;
        }

        None
    }

    pub fn semantic_verify(&self, event: &Event) -> Result<bool> {
        match event {
            Event::Node(node) => {
                if let Some(emb) = &node.embedding {
                    let context = self.get_ranked_context(HybridSearchInput {
                        query_vector: emb.clone(),
                        k: 3,
                        alpha: Some(0.4),
                        lang: node.lang.clone(),
                        as_of: None,
                        collection: node.collection.clone(),
                        ef_search: None,
                        oversample: None,
                    })?;

                    for neighbor in context {
                        if neighbor.node.impact.unwrap_or(0.0) > 0.8
                            && node.labels != neighbor.node.labels
                            && neighbor.node.labels.contains(&"MASTER".to_string())
                        {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }
            Event::Edge(_) => Ok(true),
            // A vector attachment carries no governance/axiom implication.
            Event::Vector(_) => Ok(true),
            // Retraction proposals carry no axiom-creation implication (the
            // MASTER-immutability breadth question is tracked separately).
            Event::NodeRetract { .. } => Ok(true),
            Event::RelationalSchema(_) | Event::RelationalRows { .. } => Ok(true),
            Event::Transaction(transaction) => {
                for node in &transaction.nodes {
                    if !self.semantic_verify(&Event::Node(node.clone()))? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Event::Batch(events) => {
                for e in events {
                    if !self.semantic_verify(e)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    /// Open a consensus proposal for `event`. The proposal is signed with this
    /// node's own key (the `_signature` param is ignored — an external caller has
    /// no access to the local private key, so a caller-supplied signature could
    /// never be authentic; signing here binds the proposal to this node as its
    /// authentic originator). The event must also pass `semantic_verify`, so a
    /// proposal that conflicts with an existing high-impact MASTER axiom is
    /// rejected up front rather than slipping through to quorum.
    pub fn propose_consensus(&self, event: Event, _signature: Vec<u8>) -> Result<String> {
        if !self.semantic_verify(&event)? {
            return Err(Error::from_reason(
                "proposal rejected by semantic_verify (conflicts with a governing axiom)",
            ));
        }
        let proposal_id = Uuid::new_v4().to_string();
        let event_data =
            serde_json::to_vec(&event).map_err(|e| Error::from_reason(e.to_string()))?;
        let signature = self.signing_key.sign(&event_data).to_bytes().to_vec();
        let signed_event = SignedEvent {
            event,
            signature,
            signer_peer_id: self.local_peer_id.clone(),
        };
        let proposal = ConsensusProposal {
            proposal_id: proposal_id.clone(),
            signed_event,
            votes: HashMap::new(),
            quorum_signatures: HashMap::new(),
            committed: false,
        };
        self.proposals.insert(proposal_id.clone(), proposal);
        Ok(proposal_id)
    }

    /// Canonical bytes a peer signs to cast a vote. Binding the proposal id,
    /// voter id, and the approve/reject choice prevents replaying a vote onto a
    /// different proposal or flipping its decision.
    fn vote_payload(proposal_id: &str, voter_peer_id: &str, approve: bool) -> Vec<u8> {
        format!("VOTE|{}|{}|{}", proposal_id, voter_peer_id, approve).into_bytes()
    }

    /// The ed25519 public key for `peer_id`: this node's own key for a self-vote,
    /// otherwise the key registered for that peer (via gossip Heartbeat). `None`
    /// if the peer is unknown — an unknown peer cannot have its vote verified.
    fn peer_verifying_key(&self, peer_id: &str) -> Option<VerifyingKey> {
        if peer_id == self.local_peer_id {
            return Some(self.verifying_key);
        }
        let bytes = self.peers.get(peer_id)?.verifying_key.clone();
        let arr: [u8; 32] = bytes.as_slice().try_into().ok()?;
        VerifyingKey::from_bytes(&arr).ok()
    }

    /// Verify that `se.signature` is an authentic ed25519 signature by
    /// `se.signer_peer_id` over the canonical event bytes (`serde_json::to_vec`,
    /// the same convention `persist`/`propose` sign with). Unknown signer,
    /// malformed signature, or non-matching signature all return `false`. This is
    /// the single source of truth for event-level signature checks — the WAL sync
    /// (`reconcile_state`), the consensus propose/commit paths all route here.
    fn verify_event_signature(&self, se: &SignedEvent) -> bool {
        let vkey = match self.peer_verifying_key(&se.signer_peer_id) {
            Some(k) => k,
            None => return false,
        };
        let data = match serde_json::to_vec(&se.event) {
            Ok(d) => d,
            Err(_) => return false,
        };
        let sig = match Signature::from_slice(&se.signature) {
            Ok(s) => s,
            Err(_) => return false,
        };
        vkey.verify(&data, &sig).is_ok()
    }

    /// Sign a vote with this node's key so a remote proposal-holder can verify it
    /// authentically came from this peer. Returns the detached ed25519 signature.
    pub fn sign_vote(&self, proposal_id: String, approve: bool) -> Vec<u8> {
        let payload = Self::vote_payload(&proposal_id, &self.local_peer_id, approve);
        self.signing_key.sign(&payload).to_bytes().to_vec()
    }

    pub fn submit_vote(
        &self,
        proposal_id: String,
        peer_id: String,
        approve: bool,
        signature: Vec<u8>,
    ) -> Result<bool> {
        // Verify the vote is authentically signed by `peer_id` before recording
        // it — otherwise any caller could forge votes on another peer's behalf and
        // drive a proposal to quorum. Unknown peers, malformed or non-matching
        // signatures are rejected (the vote is not counted).
        let vkey = self.peer_verifying_key(&peer_id).ok_or_else(|| {
            Error::from_reason(format!(
                "unknown voter peer '{}' (no verifying key)",
                peer_id
            ))
        })?;
        let payload = Self::vote_payload(&proposal_id, &peer_id, approve);
        let sig = Signature::from_slice(&signature)
            .map_err(|_| Error::from_reason("malformed vote signature"))?;
        vkey.verify(&payload, &sig)
            .map_err(|_| Error::from_reason("invalid vote signature"))?;

        // Serialize the vote-commit section with every other writer and with
        // save_state/compact (RCA--PERSIST-SIGNED-CHECKPOINT-RACE). Committing
        // mutates in-memory state and then appends to the WAL; without this lock
        // a checkpoint could build its payload from memory BEFORE the mutation
        // and truncate the WAL AFTER the append was fsynced and acked, erasing a
        // commit this method already reported as durable. Holding the lock means
        // the commit either lands wholly before the payload build, or wholly
        // after the checkpoint (on the new WAL, past the frontier, where tail
        // replay recovers it).
        //
        // Taken BEFORE `proposals.get_mut` so the lock order is
        // commit_lock -> proposals. `self.proposals` is touched only here and in
        // `propose_consensus` (which takes no commit_lock), so no path acquires
        // them in the opposite order and no cycle exists. The signature checks
        // above touch only `self.peers`, so they stay off this global lock.
        let _commit_guard = self.commit_lock.lock();

        if let Some(mut proposal_ref) = self.proposals.get_mut(&proposal_id) {
            let proposal = proposal_ref.value_mut();
            // Already-committed guard: once quorum is crossed and the event applied,
            // later approving votes must not re-apply or re-persist it.
            if proposal.committed {
                return Ok(true);
            }
            proposal.votes.insert(peer_id.clone(), approve);
            // Retain the verified signature as proof of the vote (quorum_signatures).
            proposal
                .quorum_signatures
                .insert(peer_id.clone(), signature);

            let approvals = proposal.votes.values().filter(|&&v| v).count();

            // Quorum is a strict majority of the swarm. `self.peers` excludes this
            // node, so the membership denominator is peers + self.
            if approvals <= self.peers.len().div_ceil(2) {
                return Ok(false);
            }

            // Last gate before the event becomes durable state: re-verify the
            // proposal's own event signature (a gossiped proposal is verified on
            // receipt, but defense-in-depth) and re-run the governance check.
            let signed_event = proposal.signed_event.clone();
            if !self.verify_event_signature(&signed_event) {
                return Err(Error::from_reason(
                    "proposal event signature invalid at commit",
                ));
            }
            if !self.semantic_verify(&signed_event.event)? {
                return Err(Error::from_reason(
                    "proposal rejected by semantic_verify at commit",
                ));
            }

            match &signed_event.event {
                Event::Node(n) => {
                    let mut n_axiom = n.clone();
                    if !n_axiom.labels.contains(&"MASTER".to_string()) {
                        n_axiom.labels.push("MASTER".to_string());
                    }
                    let u32_id = self.get_or_intern_id(&n_axiom.id);
                    self.insert_node_lean(u32_id, n_axiom.clone());
                    self.persist_signed(SignedEvent {
                        event: Event::Node(n_axiom),
                        signature: signed_event.signature.clone(),
                        signer_peer_id: signed_event.signer_peer_id.clone(),
                    })?;
                }
                Event::Edge(e) => {
                    // Index into the adjacency maps (out_idx/in_idx) so the
                    // committed edge is traversable in this process — not just
                    // present in `edges` until the next reload.
                    let ekey = self.index_edge_internal(&e.id, &e.from, &e.to);
                    self.edges.insert(ekey, e.clone());
                    self.refresh_impacts(Some(vec![e.to.clone()]));
                    self.persist_signed(signed_event.clone())?;
                }
                Event::Batch(events) => {
                    for e in events {
                        match e {
                            Event::Node(n) => {
                                let mut n_axiom = n.clone();
                                if !n_axiom.labels.contains(&"MASTER".to_string()) {
                                    n_axiom.labels.push("MASTER".to_string());
                                }
                                let u32_id = self.get_or_intern_id(&n_axiom.id);
                                self.insert_node_lean(u32_id, n_axiom);
                            }
                            Event::Edge(edge) => {
                                let ekey = self.index_edge_internal(&edge.id, &edge.from, &edge.to);
                                self.edges.insert(ekey, edge.clone());
                            }
                            _ => {}
                        }
                    }
                    self.persist_signed(signed_event.clone())?;
                }
                // A committed vector is staged + enqueued (index=true) so it is
                // searchable in this process, matching the CRDT-sync path — not
                // merely persisted for a future replay.
                Event::Vector(v) => {
                    self.replay_vector(
                        &v.collection,
                        &v.node_id,
                        v.embedding.clone(),
                        v.lang.clone().unwrap_or_else(|| "en".to_string()),
                        true,
                    );
                    self.persist_signed(signed_event.clone())?;
                }
                Event::RelationalSchema(_) | Event::RelationalRows { .. } => {
                    self.persist_signed(signed_event.clone())?;
                }
                Event::Transaction(transaction) => {
                    // ADR D2.2: the peer's origin_commit_seq is metadata — it
                    // must NOT be merged into the local counter (the retired
                    // fetch_max contaminated the replica-local clock and could
                    // collide the projection's uniqueness). The local frame
                    // stamp from persist_signed IS this transaction's sequence.
                    let seq = self.persist_signed(signed_event.clone())?;
                    self.apply_transaction_memory(transaction, true);
                    self.txn_frontier.fetch_max(seq, Ordering::SeqCst);
                }
                // A committed retraction applies like the replay path and is
                // persisted with its original proposal signature.
                Event::NodeRetract { id, .. } => {
                    if let Some(u32_id) = self.get_u32(id) {
                        self.retract_node_memory(id, u32_id);
                    }
                    self.persist_signed(signed_event.clone())?;
                }
            }
            proposal.committed = true;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn calculate_sc(&self, node: &NodeOutput) -> f64 {
        let props = self
            .get_u32(&node.id)
            .map(|u32_id| self.hydrated_props(u32_id, node))
            .unwrap_or_else(|| node.props.clone());
        let stability = node
            .props
            .get("stability")
            .or_else(|| props.get("stability"))
            .and_then(|v| v.as_str())
            .unwrap_or("active");
        match stability {
            "stable" => 1.0,
            "active" => 0.8,
            "draft" => 0.4,
            "deprecated" => 0.1,
            _ => 0.8,
        }
    }

    pub fn compute_impact(&self, node: &NodeOutput) -> f64 {
        let u32_id = match self.get_u32(&node.id) {
            Some(id) => id,
            None => return 0.7,
        };
        let hydrated = self.hydrated_node(u32_id, node);
        let incoming_count = self
            .in_idx
            .get(&u32_id)
            .map(|edges| edges.len())
            .unwrap_or(0);
        let dd = (incoming_count as f64 / 10.0).min(1.0);
        let tier = Tier::from_labels(&node.labels);
        let as_score = match tier {
            Tier::MASTER => 1.0,
            Tier::SPEC => 0.8,
            Tier::ADR => 0.6,
            Tier::USER => 0.3,
        };
        let sc = self.calculate_sc(&hydrated);
        (dd * 0.5) + (as_score * 0.3) + (sc * 0.2)
    }

    pub fn refresh_impacts(&self, affected_ids: Option<Vec<String>>) {
        let ids_to_process = match affected_ids {
            Some(ids) => ids,
            None => self
                .nodes
                .iter()
                .map(|entry| entry.value().id.clone())
                .collect(),
        };
        for id in ids_to_process {
            if let Some(u32_id) = self.get_u32(&id) {
                if let Some(mut node_ref) = self.nodes.get_mut(&u32_id) {
                    let new_impact = self.compute_impact(node_ref.value());
                    node_ref.value_mut().impact = Some(new_impact);
                }
            }
        }
    }

    /// Index an edge into the adjacency maps and return its u128 key. The edge key
    /// is the deterministic `edge_key(id)` hash (no trigram/reverse/`id_to_u32`);
    /// `from`/`to` are node ids and keep the full node intern (they are searchable).
    pub fn index_edge_internal(&self, id: &str, from: &str, to: &str) -> u128 {
        let ekey = Self::edge_key(id);
        let u32_from = self.get_or_intern_id(from);
        let u32_to = self.get_or_intern_id(to);
        self.out_idx.entry(u32_from).or_default().insert(ekey);
        self.in_idx.entry(u32_to).or_default().insert(ekey);
        ekey
    }

    fn next_clock(&self) -> LogicalClock {
        let time = self.logical_clock.fetch_add(1, Ordering::SeqCst) + 1;
        LogicalClock {
            time,
            peer_id: self.local_peer_id.clone(),
        }
    }

    /// Insert a node into the in-memory store without its embedding.
    /// The f32 vector lives in `vector_arena` + the HNSW index (the source of
    /// truth for search); keeping a third f64 copy on every node wastes ~12 KB
    /// per node (the largest avoidable per-node cost). The full embedding is
    /// still persisted in the WAL `Event::Node` for replay/arena rebuild.
    fn insert_node_lean(&self, u32_id: u32, mut node: NodeOutput) {
        node.embedding = None;
        node.props = Value::Null;
        self.nodes.insert(u32_id, node);
    }

    pub fn add_node(&self, args: NodeInput) -> Result<NodeOutput> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        self.validate_governance(&args.labels, false)?;
        let id = args.id.unwrap_or_else(|| format!("N-{}", Uuid::new_v4()));
        let u32_id = self.get_or_intern_id(&id);
        let lang = args.lang.clone().unwrap_or("en".to_string());

        let now = Utc::now();
        let expires_at = args
            .ttl
            .map(|s| (now + chrono::Duration::seconds(s as i64)).to_rfc3339());

        let mut node = NodeOutput {
            id: id.clone(),
            labels: args.labels,
            props: args.props.unwrap_or(Value::Object(Default::default())),
            impact: Some(0.7),
            embedding: None,
            lang: Some(lang.clone()),
            valid_from: args.valid_from.unwrap_or_else(|| now.to_rfc3339()),
            valid_to: None,
            caused_by: args.caused_by,
            expires_at,
            clock: self.next_clock(),
            collection: None,
        };
        if let Some(emb) = args.embedding {
            // Validate + stage BEFORE recording the collection on the node, so a
            // dim mismatch fails the add instead of persisting a bad reference.
            self.add_vector_internal(&args.collection, &id, emb.clone(), lang)?;
            node.embedding = Some(emb);
            node.collection = Some(
                args.collection
                    .clone()
                    .unwrap_or_else(|| self.default_collection.clone()),
            );
        }
        self.insert_node_lean(u32_id, node.clone());
        self.persist(&Event::Node(node.clone()))?;
        Ok(node)
    }

    pub fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        let edge = EdgeOutput {
            id: args.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            from: args.from,
            to: args.to,
            rel: args.rel,
            props: args.props.unwrap_or(Value::Object(Default::default())),
            valid_from: args.valid_from.unwrap_or_else(|| Utc::now().to_rfc3339()),
            valid_to: None,
            recorded_at: Utc::now().to_rfc3339(),
            superseded_by: None,
            impact: args.impact,
            caused_by: args.caused_by,
            clock: self.next_clock(),
        };
        let u32_id = self.index_edge_internal(&edge.id, &edge.from, &edge.to);
        self.edges.insert(u32_id, edge.clone());
        self.refresh_impacts(Some(vec![edge.to.clone()]));
        self.persist(&Event::Edge(edge.clone()))?;
        Ok(edge)
    }

    pub fn supersede_node(
        &self,
        id: String,
        new_props: Option<Value>,
        caused_by: Option<String>,
    ) -> Result<NodeOutput> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        let u32_id = match self.get_u32(&id) {
            Some(i) => i,
            None => return Err(Error::from_reason(format!("Node {} not found", id))),
        };

        let now = Utc::now().to_rfc3339();

        let mut old_node = match self.nodes.get(&u32_id) {
            Some(node) => self.hydrated_node(u32_id, node.value()),
            None => return Err(Error::from_reason("Node not in memory index")),
        };

        old_node.valid_to = Some(now.clone());
        self.persist(&Event::Node(old_node.clone()))?;

        let mut new_node = old_node.clone();
        new_node.valid_from = now.clone();
        new_node.valid_to = None;
        new_node.caused_by = caused_by;
        if let Some(props) = new_props {
            new_node.props = props;
        }
        new_node.clock = self.next_clock();

        self.insert_node_lean(u32_id, new_node.clone());
        self.persist(&Event::Node(new_node.clone()))?;

        Ok(new_node)
    }

    pub fn rebuild_index_parallel(&self) -> Result<()> {
        let _commit_guard = self.commit_lock.lock();
        self.is_rebuilding.store(true, Ordering::SeqCst);
        self.flush_index();
        let result = {
            self.rehydrate_hnsw_index();
            Ok(())
        };
        self.is_rebuilding.store(false, Ordering::SeqCst);
        result
    }

    /// Extract a queryable field from a result row as a JSON value.
    /// Absent props / unset score resolve to `Null` (never panics).
    fn hql_field_value(n: &NeighborOutput, f: &query::ast::HqlField) -> serde_json::Value {
        use query::ast::HqlField;
        match f {
            HqlField::Id => serde_json::Value::String(n.node.id.clone()),
            HqlField::Label => {
                serde_json::to_value(&n.node.labels).unwrap_or(serde_json::Value::Null)
            }
            HqlField::Score => match n.score {
                Some(s) => serde_json::json!(s),
                None => serde_json::Value::Null,
            },
            HqlField::Depth => serde_json::json!(n.depth),
            HqlField::Prop(k) => n
                .node
                .props
                .get(k)
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        }
    }

    fn hql_cmp_num(l: f64, op: query::ast::HqlOp, r: f64) -> bool {
        use query::ast::HqlOp;
        match op {
            HqlOp::Eq => l == r,
            HqlOp::Ne => l != r,
            HqlOp::Lt => l < r,
            HqlOp::Le => l <= r,
            HqlOp::Gt => l > r,
            HqlOp::Ge => l >= r,
            _ => false,
        }
    }

    fn hql_cmp_str(l: &str, op: query::ast::HqlOp, r: &str) -> bool {
        use query::ast::HqlOp;
        match op {
            HqlOp::Eq => l == r,
            HqlOp::Ne => l != r,
            HqlOp::Lt => l < r,
            HqlOp::Le => l <= r,
            HqlOp::Gt => l > r,
            HqlOp::Ge => l >= r,
            _ => false,
        }
    }

    /// Evaluate one WHERE predicate against a row. Missing field, null, or a
    /// type mismatch all yield `false` (SQL-style UNKNOWN -> excluded), for every
    /// operator including `!=`. See ADR--GENESISDB-HQL-FILTER-PROJECTION.
    fn hql_eval_predicate(n: &NeighborOutput, p: &query::ast::HqlPredicate) -> bool {
        use query::ast::{HqlField, HqlOp, HqlValue};
        // `label` is multi-valued: equality/membership tests scan the label set.
        if let HqlField::Label = p.field {
            if let HqlValue::Str(s) = &p.value {
                return match p.op {
                    HqlOp::Eq => n.node.labels.iter().any(|l| l == s),
                    HqlOp::Ne => !n.node.labels.iter().any(|l| l == s),
                    HqlOp::Contains => n.node.labels.iter().any(|l| l.contains(s.as_str())),
                    HqlOp::StartsWith => n.node.labels.iter().any(|l| l.starts_with(s.as_str())),
                    _ => false,
                };
            }
            return false; // label compared against a number
        }

        let lhs = Self::hql_field_value(n, &p.field);
        match (p.op, &p.value) {
            (HqlOp::Contains, HqlValue::Str(s)) => {
                lhs.as_str().is_some_and(|x| x.contains(s.as_str()))
            }
            (HqlOp::StartsWith, HqlValue::Str(s)) => {
                lhs.as_str().is_some_and(|x| x.starts_with(s.as_str()))
            }
            (HqlOp::Contains | HqlOp::StartsWith, _) => false,
            (op, HqlValue::Num(r)) => match lhs.as_f64() {
                Some(l) => Self::hql_cmp_num(l, op, *r),
                None => false,
            },
            (op, HqlValue::Str(r)) => match lhs.as_str() {
                Some(l) => Self::hql_cmp_str(l, op, r),
                None => false,
            },
        }
    }

    /// Numeric-when-both-numeric, else string ordering for ORDER BY.
    fn hql_cmp_order(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        if let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) {
            return x.partial_cmp(&y).unwrap_or(Ordering::Equal);
        }
        if let (Some(x), Some(y)) = (a.as_str(), b.as_str()) {
            return x.cmp(y);
        }
        a.to_string().cmp(&b.to_string())
    }

    /// Apply the optional WHERE / ORDER BY / LIMIT / RETURN clauses to a result
    /// list, in that order, then serialize. Pure post-processing — no planner.
    fn apply_hql_clauses(
        mut results: Vec<NeighborOutput>,
        clauses: &query::ast::HqlClauses,
    ) -> Result<serde_json::Value> {
        use query::ast::HqlReturn;
        let ser_err = |e: serde_json::Error| {
            Error::from_reason(format!("HQL result serialization failed: {e}"))
        };

        // 1. WHERE (conjunction of predicates)
        if !clauses.where_preds.is_empty() {
            results.retain(|n| {
                clauses
                    .where_preds
                    .iter()
                    .all(|p| Self::hql_eval_predicate(n, p))
            });
        }

        // 2. ORDER BY (nulls always last, regardless of ASC/DESC)
        if let Some((field, desc)) = &clauses.order_by {
            results.sort_by(|a, b| {
                let av = Self::hql_field_value(a, field);
                let bv = Self::hql_field_value(b, field);
                match (av.is_null(), bv.is_null()) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    (false, false) => {
                        let ord = Self::hql_cmp_order(&av, &bv);
                        if *desc {
                            ord.reverse()
                        } else {
                            ord
                        }
                    }
                }
            });
        }

        // 3. LIMIT (after filter + order)
        if let Some(n) = clauses.limit {
            results.truncate(n);
        }

        // 4. RETURN projection (None / `*` keep the full NeighborOutput shape)
        match &clauses.ret {
            None | Some(HqlReturn::All) => serde_json::to_value(&results).map_err(ser_err),
            Some(HqlReturn::Fields(fields)) => {
                let arr: Vec<serde_json::Value> = results
                    .iter()
                    .map(|n| {
                        let mut obj = serde_json::Map::new();
                        for f in fields {
                            obj.insert(f.output_key(), Self::hql_field_value(n, f));
                        }
                        serde_json::Value::Object(obj)
                    })
                    .collect();
                serde_json::to_value(arr).map_err(ser_err)
            }
        }
    }

    // --- Cypher pattern matching (ADR--GENESISDB-HQL-CYPHER-PATTERNS, path 1) ---

    /// Does a node satisfy a pattern node's `:Label` and `{k:v}` constraints?
    fn node_matches(
        &self,
        node_u32: u32,
        node: &NodeOutput,
        pat: &query::ast::NodePattern,
    ) -> bool {
        if let Some(label) = &pat.label {
            if !node.labels.iter().any(|l| l == label) {
                return false;
            }
        }
        let props = self.hydrated_props(node_u32, node);
        for (k, v) in &pat.props {
            // `{id:"..."}` addresses the node's top-level id (there is no `:id`
            // syntax and anchoring a pattern by a known id is the common case);
            // every other key is a lookup into `props`.
            if k == "id" {
                let ok = match v {
                    query::ast::HqlValue::Str(s) => &node.id == s,
                    query::ast::HqlValue::Num(n) => node.id == n.to_string(),
                };
                if !ok {
                    return false;
                }
                continue;
            }
            match props.get(k) {
                Some(av) => {
                    if !Self::json_eq_hqlvalue(av, v) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }

    /// Exact-equality of a JSON value against an inline `{k:v}` constraint value.
    fn json_eq_hqlvalue(actual: &serde_json::Value, expected: &query::ast::HqlValue) -> bool {
        match expected {
            query::ast::HqlValue::Num(n) => actual.as_f64().map(|a| a == *n).unwrap_or(false),
            query::ast::HqlValue::Str(s) => actual.as_str().map(|a| a == s).unwrap_or(false),
        }
    }

    /// Match a linear path pattern by deterministic left-to-right expansion over
    /// the graph indices, then apply the pattern's WHERE/ORDER BY/LIMIT/RETURN
    /// over the resulting variable-binding rows. No planner (ADR path 1).
    fn match_pattern(
        &self,
        pattern: &query::ast::GraphPattern,
        as_of: &Option<String>,
        clauses: &query::ast::PatternClauses,
    ) -> Result<serde_json::Value> {
        use query::ast::PatternDirection;
        let ser_err = |e: serde_json::Error| {
            Error::from_reason(format!("HQL result serialization failed: {e}"))
        };
        type Row = serde_json::Map<String, serde_json::Value>;

        let now = Utc::now().to_rfc3339();
        // 1. Anchor: if the anchor has an exact `id` constraint, skip the full
        // scan and seed directly from the interned id; otherwise fall back to
        // the existing live-node scan.
        let mut frontier: Vec<(u32, Row)> = Vec::new();
        let anchor_id =
            pattern
                .start
                .props
                .iter()
                .find_map(|(key, value)| match (key.as_str(), value) {
                    ("id", query::ast::HqlValue::Str(id)) => Some(id.clone()),
                    _ => None,
                });
        if let Some(anchor_id) = anchor_id {
            if let Some(anchor_u32) = self.get_u32(&anchor_id) {
                if let Some(entry) = self.nodes.get(&anchor_u32) {
                    let node = entry.value();
                    if Self::is_valid_as_of(&node.valid_from, &node.valid_to, as_of)
                        && self.node_matches(anchor_u32, node, &pattern.start)
                    {
                        let node = self.hydrated_node(anchor_u32, node);
                        let mut row = Row::new();
                        if let Some(v) = &pattern.start.var {
                            row.insert(v.clone(), serde_json::to_value(node).map_err(ser_err)?);
                        }
                        frontier.push((anchor_u32, row));
                    }
                }
            }
        } else {
            for entry in self.nodes.iter() {
                let node = entry.value();
                if !Self::is_valid_as_of(&node.valid_from, &node.valid_to, as_of) {
                    continue;
                }
                if !self.node_matches(*entry.key(), node, &pattern.start) {
                    continue;
                }
                let node = self.hydrated_node(*entry.key(), node);
                let mut row = Row::new();
                if let Some(v) = &pattern.start.var {
                    row.insert(v.clone(), serde_json::to_value(node).map_err(ser_err)?);
                }
                frontier.push((*entry.key(), row));
            }
        }

        // 2. Expand each (edge, node) hop; Cartesian over surviving neighbours.
        for (edge_pat, node_pat) in &pattern.hops {
            let (walk_out, walk_in) = match edge_pat.direction {
                PatternDirection::Out => (true, false),
                PatternDirection::In => (false, true),
                PatternDirection::Both => (true, true),
            };
            let mut next: Vec<(u32, Row)> = Vec::new();
            for (curr, row) in &frontier {
                let mut eids: HashSet<u128> = HashSet::new();
                if walk_out {
                    if let Some(s) = self.out_idx.get(curr) {
                        eids.extend(s.iter().copied());
                    }
                }
                if walk_in {
                    if let Some(s) = self.in_idx.get(curr) {
                        eids.extend(s.iter().copied());
                    }
                }
                for eid in &eids {
                    let edge_ref = match self.edges.get(eid) {
                        Some(e) => e,
                        None => continue,
                    };
                    let edge = edge_ref.value();
                    if !Self::is_valid_as_of(&edge.valid_from, &edge.valid_to, as_of) {
                        continue;
                    }
                    // Hide retracted edges in the current view (mirror `neighbors`).
                    if as_of.is_none() {
                        if let Some(to) = &edge.valid_to {
                            if now.as_str() >= to.as_str() {
                                continue;
                            }
                        }
                    }
                    if let Some(rt) = &edge_pat.rel_type {
                        if &edge.rel != rt {
                            continue;
                        }
                    }
                    // Far endpoint = the one that is not the current node.
                    let far_id = if self.get_u32(&edge.from) == Some(*curr) {
                        &edge.to
                    } else {
                        &edge.from
                    };
                    let far_u32 = match self.get_u32(far_id) {
                        Some(x) => x,
                        None => continue,
                    };
                    let far_ref = match self.nodes.get(&far_u32) {
                        Some(n) => n,
                        None => continue,
                    };
                    let far_node = far_ref.value();
                    if !Self::is_valid_as_of(&far_node.valid_from, &far_node.valid_to, as_of) {
                        continue;
                    }
                    if !self.node_matches(far_u32, far_node, node_pat) {
                        continue;
                    }
                    let far_node = self.hydrated_node(far_u32, far_node);
                    let mut nb = row.clone();
                    if let Some(v) = &edge_pat.var {
                        nb.insert(v.clone(), serde_json::to_value(edge).map_err(ser_err)?);
                    }
                    if let Some(v) = &node_pat.var {
                        nb.insert(v.clone(), serde_json::to_value(far_node).map_err(ser_err)?);
                    }
                    next.push((far_u32, nb));
                }
            }
            frontier = next;
        }

        let mut rows: Vec<Row> = frontier.into_iter().map(|(_, b)| b).collect();

        // 3. WHERE (conjunction), 4. ORDER BY (nulls last), 5. LIMIT.
        if !clauses.where_preds.is_empty() {
            rows.retain(|b| {
                clauses
                    .where_preds
                    .iter()
                    .all(|p| Self::pattern_eval_predicate(b, p))
            });
        }
        if let Some((qf, desc)) = &clauses.order_by {
            rows.sort_by(|a, b| {
                let av = Self::pattern_resolve(a, qf);
                let bv = Self::pattern_resolve(b, qf);
                match (av.is_null(), bv.is_null()) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    (false, false) => {
                        let ord = Self::hql_cmp_order(&av, &bv);
                        if *desc {
                            ord.reverse()
                        } else {
                            ord
                        }
                    }
                }
            });
        }
        if let Some(n) = clauses.limit {
            rows.truncate(n);
        }

        // 6. RETURN projection.
        use query::ast::PatternReturn;
        match &clauses.ret {
            None | Some(PatternReturn::All) => serde_json::to_value(rows).map_err(ser_err),
            Some(PatternReturn::Fields(fields)) => {
                let arr: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|b| {
                        let mut obj = serde_json::Map::new();
                        for f in fields {
                            obj.insert(f.output_key(), Self::pattern_resolve(b, f));
                        }
                        serde_json::Value::Object(obj)
                    })
                    .collect();
                serde_json::to_value(arr).map_err(ser_err)
            }
        }
    }

    /// Resolve a `var` / `var.field` against a binding row into a JSON value.
    fn pattern_resolve(
        binding: &serde_json::Map<String, serde_json::Value>,
        qf: &query::ast::QualField,
    ) -> serde_json::Value {
        use query::ast::HqlField;
        let entity = match binding.get(&qf.var) {
            Some(e) => e,
            None => return serde_json::Value::Null,
        };
        match &qf.field {
            None => entity.clone(),
            Some(HqlField::Id) => entity.get("id").cloned().unwrap_or(serde_json::Value::Null),
            Some(HqlField::Label) => {
                // node → labels array; edge → rel string.
                if let Some(labels) = entity.get("labels") {
                    labels.clone()
                } else if let Some(rel) = entity.get("rel") {
                    rel.clone()
                } else {
                    serde_json::Value::Null
                }
            }
            Some(HqlField::Prop(k)) => entity
                .get("props")
                .and_then(|p| p.get(k))
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            Some(HqlField::Score) | Some(HqlField::Depth) => serde_json::Value::Null,
        }
    }

    fn pattern_eval_predicate(
        binding: &serde_json::Map<String, serde_json::Value>,
        pred: &query::ast::PatternPredicate,
    ) -> bool {
        use query::ast::{HqlField, HqlOp, HqlValue};
        // Node `.label` uses membership semantics (labels array contains value);
        // an edge `.label` (rel string) falls through to normal string compare.
        if let Some(HqlField::Label) = &pred.field.field {
            if let Some(entity) = binding.get(&pred.field.var) {
                if let Some(serde_json::Value::Array(labels)) = entity.get("labels") {
                    let target = match &pred.value {
                        HqlValue::Str(s) => s.clone(),
                        HqlValue::Num(n) => n.to_string(),
                    };
                    let contains = labels.iter().any(|l| l.as_str() == Some(target.as_str()));
                    return match pred.op {
                        HqlOp::Eq => contains,
                        HqlOp::Ne => !contains,
                        _ => false,
                    };
                }
            }
        }
        let actual = Self::pattern_resolve(binding, &pred.field);
        Self::json_cmp(&actual, pred.op, &pred.value)
    }

    /// Compare a resolved JSON value against a target with SQL-style null
    /// handling: null/missing/non-coercible ⇒ false for every operator. Numeric
    /// when both parse numerically, else string.
    fn json_cmp(
        actual: &serde_json::Value,
        op: query::ast::HqlOp,
        target: &query::ast::HqlValue,
    ) -> bool {
        use query::ast::{HqlOp, HqlValue};
        if actual.is_null() {
            return false;
        }
        let as_string = |v: &serde_json::Value| -> Option<String> {
            match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            }
        };
        match op {
            HqlOp::Contains | HqlOp::StartsWith => {
                let s = match as_string(actual) {
                    Some(s) => s,
                    None => return false,
                };
                let t = match target {
                    HqlValue::Str(t) => t.clone(),
                    HqlValue::Num(n) => n.to_string(),
                };
                match op {
                    HqlOp::Contains => s.contains(&t),
                    _ => s.starts_with(&t),
                }
            }
            _ => {
                let t_num = match target {
                    HqlValue::Num(n) => Some(*n),
                    HqlValue::Str(_) => None,
                };
                let ord = if let (Some(a), Some(t)) = (actual.as_f64(), t_num) {
                    a.partial_cmp(&t)
                } else {
                    let a = match as_string(actual) {
                        Some(a) => a,
                        None => return false,
                    };
                    let t = match target {
                        HqlValue::Str(t) => t.clone(),
                        HqlValue::Num(n) => n.to_string(),
                    };
                    Some(a.cmp(&t))
                };
                let ord = match ord {
                    Some(o) => o,
                    None => return false,
                };
                use std::cmp::Ordering::*;
                match op {
                    HqlOp::Eq => ord == Equal,
                    HqlOp::Ne => ord != Equal,
                    HqlOp::Lt => ord == Less,
                    HqlOp::Le => ord != Greater,
                    HqlOp::Gt => ord == Greater,
                    HqlOp::Ge => ord != Less,
                    _ => false,
                }
            }
        }
    }

    pub fn query_ir_capabilities(&self) -> serde_json::Value {
        serde_json::json!({
            "contract_version": QUERY_IR_V1,
            "implementation_status": "partial",
            "operations": {
                "search": "implemented",
                "traverse": "implemented",
                "match_path": "planned",
                "context": "planned",
                "relational_named_query": "planned"
            },
            "limits": {
                "max_k": QUERY_IR_MAX_K,
                "max_depth": QUERY_IR_MAX_DEPTH
            }
        })
    }

    pub fn execute_query_ir_json(
        &self,
        request_json: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let request = serde_json::from_value::<QueryIrRequest>(request_json)
            .map_err(|error| Error::from_reason(format!("QUERY_IR_VALIDATION_FAILED: {error}")))?;
        self.execute_query_ir(request)
    }

    pub fn execute_query_ir(&self, request: QueryIrRequest) -> Result<serde_json::Value> {
        if request.contract_version != QUERY_IR_V1 {
            return Err(Error::from_reason(format!(
                "QUERY_IR_VERSION_UNSUPPORTED: expected '{QUERY_IR_V1}', got '{}'",
                request.contract_version
            )));
        }
        if request.request_id.trim().is_empty() {
            return Err(Error::from_reason(
                "QUERY_IR_VALIDATION_FAILED: request_id must not be empty",
            ));
        }
        if request.namespace.is_some() {
            return Err(Error::from_reason(
                "QUERY_CAPABILITY_UNSUPPORTED: namespace-scoped Query IR is not implemented",
            ));
        }
        if matches!(
            request.consistency.as_ref().map(|value| value.index),
            Some(QueryIrIndexConsistency::ReadYourWrite)
        ) {
            self.flush_index();
        }

        let as_of = request
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.valid_at.clone());
        let (operation_kind, data) = match request.operation {
            QueryIrOperation::Search {
                mode,
                target_id,
                query_vector,
                collection,
                k,
                alpha,
                language,
                ef_search,
                oversample,
            } => {
                if k == 0 || k > QUERY_IR_MAX_K {
                    return Err(Error::from_reason(format!(
                        "QUERY_RESOURCE_LIMIT_EXCEEDED: search.k must be between 1 and {QUERY_IR_MAX_K}"
                    )));
                }
                if matches!(mode, QueryIrSearchMode::Lexical) {
                    return Err(Error::from_reason(
                        "QUERY_CAPABILITY_UNSUPPORTED: lexical Query IR search is planned",
                    ));
                }
                if let Some(value) = alpha {
                    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                        return Err(Error::from_reason(
                            "QUERY_IR_VALIDATION_FAILED: search.alpha must be between 0 and 1",
                        ));
                    }
                }
                if ef_search == Some(0) || oversample == Some(0) {
                    return Err(Error::from_reason(
                        "QUERY_IR_VALIDATION_FAILED: ef_search and oversample must be positive",
                    ));
                }

                let (query_vector, resolved_collection) =
                    self.query_ir_search_vector(target_id, query_vector, collection)?;
                let results = self
                    .hybrid_search(HybridSearchInput {
                        query_vector,
                        k,
                        alpha: Some(match mode {
                            QueryIrSearchMode::Vector => 0.0,
                            QueryIrSearchMode::Hybrid => alpha.unwrap_or(0.5),
                            QueryIrSearchMode::Lexical => unreachable!(),
                        }),
                        lang: language,
                        as_of,
                        collection: resolved_collection,
                        ef_search,
                        oversample,
                    })
                    .map_err(|error| {
                        Error::from_reason(format!("QUERY_EXECUTION_FAILED: {error}"))
                    })?;
                let data = serde_json::to_value(results).map_err(|error| {
                    Error::from_reason(format!("QUERY_EXECUTION_FAILED: {error}"))
                })?;
                ("search", data)
            }
            QueryIrOperation::Traverse {
                seed_id,
                depth,
                relations,
                direction,
                limit,
            } => {
                if seed_id.trim().is_empty() {
                    return Err(Error::from_reason(
                        "QUERY_IR_VALIDATION_FAILED: traverse.seed_id must not be empty",
                    ));
                }
                if depth == 0 || depth > QUERY_IR_MAX_DEPTH {
                    return Err(Error::from_reason(format!(
                        "QUERY_RESOURCE_LIMIT_EXCEEDED: traverse.depth must be between 1 and {QUERY_IR_MAX_DEPTH}"
                    )));
                }
                if relations.is_empty() || relations.iter().any(|rel| rel.trim().is_empty()) {
                    return Err(Error::from_reason(
                        "QUERY_IR_VALIDATION_FAILED: traverse.relations must contain non-empty relation names",
                    ));
                }
                let results = self
                    .neighbors(
                        seed_id,
                        NeighborInput {
                            depth: Some(depth),
                            rel: None,
                            rels: Some(relations),
                            direction: Some(direction.as_str().to_string()),
                            as_of,
                            include_invalid: Some(false),
                            limit,
                        },
                        false,
                    )
                    .map_err(|error| {
                        Error::from_reason(format!("QUERY_EXECUTION_FAILED: {error}"))
                    })?;
                let data = serde_json::to_value(results).map_err(|error| {
                    Error::from_reason(format!("QUERY_EXECUTION_FAILED: {error}"))
                })?;
                ("traverse", data)
            }
        };

        Ok(serde_json::json!({
            "contract_version": QUERY_IR_V1,
            "request_id": request.request_id,
            "status": "ok",
            "operation_kind": operation_kind,
            "data": data,
            "meta": {
                "capability_version": ENGINE_VERSION,
                "index_lag": self.index_lag(),
                "warnings": []
            }
        }))
    }

    fn query_ir_search_vector(
        &self,
        target_id: Option<String>,
        query_vector: Option<Vec<f64>>,
        collection: Option<String>,
    ) -> Result<(Vec<f64>, Option<String>)> {
        match (target_id, query_vector) {
            (Some(_), Some(_)) | (None, None) => Err(Error::from_reason(
                "QUERY_IR_VALIDATION_FAILED: search requires exactly one of target_id or query_vector",
            )),
            (None, Some(vector)) => {
                if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
                    return Err(Error::from_reason(
                        "QUERY_IR_VALIDATION_FAILED: query_vector must contain finite values",
                    ));
                }
                Ok((vector, collection))
            }
            (Some(target_id), None) => {
                let node_u32 = self.get_u32(&target_id).ok_or_else(|| {
                    Error::from_reason(format!(
                        "QUERY_TARGET_NOT_FOUND: node '{target_id}' does not exist"
                    ))
                })?;
                let node = self.nodes.get(&node_u32).ok_or_else(|| {
                    Error::from_reason(format!(
                        "QUERY_TARGET_NOT_FOUND: node '{target_id}' is not live"
                    ))
                })?;
                let node_collection = node
                    .collection
                    .clone()
                    .unwrap_or_else(|| self.default_collection.clone());
                if let Some(requested) = collection.as_ref() {
                    if requested != &node_collection {
                        return Err(Error::from_reason(format!(
                            "QUERY_COLLECTION_MISMATCH: node '{target_id}' belongs to '{node_collection}', requested '{requested}'"
                        )));
                    }
                }
                let vector = self
                    .reconstruct_embedding(node.value(), node_u32)
                    .ok_or_else(|| {
                        Error::from_reason(format!(
                            "QUERY_TARGET_NOT_FOUND: node '{target_id}' has no stored embedding"
                        ))
                    })?;
                Ok((vector, node.collection.clone()))
            }
        }
    }

    pub fn execute_hql(&self, query: &str) -> Result<serde_json::Value> {
        fn to_value<T: serde::Serialize>(res: T) -> Result<serde_json::Value> {
            serde_json::to_value(res)
                .map_err(|e| Error::from_reason(format!("HQL result serialization failed: {e}")))
        }
        fn resolved_target_id(storage: &Storage, target: &str, fuzzy: bool) -> Result<String> {
            if fuzzy {
                storage.find_fuzzy_id(target).ok_or_else(|| {
                    Error::from_reason(format!(
                        "HQL: target '{target}' does not resolve to a node and no vector was given"
                    ))
                })
            } else {
                Ok(target.to_string())
            }
        }
        fn hql_query_vector(
            storage: &Storage,
            target: &str,
            fuzzy: bool,
            vector: Option<Vec<f64>>,
            requested_collection: &Option<String>,
        ) -> Result<(Vec<f64>, Option<String>)> {
            if let Some(vector) = vector {
                return Ok((vector, requested_collection.clone()));
            }
            let resolved = resolved_target_id(storage, target, fuzzy)?;
            let node_u32 = storage.get_u32(&resolved).ok_or_else(|| {
                Error::from_reason(format!(
                    "HQL: target '{target}' does not resolve to a node and no vector was given"
                ))
            })?;
            let node = storage.nodes.get(&node_u32).ok_or_else(|| {
                Error::from_reason(format!(
                    "HQL: target '{resolved}' does not resolve to a live node and no vector was given"
                ))
            })?;
            let node_collection = node
                .collection
                .clone()
                .unwrap_or_else(|| storage.default_collection.clone());
            if let Some(requested) = requested_collection {
                if requested != &node_collection {
                    return Err(Error::from_reason(format!(
                        "HQL: node '{resolved}' lives in collection '{node_collection}' but IN '{requested}' was given; omit IN or match the node's collection"
                    )));
                }
            }
            let query_vector = storage
                .reconstruct_embedding(node.value(), node_u32)
                .ok_or_else(|| {
                    Error::from_reason(format!(
                        "HQL: target '{resolved}' has no stored embedding and no vector was given"
                    ))
                })?;
            Ok((query_vector, node.collection.clone()))
        }
        fn query_ir_neighbors(
            storage: &Storage,
            request: QueryIrRequest,
        ) -> Result<Vec<NeighborOutput>> {
            let response = storage.execute_query_ir(request)?;
            serde_json::from_value(response["data"].clone()).map_err(|error| {
                Error::from_reason(format!("HQL compatibility decode failed: {error}"))
            })
        }
        let command = HqlCommand::try_from(query).map_err(Error::from_reason)?;
        match command {
            HqlCommand::Search {
                vector,
                k,
                ef_search,
                oversample,
                fuzzy,
                target,
                lang,
                as_of,
                collection,
                clauses,
            } => {
                let (query_vector, resolved_collection) =
                    hql_query_vector(self, &target, fuzzy, vector, &collection)?;
                let res = if (1..=QUERY_IR_MAX_K).contains(&k) {
                    query_ir_neighbors(
                        self,
                        QueryIrRequest {
                            contract_version: QUERY_IR_V1.to_string(),
                            request_id: "hql-compat".to_string(),
                            namespace: None,
                            temporal: Some(QueryIrTemporal { valid_at: as_of }),
                            consistency: None,
                            operation: QueryIrOperation::Search {
                                mode: QueryIrSearchMode::Vector,
                                target_id: None,
                                query_vector: Some(query_vector),
                                collection: resolved_collection,
                                k,
                                alpha: Some(0.0),
                                language: lang,
                                ef_search,
                                oversample,
                            },
                        },
                    )?
                } else {
                    self.hybrid_search(HybridSearchInput {
                        query_vector,
                        k,
                        alpha: Some(0.0),
                        lang,
                        as_of,
                        collection: resolved_collection,
                        ef_search,
                        oversample,
                    })?
                };
                Self::apply_hql_clauses(res, &clauses)
            }
            HqlCommand::Traverse {
                seed,
                depth,
                rel,
                rels,
                direction,
                fuzzy,
                as_of,
                clauses,
            } => {
                let resolved_seed = if fuzzy {
                    self.find_fuzzy_id(&seed).unwrap_or(seed)
                } else {
                    seed
                };
                let (target_rel, is_inferred) = match rel {
                    query::ast::HqlRel::Physical(r) => (r, false),
                    query::ast::HqlRel::Inferred(r) => (r, true),
                };
                let direction = direction.unwrap_or_else(|| "out".to_string());
                let res = if is_inferred
                    || target_rel == "ANY"
                    || depth == 0
                    || depth > QUERY_IR_MAX_DEPTH
                {
                    self.neighbors(
                        resolved_seed,
                        NeighborInput {
                            depth: Some(depth),
                            rel: Some(target_rel),
                            rels,
                            direction: Some(direction),
                            as_of,
                            include_invalid: Some(false),
                            limit: None,
                        },
                        is_inferred,
                    )?
                } else {
                    let relations = rels.unwrap_or_else(|| vec![target_rel]);
                    let direction = match direction.as_str() {
                        "in" => QueryIrDirection::In,
                        "both" => QueryIrDirection::Both,
                        _ => QueryIrDirection::Out,
                    };
                    query_ir_neighbors(
                        self,
                        QueryIrRequest {
                            contract_version: QUERY_IR_V1.to_string(),
                            request_id: "hql-compat".to_string(),
                            namespace: None,
                            temporal: Some(QueryIrTemporal { valid_at: as_of }),
                            consistency: None,
                            operation: QueryIrOperation::Traverse {
                                seed_id: resolved_seed,
                                depth,
                                relations,
                                direction,
                                limit: None,
                            },
                        },
                    )?
                };
                Self::apply_hql_clauses(res, &clauses)
            }
            HqlCommand::Hybrid {
                vector,
                alpha,
                k,
                ef_search,
                oversample,
                fuzzy,
                target,
                lang,
                as_of,
                collection,
                clauses,
            } => {
                let (query_vector, resolved_collection) =
                    hql_query_vector(self, &target, fuzzy, vector, &collection)?;
                let res = if (1..=QUERY_IR_MAX_K).contains(&k) {
                    query_ir_neighbors(
                        self,
                        QueryIrRequest {
                            contract_version: QUERY_IR_V1.to_string(),
                            request_id: "hql-compat".to_string(),
                            namespace: None,
                            temporal: Some(QueryIrTemporal { valid_at: as_of }),
                            consistency: None,
                            operation: QueryIrOperation::Search {
                                mode: QueryIrSearchMode::Hybrid,
                                target_id: None,
                                query_vector: Some(query_vector),
                                collection: resolved_collection,
                                k,
                                alpha: Some(alpha),
                                language: lang,
                                ef_search,
                                oversample,
                            },
                        },
                    )?
                } else {
                    self.hybrid_search(HybridSearchInput {
                        query_vector,
                        k,
                        alpha: Some(alpha),
                        lang,
                        as_of,
                        collection: resolved_collection,
                        ef_search,
                        oversample,
                    })?
                };
                Self::apply_hql_clauses(res, &clauses)
            }
            HqlCommand::Context {
                target,
                tier,
                budget,
                fuzzy,
            } => {
                let res = self.retrieve_context(&target, &tier, budget, fuzzy)?;
                to_value(res)
            }
            HqlCommand::MatchPattern {
                pattern,
                as_of,
                clauses,
            } => self.match_pattern(&pattern, &as_of, &clauses),
        }
    }

    pub fn execute_hql_read_only(&self, query: &str) -> Result<serde_json::Value> {
        let command = HqlCommand::try_from(query).map_err(Error::from_reason)?;
        match command {
            HqlCommand::Search { .. }
            | HqlCommand::Traverse { .. }
            | HqlCommand::Hybrid { .. }
            | HqlCommand::Context { .. }
            | HqlCommand::MatchPattern { .. } => self.execute_hql(query),
        }
    }

    fn studio_graph_node(&self, node_u32: u32, node: &NodeOutput) -> StudioGraphNode {
        let hydrated = self.hydrated_node(node_u32, node);
        let label = hydrated
            .props
            .get("title")
            .or_else(|| hydrated.props.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(&hydrated.id)
            .to_string();
        StudioGraphNode {
            id: hydrated.id,
            label,
            labels: hydrated.labels,
            collection: hydrated.collection,
            valid_from: hydrated.valid_from,
            valid_to: hydrated.valid_to,
            caused_by: hydrated.caused_by,
            impact: hydrated.impact,
        }
    }

    fn studio_graph_edge(edge: &EdgeOutput) -> StudioGraphEdge {
        StudioGraphEdge {
            id: edge.id.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            relation: edge.rel.clone(),
            valid_from: edge.valid_from.clone(),
            valid_to: edge.valid_to.clone(),
            caused_by: edge.caused_by.clone(),
        }
    }

    pub fn studio_capabilities(
        &self,
        mode: &str,
        auth_features: Vec<String>,
    ) -> StudioCapabilities {
        StudioCapabilities {
            protocol_version: STUDIO_PROTOCOL_VERSION.to_string(),
            engine_version: ENGINE_VERSION.to_string(),
            mode: mode.to_string(),
            read_features: vec![
                "status.read".to_string(),
                "relational.read".to_string(),
                "graph.scene.read".to_string(),
                "graph.scene.expand".to_string(),
                "vector.read".to_string(),
                "hql.read".to_string(),
                "entity.inspect".to_string(),
            ],
            write_features: Vec::new(),
            auth_features,
            limits: StudioLimits {
                initial_scene_nodes: STUDIO_SCENE_PAGE_LIMIT,
                scene_node_ceiling: STUDIO_SCENE_NODE_CEILING,
                scene_edge_ceiling: STUDIO_SCENE_EDGE_CEILING,
                expansion_nodes: 100,
            },
            consistency: "stable-frontier".to_string(),
        }
    }

    pub fn studio_graph_scene(&self, request: StudioGraphSceneRequest) -> Result<StudioGraphScene> {
        let limit = request.limit.unwrap_or(240);
        if limit == 0 || limit > STUDIO_SCENE_PAGE_LIMIT {
            return Err(Error::from_reason(format!(
                "STUDIO_SCENE_LIMIT_EXCEEDED: limit must be 1..{}",
                STUDIO_SCENE_PAGE_LIMIT
            )));
        }
        let offset = request.offset.unwrap_or(0);
        if offset >= STUDIO_SCENE_NODE_CEILING {
            return Err(Error::from_reason(format!(
                "STUDIO_SCENE_LIMIT_EXCEEDED: offset must be below {}",
                STUDIO_SCENE_NODE_CEILING
            )));
        }
        let direction = request
            .direction
            .clone()
            .unwrap_or_else(|| "both".to_string())
            .to_ascii_lowercase();
        if !matches!(direction.as_str(), "out" | "in" | "both") {
            return Err(Error::from_reason(
                "STUDIO_SCENE_INVALID_DIRECTION: expected out, in, or both",
            ));
        }
        let now = Utc::now().to_rfc3339();
        let mut candidates: Vec<(u32, NodeOutput)> = if let Some(seed) = request.seed.as_ref() {
            let seed_u32 = self
                .get_u32(seed)
                .ok_or_else(|| Error::from_reason("STUDIO_ENTITY_NOT_FOUND"))?;
            let seed_node = self
                .nodes
                .get(&seed_u32)
                .map(|node| self.hydrated_node(seed_u32, node.value()))
                .ok_or_else(|| Error::from_reason("STUDIO_ENTITY_NOT_FOUND"))?;
            if !Self::is_currently_visible(
                &seed_node.valid_from,
                &seed_node.valid_to,
                &request.as_of,
                false,
                &now,
            ) {
                return Err(Error::from_reason("STUDIO_ENTITY_NOT_FOUND_AT_VIEW"));
            }
            let mut selected = vec![(seed_u32, seed_node)];
            selected.extend(
                self.neighbors(
                    seed.clone(),
                    NeighborInput {
                        depth: Some(1),
                        rel: None,
                        rels: None,
                        direction: Some(direction),
                        as_of: request.as_of.clone(),
                        include_invalid: Some(false),
                        limit: Some(STUDIO_SCENE_NODE_CEILING as u32),
                    },
                    false,
                )?
                .into_iter()
                .filter_map(|result| {
                    self.get_u32(&result.node.id)
                        .map(|node_u32| (node_u32, result.node))
                }),
            );
            selected
        } else {
            self.nodes
                .iter()
                .filter(|entry| {
                    Self::is_currently_visible(
                        &entry.valid_from,
                        &entry.valid_to,
                        &request.as_of,
                        false,
                        &now,
                    )
                })
                .map(|entry| {
                    (
                        *entry.key(),
                        self.hydrated_node(*entry.key(), entry.value()),
                    )
                })
                .collect()
        };
        candidates.sort_by(|left, right| left.1.id.cmp(&right.1.id));
        candidates.dedup_by(|left, right| left.1.id == right.1.id);
        let candidate_count = candidates.len().min(STUDIO_SCENE_NODE_CEILING);
        let page: Vec<(u32, NodeOutput)> = candidates
            .into_iter()
            .take(STUDIO_SCENE_NODE_CEILING)
            .skip(offset)
            .take(limit)
            .collect();
        let selected_ids: HashSet<String> = page.iter().map(|(_, node)| node.id.clone()).collect();
        let nodes: Vec<StudioGraphNode> = page
            .iter()
            .map(|(node_u32, node)| self.studio_graph_node(*node_u32, node))
            .collect();
        let mut all_edges: Vec<StudioGraphEdge> = self
            .edges
            .iter()
            .filter(|entry| {
                let edge = entry.value();
                selected_ids.contains(&edge.from)
                    && selected_ids.contains(&edge.to)
                    && Self::is_currently_visible(
                        &edge.valid_from,
                        &edge.valid_to,
                        &request.as_of,
                        false,
                        &now,
                    )
            })
            .map(|entry| Self::studio_graph_edge(entry.value()))
            .collect();
        all_edges.sort_by(|left, right| left.id.cmp(&right.id));
        let edges_truncated = all_edges.len() > STUDIO_SCENE_EDGE_CEILING;
        all_edges.truncate(STUDIO_SCENE_EDGE_CEILING);
        let next_offset = offset.saturating_add(nodes.len());
        let nodes_truncated = next_offset < candidate_count;
        let mut groups: Vec<String> = nodes
            .iter()
            .flat_map(|node| node.labels.iter().cloned())
            .collect();
        groups.sort();
        groups.dedup();
        let mut warnings = Vec::new();
        if edges_truncated {
            warnings.push("STUDIO_SCENE_EDGE_CEILING_REACHED".to_string());
        }
        Ok(StudioGraphScene {
            scene_id: Uuid::new_v4().to_string(),
            frontier: self.stable_frontier(),
            nodes,
            edges: all_edges,
            groups,
            truncated: nodes_truncated || edges_truncated,
            continuation: nodes_truncated.then(|| next_offset.to_string()),
            warnings,
        })
    }

    pub fn studio_inspect_entity(&self, entity_id: &str) -> Result<StudioEntityInspection> {
        let node_u32 = self
            .get_u32(entity_id)
            .ok_or_else(|| Error::from_reason("STUDIO_ENTITY_NOT_FOUND"))?;
        let hydrated = self
            .node_view_u32(node_u32)
            .ok_or_else(|| Error::from_reason("STUDIO_ENTITY_NOT_FOUND"))?;
        let now = Utc::now().to_rfc3339();
        let mut incident_edges: Vec<StudioGraphEdge> = self
            .edges
            .iter()
            .filter(|entry| {
                let edge = entry.value();
                (edge.from == entity_id || edge.to == entity_id)
                    && Self::is_currently_visible(
                        &edge.valid_from,
                        &edge.valid_to,
                        &None,
                        false,
                        &now,
                    )
                    && self.endpoint_currently_visible(&edge.from, &None)
                    && self.endpoint_currently_visible(&edge.to, &None)
            })
            .map(|entry| Self::studio_graph_edge(entry.value()))
            .collect();
        incident_edges.sort_by(|left, right| left.id.cmp(&right.id));
        incident_edges.truncate(1000);
        let collection_name = hydrated
            .collection
            .clone()
            .unwrap_or_else(|| self.default_collection.clone());
        let vector_present = self
            .collections
            .get(&collection_name)
            .map(|collection| {
                collection
                    .metadata
                    .read()
                    .iter()
                    .any(|metadata| metadata.node_u32 == node_u32)
            })
            .unwrap_or(false);
        let index_lag = self.index_lag();
        let mut availability = HashMap::new();
        availability.insert("relational".to_string(), "available".to_string());
        availability.insert("graph".to_string(), "available".to_string());
        availability.insert(
            "vector".to_string(),
            if vector_present && index_lag > 0 {
                "stale"
            } else if vector_present {
                "available"
            } else {
                "not_present"
            }
            .to_string(),
        );
        availability.insert("temporal".to_string(), "available".to_string());
        Ok(StudioEntityInspection {
            entity_id: entity_id.to_string(),
            frontier: self.stable_frontier(),
            node: self.studio_graph_node(node_u32, &hydrated),
            properties: hydrated.props,
            incident_edges,
            vector_collection: vector_present.then_some(collection_name),
            vector_present,
            index_lag,
            availability,
        })
    }

    fn is_valid_as_of(valid_from: &str, valid_to: &Option<String>, as_of: &Option<String>) -> bool {
        if let Some(as_of_str) = as_of {
            if valid_from > as_of_str.as_str() {
                return false;
            }
            if let Some(to) = valid_to {
                if as_of_str.as_str() >= to.as_str() {
                    return false;
                }
            }
        }
        true
    }

    /// Shared bitemporal current-view rule, used by every read path that walks
    /// live nodes/edges (`neighbors`, `query`): a node/edge is visible when
    /// `is_valid_as_of` bounds its window relative to `as_of` AND, when no
    /// `as_of` is given (the "now" / current view), it hasn't been retracted
    /// or superseded in the past. `is_valid_as_of` alone doesn't cover the
    /// no-`as_of` case: a `valid_to` set at some point in the past is still
    /// "valid" by that check when `as_of` is `None`, so the retraction check
    /// below is what actually hides it from the current view. `include_invalid`
    /// is the caller's opt-in escape hatch to see retracted/superseded items
    /// even in the current view (mirrors `neighbors`' `include_invalid`).
    /// `now` is the caller's RFC3339 clock reading, taken ONCE per query —
    /// this helper sits on per-edge hot loops, so it must not allocate a
    /// fresh timestamp per call.
    fn is_currently_visible(
        valid_from: &str,
        valid_to: &Option<String>,
        as_of: &Option<String>,
        include_invalid: bool,
        now: &str,
    ) -> bool {
        if !Self::is_valid_as_of(valid_from, valid_to, as_of) {
            return false;
        }
        if as_of.is_none() && !include_invalid {
            if let Some(to) = valid_to {
                if now >= to.as_str() {
                    return false;
                }
            }
        }
        true
    }

    pub fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let coll = self.resolve_collection(&args.collection)?;
        // Dim validation closes the silent cross-space bug: a query from a
        // different model/dim is rejected, not ranked into garbage.
        if args.query_vector.len() != coll.dim as usize {
            return Err(Error::from_reason(format!(
                "query dim {} != collection '{}' dim {}",
                args.query_vector.len(),
                coll.name,
                coll.dim
            )));
        }
        let mut query_f32: Vec<f32> = args.query_vector.into_iter().map(|v| v as f32).collect();
        if let Some(lang) = args.lang {
            if let Some(centroid) = self.lang_centroids.get(&lang) {
                for (i, val) in query_f32.iter_mut().enumerate() {
                    if i < centroid.len() {
                        *val += centroid[i];
                    }
                }
            }
        }
        // Cosine collections store normalized vectors; normalize the query too.
        if coll.metric == Metric::Cosine {
            let norm: f32 = query_f32.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in query_f32.iter_mut() {
                    *x /= norm;
                }
            }
        }
        // VecIndex quantizes the query per mode and returns (arena_id, distance_f32).
        // ef_search resolution: per-query override → per-collection default → engine-global.
        let ef = args
            .ef_search
            .map(|e| e as usize)
            .or_else(|| coll.ef_search.map(|e| e as usize))
            .unwrap_or_else(|| self.ef_search.load(Ordering::Relaxed));
        // Over-fetch more quantized candidates when a rerank sidecar is present, so
        // the exact re-score has a wider pool to recover recall from. The
        // multiplier is a per-query override → `RERANK_OVERFETCH` default; it only
        // matters on the sidecar branch (the `else k2` path is untouched).
        let overfetch = args
            .oversample
            .map(|o| o as usize)
            .unwrap_or(RERANK_OVERFETCH);
        let k2 = (args.k * 2) as usize;
        let fetch = if coll.f32_sidecar.is_some() {
            (args.k as usize).saturating_mul(overfetch).max(k2)
        } else {
            k2
        };
        // When a rerank sidecar is present and the over-fetch would already pull
        // ~every slot, skip the approximate HNSW prefilter and score the full
        // sidecar exactly. The HNSW prefilter can nondeterministically drop a
        // candidate whose quantized code ties with another (e.g. two same-sign
        // BQ vectors collapse to one code), which would deny rerank the true
        // nearest. Exact brute force in this regime makes rerank deterministic
        // and recall exact. Large collections (fetch << slot count — e.g. the
        // 1M recall benchmarks) keep the HNSW path unchanged.
        let exact_rerank_slots = coll.f32_sidecar.as_ref().and_then(|s| {
            let n = s.read().len_rows();
            (n > 0 && fetch >= n).then_some(n)
        });
        let mut results = if let Some(n) = exact_rerank_slots {
            (0..n).map(|i| (i, 0.0f32)).collect()
        } else {
            // BQ centering: pack the query with the SAME center the index used.
            // `search_f32` centers a local copy only — `query_f32` stays raw for
            // the exact-f32 rerank stage below. `None` (non-BQ / uncentered) ⇒
            // byte-identical to the pre-P1a query path.
            let center = coll.center_snapshot();
            // SQ8: pack the query with the collection's scale (calibrated or fixed),
            // matching the indexed codes. Non-SQ8 ⇒ SQ8_FIXED, ignored by the arm.
            let sq8 = coll.sq8_snapshot();
            let hits = {
                let hnsw_lock = coll.hnsw.read();
                match &*hnsw_lock {
                    Some(idx) => idx.search_f32(&query_f32, fetch, ef, center.as_deref(), sq8),
                    None => return Err(Error::from_reason("HNSW not init")),
                }
            };
            // Recall floor (RCA--HNSW-UNDER-RETURN-SMALL-GRAPH). On a small
            // graph hnsw_rs can return ONLY the entry point — measured 2/150 on
            // a 24-vector collection, always exactly 1 hit — when that entry
            // point's layer-0 neighbour list was pruned empty. Recall then
            // collapses (1 of 10 asked for) instead of degrading, and the
            // caller cannot tell. When the index demonstrably under-delivers
            // AND the collection is small enough to scan outright, rebuild the
            // candidates exactly.
            //
            // Both conditions are load-bearing: a short return on a large
            // collection can be legitimate, and scanning one would be ruinous.
            // On a healthy graph `hits.len() == fetch.min(slots)`, so this
            // costs one length read and a compare — the scan never runs.
            let slots = { coll.metadata.read().len() };
            if hits.len() < fetch.min(slots) && slots <= EXACT_SCAN_MAX_SLOTS {
                coll.exact_candidates(&query_f32, fetch)
            } else {
                hits
            }
        };
        // f32-sidecar rerank: replace each candidate's quantized distance with the
        // exact f32 distance, re-sort ascending, and keep the best k*2 for the
        // hybrid blend below. The arena_id (d_id) indexes the sidecar at d_id*dim.
        // A candidate the sidecar is missing keeps its quantized distance, so an
        // absent/truncated `fvec_<name>.bin` degrades to quantized-only search
        // rather than silently dropping every hit (it would otherwise return empty).
        if let Some(sidecar) = &coll.f32_sidecar {
            let sc = sidecar.read();
            // Positioned disk read per candidate (LRU-fronted so a small/hot
            // collection doesn't pay N cold reads/query). A missing/short row
            // (`row` -> None) keeps the candidate's quantized distance — never
            // drops it — so a truncated fvec degrades to quantized, not empty.
            results = results
                .into_iter()
                .map(|(d_id, qd)| match sc.row(d_id) {
                    Some(v) => (d_id, exact_l2(&query_f32, &v)),
                    None => (d_id, qd),
                })
                .collect();
            results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(k2);
        }
        let mut hybrid_results = Vec::new();
        let meta_arena = coll.metadata.read();
        // Default to pure vector similarity. K-Impact blending is opt-in via an
        // explicit `alpha > 0` — see ADR--GENESISDB-KIMPACT-AS-SIGNAL. (Was 0.5,
        // which silently mixed graph-authority into every query.)
        let alpha = args.alpha.unwrap_or(0.0);

        for (d_id, distance) in results {
            if let Some(meta) = meta_arena.get(d_id) {
                {
                    let u32_id = meta.node_u32; // A2: id interned in metadata
                    if let Some(node) = self.nodes.get(&u32_id) {
                        let node_out = self.hydrated_node(u32_id, node.value());

                        if !Self::is_valid_as_of(
                            &node_out.valid_from,
                            &node_out.valid_to,
                            &args.as_of,
                        ) {
                            continue;
                        }

                        let similarity = 1.0 - distance as f64;
                        let reasoning_score =
                            (similarity * (1.0 - alpha)) + (node_out.impact.unwrap_or(0.0) * alpha);
                        // Carry the ranking score in `score`; leave `node.impact`
                        // as the node's true graph-authority signal (don't clobber
                        // it) so the caller can fuse it itself.
                        hybrid_results.push(NeighborOutput {
                            node: node_out,
                            path: Vec::new(),
                            depth: 0,
                            score: Some(reasoning_score),
                        });
                    }
                }
            }
        }
        // NaN-safe: a poisoned (NaN) score must not panic the whole query.
        // Treat incomparable scores as equal so sorting degrades gracefully.
        hybrid_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // Dedupe by node id, keeping the highest-scoring hit. A node may hold more
        // than one arena/HNSW slot in a collection — e.g. after `add_vector`
        // supersedes a prior vector, the orphaned slot lingers until compaction —
        // so the raw HNSW result set can surface the same node twice. Sorted
        // descending by score, `retain` keeps the first (best) occurrence.
        let mut seen: HashSet<String> = HashSet::new();
        hybrid_results.retain(|n| seen.insert(n.node.id.clone()));
        hybrid_results.truncate(args.k as usize);
        Ok(hybrid_results)
    }

    pub fn get_ranked_context(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let mut context_args = args;
        context_args.alpha = Some(0.4);
        self.hybrid_search(context_args)
    }

    pub fn neighbors(
        &self,
        seed: String,
        args: NeighborInput,
        is_inferred: bool,
    ) -> Result<Vec<NeighborOutput>> {
        let u32_seed = match self.get_u32(&seed) {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };
        let depth = args.depth.unwrap_or(1);

        // Rel filter: args.rels (non-empty) overrides args.rel; "ANY" or None → no filter.
        let rels_filter: Option<HashSet<String>> = match args.rels.as_ref() {
            Some(v) if !v.is_empty() => Some(v.iter().cloned().collect()),
            _ => match args.rel.as_deref() {
                None | Some("ANY") => None,
                Some(r) => {
                    let mut s = HashSet::new();
                    s.insert(r.to_string());
                    Some(s)
                }
            },
        };
        let rel_allowed = |rel: &str| -> bool {
            match &rels_filter {
                None => true,
                Some(s) => s.contains(rel),
            }
        };

        // Direction: "out" (default, back-compat), "in", "both". Case-insensitive.
        let dir = args
            .direction
            .as_deref()
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_else(|| "out".to_string());
        let walk_out = dir == "out" || dir == "both";
        let walk_in = dir == "in" || dir == "both";

        let lim = args.limit.map(|l| l as usize);
        // Retraction visibility: by default a retracted edge (valid_to passed) is
        // hidden from the current view; `include_invalid = true` surfaces it.
        let include_invalid = args.include_invalid.unwrap_or(false);
        let now = Utc::now().to_rfc3339();
        let mut results = Vec::new();
        let mut visited = HashSet::new();
        visited.insert(u32_seed);
        let mut queue = VecDeque::new();
        queue.push_back((u32_seed, Vec::new(), 0));
        while let Some((curr_u32, path, curr_depth)) = queue.pop_front() {
            if curr_depth >= depth && !is_inferred {
                continue;
            }

            // Collect candidate edge ids from chosen directions, dedup by eid.
            let mut eid_set: HashSet<u128> = HashSet::new();
            if walk_out {
                if let Some(out_eids) = self.out_idx.get(&curr_u32) {
                    eid_set.extend(out_eids.iter().copied());
                }
            }
            if walk_in {
                if let Some(in_eids) = self.in_idx.get(&curr_u32) {
                    eid_set.extend(in_eids.iter().copied());
                }
            }

            for eid in eid_set.iter() {
                if let Some(edge_ref) = self.edges.get(eid) {
                    let edge = edge_ref.value();

                    // Bitemporal current-view check for Edges (time-travel bound +
                    // retraction hiding); see `is_currently_visible`.
                    if !Self::is_currently_visible(
                        &edge.valid_from,
                        &edge.valid_to,
                        &args.as_of,
                        include_invalid,
                        &now,
                    ) {
                        continue;
                    }
                    if !rel_allowed(&edge.rel) {
                        continue;
                    }

                    // Pick the far endpoint by u32 identity rather than by
                    // reverse-mapping curr_u32 back to its string: the near
                    // endpoint is whichever of from/to interns to curr_u32. This
                    // needs no u32->id reverse map and avoids a per-edge string
                    // clone (ADR--GENESISDB-NODE-ID-INTERNING, Layer A).
                    let next_id = if self.get_u32(&edge.from) == Some(curr_u32) {
                        &edge.to
                    } else {
                        &edge.from
                    };
                    if let Some(next_u32) = self.get_u32(next_id) {
                        if !visited.contains(&next_u32) {
                            visited.insert(next_u32);
                            if let Some(node_ref) = self.nodes.get(&next_u32) {
                                let node = node_ref.value();

                                // Time-travel check for Nodes
                                if !Self::is_valid_as_of(
                                    &node.valid_from,
                                    &node.valid_to,
                                    &args.as_of,
                                ) {
                                    continue;
                                }

                                let mut new_path = path.clone();
                                new_path.push(edge.clone());
                                results.push(NeighborOutput {
                                    node: self.hydrated_node(next_u32, node),
                                    path: new_path.clone(),
                                    depth: curr_depth + 1,
                                    score: None,
                                });
                                if let Some(l) = lim {
                                    if results.len() >= l {
                                        return Ok(results);
                                    }
                                }
                                if is_inferred || (curr_depth + 1 < depth) {
                                    queue.push_back((next_u32, new_path, curr_depth + 1));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    /// Bitemporal current-view semantics (mirrors `neighbors` / HQL
    /// `TRAVERSE ... AS OF`). By default (`as_of` and `include_invalid` both
    /// absent) this returns only edges that are valid *now*: a retracted edge
    /// (`retract_edge` set its `valid_to` in the past) is excluded, and so is
    /// an edge whose `from`/`to` node is currently out of its bitemporal
    /// window — e.g. a node that was `supersede_node`d, whose live version's
    /// `valid_from` has advanced past the requested view. This closes a
    /// correctness gap where the old raw scan returned every edge ever
    /// written, regardless of retraction/supersession.
    ///
    /// Two optional `QueryInput` fields change that, backward-compatibly:
    ///   - `as_of` (RFC3339 timestamp): evaluate visibility at that point in
    ///     time instead of "now" — same time-travel semantics used elsewhere
    ///     (HQL `AS OF`, `neighbors`'s `as_of`).
    ///   - `include_invalid: true`: escape hatch that restores the historical
    ///     raw-scan behavior, surfacing retracted/superseded items too.
    ///
    /// Absent both fields, behavior for data that was never retracted or
    /// superseded is unchanged from before this method enforced visibility.
    pub fn query(&self, args: QueryInput) -> Result<Vec<EdgeOutput>> {
        let include_invalid = args.include_invalid.unwrap_or(false);
        let now = Utc::now().to_rfc3339();
        let mut res = Vec::new();
        for r in self.edges.iter() {
            let e = r.value();
            if let Some(ref f) = args.from {
                if e.from != *f {
                    continue;
                }
            }
            if let Some(ref t) = args.to {
                if e.to != *t {
                    continue;
                }
            }
            if !Self::is_currently_visible(
                &e.valid_from,
                &e.valid_to,
                &args.as_of,
                include_invalid,
                &now,
            ) {
                continue;
            }
            // Endpoint nodes must also be in view: a node superseded after
            // `as_of` (or, in the current view, one whose live `valid_from`
            // is still ahead of "now" post-supersession) hides the edge too —
            // same rule `neighbors` applies to the far node on each hop.
            if !self.endpoint_currently_visible(&e.from, &args.as_of)
                || !self.endpoint_currently_visible(&e.to, &args.as_of)
            {
                continue;
            }
            res.push(e.clone());
        }
        Ok(res)
    }

    /// Is the node named `id` within its bitemporal window as of `as_of` (or
    /// "now" when `as_of` is `None`)? A dangling reference (no such node in
    /// the live index) is treated as visible — existence isn't `query`'s
    /// concern, only bitemporal validity of nodes that DO exist, matching the
    /// node time-travel check `neighbors` runs on the far endpoint of a hop.
    fn endpoint_currently_visible(&self, id: &str, as_of: &Option<String>) -> bool {
        match self.get_u32(id).and_then(|u32_id| self.nodes.get(&u32_id)) {
            Some(node_ref) => {
                let now = Utc::now().to_rfc3339();
                Self::is_currently_visible(
                    &node_ref.valid_from,
                    &node_ref.valid_to,
                    as_of,
                    false,
                    &now,
                )
            }
            None => true,
        }
    }

    pub fn detect_communities(&self) -> Result<()> {
        // Community detection runs over the default collection's vector space.
        let coll = self.default_coll();
        let mut meta_arena = coll.metadata.write();
        let mut new_clusters = Vec::with_capacity(meta_arena.len());
        for meta in meta_arena.iter() {
            let mut freq = HashMap::new();
            {
                let u32_id = meta.node_u32; // A2: id interned in metadata
                let out_eids = self
                    .out_idx
                    .get(&u32_id)
                    .map(|v| v.value().clone())
                    .unwrap_or_default();
                for eid in out_eids {
                    if let Some(edge) = self.edges.get(&eid) {
                        let other_id = if self.get_u32(&edge.from) == Some(meta.node_u32) {
                            &edge.to
                        } else {
                            &edge.from
                        };
                        if let Some(to_u32) = self.get_u32(other_id) {
                            if let Some(a_id) = coll.node_to_arena.get(&to_u32) {
                                if let Some(other_meta) = meta_arena.get(*a_id as usize) {
                                    *freq.entry(other_meta.cluster_id).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
                let in_eids = self
                    .in_idx
                    .get(&u32_id)
                    .map(|v| v.value().clone())
                    .unwrap_or_default();
                for eid in in_eids {
                    if let Some(edge) = self.edges.get(&eid) {
                        let other_id = if self.get_u32(&edge.from) == Some(meta.node_u32) {
                            &edge.to
                        } else {
                            &edge.from
                        };
                        if let Some(to_u32) = self.get_u32(other_id) {
                            if let Some(a_id) = coll.node_to_arena.get(&to_u32) {
                                if let Some(other_meta) = meta_arena.get(*a_id as usize) {
                                    *freq.entry(other_meta.cluster_id).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
            if let Some((&best_cluster, _)) = freq.iter().max_by_key(|&(_, count)| count) {
                new_clusters.push(best_cluster);
            } else {
                new_clusters.push(meta.cluster_id);
            }
        }
        for (i, meta) in meta_arena.iter_mut().enumerate() {
            meta.cluster_id = new_clusters[i];
        }
        Ok(())
    }

    pub fn cosine_similarity(v1: &[f64], v2: &[f64]) -> f64 {
        if v1.len() != v2.len() || v1.is_empty() {
            return 0.0;
        }
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;
        for i in 0..v1.len() {
            dot += v1[i] * v2[i];
            norm_a += v1[i].powi(2);
            norm_b += v2[i].powi(2);
        }
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }

    pub fn generate_meta_graph(&self) -> Result<()> {
        // Meta-graph is built over the default collection's vector space.
        let coll = self.default_coll();
        let dim = coll.dim as usize;
        let mut cluster_groups: HashMap<u32, Vec<u32>> = HashMap::new();
        let meta_arena = coll.metadata.read();
        for meta in meta_arena.iter() {
            cluster_groups
                .entry(meta.cluster_id)
                .or_default()
                .push(meta.node_u32);
        }
        let vec_arena = coll.arena.read();
        let now = Utc::now().to_rfc3339();

        for (c_id, members) in cluster_groups.iter() {
            let mut centroid = vec![0.0; dim];
            let mut total_impact = 0.0;
            let mut count = 0;
            for &u32_id in members {
                if let Some(node) = self.nodes.get(&u32_id) {
                    total_impact += node.value().impact.unwrap_or(0.0);
                    if let Some(a_id) = coll.node_to_arena.get(&u32_id) {
                        if let Some(meta) = meta_arena.get(*a_id as usize) {
                            let start = meta.embedding_offset as usize;
                            let len = meta.vector_dim as usize;
                            if start + len <= vec_arena.len() {
                                // f32_at dequantizes for SQ8 — heuristic centroid math
                                // tolerates the quantization noise.
                                for (i, val) in vec_arena.f32_at(start, len).iter().enumerate() {
                                    if i < dim {
                                        centroid[i] += *val as f64;
                                    }
                                }
                                count += 1;
                            }
                        }
                    }
                }
            }
            if count > 0 {
                for val in centroid.iter_mut() {
                    *val /= count as f64;
                }

                let mut drift = None;
                if let Some(history) = self.meta_history.get(c_id) {
                    if let Some(prev) = history.last() {
                        let sim = Self::cosine_similarity(&centroid, &prev.centroid);
                        drift = Some(1.0 - sim);
                    }
                }

                let sn = SuperNode {
                    cluster_id: *c_id,
                    theme: format!("Theme-{}", c_id),
                    member_count: members.len() as u32,
                    impact: total_impact / members.len() as f64,
                    centroid: centroid.clone(),
                    timestamp: now.clone(),
                    drift,
                };

                self.meta_nodes.insert(*c_id, sn.clone());
                self.meta_history.entry(*c_id).or_default().push(sn);
            }
        }
        for entry in self.edges.iter() {
            let edge = entry.value();
            if let (Some(from_u32), Some(to_u32)) =
                (self.get_u32(&edge.from), self.get_u32(&edge.to))
            {
                if let (Some(from_cid), Some(to_id)) = (
                    coll.node_to_arena.get(&from_u32),
                    coll.node_to_arena.get(&to_u32),
                ) {
                    let c1 = meta_arena[*from_cid as usize].cluster_id;
                    let c2 = meta_arena[*to_id as usize].cluster_id;
                    if c1 != c2 {
                        let key = format!("{}:{}", c1, c2);
                        let mut meta_edge =
                            self.meta_edges.entry(key.clone()).or_insert(MetaEdge {
                                from_cluster: c1,
                                to_cluster: c2,
                                weight: 0,
                            });
                        meta_edge.weight += 1;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn perform_autonomic_optimization(&self) -> Result<()> {
        println!("Mark VI: Executing Autonomic Maintenance...");
        self.prune_orphaned_nodes()?;
        self.generate_meta_graph()?;
        Ok(())
    }

    pub fn prune_orphaned_nodes(&self) -> Result<()> {
        let mut to_delete = Vec::new();
        let now = Utc::now().to_rfc3339();

        for entry in self.nodes.iter() {
            let node = entry.value();
            let u32_id = entry.key();

            // TTL Expiration Check
            if let Some(expires_at) = &node.expires_at {
                if now > *expires_at {
                    to_delete.push(node.id.clone());
                    println!("Mark VII: TTL Expired for node '{}'", node.id);
                    continue;
                }
            }

            // Legacy Orphan Pruning
            let is_master = node.labels.contains(&"MASTER".to_string());
            if !is_master {
                let has_in = self.in_idx.contains_key(u32_id);
                let has_out = self.out_idx.contains_key(u32_id);
                if !has_in && !has_out {
                    to_delete.push(node.id.clone());
                    println!("Mark VI: Pruning orphaned node '{}'", node.id);
                }
            }
        }

        for id in to_delete {
            let _ = self.retract_node(&id);
        }
        Ok(())
    }

    pub fn retract_node(&self, id: &str) -> Result<()> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        let u32_id = match self.get_u32(id) {
            Some(i) => i,
            None => return Ok(()),
        };
        // RCA--SLICE0-DURABILITY defect 2: the retraction must be durable
        // BEFORE the live view mutates — a crash after this call returns must
        // not resurrect the node on replay (the hourly autonomic TTL/orphan
        // prune goes through here too). Persist-first also means a failed
        // persist leaves the node fully intact: no phantom delete. The
        // projection arm deletes the node's props/label rows while `id` still
        // resolves in `id_to_u32`.
        let event = Event::NodeRetract {
            id: id.to_string(),
            clock: self.next_clock(),
        };
        self.persist(&event)?;
        self.retract_node_memory(id, u32_id);
        Ok(())
    }

    /// In-memory half of a node retraction: unwire incident edges from both
    /// adjacency indexes, drop the edges, the interning entry, the per-collection
    /// arena mappings, and the node itself. Shared by `retract_node` (under
    /// `commit_lock`, after the NodeRetract frame is durable), journal replay,
    /// and CRDT reconcile. (Arena slots are reclaimed lazily by compaction;
    /// trigram postings are not swept — pre-existing behavior.)
    fn retract_node_memory(&self, id: &str, u32_id: u32) {
        // 1. Collect all edges to remove
        let mut edges_to_remove = Vec::new();
        if let Some(eids) = self.out_idx.get(&u32_id) {
            for eid in eids.iter() {
                edges_to_remove.push(*eid);
            }
        }
        if let Some(eids) = self.in_idx.get(&u32_id) {
            for eid in eids.iter() {
                edges_to_remove.push(*eid);
            }
        }

        // 2. Comprehensive bi-directional index cleanup
        for eid in edges_to_remove {
            if let Some(edge_ref) = self.edges.get(&eid) {
                let edge = edge_ref.value();
                if let (Some(from_u32), Some(to_u32)) =
                    (self.get_u32(&edge.from), self.get_u32(&edge.to))
                {
                    // Remove from source node's out-index
                    if let Some(mut out_set) = self.out_idx.get_mut(&from_u32) {
                        out_set.remove(&eid);
                    }
                    // Remove from target node's in-index
                    if let Some(mut in_set) = self.in_idx.get_mut(&to_u32) {
                        in_set.remove(&eid);
                    }
                }
                // Edges carry no `id_to_u32` entry under numeric edge keys
                // (ADR--GENESISDB-EDGE-NUMERIC-KEYS) — nothing to sweep there.
            }
            self.edges.remove(&eid);
        }

        // 3. Remove node and primary indices
        self.id_to_u32.remove(id);
        self.out_idx.remove(&u32_id);
        self.in_idx.remove(&u32_id);
        // The node may have a vector in any collection — drop the mapping in all.
        // (Arena slots are reclaimed lazily by compaction, as before.)
        for c in self.collections.iter() {
            c.value().node_to_arena.remove(&u32_id);
        }
        self.nodes.remove(&u32_id);
    }

    pub fn reconcile_state(&self, signed_events: Vec<SignedEvent>) -> Result<()> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        self.reconcile_state_unlocked(signed_events)
    }

    fn reconcile_state_unlocked(&self, signed_events: Vec<SignedEvent>) -> Result<()> {
        for signed_event in signed_events {
            let event = &signed_event.event;
            let signer_id = &signed_event.signer_peer_id;

            // 1. Verify Signature (local events are self-trusted; remote events
            // must carry an authentic signature from their registered peer key).
            if signer_id != &self.local_peer_id && !self.verify_event_signature(&signed_event) {
                println!(
                    "Mark X: REJECTED event from {}. Invalid signature or unknown peer.",
                    signer_id
                );
                continue;
            }

            // 2. Apply Event logic
            match event {
                Event::Node(remote_node) => {
                    let u32_id = self.get_or_intern_id(&remote_node.id);
                    let mut apply = true;
                    if let Some(local_node) = self.nodes.get(&u32_id) {
                        if remote_node.clock < local_node.value().clock {
                            apply = false;
                        }
                    }
                    if apply {
                        // Sync local clock
                        let mut current = self.logical_clock.load(Ordering::SeqCst);
                        while remote_node.clock.time > current {
                            match self.logical_clock.compare_exchange_weak(
                                current,
                                remote_node.clock.time,
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            ) {
                                Ok(_) => break,
                                Err(actual) => current = actual,
                            }
                        }

                        if let Some(emb) = &remote_node.embedding {
                            self.replay_vector(
                                &remote_node.collection,
                                &remote_node.id,
                                emb.clone(),
                                remote_node.lang.clone().unwrap_or("en".to_string()),
                                true,
                            );
                        }
                        self.insert_node_lean(u32_id, remote_node.clone());
                        self.persist_signed(signed_event.clone())?;
                    }
                }
                Event::Edge(remote_edge) => {
                    let mut apply = true;
                    if let Some(local_edge) = self.edges.get(&Self::edge_key(&remote_edge.id)) {
                        if remote_edge.clock < local_edge.value().clock {
                            apply = false;
                        }
                    }
                    if apply {
                        let mut current = self.logical_clock.load(Ordering::SeqCst);
                        while remote_edge.clock.time > current {
                            match self.logical_clock.compare_exchange_weak(
                                current,
                                remote_edge.clock.time,
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            ) {
                                Ok(_) => break,
                                Err(actual) => current = actual,
                            }
                        }
                        let u32_id = self.index_edge_internal(
                            &remote_edge.id,
                            &remote_edge.from,
                            &remote_edge.to,
                        );
                        self.edges.insert(u32_id, remote_edge.clone());
                        self.persist_signed(signed_event.clone())?;
                    }
                }
                Event::Vector(remote_vec) => {
                    // Advance the local clock to the vector's time (like Node/Edge),
                    // so a peer that pulls and re-emits keeps logical time monotone.
                    let mut current = self.logical_clock.load(Ordering::SeqCst);
                    while remote_vec.clock.time > current {
                        match self.logical_clock.compare_exchange_weak(
                            current,
                            remote_vec.clock.time,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            Ok(_) => break,
                            Err(actual) => current = actual,
                        }
                    }
                    // Runtime sync: stage AND enqueue (index=true) — no rehydrate
                    // follows. Auto-provisions the collection if this peer lacks it.
                    // Vectors are append-applied (no LWW): a node holds at most one
                    // vector per collection, deduped at query time.
                    self.replay_vector(
                        &remote_vec.collection,
                        &remote_vec.node_id,
                        remote_vec.embedding.clone(),
                        remote_vec.lang.clone().unwrap_or_else(|| "en".to_string()),
                        true,
                    );
                    self.persist_signed(signed_event.clone())?;
                }
                Event::RelationalSchema(_) | Event::RelationalRows { .. } => {
                    self.persist_signed(signed_event.clone())?;
                }
                Event::Transaction(transaction) => {
                    // ADR D2.2: no fetch_max of the peer's origin sequence —
                    // see the consensus-commit arm for the rationale.
                    let seq = self.persist_signed(signed_event.clone())?;
                    self.apply_transaction_memory(transaction, true);
                    self.txn_frontier.fetch_max(seq, Ordering::SeqCst);
                }
                Event::Batch(inner_events) => {
                    // Recursive call needs SignedEvent wrapping, but for now we handle batches as single signed units
                    // To keep it simple, we wrap inner events or just apply them since the batch itself is verified.
                    let wrapped_inners: Vec<SignedEvent> = inner_events
                        .iter()
                        .map(|e| SignedEvent {
                            event: e.clone(),
                            signature: signed_event.signature.clone(), // Reuse batch signature
                            signer_peer_id: signer_id.clone(),
                        })
                        .collect();
                    let _ = self.reconcile_state_unlocked(wrapped_inners);
                }
                Event::NodeRetract { id, clock } => {
                    // Node-style LWW: the retraction applies unless the local
                    // copy is strictly newer (same rule as the Node arm above).
                    // No resident node ⇒ nothing to remove and nothing to
                    // re-persist (the retraction is already in the sender's
                    // journal; without a persistent tombstone store, ignoring
                    // it here is idempotent — pulls just keep offering it).
                    let apply = self
                        .get_u32(id)
                        .and_then(|u| self.nodes.get(&u).map(|n| (u, n.value().clock.clone())))
                        .filter(|(_, local_clock)| !(clock < local_clock));
                    if let Some((u32_id, _)) = apply {
                        let mut current = self.logical_clock.load(Ordering::SeqCst);
                        while clock.time > current {
                            match self.logical_clock.compare_exchange_weak(
                                current,
                                clock.time,
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            ) {
                                Ok(_) => break,
                                Err(actual) => current = actual,
                            }
                        }
                        self.retract_node_memory(id, u32_id);
                        self.persist_signed(signed_event.clone())?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Append an already-signed (peer) event verbatim — original signature
    /// preserved (I4) — stamped with a FRESH LOCAL frame seq (ADR D2.1: a frame
    /// ingested from a peer is local history; its origin sequence is metadata).
    ///
    /// **The caller must hold `commit_lock`.** This appends to the journal, and
    /// a concurrent `save_state`/fold checkpoint builds its payload from live
    /// memory — an append that is acked between that read and the fold is
    /// erased despite being fsynced (RCA--PERSIST-SIGNED-CHECKPOINT-RACE).
    /// Callers: `submit_vote` and `reconcile_state_unlocked`, both under the
    /// lock (save_state/compact hold the SAME mutex, so this fully serializes
    /// them against a concurrent fold regardless of the journal format).
    pub fn persist_signed(&self, signed_event: SignedEvent) -> Result<u64> {
        // If nobody holds the lock, `try_lock` succeeds and the invariant was
        // violated; if WE hold it, `try_lock` always fails (not reentrant), so
        // this never false-positives on a correct caller.
        debug_assert!(
            self.commit_lock.try_lock().is_none(),
            "persist_signed requires the caller to hold commit_lock \
             (see RCA--PERSIST-SIGNED-CHECKPOINT-RACE)"
        );
        let (ack_tx, ack_rx) = unbounded();
        let event = signed_event.event.clone();
        self.wal_sender
            .send(WalMsg::Append(Box::new(signed_event), ack_tx))
            .map_err(|_| Error::from_reason("wal disconnected"))?;
        let seq = ack_rx
            .recv()
            .ok()
            .flatten()
            .ok_or_else(|| Error::from_reason("wal append failed"))?;
        self.commit_sequence.fetch_max(seq, Ordering::SeqCst);
        self.projection_apply_event(&event, seq)?;
        Ok(seq)
    }

    pub fn get_logical_clock(&self) -> u32 {
        self.logical_clock.load(Ordering::SeqCst)
    }

    pub fn retrieve_context(
        &self,
        target_id: &str,
        tier_str: &str,
        budget: Option<u32>,
        fuzzy: bool,
    ) -> Result<ContextPackage> {
        let tier = ScalingTier::parse(tier_str);
        let hops = tier.hops();
        let target_id_resolved = if fuzzy {
            self.find_fuzzy_id(target_id)
                .unwrap_or(target_id.to_string())
        } else {
            target_id.to_string()
        };

        // 1. Graph Expansion (BFS)
        let mut nodes = HashMap::new();
        let mut edges = Vec::new();
        let mut queue = VecDeque::new();
        // Coverage instrumentation: deepest depth of any node actually placed
        // in the package, and whether traversal stopped at the tier boundary
        // with reachable graph still beyond it.
        let mut hops_served: u32 = 0;
        let mut ceiling_hit = false;

        if let Some(u32_id) = self.get_u32(&target_id_resolved) {
            queue.push_back((u32_id, 0));
            if let Some(node) = self.nodes.get(&u32_id) {
                nodes.insert(u32_id, self.hydrated_node(u32_id, node.value()));
            }
        }

        let mut visited = HashSet::new();
        while let Some((curr_u32, curr_depth)) = queue.pop_front() {
            if curr_depth >= hops {
                // Boundary node: discovered but never expanded. Report a
                // ceiling only when the graph genuinely continues past it —
                // i.e. it still has an incident edge whose far endpoint is not
                // already in the package. A boundary leaf, or an isolated node
                // at H0, is *complete* coverage, not a truncated frontier.
                // BFS is level-order, so by the time the first depth==hops node
                // is popped, every node within the radius is already in `nodes`.
                if !ceiling_hit
                    && !visited.contains(&curr_u32)
                    && self.has_edge_beyond(curr_u32, &nodes)
                {
                    ceiling_hit = true;
                }
                continue;
            }
            if visited.contains(&curr_u32) {
                continue;
            }
            visited.insert(curr_u32);

            if let Some(eids) = self.out_idx.get(&curr_u32) {
                for eid in eids.iter() {
                    if let Some(edge_ref) = self.edges.get(eid) {
                        let edge = edge_ref.value();
                        edges.push(edge.clone());
                        if let Some(next_u32) = self.get_u32(&edge.to) {
                            if let std::collections::hash_map::Entry::Vacant(e) =
                                nodes.entry(next_u32)
                            {
                                if let Some(node) = self.nodes.get(&next_u32) {
                                    e.insert(self.hydrated_node(next_u32, node.value()));
                                    hops_served = hops_served.max(curr_depth + 1);
                                    queue.push_back((next_u32, curr_depth + 1));
                                }
                            }
                        }
                    }
                }
            }
            // Also back-links for context
            if let Some(eids) = self.in_idx.get(&curr_u32) {
                for eid in eids.iter() {
                    if let Some(edge_ref) = self.edges.get(eid) {
                        let edge = edge_ref.value();
                        edges.push(edge.clone());
                        if let Some(prev_u32) = self.get_u32(&edge.from) {
                            if let std::collections::hash_map::Entry::Vacant(e) =
                                nodes.entry(prev_u32)
                            {
                                if let Some(node) = self.nodes.get(&prev_u32) {
                                    e.insert(self.hydrated_node(prev_u32, node.value()));
                                    hops_served = hops_served.max(curr_depth + 1);
                                    queue.push_back((prev_u32, curr_depth + 1));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Ranking & Budget Check
        let node_list: Vec<NodeOutput> = nodes.into_values().collect();
        let total_chars: usize = node_list.iter().map(|n| n.props.to_string().len()).sum();
        let token_estimate = (total_chars / 4) as u32;

        let mut super_nodes = Vec::new();
        let mut final_nodes = node_list;
        let mut truncated = false;

        if let Some(b) = budget {
            if token_estimate > b {
                // Compression: Switch to SuperNodes for high-level context
                println!(
                    "GRL: Budget exceeded ({} > {}). Compressing to SuperNodes.",
                    token_estimate, b
                );
                for entry in self.meta_nodes.iter() {
                    super_nodes.push(entry.value().clone());
                }
                final_nodes.clear(); // Prune atoms
                edges.clear();
                truncated = true;
            }
        }

        Ok(ContextPackage {
            nodes: final_nodes,
            edges,
            super_nodes,
            token_estimate,
            reasoning_path: format!(
                "Resolved {} as of Tier {} ({} hops)",
                target_id_resolved, tier_str, hops
            ),
            coverage: CoverageReport {
                hops_requested: hops,
                hops_served,
                ceiling_hit,
                truncated,
            },
        })
    }

    /// True if `u32_id` has at least one incident edge, in either direction,
    /// whose far endpoint is not already in `included`. Distinguishes a real
    /// truncated frontier from the edge of a fully-served subgraph, so
    /// `CoverageReport::ceiling_hit` reports a fact rather than merely
    /// "traversal stopped here".
    fn has_edge_beyond(&self, u32_id: u32, included: &HashMap<u32, NodeOutput>) -> bool {
        if let Some(eids) = self.out_idx.get(&u32_id) {
            for eid in eids.iter() {
                if let Some(edge_ref) = self.edges.get(eid) {
                    if let Some(next) = self.get_u32(&edge_ref.value().to) {
                        if !included.contains_key(&next) {
                            return true;
                        }
                    }
                }
            }
        }
        if let Some(eids) = self.in_idx.get(&u32_id) {
            for eid in eids.iter() {
                if let Some(edge_ref) = self.edges.get(eid) {
                    if let Some(prev) = self.get_u32(&edge_ref.value().from) {
                        if !included.contains_key(&prev) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn start_gossip_manager(storage: Arc<Self>) {
        // An MCP stdio process has a protocol-exclusive stdout channel. The
        // gossip manager emits operational text through native stdout, which
        // would corrupt Content-Length MCP frames, so the MCP launcher opts
        // out explicitly. Normal engine/server processes keep gossip enabled.
        if std::env::var_os("GENESIS_MCP_STDIO").is_some() {
            return;
        }
        let _peer_id = storage.local_peer_id.clone();
        let _verifying_key_bytes = storage.verifying_key.to_bytes().to_vec();

        tokio::spawn(async move {
            let socket = match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => {
                    let addr = match s.local_addr() {
                        Ok(a) => a,
                        Err(e) => {
                            println!("Gossip: Failed to read local socket addr: {}", e);
                            return;
                        }
                    };
                    storage
                        .gossip_port
                        .store(addr.port() as u32, Ordering::SeqCst);
                    println!("Gossip: Bound to UDP port {}", addr.port());
                    s
                }
                Err(e) => {
                    println!("Gossip: Failed to bind UDP socket: {}", e);
                    return;
                }
            };
            if let Err(e) = socket.set_broadcast(true) {
                println!("Gossip: Failed to enable broadcast: {}", e);
                return;
            }

            let mut buf = [0u8; 65535];
            let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                tokio::select! {
                    _ = heartbeat_interval.tick() => {
                        let msg = GossipMessage::Heartbeat {
                            peer_id: storage.local_peer_id.clone(),
                            merkle_root: storage.get_merkle_root(),
                            logical_time: storage.get_logical_clock(),
                            port: storage.gossip_port.load(Ordering::SeqCst) as u16,
                            verifying_key: storage.verifying_key.to_bytes().to_vec(),
                        };
                        if let Ok(data) = serde_json::to_vec(&msg) {
                            let _ = socket.send_to(&data, "255.255.255.255:30001").await;
                        }
                    }
                    result = socket.recv_from(&mut buf) => {
                        if let Ok((len, addr)) = result {
                            if let Ok(msg) = serde_json::from_slice::<GossipMessage>(&buf[..len]) {
                                match msg {
                                    GossipMessage::Heartbeat { peer_id: p_id, merkle_root, logical_time: _, port, verifying_key } => {
                                        if p_id != storage.local_peer_id {
                                            let peer_addr = format!("{}:{}", addr.ip(), port);
                                            storage.peers.insert(p_id.clone(), SyncPeer {
                                                id: p_id.clone(),
                                                addr: peer_addr.clone(),
                                                last_seen: Utc::now().timestamp() as u32,
                                                verifying_key,
                                            });

                                            if merkle_root != storage.get_merkle_root() {
                                                let req = GossipMessage::PullRequest {
                                                    from_clock: storage.get_logical_clock(),
                                                    target_peer_id: storage.local_peer_id.clone(),
                                                    // WP-1.2: per-peer responder-domain
                                                    // cursor tracking lands with the
                                                    // bootstrap channel (WP-1.3); until
                                                    // then requesters stay on the
                                                    // Lamport cursor.
                                                    from_commit_seq: None,
                                                };
                                                if let Ok(data) = serde_json::to_vec(&req) {
                                                    let _ = socket.send_to(&data, peer_addr).await;
                                                }
                                            }
                                        }
                                    }
                                    GossipMessage::PullRequest { from_clock, target_peer_id, from_commit_seq } => {
                                        // WP-1.2 (ADR D4): a commit_seq cursor below our
                                        // history horizon cannot be served by delta —
                                        // answer BeyondHorizon so the requester abandons
                                        // delta-pull (bootstrap channel lands in WP-1.3).
                                        if let Some(cursor) = from_commit_seq {
                                            let horizon = storage.history_horizon();
                                            if cursor < horizon {
                                                if let Some(reply_addr) = storage.peers.get(&target_peer_id).map(|p| p.addr.clone()) {
                                                    if let Ok(data) = serde_json::to_vec(&GossipMessage::BeyondHorizon { horizon }) {
                                                        let _ = socket.send_to(&data, reply_addr).await;
                                                    }
                                                }
                                                continue;
                                            }
                                        }
                                        // Anti-entropy: reply with our events newer than the
                                        // requester's clock, addressed to its registered gossip
                                        // addr (the heartbeat port, not the ephemeral src port).
                                        // Bound the reply to one UDP datagram; the requester's
                                        // clock advances on apply, so the next heartbeat round
                                        // pulls the remainder until the roots stop differing.
                                        if let Some(reply_addr) = storage.peers.get(&target_peer_id).map(|p| p.addr.clone()) {
                                            let mut batch = Vec::new();
                                            let mut bytes = 0usize;
                                            let events = match from_commit_seq {
                                                // Frame-cursor path (WP-1.2): serves every
                                                // frame incl. relational-only transactions.
                                                Some(cursor) => storage.events_since_seq(cursor),
                                                None => storage.events_since(from_clock),
                                            };
                                            for ev in events {
                                                let sz = serde_json::to_vec(&ev).map(|v| v.len()).unwrap_or(0) + 2;
                                                if !batch.is_empty() && bytes + sz > 60_000 { break; }
                                                bytes += sz;
                                                batch.push(ev);
                                            }
                                            if !batch.is_empty() {
                                                if let Ok(data) = serde_json::to_vec(&GossipMessage::PushDelta { events: batch }) {
                                                    let _ = socket.send_to(&data, reply_addr).await;
                                                }
                                            }
                                        }
                                    }
                                    GossipMessage::PushDelta { events } => {
                                        let _ = storage.reconcile_state(events);
                                    }
                                    GossipMessage::BeyondHorizon { horizon } => {
                                        // WP-1.2 requester handling: delta-pull cannot
                                        // proceed — mark and stop. The snapshot-bootstrap
                                        // transfer channel is WP-1.3; until retention can
                                        // advance a horizon past a live cursor this arm
                                        // is wire-defined but not triggered in practice.
                                        println!(
                                            "Sync: peer horizon {} exceeds our cursor — \
                                             delta-pull abandoned, awaiting bootstrap (WP-1.3).",
                                            horizon
                                        );
                                    }
                                    GossipMessage::ConsensusPropose { proposal } => {
                                        // Verify the proposal's event is authentically signed by
                                        // its claimed originator before storing it. An unsigned or
                                        // forged proposal could otherwise be driven to quorum and
                                        // applied as a MASTER axiom, bypassing governance.
                                        if storage.verify_event_signature(&proposal.signed_event) {
                                            storage.proposals.insert(proposal.proposal_id.clone(), *proposal);
                                        } else {
                                            println!("Mark X: REJECTED proposal {} — invalid event signature or unknown signer.", proposal.proposal_id);
                                        }
                                    }
                                    GossipMessage::ConsensusVote { proposal_id, voter_peer_id, approve, signature } => {
                                        // submit_vote verifies the vote's ed25519 signature
                                        // against the voter's registered key before counting
                                        // it (see Storage::submit_vote); forged or unsigned
                                        // votes are rejected and never reach quorum.
                                        let _ = storage.submit_vote(proposal_id, voter_peer_id, approve, signature);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        // Also bind discovery listener on the fixed port
        tokio::spawn(async move {
            let socket = match tokio::net::UdpSocket::bind("0.0.0.0:30001").await {
                Ok(s) => s,
                Err(_) => return,
            };
            if let Err(e) = socket.set_broadcast(true) {
                println!(
                    "Gossip: Failed to enable broadcast on discovery listener: {}",
                    e
                );
                return;
            }
            let mut buf = [0u8; 65535];
            loop {
                if let Ok((len, _addr)) = socket.recv_from(&mut buf).await {
                    if let Ok(GossipMessage::Heartbeat { .. }) =
                        serde_json::from_slice::<GossipMessage>(&buf[..len])
                    {
                        // Discovery logic handled via broadcast in main loop
                    }
                }
            }
        });
    }

    /// Rename `from`→`to`, retrying transient Windows sharing violations. The AV
    /// scanner / search indexer briefly opens a freshly-written file, and Windows
    /// then fails `MoveFileEx` with ACCESS_DENIED/SHARING_VIOLATION (os err 5/32/33)
    /// until it releases — a few ms later. Without the retry the `.ok()` callers in
    /// `save_state` silently drop the snapshot under parallel load, leaving reload
    /// to fall back to a (correct but slower) full WAL replay. Best-effort: a final
    /// failure returns `false` rather than erroring, since the snapshot is only an
    /// instant-load optimization layered on the authoritative WAL.
    fn rename_with_retry(from: &std::path::Path, to: &std::path::Path) -> bool {
        for _ in 0..50 {
            match fs::rename(from, to) {
                Ok(_) => return true,
                Err(e) if matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33)) => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(_) => return false,
            }
        }
        fs::rename(from, to).is_ok()
    }

    pub fn save_state(&self) -> Result<()> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        self.save_state_unlocked()
    }

    fn save_state_unlocked(&self) -> Result<()> {
        self.ensure_writable()?;
        // WP-1.2 ordering (I7/I9: seal durability strictly precedes manifest
        // advance): build the live-state payload, FOLD the journal first (base
        // segment durable via the writer thread), then write snapshot files
        // with state.json renamed last. The frame-seq cursor makes every crash
        // window safe: replay is seq-filtered and idempotent, and the folded
        // journal remains a complete recovery source on its own (I8), so a
        // crash between the fold and the state.json rename merely re-applies
        // frames the snapshot would have covered. Writers serialize on
        // commit_lock (held by save_state), so live state cannot move between
        // the build and the fold.
        let frontier_seq = self.stable_frontier();
        // RCA--SLICE0-DURABILITY defect 3: a snapshot written WITHOUT a journal
        // cursor would make the next open skip tail replay entirely — silently
        // dropping every write acked after this point. If the payload build
        // fails, abort the checkpoint loudly instead: the previous snapshot +
        // the (unfolded) journal remain a complete recovery source (I8).
        let wal_payload = self.build_compacted_wal()?;
        {
            let (payload, count) = &wal_payload;
            // A failed FOLD is safe (the journal keeps full history and the
            // cursor below still filters correctly) — but it must not be
            // silent: the journal will regrow until a fold succeeds.
            if self
                .checkpoint_wal_payload(payload.clone(), frontier_seq, *count)
                .is_err()
            {
                println!(
                    "Mark IX: journal fold failed — snapshot proceeding; journal retains full history until the next successful fold."
                );
            }
        }
        let temp_dir = self.path.join("temp_save");
        if temp_dir.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
        }
        fs::create_dir_all(&temp_dir).ok();

        // 1. Per-collection arenas + metadata + a manifest. HNSW is NOT dumped —
        //    it rehydrates cheaply from each arena on load (the arena is the
        //    source of truth). state.json's `collections` array drives reload.
        let mut manifest: Vec<serde_json::Value> = Vec::new();
        for c in self.collections.iter() {
            let coll = c.value();
            let arena = coll.arena.read();
            fs::write(
                temp_dir.join(format!("vec_{}.bin", coll.name)),
                arena.to_bytes(),
            )
            .map_err(|e| Error::from_reason(e.to_string()))?;
            let meta = coll.metadata.read();
            let meta_data = encode_metadata_snapshot(&meta)?;
            fs::write(temp_dir.join(format!("meta_{}.bin", coll.name)), meta_data)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            // Rerank sidecar (exact f32) — only when the collection carries one.
            // Backing is now ON DISK; stream each row through the reader into the
            // temp fvec instead of re-materializing a resident Vec<f32>. Streaming
            // `0..len_rows` (not a raw byte copy of the live file) guarantees the
            // snapshot is exactly `live_count` rows even if the live file ever held
            // trailing bytes. Assert size == rows*dim*4 before adopting the temp.
            if let Some(sidecar) = &coll.f32_sidecar {
                let s = sidecar.read();
                let dim = coll.dim as usize;
                let rows = s.len_rows();
                let out_path = temp_dir.join(format!("fvec_{}.bin", coll.name));
                let out = FileOpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&out_path)
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                let mut w = std::io::BufWriter::new(out);
                for i in 0..rows {
                    // A row the live file can't serve (should not happen: live fvec
                    // is lock-step with the arena) is written as zeros so the file
                    // stays exactly rows*dim wide and reload's len guard still holds.
                    let row = s.row(i).unwrap_or_else(|| vec![0.0f32; dim]);
                    for v in &row {
                        w.write_all(&v.to_le_bytes())
                            .map_err(|e| Error::from_reason(e.to_string()))?;
                    }
                }
                w.flush().map_err(|e| Error::from_reason(e.to_string()))?;
                let written = fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
                debug_assert_eq!(
                    written,
                    (rows * dim * 4) as u64,
                    "fvec snapshot size must equal rows*dim*4"
                );
            }
            // P1a — BQ centering vector (flat le-f32, len == dim). Written only
            // when present (BQ + compacted). Absent on reload ⇒ `None` ⇒
            // uncentered, so pre-P1a snapshots keep their exact behavior.
            if let Some(center) = coll.bq_center.read().as_ref() {
                let bytes: Vec<u8> = center.iter().flat_map(|f| f.to_le_bytes()).collect();
                fs::write(temp_dir.join(format!("bqmean_{}.bin", coll.name)), bytes)
                    .map_err(|e| Error::from_reason(e.to_string()))?;
            }
            // P2b — calibrated SQ8 (scale,bias) = 8 le-f32 bytes. Written only when
            // calibrated; absent ⇒ fixed scale (back-compat).
            if let Some((scale, bias)) = *coll.sq8_scale.read() {
                let mut bytes = scale.to_le_bytes().to_vec();
                bytes.extend_from_slice(&bias.to_le_bytes());
                fs::write(temp_dir.join(format!("sq8scale_{}.bin", coll.name)), bytes)
                    .map_err(|e| Error::from_reason(e.to_string()))?;
            }
            manifest.push(serde_json::json!({
                "name": coll.name, "model": coll.model, "dim": coll.dim,
                "metric": coll.metric.as_str(), "quant": coll.quant.as_str(),
                // Per-collection default ef_search; absent ⇒ None (engine-global).
                "ef_search": coll.ef_search,
                // f32-sidecar rerank; absent ⇒ false (no sidecar loaded).
                "rerank": coll.f32_sidecar.is_some(),
                // SQ8 calibrated-scale opt-in; absent ⇒ false (fixed scale).
                "sq8_calibrate": coll.sq8_calibrate,
                // meta format version: 1 = NodeMetadata.node_u32 (A2). Absent ⇒ 0
                // (pre-A2 String layout), migrated on load.
                "mv": 1
            }));
        }

        // 2. Save DashMaps (Partial state for instant load)
        let nodes: Vec<(u32, NodeOutput)> = self
            .nodes
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();
        let edges: Vec<(u128, EdgeOutput)> = self
            .edges
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();
        if let Ok(bytes) = serde_json::to_vec(&nodes) {
            fs::write(temp_dir.join("nodes.bin"), bytes).ok();
        }
        if let Ok(bytes) = serde_json::to_vec(&edges) {
            fs::write(temp_dir.join("edges.bin"), bytes).ok();
        }
        self.projection_snapshot(&temp_dir.join(PROJECTION_DB_FILE))?;

        // 3. Save Global Metadata (incl. collections manifest)
        let mut state = serde_json::json!({
            "logical_clock": self.get_logical_clock(),
            "peer_id": self.local_peer_id,
            "collections": manifest,
            "schema_version": SCHEMA_VERSION,
            "timestamp": Utc::now().to_rfc3339(),
        });
        // The tail-replay cursor (SPEC §3): frames with seq > frontier_seq
        // on the next open are writes acked AFTER this snapshot. Position-
        // independent, so no prefix hash is needed (supersedes PR #100's
        // byte cursor — spec updated in this PR). Always present: save_state
        // aborts before this point if the payload build fails, so a snapshot
        // can no longer exist without its cursor (RCA--SLICE0-DURABILITY
        // defect 3). The write error is propagated for the same reason — a
        // half-written state.json must fail the save, not limp into the swap.
        state["journal"] = serde_json::json!({
            "frontier_seq": frontier_seq,
            "txn_frontier": self.txn_frontier(),
            "tx_epoch_start": 1,
            "format_version": JOURNAL_FORMAT_VERSION,
        });
        fs::write(temp_dir.join("state.json"), state.to_string())
            .map_err(|e| Error::from_reason(format!("state.json write failed: {}", e)))?;

        // Atomic-ish swap: per-collection filenames are dynamic, so move every
        // file produced into the db root instead of a fixed list. `state.json` is
        // moved LAST and on its own: the load path only does an instant load when
        // state.json exists, so renaming it last means a crash mid-swap leaves no
        // state.json → reload falls back to the (still-intact) WAL. Truncating the
        // WAL below is therefore safe only after state.json is durably in place.
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.file_name().is_some_and(|n| n == "state.json") {
                    continue;
                }
                if let Some(name) = p.file_name() {
                    Self::rename_with_retry(&p, &self.path.join(name));
                }
            }
        }
        let state_tmp = temp_dir.join("state.json");
        if state_tmp.exists() {
            Self::rename_with_retry(&state_tmp, &self.path.join("state.json"));
        }
        let _ = fs::remove_dir_all(&temp_dir);

        // The journal fold already ran BEFORE the snapshot swap (see the top of
        // this function — I7/I9 ordering). Nothing to do here.

        println!(
            "Mark IX: State persisted successfully to {}",
            self.path.display()
        );
        Ok(())
    }

    fn is_backup_artifact_name(name: &str) -> bool {
        // WP-1.2: the journal lives in subdirectories. Manifest paths use '/'
        // only; the two allowed subdir shapes are exact-prefix whitelisted and
        // their file component may not contain separators or dots-only names
        // (path-traversal is structurally impossible).
        if name == "wal/active.gwal" {
            return true;
        }
        if let Some(seg) = name.strip_prefix("journal/") {
            return seg.ends_with(".gseg")
                && !seg.contains(['/', '\\'])
                && !seg.starts_with('.')
                && seg.len() > ".gseg".len();
        }
        matches!(
            name,
            "identity.bin"
                | "genesis-graph.wal" // legacy bundles (pre-WP-1.2) still restore
                | "state.json"
                | "nodes.bin"
                | "edges.bin"
                | PROJECTION_DB_FILE
        ) || ((name.starts_with("vec_")
            || name.starts_with("meta_")
            || name.starts_with("fvec_")
            || name.starts_with("bqmean_")
            || name.starts_with("sq8scale_"))
            && name.ends_with(".bin"))
    }

    fn sha256_file(path: &Path) -> Result<(u64, String)> {
        let mut file = File::open(path).map_err(|e| Error::from_reason(e.to_string()))?;
        let mut hasher = Sha256::new();
        let mut byte_count = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            byte_count += read as u64;
        }
        Ok((byte_count, hex::encode(hasher.finalize())))
    }

    fn write_u32(writer: &mut File, value: u32) -> Result<()> {
        writer
            .write_all(&value.to_le_bytes())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    fn write_u64(writer: &mut File, value: u64) -> Result<()> {
        writer
            .write_all(&value.to_le_bytes())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    fn read_u32(reader: &mut File) -> Result<u32> {
        let mut bytes = [0u8; 4];
        reader
            .read_exact(&mut bytes)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u64(reader: &mut File) -> Result<u64> {
        let mut bytes = [0u8; 8];
        reader
            .read_exact(&mut bytes)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn backup_info(manifest: &BackupManifest, bundle_path: PathBuf) -> Result<BackupBundleInfo> {
        let (byte_count, sha256) = Self::sha256_file(&bundle_path)?;
        Ok(BackupBundleInfo {
            format_version: manifest.format_version,
            engine_name: manifest.engine_name.clone(),
            engine_version: manifest.engine_version.clone(),
            schema_version: manifest.schema_version,
            stable_frontier: manifest.stable_frontier,
            logical_clock: manifest.logical_clock,
            created_at: manifest.created_at.clone(),
            bundle_path,
            byte_count,
            sha256,
        })
    }

    fn validate_backup_manifest(manifest: &BackupManifest) -> Result<()> {
        if manifest.format_version != BACKUP_FORMAT_VERSION {
            return Err(Error::from_reason("unsupported Genesis backup format"));
        }
        if manifest.engine_name != ENGINE_NAME || manifest.engine_version != ENGINE_VERSION {
            return Err(Error::from_reason("incompatible Genesis engine backup"));
        }
        if manifest.schema_version > SCHEMA_VERSION {
            return Err(Error::from_reason(
                "backup schema requires a newer Genesis engine",
            ));
        }

        let mut names = HashSet::new();
        for artifact in &manifest.artifacts {
            // Whitelisted subdir artifacts (wal/, journal/) legitimately carry
            // one '/'; everything else must be a bare file name. Either way the
            // whitelist itself forbids traversal components.
            let flat_ok = Path::new(&artifact.path)
                .file_name()
                .and_then(|name| name.to_str())
                == Some(artifact.path.as_str());
            let subdir_ok =
                artifact.path.starts_with("wal/") || artifact.path.starts_with("journal/");
            if !Self::is_backup_artifact_name(&artifact.path)
                || !(flat_ok || subdir_ok)
                || artifact.sha256.len() != 64
                || hex::decode(&artifact.sha256).is_err()
                || !names.insert(artifact.path.clone())
            {
                return Err(Error::from_reason(
                    "invalid Genesis backup manifest artifact",
                ));
            }
        }
        for required in ["identity.bin", "state.json", PROJECTION_DB_FILE] {
            if !names.contains(required) {
                return Err(Error::from_reason("Genesis backup manifest is incomplete"));
            }
        }
        // The journal: framed bundles carry wal/active.gwal; pre-WP-1.2 bundles
        // carry genesis-graph.wal (restore migrates it at the verification open).
        if !names.contains("wal/active.gwal") && !names.contains("genesis-graph.wal") {
            return Err(Error::from_reason("Genesis backup manifest is incomplete"));
        }
        Ok(())
    }

    /// Creates one opaque, integrity-checked backup bundle. The caller never
    /// receives individual projection files.
    pub fn export_backup(&self, request: BackupExportRequest) -> Result<BackupBundleInfo> {
        self.ensure_writable()?;
        // Public write paths share this barrier so the captured files represent
        // one declared frontier rather than a mix of concurrent mutations.
        let _commit_guard = self.commit_lock.lock();
        if request.destination.exists() {
            return Err(Error::from_reason("backup destination already exists"));
        }
        let parent = request
            .destination
            .parent()
            .ok_or_else(|| Error::from_reason("backup destination has no parent"))?;
        let file_name = request
            .destination
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| Error::from_reason("backup destination has no file name"))?;
        fs::create_dir_all(parent).map_err(|e| Error::from_reason(e.to_string()))?;
        let parent = fs::canonicalize(parent).map_err(|e| Error::from_reason(e.to_string()))?;
        let destination = parent.join(file_name);
        let live_root =
            fs::canonicalize(&self.path).map_err(|e| Error::from_reason(e.to_string()))?;
        if destination.starts_with(&live_root) {
            return Err(Error::from_reason(
                "backup destination must be outside the live Genesis root",
            ));
        }

        self.flush_index();
        self.save_state_unlocked()?;

        let mut artifacts = Vec::new();
        for entry in fs::read_dir(&self.path).map_err(|e| Error::from_reason(e.to_string()))? {
            let entry = entry.map_err(|e| Error::from_reason(e.to_string()))?;
            if !entry
                .file_type()
                .map_err(|e| Error::from_reason(e.to_string()))?
                .is_file()
            {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if Self::is_backup_artifact_name(&name) {
                let (byte_count, sha256) = Self::sha256_file(&entry.path())?;
                artifacts.push(BackupArtifact {
                    path: name,
                    byte_count,
                    sha256,
                });
            }
        }
        // WP-1.2: the journal lives in subdirectories — one bundle must carry
        // it (manifest paths use '/', platform-independent).
        let active = self.path.join("wal").join("active.gwal");
        if active.is_file() {
            let (byte_count, sha256) = Self::sha256_file(&active)?;
            artifacts.push(BackupArtifact {
                path: "wal/active.gwal".to_string(),
                byte_count,
                sha256,
            });
        }
        if let Ok(entries) = fs::read_dir(Self::journal_seg_dir(&self.path)) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "gseg") {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        let (byte_count, sha256) = Self::sha256_file(&p)?;
                        artifacts.push(BackupArtifact {
                            path: format!("journal/{name}"),
                            byte_count,
                            sha256,
                        });
                    }
                }
            }
        }
        artifacts.sort_by(|left, right| left.path.cmp(&right.path));
        let manifest = BackupManifest {
            format_version: BACKUP_FORMAT_VERSION,
            engine_name: ENGINE_NAME.to_string(),
            engine_version: ENGINE_VERSION.to_string(),
            schema_version: SCHEMA_VERSION,
            stable_frontier: self.stable_frontier(),
            logical_clock: self.get_logical_clock(),
            created_at: Utc::now().to_rfc3339(),
            artifacts,
        };
        Self::validate_backup_manifest(&manifest)?;

        let temporary = parent.join(format!(".{file_name}.{}.partial", Uuid::new_v4()));
        let result = (|| -> Result<()> {
            let mut output = FileOpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            output
                .write_all(BACKUP_MAGIC)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            let manifest_bytes =
                serde_json::to_vec(&manifest).map_err(|e| Error::from_reason(e.to_string()))?;
            Self::write_u32(&mut output, BACKUP_MANIFEST_NAME.len() as u32)?;
            output
                .write_all(BACKUP_MANIFEST_NAME)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            Self::write_u64(&mut output, manifest_bytes.len() as u64)?;
            output
                .write_all(&manifest_bytes)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            for artifact in &manifest.artifacts {
                let path_bytes = artifact.path.as_bytes();
                Self::write_u32(&mut output, path_bytes.len() as u32)?;
                output
                    .write_all(path_bytes)
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                Self::write_u64(&mut output, artifact.byte_count)?;
                let mut input = File::open(self.path.join(&artifact.path))
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                let mut copied = 0u64;
                let mut hasher = Sha256::new();
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    let read = input
                        .read(&mut buffer)
                        .map_err(|e| Error::from_reason(e.to_string()))?;
                    if read == 0 {
                        break;
                    }
                    output
                        .write_all(&buffer[..read])
                        .map_err(|e| Error::from_reason(e.to_string()))?;
                    hasher.update(&buffer[..read]);
                    copied += read as u64;
                }
                if copied != artifact.byte_count
                    || hex::encode(hasher.finalize()) != artifact.sha256
                {
                    return Err(Error::from_reason(
                        "Genesis backup source artifact changed during export",
                    ));
                }
            }
            output
                .sync_all()
                .map_err(|e| Error::from_reason(e.to_string()))?;
            fs::rename(&temporary, &destination).map_err(|e| Error::from_reason(e.to_string()))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result?;
        Self::backup_info(&manifest, destination)
    }

    /// Verifies one opaque bundle before publishing it as a non-existing root.
    pub fn restore_backup(request: BackupRestoreRequest) -> Result<BackupBundleInfo> {
        if request.target_root.exists() {
            return Err(Error::from_reason("restore target must not already exist"));
        }
        let parent = request
            .target_root
            .parent()
            .ok_or_else(|| Error::from_reason("restore target has no parent"))?;
        let parent = fs::canonicalize(parent).map_err(|e| Error::from_reason(e.to_string()))?;
        let target_name = request
            .target_root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| Error::from_reason("restore target has no name"))?;
        let target_root = parent.join(target_name);
        let staging = parent.join(format!(".genesis-restore-{}", Uuid::new_v4()));
        let bundle_path = fs::canonicalize(&request.bundle_path)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let result = (|| -> Result<BackupManifest> {
            let mut input =
                File::open(&bundle_path).map_err(|e| Error::from_reason(e.to_string()))?;
            let mut magic = vec![0u8; BACKUP_MAGIC.len()];
            input
                .read_exact(&mut magic)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            if magic.as_slice() != BACKUP_MAGIC {
                return Err(Error::from_reason("invalid Genesis backup bundle"));
            }
            let manifest_name_len = Self::read_u32(&mut input)? as usize;
            if manifest_name_len != BACKUP_MANIFEST_NAME.len() {
                return Err(Error::from_reason(
                    "Genesis backup manifest name is invalid",
                ));
            }
            let mut manifest_name = vec![0u8; manifest_name_len];
            input
                .read_exact(&mut manifest_name)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            if manifest_name.as_slice() != BACKUP_MANIFEST_NAME {
                return Err(Error::from_reason(
                    "Genesis backup manifest name is invalid",
                ));
            }
            let manifest_len = Self::read_u64(&mut input)?;
            if manifest_len == 0 || manifest_len > 4 * 1024 * 1024 {
                return Err(Error::from_reason("invalid Genesis backup manifest length"));
            }
            let mut manifest_bytes = vec![0u8; manifest_len as usize];
            input
                .read_exact(&mut manifest_bytes)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            let manifest: BackupManifest = serde_json::from_slice(&manifest_bytes)
                .map_err(|_| Error::from_reason("invalid Genesis backup manifest"))?;
            Self::validate_backup_manifest(&manifest)?;

            fs::create_dir(&staging).map_err(|e| Error::from_reason(e.to_string()))?;
            for artifact in &manifest.artifacts {
                let path_len = Self::read_u32(&mut input)? as usize;
                if path_len == 0 || path_len > 1024 {
                    return Err(Error::from_reason("invalid Genesis backup artifact path"));
                }
                let mut path_bytes = vec![0u8; path_len];
                input
                    .read_exact(&mut path_bytes)
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                let path = String::from_utf8(path_bytes)
                    .map_err(|_| Error::from_reason("invalid Genesis backup artifact encoding"))?;
                let byte_count = Self::read_u64(&mut input)?;
                if path != artifact.path || byte_count != artifact.byte_count {
                    return Err(Error::from_reason(
                        "Genesis backup artifact does not match manifest",
                    ));
                }
                let output_path = staging.join(&path);
                // Subdir artifacts (wal/, journal/) need their parent created;
                // the manifest whitelist already forbids traversal components.
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| Error::from_reason(e.to_string()))?;
                }
                let mut output = FileOpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&output_path)
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                let mut remaining = byte_count;
                let mut buffer = [0u8; 64 * 1024];
                while remaining > 0 {
                    let wanted = remaining.min(buffer.len() as u64) as usize;
                    input
                        .read_exact(&mut buffer[..wanted])
                        .map_err(|e| Error::from_reason(e.to_string()))?;
                    output
                        .write_all(&buffer[..wanted])
                        .map_err(|e| Error::from_reason(e.to_string()))?;
                    remaining -= wanted as u64;
                }
                output
                    .sync_all()
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                let (_, sha256) = Self::sha256_file(&output_path)?;
                if sha256 != artifact.sha256 {
                    return Err(Error::from_reason(
                        "Genesis backup artifact digest mismatch",
                    ));
                }
            }
            let mut trailing = [0u8; 1];
            if input
                .read(&mut trailing)
                .map_err(|e| Error::from_reason(e.to_string()))?
                != 0
            {
                return Err(Error::from_reason(
                    "Genesis backup bundle has trailing data",
                ));
            }

            // A fresh open validates the engine's own snapshot/WAL compatibility
            // before the staging directory becomes the caller-visible target.
            drop(Storage::open(OpenOptions {
                path: staging.display().to_string(),
                page_cache_mb: Some(16),
                read_only: Some(false),
                vector_dim: Some(0),
            })?);
            Ok(manifest)
        })();
        match result {
            Ok(manifest) => {
                fs::rename(&staging, &target_root)
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                Self::backup_info(&manifest, bundle_path)
            }
            Err(error) => {
                let _ = fs::remove_dir_all(&staging);
                Err(error)
            }
        }
    }

    fn try_load_state(&self) -> bool {
        let state_path = self.path.join("state.json");
        if !state_path.exists() {
            return false;
        }

        println!("Mark IX: Attempting instant load from binary state...");

        let start = Instant::now();
        let state_val: serde_json::Value = match fs::read_to_string(&state_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
        {
            Some(v) => v,
            None => return false,
        };

        // 1. Load vector collections. New format: `collections` manifest +
        //    vec_<name>.bin / meta_<name>.bin. Legacy: a single vector.bin /
        //    meta.bin pair, migrated transparently into the `default` collection.
        self.collections.clear();
        // Pre-A2 (mv absent) collections store String node ids in meta; they can't
        // be interned to u32 here because id_to_u32 is only populated when nodes.bin
        // loads (step 2). Stash the raw legacy metas and migrate them in step 3.
        let mut legacy_meta: HashMap<String, Vec<NodeMetadataV0>> = HashMap::new();
        if let Some(colls) = state_val["collections"].as_array() {
            for cm in colls {
                let name = cm["name"].as_str().unwrap_or("default").to_string();
                let model = cm["model"].as_str().unwrap_or("default").to_string();
                let dim = cm["dim"].as_u64().unwrap_or(0) as u16;
                let metric = Metric::parse(cm["metric"].as_str().unwrap_or("L2"));
                let quant = Quant::parse(cm["quant"].as_str().unwrap_or("none"));
                let ef_search = cm["ef_search"].as_u64().map(|e| e as u32);
                let rerank = cm["rerank"].as_bool().unwrap_or(false);
                // P2b — whether this SQ8 collection opted into a calibrated scale.
                let sq8_calibrate = cm["sq8_calibrate"].as_bool().unwrap_or(false);
                // Open the on-disk sidecar (if any) WITHOUT truncating — adopt the
                // rows a prior save/compaction wrote. We validate row-count against
                // the arena below and drop the sidecar on mismatch.
                let sidecar_path = if rerank && matches!(quant, Quant::ScalarU8 | Quant::Binary) {
                    Some(self.path.join(format!("fvec_{}.bin", name)))
                } else {
                    None
                };
                let mut coll = VectorCollection::new(
                    name.clone(),
                    model,
                    dim,
                    metric,
                    quant,
                    ef_search,
                    rerank,
                    sidecar_path,
                    false, // load: adopt existing on-disk rows, don't truncate
                    sq8_calibrate,
                );
                if let Ok(data) = fs::read(self.path.join(format!("vec_{}.bin", name))) {
                    *coll.arena.write() = ArenaStore::from_bytes(&data, quant, dim as usize);
                }
                // P2b — adopt the persisted calibrated SQ8 (scale,bias), if any. The
                // arena's u8 codes are scale-agnostic bytes, so inject the scale into
                // BOTH the collection snapshot and the arena (kept lock-step). Absent
                // /short file ⇒ leave `None` ⇒ fixed scale (back-compat).
                if quant == Quant::ScalarU8 {
                    if let Ok(bytes) = fs::read(self.path.join(format!("sq8scale_{}.bin", name))) {
                        if bytes.len() == 8 {
                            let scale = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
                            let bias = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
                            *coll.sq8_scale.write() = Some((scale, bias));
                            coll.arena.write().set_sq8_scale(scale, bias);
                        }
                    }
                }
                // P1a — adopt the persisted BQ centering vector, if any. Only for
                // BQ collections and only when the file is exactly `dim` f32s (a
                // short/absent file ⇒ leave `None` ⇒ uncentered, back-compat). The
                // arena above already holds centered codes, so the center just has
                // to match what packed them; every later insert/query reuses it.
                if quant == Quant::Binary && dim > 0 {
                    if let Ok(bytes) = fs::read(self.path.join(format!("bqmean_{}.bin", name))) {
                        if bytes.len() == dim as usize * 4 {
                            let center: Vec<f32> = bytes
                                .chunks_exact(4)
                                .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                                .collect();
                            *coll.bq_center.write() = Some(center);
                        }
                    }
                }
                // Rerank sidecar is on-disk (fvec_<name>.bin), opened by `new`.
                // Only ADOPT it if its row count exactly parallels the arena — a
                // missing / truncated fvec (partial or crash-torn snapshot) is
                // dropped to `None` so search degrades to quantized-only (the query
                // path falls back per-candidate), never to empty results. `new`
                // .create(true)'d an empty file for a missing fvec, which fails this
                // guard (0 rows ≠ arena rows) and is correctly discarded.
                if coll.f32_sidecar.is_some() {
                    let arena_rows = {
                        let a = coll.arena.read();
                        if dim == 0 {
                            0
                        } else {
                            a.len() / dim as usize
                        }
                    };
                    let sidecar_rows = coll
                        .f32_sidecar
                        .as_ref()
                        .map(|s| s.read().len_rows())
                        .unwrap_or(0);
                    if sidecar_rows != arena_rows {
                        coll.f32_sidecar = None;
                    }
                }
                if let Ok(data) = fs::read(self.path.join(format!("meta_{}.bin", name))) {
                    match decode_metadata_snapshot(&data) {
                        Some(Ok(meta)) => {
                            coll.count.store(meta.len(), Ordering::Relaxed);
                            *coll.metadata.write() = meta;
                        }
                        Some(Err(e)) => {
                            // GBP1 magic present but the postcard body didn't decode:
                            // corruption, not an unrecognized legacy format. Fail
                            // loudly rather than silently falling through to the
                            // bincode arms below (ADR--GENESISDB-BINCODE-EXIT).
                            panic!(
                                "corrupt metadata snapshot meta_{}.bin: GBP1 magic \
                                 present but postcard body failed to decode: {}",
                                name, e
                            );
                        }
                        None if cm["mv"].as_u64().unwrap_or(0) >= 1 => {
                            if let Ok(meta) = bincode::deserialize::<Vec<NodeMetadata>>(&data) {
                                coll.count.store(meta.len(), Ordering::Relaxed);
                                *coll.metadata.write() = meta;
                            }
                        }
                        None => {
                            if let Ok(v0) = bincode::deserialize::<Vec<NodeMetadataV0>>(&data) {
                                // Pre-A2 String layout — migrate to interned u32 in step 3.
                                coll.count.store(v0.len(), Ordering::Relaxed);
                                legacy_meta.insert(name.clone(), v0);
                            }
                        }
                    }
                }
                self.collections.insert(name, Arc::new(coll));
            }
        } else if let Ok(data) = fs::read(self.path.join("vector.bin")) {
            // Legacy single-space DB -> wrap as the `default` collection.
            let dim = state_val["vector_dim"].as_u64().unwrap_or(1536) as u16;
            let coll = VectorCollection::new(
                "default".to_string(),
                "default".to_string(),
                dim,
                Metric::L2,
                Quant::None,
                None,
                false,
                None, // legacy single-space DB is Quant::None + no rerank
                true,
                false, // Quant::None ⇒ no SQ8 calibration
            );
            *coll.arena.write() = ArenaStore::from_bytes(&data, Quant::None, dim as usize);
            if let Ok(md) = fs::read(self.path.join("meta.bin")) {
                match decode_metadata_snapshot(&md) {
                    Some(Ok(meta)) => {
                        coll.count.store(meta.len(), Ordering::Relaxed);
                        *coll.metadata.write() = meta;
                    }
                    Some(Err(e)) => {
                        panic!(
                            "corrupt metadata snapshot meta.bin: GBP1 magic present \
                             but postcard body failed to decode: {}",
                            e
                        );
                    }
                    None => {
                        // Legacy single-space snapshots always predate A2 (String layout).
                        if let Ok(v0) = bincode::deserialize::<Vec<NodeMetadataV0>>(&md) {
                            coll.count.store(v0.len(), Ordering::Relaxed);
                            legacy_meta.insert("default".to_string(), v0);
                        }
                    }
                }
            }
            self.collections
                .insert("default".to_string(), Arc::new(coll));
        } else {
            return false;
        }
        // Guarantee the default collection always exists post-load.
        if !self.collections.contains_key(&self.default_collection) {
            self.collections.insert(
                self.default_collection.clone(),
                Arc::new(VectorCollection::new(
                    self.default_collection.clone(),
                    "default".to_string(),
                    1536,
                    Metric::L2,
                    Quant::None,
                    None,
                    false,
                    None, // default fallback is Quant::None + no rerank
                    true,
                    false, // Quant::None ⇒ no SQ8 calibration
                )),
            );
        }

        // 2. Load Maps
        if let Ok(data) = fs::read(self.path.join("nodes.bin")) {
            match serde_json::from_slice::<Vec<(u32, NodeOutput)>>(&data) {
                Ok(nodes) => {
                    println!("Mark IX: Loading {} nodes from snapshot", nodes.len());
                    let mut max_u32 = 0;
                    for (k, v) in nodes {
                        if k > max_u32 {
                            max_u32 = k;
                        }
                        self.id_to_u32.insert(v.id.clone(), k);
                        // Rebuild the trigram index for this node id under its
                        // SAVED key `k` (no re-interning — `get_or_intern_id`
                        // would mint a fresh u32 and desync the maps). Without
                        // this, `find_fuzzy_id` is dead after every snapshot
                        // instant-load until new nodes are added. Nodes only —
                        // edges intentionally skip trigram
                        // (ADR--GENESISDB-EDGE-ID-INTERNING), and edges aren't
                        // loaded here.
                        for trigram in Self::tokenize_id(&v.id) {
                            self.trigram_index.entry(trigram).or_default().insert(k);
                        }
                        self.insert_node_lean(k, v);
                    }
                    self.next_u32.store(max_u32 + 1, Ordering::SeqCst);
                }
                Err(e) => {
                    println!("Mark IX: Failed to deserialize nodes: {}", e);
                    return false;
                }
            }
        } else {
            println!("Mark IX: nodes.bin not found");
        }

        if let Ok(data) = fs::read(self.path.join("edges.bin")) {
            // Deserialize as u128 tuples. Legacy snapshots wrote u32/u64 keys; JSON
            // numbers widen into u128 fine, and the saved key is ignored anyway
            // (re-derived below), so all widths load transparently.
            if let Ok(edges) = serde_json::from_slice::<Vec<(u128, EdgeOutput)>>(&data) {
                println!("Mark IX: Loading {} edges from snapshot", edges.len());
                for (_saved_k, v) in edges {
                    // Re-derive the edge key deterministically from the edge id
                    // (ADR--GENESISDB-EDGE-NUMERIC-KEYS). This reproduces the
                    // same u64 the edge was saved under for new snapshots, and
                    // re-keys legacy u32 snapshots consistently across the edges
                    // map + out_idx/in_idx — so adjacency never desyncs. Edges
                    // carry no `id_to_u32` entry, and edge keys are NOT in the
                    // node `next_u32` id-space, so no counter bump is needed.
                    let k = Self::edge_key(&v.id);
                    let from_u32 = self.get_or_intern_id(&v.from);
                    let to_u32 = self.get_or_intern_id(&v.to);
                    self.out_idx.entry(from_u32).or_default().insert(k);
                    self.in_idx.entry(to_u32).or_default().insert(k);
                    self.edges.insert(k, v);
                }
            }
        }

        // 3. Rebuild each collection's node_to_arena from its metadata (needs
        //    id_to_u32, populated by the node load above). HNSW itself is
        //    rehydrated from the arenas by the caller (open -> rehydrate_hnsw_index).
        for c in self.collections.iter() {
            let coll = c.value();
            if let Some(v0s) = legacy_meta.remove(coll.name.as_str()) {
                // Pre-A2 migration: intern each String id (now that id_to_u32 is
                // ready) into the new u32 metadata, and rebuild node_to_arena.
                let mut migrated = Vec::with_capacity(v0s.len());
                for v0 in v0s {
                    let nu = self
                        .get_u32(&v0.node_id)
                        .unwrap_or_else(|| self.get_or_intern_id(&v0.node_id));
                    coll.node_to_arena.insert(nu, v0.arena_id);
                    migrated.push(NodeMetadata {
                        arena_id: v0.arena_id,
                        node_u32: nu,
                        timestamp: v0.timestamp,
                        vector_dim: v0.vector_dim,
                        embedding_offset: v0.embedding_offset,
                        gks_attributes: v0.gks_attributes,
                        lang: v0.lang,
                        cluster_id: v0.cluster_id,
                    });
                }
                *coll.metadata.write() = migrated;
            } else {
                let meta = coll.metadata.read();
                for m in meta.iter() {
                    coll.node_to_arena.insert(m.node_u32, m.arena_id);
                }
            }
        }

        // 4. Sync Global State (clock) from the already-parsed manifest.
        if let Some(clock) = state_val["logical_clock"].as_u64() {
            self.logical_clock.store(clock as u32, Ordering::SeqCst);
        }

        println!("Mark IX: Instant load complete in {:?}", start.elapsed());
        true
    }

    pub fn execute_batch(&self, input: BatchInput) -> Result<BatchOutput> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();

        // 1. Validation Phase (All-or-Nothing)
        for node in &input.nodes {
            self.validate_governance(&node.labels, false)?;
        }

        let mut output_nodes = Vec::with_capacity(input.nodes.len());
        let mut output_edges = Vec::with_capacity(input.edges.len());
        let mut events = Vec::with_capacity(input.nodes.len() + input.edges.len());

        // 2. Processing Phase (In-Memory Prep)
        let now = Utc::now();

        for args in input.nodes {
            let id = args.id.unwrap_or_else(|| format!("N-{}", Uuid::new_v4()));
            let lang = args.lang.unwrap_or("en".to_string());
            let expires_at = args
                .ttl
                .map(|s| (now + chrono::Duration::seconds(s as i64)).to_rfc3339());

            // Resolve + dim-validate the target collection up-front so a bad
            // vector fails the whole batch BEFORE the WAL write (all-or-nothing).
            let coll_name = if let Some(emb) = &args.embedding {
                let coll = self.resolve_collection(&args.collection)?;
                if emb.len() != coll.dim as usize {
                    return Err(Error::from_reason(format!(
                        "embedding dim {} != collection '{}' dim {}",
                        emb.len(),
                        coll.name,
                        coll.dim
                    )));
                }
                Some(coll.name.clone())
            } else {
                None
            };

            let node = NodeOutput {
                id: id.clone(),
                labels: args.labels,
                props: args.props.unwrap_or(Value::Object(Default::default())),
                impact: Some(0.7),
                embedding: args.embedding.clone(),
                lang: Some(lang.clone()),
                valid_from: args.valid_from.unwrap_or_else(|| now.to_rfc3339()),
                valid_to: None,
                caused_by: args.caused_by,
                expires_at,
                clock: self.next_clock(),
                collection: coll_name,
            };

            events.push(Event::Node(node.clone()));
            output_nodes.push(node);
        }

        for args in input.edges {
            let edge = EdgeOutput {
                id: args.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                from: args.from,
                to: args.to,
                rel: args.rel,
                props: args.props.unwrap_or(Value::Object(Default::default())),
                valid_from: Utc::now().to_rfc3339(),
                valid_to: None,
                recorded_at: Utc::now().to_rfc3339(),
                superseded_by: None,
                impact: args.impact,
                caused_by: args.caused_by,
                clock: self.next_clock(),
            };
            events.push(Event::Edge(edge.clone()));
            output_edges.push(edge);
        }

        // 3. Persistence Phase (Atomic WAL Write)
        self.persist(&Event::Batch(events.clone()))?;

        // 4. Memory Index Phase — collect vectors per collection and build each
        //    HNSW graph once via parallel_insert instead of N single inserts.
        //    Items grouped by collection: (node_u32, node_id, emb, lang).
        let mut by_coll: HashMap<String, CollVecBatch> = HashMap::new();
        for event in events {
            match event {
                Event::Node(n) => {
                    let u32_id = self.get_or_intern_id(&n.id);
                    if let Some(emb) = n.embedding.clone() {
                        let cn = n
                            .collection
                            .clone()
                            .unwrap_or_else(|| self.default_collection.clone());
                        by_coll.entry(cn).or_default().push((
                            u32_id,
                            n.id.clone(),
                            emb,
                            n.lang.clone().unwrap_or_else(|| "en".to_string()),
                        ));
                    }
                    self.insert_node_lean(u32_id, n);
                }
                Event::Edge(e) => {
                    let u32_id = self.index_edge_internal(&e.id, &e.from, &e.to);
                    self.edges.insert(u32_id, e);
                }
                _ => {}
            }
        }
        for (cn, items) in by_coll {
            // Collection existence was validated in the processing phase. Stage
            // all vectors synchronously (durable arena), then defer the HNSW
            // build to the indexing thread as one parallel_insert job.
            if let Ok(coll) = self.resolve_collection(&Some(cn)) {
                let staged: Vec<(Vec<f32>, u32)> = items
                    .into_iter()
                    .map(|(nu, _id, emb, lang)| {
                        let e = coll.prep(emb);
                        let aid = coll.stage(nu, &e, lang);
                        (e, aid)
                    })
                    .collect();
                self.enqueue_batch(&coll, staged);
            }
        }

        Ok(BatchOutput {
            nodes: output_nodes,
            edges: output_edges,
        })
    }

    pub fn perform_index_compaction(&self) -> Result<()> {
        println!("Mark IX: Starting Index Compaction...");
        let start = Instant::now();
        // Drain pending HNSW inserts first — compaction reassigns arena ids, so
        // a queued insert with a stale id must not run against the new arena.
        self.flush_index();

        // 1. Identify Live Set
        let live_nodes: HashSet<u32> = self.nodes.iter().map(|e| *e.key()).collect();

        // 2. Compact each collection's arena independently (drop dead-node slots,
        //    rebuild node_to_arena), then rehydrate its HNSW.
        for c in self.collections.iter() {
            let coll = c.value();
            let mut meta_arena = coll.metadata.write();
            let mut vec_arena = coll.arena.write();
            // Rerank sidecar is compacted in lock-step (lock order: meta → arena
            // → sidecar). Backing is now ON DISK. We first collect each surviving
            // old row index (keyed by `embedding_offset/dim` in the OLD arena — the
            // SAME order the arena remap uses), THEN rewrite the on-disk fvec IN
            // PLACE through the held reader: truncate to empty and re-append each
            // live row via positioned writes. No tmp file / rename (which fails
            // over an open handle on Windows) and no resident Vec<f32> — rows are
            // read one at a time off disk and written one at a time back. All under
            // the held sidecar write guard, so no reader sees a half-rewritten file.
            let sidecar_guard = coll.f32_sidecar.as_ref().map(|s| s.write());
            let dim = coll.dim as usize;

            let mut new_meta = Vec::with_capacity(live_nodes.len());
            let mut new_vec = ArenaStore::new(coll.quant, coll.dim as usize);
            // Old arena row indices of the survivors, in new-arena order.
            let mut surviving_old_rows: Vec<usize> = Vec::new();
            coll.node_to_arena.clear();

            for meta in meta_arena.iter() {
                {
                    let u32_id = meta.node_u32; // A2: id interned in metadata
                    if live_nodes.contains(&u32_id) {
                        let start_off = meta.embedding_offset as usize;
                        let len = meta.vector_dim as usize;
                        if start_off + len <= vec_arena.len() {
                            let new_offset = new_vec.len() as u64;
                            new_vec.append_range(&vec_arena, start_off, len);
                            if sidecar_guard.is_some() {
                                surviving_old_rows.push(start_off.checked_div(dim).unwrap_or(0));
                            }
                            let new_arena_id = new_meta.len() as u32;
                            let mut meta_clone = meta.clone();
                            meta_clone.arena_id = new_arena_id;
                            meta_clone.embedding_offset = new_offset;
                            coll.node_to_arena.insert(u32_id, new_arena_id);
                            new_meta.push(meta_clone);
                        }
                    }
                }
            }
            coll.count.store(new_meta.len(), Ordering::Relaxed);
            *meta_arena = new_meta;
            *vec_arena = new_vec;

            // In-place on-disk rewrite of the sidecar to match the compacted arena.
            // Read every survivor row off the OLD file first (into a transient
            // per-row buffer, one at a time), truncate the file, then append the
            // rows back in the new order. On any I/O error we truncate to empty so
            // the load-time len guard drops the (now-desynced) sidecar to quantized,
            // never rerank on torn data. `f32_sidecar` lives behind an `Arc`, so the
            // Option can't be nulled here — the reader's file just ends up empty.
            if let Some(guard) = sidecar_guard.as_deref() {
                // Snapshot the old rows before truncation (positioned reads off the
                // still-intact file; each is dropped after we buffer its bytes).
                let mut rewrite_ok = true;
                let mut row_bytes: Vec<Vec<f32>> = Vec::with_capacity(surviving_old_rows.len());
                for &old_row in &surviving_old_rows {
                    match guard.row(old_row) {
                        Some(r) => row_bytes.push(r),
                        None => {
                            rewrite_ok = false;
                            break;
                        }
                    }
                }
                // Truncate to empty (also clears the stale LRU) then re-append.
                if guard.truncate_empty().is_err() {
                    rewrite_ok = false;
                }
                if rewrite_ok {
                    for r in &row_bytes {
                        if guard.write_rows(r).is_err() {
                            // Partial write: truncate back to empty so we never
                            // present a torn tail; degrade to quantized on reload.
                            let _ = guard.truncate_empty();
                            break;
                        }
                    }
                }

                // P1a — BQ per-dim centering. `row_bytes` holds every surviving
                // row's EXACT f32 (raw, uncentered — the sidecar keeps it raw) in
                // new-arena order, so this is the moment the mean can be computed.
                // We (1) install the new center and (2) re-pack the BQ arena with
                // it, so the arena codes, the rehydrated HNSW (step 3, from the
                // arena), and every subsequent insert/query all agree on one center.
                // Non-BQ collections and empty/failed rewrites are left untouched.
                if rewrite_ok && coll.quant == Quant::Binary && !row_bytes.is_empty() {
                    let mut center = vec![0.0f32; dim];
                    for r in &row_bytes {
                        for (i, &x) in r.iter().take(dim).enumerate() {
                            center[i] += x;
                        }
                    }
                    let inv = 1.0 / row_bytes.len() as f32;
                    for m in center.iter_mut() {
                        *m *= inv;
                    }
                    // Re-pack the arena as centered sign codes; offsets stay i*dim
                    // (fixed dim), matching the meta already written above.
                    let mut recentered = ArenaStore::new(coll.quant, dim);
                    for r in &row_bytes {
                        recentered.push_f32(r, Some(&center));
                    }
                    *vec_arena = recentered;
                    *coll.bq_center.write() = Some(center);
                }

                // P2b — SQ8 calibrated scale (opt-in). Same moment/structure as BQ
                // centering: derive a robust [lo,hi] quantile range from the
                // survivors' exact f32, map it affinely to [0,255], and re-pack the
                // arena with the new (scale,bias) so its codes, the rehydrated HNSW,
                // and later inserts/queries all share one scale. A degenerate range
                // (all-equal) keeps the fixed scale.
                if rewrite_ok
                    && coll.quant == Quant::ScalarU8
                    && coll.sq8_calibrate
                    && !row_bytes.is_empty()
                {
                    let mut vals: Vec<f32> = row_bytes.iter().flatten().copied().collect();
                    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let n = vals.len();
                    let lo = vals[((n as f32 * 0.005) as usize).min(n - 1)];
                    let hi = vals[((n as f32 * 0.995) as usize).min(n - 1)];
                    if hi - lo > 1e-6 {
                        let scale = 255.0 / (hi - lo);
                        let bias = -lo * scale;
                        let mut recalibrated = ArenaStore::new(coll.quant, dim);
                        recalibrated.set_sq8_scale(scale, bias);
                        for r in &row_bytes {
                            recalibrated.push_f32(r, None);
                        }
                        *vec_arena = recalibrated;
                        *coll.sq8_scale.write() = Some((scale, bias));
                    }
                }
            }
        }

        // 3. Rebuild every collection's HNSW from its compacted arena.
        self.rehydrate_hnsw_index();

        // 4. Prune Adjacency Indices
        let mut orphaned_indices = Vec::new();
        for entry in self.out_idx.iter() {
            if !live_nodes.contains(entry.key()) {
                orphaned_indices.push(*entry.key());
            }
        }
        for k in orphaned_indices {
            self.out_idx.remove(&k);
        }

        let mut orphaned_in = Vec::new();
        for entry in self.in_idx.iter() {
            if !live_nodes.contains(entry.key()) {
                orphaned_in.push(*entry.key());
            }
        }
        for k in orphaned_in {
            self.in_idx.remove(&k);
        }

        println!(
            "Mark IX: Index Compaction complete in {:?}. Arenas resized to {} nodes.",
            start.elapsed(),
            live_nodes.len()
        );
        Ok(())
    }

    pub fn set_language_centroid(&self, lang: String, vector: Vec<f64>) {
        let v_f32: Vec<f32> = vector.into_iter().map(|v| v as f32).collect();
        self.lang_centroids.insert(lang, v_f32);
    }

    /// Order-independent digest of the current graph state. Two peers that hold
    /// the same nodes, edges, and per-node vector presence produce the SAME root
    /// regardless of the order events landed in their WALs. This is the gossip
    /// divergence trigger (Heartbeat compares roots → issue a PullRequest), so an
    /// order-independent root lets peers actually converge instead of re-pulling
    /// forever on insertion-order alone.
    ///
    /// Previously this hashed the WAL line-by-line in file order, so two peers at
    /// identical state but different write order diverged permanently. We digest
    /// the canonical in-memory state instead. Each entry is a length-prefixed field
    /// sequence (NOT delimiter-joined): node/edge ids are arbitrary user strings
    /// (may contain `|`/`\n`), so a joined format would let a crafted id forge
    /// another entry and collide two distinct states to one root. Nodes/edges carry
    /// `id, clock.time, clock.peer, valid_to` (clock = supersession/version,
    /// `valid_to` = edge retraction); each (collection, node-id) with a vector adds
    /// a presence entry so a vector-only difference still flips the root and pulls
    /// (the old WAL-hash root included `Event::Vector` lines — this preserves that
    /// signal). Presence only: a re-`add_vector` superseding the same
    /// (node, collection) with a different embedding is not distinguished here.
    pub fn get_merkle_root(&self) -> String {
        // Unambiguous entry encoding: each field is `u32 little-endian length ++ bytes`.
        fn entry(parts: &[&[u8]]) -> Vec<u8> {
            let mut b = Vec::new();
            for p in parts {
                b.extend_from_slice(&(p.len() as u32).to_le_bytes());
                b.extend_from_slice(p);
            }
            b
        }
        let mut entries: Vec<Vec<u8>> = Vec::with_capacity(self.nodes.len() + self.edges.len());
        for e in self.nodes.iter() {
            let n = e.value();
            let t = n.clock.time.to_le_bytes();
            entries.push(entry(&[
                b"N",
                n.id.as_bytes(),
                &t,
                n.clock.peer_id.as_bytes(),
                n.valid_to.as_deref().unwrap_or("").as_bytes(),
            ]));
        }
        for e in self.edges.iter() {
            let ed = e.value();
            let t = ed.clock.time.to_le_bytes();
            entries.push(entry(&[
                b"E",
                ed.id.as_bytes(),
                &t,
                ed.clock.peer_id.as_bytes(),
                ed.valid_to.as_deref().unwrap_or("").as_bytes(),
            ]));
        }
        // Vector presence per (collection, node id) — peer-independent string ids,
        // not local u32 interning. Restores the divergence signal for vectors.
        for c in self.collections.iter() {
            let coll = c.value();
            for kv in coll.node_to_arena.iter() {
                if let Some(node) = self.nodes.get(kv.key()) {
                    entries.push(entry(&[
                        b"V",
                        coll.name.as_bytes(),
                        node.value().id.as_bytes(),
                    ]));
                }
            }
        }
        if entries.is_empty() {
            return "0".repeat(64);
        }
        entries.sort();
        let mut hasher = Sha256::new();
        for e in &entries {
            hasher.update(e);
        }
        hex::encode(hasher.finalize())
    }

    /// Logical time of an event (max over a batch). `Event::Vector` now carries a
    /// clock, so it is time-filterable and included in anti-entropy pull deltas.
    fn event_time(e: &Event) -> Option<u32> {
        match e {
            Event::Node(n) => Some(n.clock.time),
            Event::Edge(ed) => Some(ed.clock.time),
            Event::Batch(v) => v.iter().filter_map(Self::event_time).max(),
            Event::Vector(v) => Some(v.clock.time),
            // Retractions replicate like node upserts (clock-stamped LWW).
            Event::NodeRetract { clock, .. } => Some(clock.time),
            Event::RelationalSchema(_) | Event::RelationalRows { .. } => None,
            Event::Transaction(transaction) => transaction
                .nodes
                .iter()
                .map(|node| node.clock.time)
                .chain(transaction.edges.iter().map(|edge| edge.clock.time))
                .chain(transaction.vectors.iter().map(|vector| vector.clock.time))
                .max(),
        }
    }

    /// Anti-entropy source side: the WAL `SignedEvent`s strictly newer than
    /// `from_clock`, sorted ascending by logical time. A peer that pulls these and
    /// applies them via `reconcile_state` converges its graph state toward ours;
    /// because they advance the requester's clock, the next round pulls only the
    /// remainder (so batching by the caller is safe). Events are already signed by
    /// their original author, so they verify on the receiver.
    /// `Event::Vector` (secondary add_vector embeddings) now carries a clock too, so
    /// it is included here and replicates like nodes/edges. (Legacy pre-clock WAL
    /// entries deserialize to a zero clock and are not `> from_clock`, so they don't
    /// re-sync on their own — re-`add_vector` re-stamps them with a live clock.)
    pub fn events_since(&self, from_clock: u32) -> Vec<SignedEvent> {
        let mut out: Vec<(u32, SignedEvent)> = Vec::new();
        self.scan_journal(None, true, &mut |_, se| {
            if let Some(t) = Self::event_time(&se.event) {
                if t > from_clock {
                    out.push((t, se));
                }
            }
        });
        out.sort_by_key(|(t, _)| *t);
        out.into_iter().map(|(_, se)| se).collect()
    }

    /// WP-1.2 (ADR D4): serve frames newer than a responder-domain commit_seq
    /// cursor — the new sync path. Unlike the Lamport filter above this also
    /// serves relational-only transactions (every frame has a seq), closing the
    /// `event_time = None` gap. Frame order IS local order, so no re-sort.
    pub fn events_since_seq(&self, from_seq: u64) -> Vec<SignedEvent> {
        // Cursor 0 = the peer has nothing, so send everything (including a base
        // segment folded at seq 0); any other value is an exclusive cursor.
        let cursor = if from_seq == 0 { None } else { Some(from_seq) };
        let mut out: Vec<SignedEvent> = Vec::new();
        self.scan_journal(cursor, false, &mut |_, se| {
            out.push(se);
        });
        out
    }

    /// Reconstruct a live node's primary embedding from its collection arena, at
    /// the arena's resident fidelity (exact for `Quant::None`, dequantized
    /// otherwise — the SAME fidelity the snapshot restores, since the snapshot also
    /// persists the arena, not the original f64). `compact` needs this because the
    /// resident `NodeOutput` is stored lean (`insert_node_lean` nulls `embedding`),
    /// so a node cloned out of `self.nodes` carries no vector; without
    /// re-attaching it the rewritten WAL would silently drop every embedding and
    /// stop being a self-contained recovery source. Streamed per node, so it never
    /// materializes all embeddings at once. Returns `None` for vector-less nodes.
    fn reconstruct_embedding(&self, node: &NodeOutput, node_u32: u32) -> Option<Vec<f64>> {
        let coll = self.resolve_collection(&node.collection).ok()?;
        let aid = *coll.node_to_arena.get(&node_u32)?;
        let meta = coll.metadata.read();
        let m = meta.get(aid as usize)?;
        let (start, len) = (m.embedding_offset as usize, m.vector_dim as usize);
        let arena = coll.arena.read();
        if start + len > arena.len() {
            return None;
        }
        Some(
            arena
                .f32_at(start, len)
                .into_iter()
                .map(|x| x as f64)
                .collect(),
        )
    }

    pub fn compact(&self) -> Result<()> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        self.compact_unlocked()
    }

    fn compact_unlocked(&self) -> Result<()> {
        let frontier_seq = self.stable_frontier();
        let (buf, count) = self.build_compacted_wal()?;
        self.checkpoint_wal_payload(buf, frontier_seq, count)
    }

    /// Serialize current live state as the fold payload (the `SignedEvent`
    /// JSONL lines the writer thread frames into a base segment at the fold
    /// frontier — WP-1.2). Split out of `compact_unlocked` so
    /// `save_state_unlocked` can build the payload, fold FIRST (I7/I9), then
    /// snapshot. Callers must hold `commit_lock` (writers serialize on it, so
    /// live state cannot move between the build and the fold).
    fn build_compacted_wal(&self) -> Result<(Vec<u8>, usize)> {
        self.ensure_writable()?;
        // Build the live-state snapshot in memory, then hand it to the WAL thread
        // to atomically replace the file (serialized with appends; see WalMsg).
        let mut buf: Vec<u8> = Vec::new();

        let now = Utc::now().to_rfc3339();
        let mut count = 0;

        // 1. Write current live nodes. The resident node is lean (no embedding),
        //    so re-attach its vector from the arena — otherwise a WAL-only reload
        //    (no snapshot) would reconstruct the graph but lose every embedding.
        for entry in self.nodes.iter() {
            let node = entry.value();
            if let Some(exp) = &node.expires_at {
                if now > *exp {
                    continue;
                }
            }
            if node.valid_to.is_none() {
                let mut node = node.clone();
                node.props = self.hydrated_props(*entry.key(), &node);
                if node.embedding.is_none() {
                    node.embedding = self.reconstruct_embedding(&node, *entry.key());
                }
                if let Ok(json) = serde_json::to_string(&self.sign_event(&Event::Node(node))) {
                    buf.extend_from_slice(json.as_bytes());
                    buf.push(b'\n');
                    count += 1;
                }
            }
        }

        // 2. Write current live edges
        for entry in self.edges.iter() {
            let edge = entry.value();
            if edge.valid_to.is_none() {
                if let Ok(json) =
                    serde_json::to_string(&self.sign_event(&Event::Edge(edge.clone())))
                {
                    buf.extend_from_slice(json.as_bytes());
                    buf.push(b'\n');
                    count += 1;
                }
            }
        }

        // 3. Carry forward live secondary vectors (Event::Vector from add_vector).
        // Primary embeddings ride on Event::Node (written above, lossless); secondary
        // ones live only in the WAL, and the resident arena is potentially quantized
        // — so reconstructing them losslessly means reading the pre-compact WAL, not
        // the arena. Keep the latest (highest-clock) vector per (node, collection)
        // for still-live nodes; drop the rest.
        {
            let mut latest: HashMap<(String, Option<String>), VectorEvent> = HashMap::new();
            // Pre-fold journal scan (segments + active; legacy too — secondary
            // vectors may predate migration): keep the latest per (node,
            // collection) for still-live nodes.
            self.scan_journal(None, true, &mut |_, se| {
                if let Event::Vector(v) = se.event {
                    let live = self
                        .get_u32(&v.node_id)
                        .is_some_and(|u| self.nodes.contains_key(&u));
                    if !live {
                        return;
                    }
                    let key = (v.node_id.clone(), v.collection.clone());
                    match latest.get(&key) {
                        Some(prev) if prev.clock.time >= v.clock.time => {}
                        _ => {
                            latest.insert(key, v);
                        }
                    }
                }
            });
            for v in latest.into_values() {
                if let Ok(json) = serde_json::to_string(&self.sign_event(&Event::Vector(v))) {
                    buf.extend_from_slice(json.as_bytes());
                    buf.push(b'\n');
                    count += 1;
                }
            }
        }

        // 4. Checkpoint the current relational state as Genesis events. SQLite is
        // a rebuildable projection, so compaction must never make it the only copy.
        for event in self.relational_snapshot_events()? {
            let json = serde_json::to_string(&self.sign_event(&event))
                .map_err(|e| Error::from_reason(e.to_string()))?;
            buf.extend_from_slice(json.as_bytes());
            buf.push(b'\n');
            count += 1;
        }

        // 5. Preserve transaction identities and stable frontier across WAL
        // compaction without replaying their logical mutations a second time.
        for event in self.transaction_checkpoint_events()? {
            let json = serde_json::to_string(&self.sign_event(&event))
                .map_err(|e| Error::from_reason(e.to_string()))?;
            buf.extend_from_slice(json.as_bytes());
            buf.push(b'\n');
            count += 1;
        }

        Ok((buf, count))
    }

    /// Hand a live-state payload (JSONL SignedEvent lines) to the writer thread
    /// for a fold-to-base checkpoint and block until the base segment is
    /// durable and the journal rotated (I9; executes on the handle owner).
    fn checkpoint_wal_payload(
        &self,
        payload_lines: Vec<u8>,
        frontier_seq: u64,
        count: usize,
    ) -> Result<()> {
        let (ack_tx, ack_rx) = unbounded();
        self.wal_sender
            .send(WalMsg::Checkpoint {
                payload_lines,
                frontier_seq,
                ack: ack_tx,
            })
            .map_err(|_| Error::from_reason("wal disconnected"))?;
        if !ack_rx.recv().unwrap_or(false) {
            return Err(Error::from_reason("journal fold failed"));
        }
        println!(
            "Journal folded to base segment @seq {} ({} live events).",
            frontier_seq, count
        );
        Ok(())
    }

    // === WP-1.2 journal machinery (SPEC--GENESISDB-JOURNAL-FORMAT-V1) ===

    fn journal_seg_dir(root: &std::path::Path) -> PathBuf {
        root.join("journal")
    }

    /// I9 hygiene at open: crashed seals leave `*.tmp` — delete them. Their
    /// frames are still in the (untruncated) active file or older segments.
    fn journal_cleanup(root: &std::path::Path) {
        let seg_dir = Self::journal_seg_dir(root);
        if let Ok(entries) = fs::read_dir(&seg_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "tmp") {
                    let _ = fs::remove_file(&p);
                }
            }
        }
    }

    /// I9: truncate a torn active-file tail (crash mid-append) so the writer
    /// thread appends after the last VALID frame. A partial header truncates to
    /// zero (the writer rewrites it).
    fn journal_truncate_torn_active(active: &std::path::Path) {
        let Ok(bytes) = fs::read(active) else {
            return;
        };
        let valid_end = if bytes.len() < ACTIVE_HEADER_LEN || bytes[0..4] != ACTIVE_MAGIC {
            0
        } else {
            walk_frames(&bytes, ACTIVE_HEADER_LEN, |_, _| {})
        };
        if (valid_end as u64) < bytes.len() as u64 {
            if let Ok(f) = FileOpenOptions::new().write(true).open(active) {
                let _ = f.set_len(valid_end as u64);
                let _ = f.sync_all();
            }
        }
    }

    /// All sealed segments, sorted by (min_seq, max_seq). Unreadable headers
    /// are skipped (recovery degrades explicitly at read time, never silently).
    fn journal_list_segments(root: &std::path::Path) -> Vec<SegmentInfo> {
        let mut out: Vec<SegmentInfo> = Vec::new();
        if let Ok(entries) = fs::read_dir(Self::journal_seg_dir(root)) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "gseg") {
                    if let Some(info) = read_segment_header(&p) {
                        out.push(info);
                    }
                }
            }
        }
        out.sort_by_key(|s| (s.min_seq, s.max_seq));
        out
    }

    /// Seed for the writer thread's frame counter: max stamped seq across
    /// sealed segments and the (already tear-truncated) active file. Never read
    /// from the projection (ADR D2.4).
    fn journal_max_seq(root: &std::path::Path, active: &std::path::Path) -> u64 {
        let mut max_seq = Self::journal_list_segments(root)
            .into_iter()
            .map(|s| s.max_seq)
            .max()
            .unwrap_or(0);
        if let Ok(bytes) = fs::read(active) {
            if bytes.len() >= ACTIVE_HEADER_LEN && bytes[0..4] == ACTIVE_MAGIC {
                walk_frames(&bytes, ACTIVE_HEADER_LEN, |seq, _| {
                    if seq > max_seq {
                        max_seq = seq;
                    }
                });
            }
        }
        max_seq
    }

    /// Writer-thread helper: open (or create+header) the active journal file
    /// for append.
    fn journal_open_active(
        active: &std::path::Path,
        next_seq: u64,
    ) -> Option<std::io::BufWriter<File>> {
        let file = FileOpenOptions::new()
            .append(true)
            .create(true)
            .open(active)
            .ok()?;
        let mut writer = std::io::BufWriter::with_capacity(128 * 1024, file);
        let len = writer.get_ref().metadata().map(|m| m.len()).unwrap_or(0);
        if len == 0 {
            writer.write_all(&active_header(next_seq)).ok()?;
            writer.flush().ok()?;
            writer.get_mut().sync_all().ok()?;
        }
        Some(writer)
    }

    fn journal_active_len(writer: &std::io::BufWriter<File>) -> u64 {
        writer.get_ref().metadata().map(|m| m.len()).unwrap_or(0)
    }

    /// Writer-thread helper: replace the active file with a fresh header-only
    /// file through a truncate handle, then restore an append handle (the
    /// PR #28 dance — only the handle owner can do this on Windows).
    fn journal_reset_active(
        writer: &mut std::io::BufWriter<File>,
        active: &std::path::Path,
        next_seq: u64,
    ) -> bool {
        let Ok(f) = FileOpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(active)
        else {
            return false;
        };
        *writer = std::io::BufWriter::with_capacity(128 * 1024, f);
        let ok = writer.write_all(&active_header(next_seq)).is_ok()
            && writer.flush().is_ok()
            && writer.get_mut().sync_all().is_ok();
        if let Ok(f) = FileOpenOptions::new()
            .append(true)
            .create(true)
            .open(active)
        {
            *writer = std::io::BufWriter::with_capacity(128 * 1024, f);
        }
        ok
    }

    /// Writer-thread helper: seal the current active file's frames into a
    /// history segment (size-threshold seal, spec §2), then start a fresh
    /// active file. Seal durability strictly precedes the reset (I9).
    fn journal_seal_active(
        writer: &mut std::io::BufWriter<File>,
        root: &std::path::Path,
        active: &std::path::Path,
        next_seq: u64,
    ) -> bool {
        let _ = writer.flush();
        let _ = writer.get_mut().sync_all();
        let Ok(bytes) = fs::read(active) else {
            return false;
        };
        if bytes.len() <= ACTIVE_HEADER_LEN || bytes[0..4] != ACTIVE_MAGIC {
            return true; // nothing to seal
        }
        let mut min_seq = u64::MAX;
        let mut max_seq = 0u64;
        let mut count: u32 = 0;
        let valid_end = walk_frames(&bytes, ACTIVE_HEADER_LEN, |seq, _| {
            min_seq = min_seq.min(seq);
            max_seq = max_seq.max(seq);
            count += 1;
        });
        if count == 0 {
            return true;
        }
        let seg_path = Self::journal_seg_dir(root).join(format!("J{min_seq:012}.gseg"));
        if write_segment(
            &seg_path,
            SEG_KIND_HISTORY,
            CODEC_LZ4,
            min_seq,
            max_seq,
            count,
            &bytes[ACTIVE_HEADER_LEN..valid_end],
        )
        .is_err()
        {
            return false;
        }
        Self::journal_reset_active(writer, active, next_seq)
    }

    /// Writer-thread helper: fold-to-base checkpoint (ADR D3 `frontier_only`
    /// interim profile). Frames every payload JSONL line at `frontier_seq` into
    /// a base segment, then drops all other journal files and resets the active
    /// file. Order (I9): base durable → drop olds → reset active. A crash at
    /// any point leaves a journal that fully recovers live state (frames ≤
    /// frontier re-apply idempotently under LWW).
    fn journal_fold(
        writer: &mut std::io::BufWriter<File>,
        root: &std::path::Path,
        active: &std::path::Path,
        payload_lines: &[u8],
        frontier_seq: u64,
        next_seq: u64,
    ) -> bool {
        let _ = writer.flush();
        let _ = writer.get_mut().sync_all();
        let mut body: Vec<u8> = Vec::with_capacity(payload_lines.len() + 4096);
        let mut count: u32 = 0;
        for line in payload_lines.split(|b| *b == b'\n') {
            if line.is_empty() {
                continue;
            }
            encode_frame(&mut body, frontier_seq, line);
            count += 1;
        }
        let seg_path = Self::journal_seg_dir(root).join(format!("B{frontier_seq:012}.gseg"));
        if write_segment(
            &seg_path,
            SEG_KIND_BASE,
            CODEC_LZ4,
            frontier_seq,
            frontier_seq,
            count,
            &body,
        )
        .is_err()
        {
            return false;
        }
        if let Ok(entries) = fs::read_dir(Self::journal_seg_dir(root)) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p != seg_path && p.extension().is_some_and(|e| e == "gseg") {
                    let _ = fs::remove_file(&p);
                }
            }
        }
        Self::journal_reset_active(writer, active, next_seq)
    }

    /// One-way migration (ADR §4): seal the legacy JSONL WAL as segment 0
    /// (kind=legacy_jsonl — recovery-only, pre-epoch, not tx-addressable) with
    /// bounded memory (two streaming passes: sha256, then lz4 frame), then
    /// remove the legacy file. Lossless: the segment stores the exact legacy
    /// bytes.
    fn migrate_legacy_wal(root: &std::path::Path, legacy: &std::path::Path) -> Result<()> {
        let seg_dir = Self::journal_seg_dir(root);
        fs::create_dir_all(&seg_dir).map_err(|e| Error::from_reason(e.to_string()))?;
        let body_len = fs::metadata(legacy)
            .map_err(|e| Error::from_reason(e.to_string()))?
            .len();
        // Pass 1: sha256 over the raw legacy bytes.
        let mut hasher = Sha256::new();
        {
            let mut f = File::open(legacy).map_err(|e| Error::from_reason(e.to_string()))?;
            let mut buf = vec![0u8; 1024 * 1024];
            loop {
                match f.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => hasher.update(&buf[..n]),
                    Err(e) => return Err(Error::from_reason(e.to_string())),
                }
            }
        }
        let sha: [u8; 32] = hasher.finalize().into();
        // Header/footer identical to write_segment, body lz4-frame-streamed.
        let mut header = Vec::with_capacity(SEG_HEADER_LEN);
        header.extend_from_slice(&SEG_MAGIC);
        header.extend_from_slice(&JOURNAL_FORMAT_VERSION.to_le_bytes());
        header.push(SEG_KIND_LEGACY_JSONL);
        header.push(CODEC_LZ4);
        header.extend_from_slice(&0u64.to_le_bytes());
        header.extend_from_slice(&0u64.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes());
        let mut crc_input = Vec::with_capacity(SEG_HEADER_LEN + 40);
        crc_input.extend_from_slice(&header);
        crc_input.extend_from_slice(&sha);
        crc_input.extend_from_slice(&body_len.to_le_bytes());
        let crc = crc32c::crc32c(&crc_input);

        let final_path = seg_dir.join("J000000000000.gseg");
        let tmp_path = final_path.with_extension("gseg.tmp");
        {
            let mut out = File::create(&tmp_path).map_err(|e| Error::from_reason(e.to_string()))?;
            out.write_all(&header)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            {
                // True streaming: FrameEncoder wraps the destination file
                // directly, so `std::io::copy` moves the legacy WAL through in
                // fixed-size chunks — the multi-GB legacy file is never
                // materialized whole (bounded memory, ADR §4/mobile budget).
                let mut enc = lz4_flex::frame::FrameEncoder::new(&mut out);
                let mut src = File::open(legacy).map_err(|e| Error::from_reason(e.to_string()))?;
                std::io::copy(&mut src, &mut enc).map_err(|e| Error::from_reason(e.to_string()))?;
                enc.finish()
                    .map_err(|e| Error::from_reason(e.to_string()))?;
            }
            out.write_all(&sha)
                .map_err(|e| Error::from_reason(e.to_string()))?;
            out.write_all(&body_len.to_le_bytes())
                .map_err(|e| Error::from_reason(e.to_string()))?;
            out.write_all(&crc.to_le_bytes())
                .map_err(|e| Error::from_reason(e.to_string()))?;
            out.sync_all()
                .map_err(|e| Error::from_reason(e.to_string()))?;
        }
        if final_path.exists() {
            fs::remove_file(&final_path).map_err(|e| Error::from_reason(e.to_string()))?;
        }
        fs::rename(&tmp_path, &final_path).map_err(|e| Error::from_reason(e.to_string()))?;
        fs::remove_file(legacy).map_err(|e| Error::from_reason(e.to_string()))?;
        println!(
            "Journal migration: legacy WAL sealed as segment 0 ({} bytes raw).",
            body_len
        );
        Ok(())
    }

    /// Walk every journal event in recovery order — legacy sources (seq 0,
    /// pre-epoch) first, then sealed segments by seq range, then the active
    /// file — invoking `f(seq, event)` for frames with seq > `from_seq`.
    /// `include_legacy=false` on tail replay: the snapshot already covers the
    /// pre-epoch content (legacy events carry no stamp to filter by).
    /// `from_seq`: `None` = the caller has applied nothing, send everything;
    /// `Some(f)` = the caller is current through frame `f`, send frames after
    /// it. The distinction matters because a fold on a journal that never
    /// stamped a frame (e.g. a legacy DB migrated but not yet written to)
    /// produces a base segment at seq 0 holding real live state — under a bare
    /// `u64` cursor, `0 > 0` is false and that state would be silently skipped.
    fn scan_journal(
        &self,
        from_seq: Option<u64>,
        include_legacy: bool,
        f: &mut dyn FnMut(u64, SignedEvent),
    ) {
        use std::io::BufRead;
        let emit_lines = |bytes: &[u8], f: &mut dyn FnMut(u64, SignedEvent)| {
            for line in bytes.split(|b| *b == b'\n') {
                if line.is_empty() {
                    continue;
                }
                if let Ok(se) = serde_json::from_slice::<SignedEvent>(line) {
                    f(0, se);
                }
            }
        };
        // Pre-migration / read-only legacy file, in place.
        if include_legacy && self.legacy_log_path.exists() {
            if let Ok(file) = File::open(&self.legacy_log_path) {
                for line in std::io::BufReader::new(file).lines().map_while(|r| r.ok()) {
                    if let Ok(se) = serde_json::from_str::<SignedEvent>(&line) {
                        f(0, se);
                    }
                }
            }
        }
        for seg in Self::journal_list_segments(&self.path) {
            if seg.kind == SEG_KIND_LEGACY_JSONL {
                if include_legacy {
                    if let Some((_, body)) = read_segment_body(&seg.path) {
                        emit_lines(&body, f);
                    }
                }
                continue;
            }
            if from_seq.is_some_and(|cursor| seg.max_seq <= cursor) {
                continue;
            }
            if let Some((_, body)) = read_segment_body(&seg.path) {
                walk_frames(&body, 0, |seq, payload| {
                    if from_seq.is_none_or(|cursor| seq > cursor) {
                        if let Ok(se) = serde_json::from_slice::<SignedEvent>(payload) {
                            f(seq, se);
                        }
                    }
                });
            }
        }
        if let Ok(bytes) = fs::read(&self.log_path) {
            if bytes.len() > ACTIVE_HEADER_LEN && bytes[0..4] == ACTIVE_MAGIC {
                walk_frames(&bytes, ACTIVE_HEADER_LEN, |seq, payload| {
                    if from_seq.is_none_or(|cursor| seq > cursor) {
                        if let Ok(se) = serde_json::from_slice::<SignedEvent>(payload) {
                            f(seq, se);
                        }
                    }
                });
            }
        }
    }

    /// Apply journal events into memory (`None` = full replay, `Some(f)` =
    /// tail replay past a snapshot frontier). Vectors are staged only
    /// (index=false): rehydrate_hnsw_index after load builds every index once
    /// for all recovery paths. Idempotent: LWW upserts.
    fn replay_journal(&self, from_seq: Option<u64>, include_legacy: bool) {
        self.scan_journal(from_seq, include_legacy, &mut |seq, signed_event| {
            self.apply_replay_event(seq, signed_event.event);
        });
    }

    fn apply_replay_event(&self, seq: u64, event: Event) {
        match event {
            Event::Node(n) => {
                let u32_id = self.get_or_intern_id(&n.id);
                if let Some(emb) = n.embedding.clone() {
                    self.replay_vector(
                        &n.collection,
                        &n.id,
                        emb,
                        n.lang.clone().unwrap_or("en".to_string()),
                        false,
                    );
                }
                self.insert_node_lean(u32_id, n);
            }
            Event::Edge(e) => {
                let u32_id = self.index_edge_internal(&e.id, &e.from, &e.to);
                self.edges.insert(u32_id, e);
            }
            Event::Vector(v) => {
                self.replay_vector(
                    &v.collection,
                    &v.node_id,
                    v.embedding,
                    v.lang.clone().unwrap_or_else(|| "en".to_string()),
                    false,
                );
            }
            Event::Batch(events) => {
                for batch_event in events {
                    self.apply_replay_event(seq, batch_event);
                }
            }
            Event::RelationalSchema(_) | Event::RelationalRows { .. } => {
                // Relational state is rebuilt by projection_sync_on_open.
            }
            Event::Transaction(transaction) => {
                self.apply_transaction_memory(&transaction, false);
                self.txn_frontier.fetch_max(seq, Ordering::SeqCst);
            }
            // Durable retraction (RCA--SLICE0-DURABILITY defect 2): replay the
            // removal so a crash after the retraction ack no longer resurrects
            // the node. Frame order guarantees this runs after the node's own
            // upsert frames.
            Event::NodeRetract { id, .. } => {
                if let Some(u32_id) = self.get_u32(&id) {
                    self.retract_node_memory(&id, u32_id);
                }
            }
        }
    }
    /// Bitemporal retraction (soft-delete): set the edge's `valid_to` so it is no
    /// longer live in the current view, while preserving the relationship for
    /// time-travel queries (`as_of` before `at`, or `include_invalid = true`).
    /// `at` defaults to now. Returns the retracted edge, or `None` if no edge with
    /// `id` exists. Idempotent on the live/retracted distinction — re-retracting
    /// just moves `valid_to`.
    pub fn retract_edge(&self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> {
        self.ensure_writable()?;
        let _commit_guard = self.commit_lock.lock();
        let ekey = Self::edge_key(&id);
        let mut edge = match self.edges.get(&ekey) {
            Some(e) => e.value().clone(),
            None => return Ok(None),
        };
        edge.valid_to = Some(at.unwrap_or_else(|| Utc::now().to_rfc3339()));
        edge.clock = self.next_clock(); // advance for CRDT LWW: the retraction must win
        self.edges.insert(ekey, edge.clone());
        self.refresh_impacts(Some(vec![edge.to.clone()]));
        self.persist(&Event::Edge(edge.clone()))?;
        Ok(Some(edge))
    }
    pub fn status_sync(&self) -> DatabaseStatus {
        DatabaseStatus {
            open: true,
            read_only: self.read_only,
            page_cache_mb: 512,
        }
    }
    // Bulk paths route through execute_batch so each chunk is ONE Event::Batch =
    // ONE WAL fsync (vs one fsync per item). Chunked to bound the size of a
    // single serialized batch / fsync.
    const BULK_CHUNK: usize = 1024;
    pub fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> {
        let mut it = inputs.into_iter();
        loop {
            let chunk: Vec<NodeInput> = it.by_ref().take(Self::BULK_CHUNK).collect();
            if chunk.is_empty() {
                break;
            }
            self.execute_batch(BatchInput {
                nodes: chunk,
                edges: Vec::new(),
            })?;
        }
        Ok(())
    }
    pub fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> {
        let mut it = inputs.into_iter();
        loop {
            let chunk: Vec<EdgeInput> = it.by_ref().take(Self::BULK_CHUNK).collect();
            if chunk.is_empty() {
                break;
            }
            self.execute_batch(BatchInput {
                nodes: Vec::new(),
                edges: chunk,
            })?;
        }
        Ok(())
    }

    pub fn calculate_structural_gaps(&self) -> Result<Vec<GapSuggestion>> {
        let mut gaps = Vec::new();
        let mut cluster_centroids: HashMap<u32, Vec<f32>> = HashMap::new();
        let mut cluster_member_count: HashMap<u32, u32> = HashMap::new();
        let mut cluster_impact: HashMap<u32, f64> = HashMap::new();
        // Structural gaps are computed over the default collection's space.
        let coll = self.default_coll();
        let meta_arena = coll.metadata.read();
        let vec_arena = coll.arena.read();
        for meta in meta_arena.iter() {
            let c_id = meta.cluster_id;
            let start = meta.embedding_offset as usize;
            let len = meta.vector_dim as usize;
            if start + len <= vec_arena.len() {
                let vec = vec_arena.f32_at(start, len);
                let entry = cluster_centroids
                    .entry(c_id)
                    .or_insert_with(|| vec![0.0; meta.vector_dim as usize]);
                for (i, val) in vec.iter().enumerate() {
                    entry[i] += val;
                }
                *cluster_member_count.entry(c_id).or_insert(0) += 1;
                {
                    let u32_id = meta.node_u32; // A2: id interned in metadata
                    if let Some(node) = self.nodes.get(&u32_id) {
                        *cluster_impact.entry(c_id).or_insert(0.0) +=
                            node.value().impact.unwrap_or(0.0);
                    }
                }
            }
        }
        for (c_id, centroid) in cluster_centroids.iter_mut() {
            let count = cluster_member_count[c_id] as f32;
            for val in centroid.iter_mut() {
                *val /= count;
            }
        }
        let cluster_ids: Vec<u32> = cluster_centroids.keys().cloned().collect();
        for i in 0..cluster_ids.len() {
            for j in i + 1..cluster_ids.len() {
                let id_a = cluster_ids[i];
                let id_b = cluster_ids[j];
                let avg_impact_a = cluster_impact[&id_a] / cluster_member_count[&id_a] as f64;
                let avg_impact_b = cluster_impact[&id_b] / cluster_member_count[&id_b] as f64;
                if avg_impact_a < 0.5 || avg_impact_b < 0.5 {
                    continue;
                }
                let dist = DistL2 {}.eval(&cluster_centroids[&id_a], &cluster_centroids[&id_b]);
                let similarity = 1.0 / (1.0 + dist as f64);
                if similarity > 0.75 {
                    gaps.push(GapSuggestion {
                        cluster_a: id_a, cluster_b: id_b, similarity,
                        reason: format!("High-authority clusters ({:.2} & {:.2}) are semantically related but physically disconnected.", avg_impact_a, avg_impact_b),
                    });
                }
            }
        }
        Ok(gaps)
    }

    pub fn get_meta_history(&self, cluster_id: u32) -> Vec<SuperNode> {
        self.meta_history
            .get(&cluster_id)
            .map(|v| v.value().clone())
            .unwrap_or_default()
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        if !self.read_only {
            let _ = self.save_state();
        }
        // Shut the background workers down deterministically instead of leaving
        // them detached. Each worker's recv() loop exits only once its sender is
        // dropped; the senders are still-live struct fields here, so swap each for
        // a throwaway channel — that drops the original (no other clones exist) and
        // closes the queue — then join the thread so any in-flight WAL flush / HNSW
        // insert finishes before the process tears down. Pending (un-joined) index
        // jobs are safe to abandon: their vectors are already in the durable arena
        // and the HNSW rehydrates from it on load.
        let (dead_wal, _) = unbounded();
        drop(std::mem::replace(&mut self.wal_sender, dead_wal));
        let (dead_idx, _) = bounded(1);
        drop(std::mem::replace(&mut self.index_tx, dead_idx));
        if let Some(h) = self.wal_handle.take() {
            let _ = h.join();
        }
        if let Some(h) = self.index_handle.take() {
            let _ = h.join();
        }
        if !self.read_only {
            println!("Mark IX: Graceful shutdown. State saved.");
        }
    }
}

#[cfg_attr(feature = "napi-bindings", napi(object))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GapSuggestion {
    pub cluster_a: u32,
    pub cluster_b: u32,
    pub similarity: f64,
    pub reason: String,
}

#[cfg(feature = "napi-bindings")]
#[napi]
pub struct GenesisDatabase {
    inner: Arc<Storage>,
}

#[cfg(feature = "napi-bindings")]
#[napi]
impl GenesisDatabase {
    #[napi(factory)]
    pub fn open(opts: OpenOptions) -> Result<Self> {
        let storage = Arc::new(Storage::open(opts)?);
        Storage::start_autonomic_loop(Arc::clone(&storage));
        Storage::start_gossip_manager(Arc::clone(&storage));
        Ok(Self { inner: storage })
    }
    #[napi]
    pub async fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.bulk_add_nodes(inputs))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.bulk_add_edges(inputs))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn rebuild_index_parallel(&self) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.rebuild_index_parallel())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn add_node(&self, args: NodeInput) -> Result<NodeOutput> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.add_node(args))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.add_edge(args))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn register_relational_schema(&self, package_json: String) -> Result<u32> {
        let package = serde_json::from_str::<RelationalSchemaPackage>(&package_json)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.register_relational_schema(package))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn get_relational_schema(&self, namespace: String) -> Result<String> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let package = i.get_relational_schema(&namespace)?;
            serde_json::to_string(&package).map_err(|e| Error::from_reason(e.to_string()))
        })
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn apply_relational_batch(&self, batch_json: String) -> Result<String> {
        let batch = serde_json::from_str::<RelationalMutationBatch>(&batch_json)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let result = i.apply_relational_batch(batch)?;
            serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
        })
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn apply_relational_rows(
        &self,
        namespace: String,
        mutations_json: String,
    ) -> Result<()> {
        let mutations = serde_json::from_str::<Vec<RelationalRowMutation>>(&mutations_json)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.apply_relational_rows(&namespace, mutations))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn query_relational(&self, query_json: String) -> Result<String> {
        let query = serde_json::from_str::<RelationalQuery>(&query_json)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let rows = i.query_relational(query)?;
            serde_json::to_string(&rows).map_err(|e| Error::from_reason(e.to_string()))
        })
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn supersede_node(
        &self,
        id: String,
        new_props: Option<serde_json::Value>,
        caused_by: Option<String>,
    ) -> Result<NodeOutput> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.supersede_node(id, new_props, caused_by))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn execute_named_query(&self, request_json: String) -> Result<String> {
        let request = serde_json::from_str::<NamedQueryRequest>(&request_json)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let rows = i.execute_named_query(request)?;
            serde_json::to_string(&rows).map_err(|e| Error::from_reason(e.to_string()))
        })
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn commit_transaction(&self, transaction_json: String) -> Result<String> {
        let transaction = serde_json::from_str::<GenesisTransaction>(&transaction_json)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let result = i.commit_transaction(transaction)?;
            serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
        })
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?
    }
    /// WP-1.2 REDEFINITION (ADR D2.3, GBP1-noted): the FRAME frontier — the
    /// commit_seq of the last durable journal frame, advancing on EVERY
    /// mutation (previously: last transaction commit_sequence, which is now
    /// `txn_frontier`). SDKs that fed this into `expected_frontier` must switch
    /// to `txn_frontier`.
    #[napi]
    pub fn stable_frontier(&self) -> i64 {
        self.inner.stable_frontier().min(i64::MAX as u64) as i64
    }
    /// Frame seq of the last transaction frame — the value
    /// `GenesisTransaction.expected_frontier` CASes against (WP-1.2).
    #[napi]
    pub fn txn_frontier(&self) -> i64 {
        self.inner.txn_frontier().min(i64::MAX as u64) as i64
    }
    #[napi]
    pub async fn retract_edge(&self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.retract_edge(id, at))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn retrieve_context(
        &self,
        target_id: String,
        tier: String,
        budget: Option<u32>,
        fuzzy: bool,
    ) -> Result<ContextPackage> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.retrieve_context(&target_id, &tier, budget, fuzzy))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    /// Executes a versioned Typed Query IR request. Query IR is the primary
    /// machine contract; HQL remains available as a compatibility frontend.
    #[napi]
    pub async fn execute_query_ir(&self, request: Value) -> Result<Value> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.execute_query_ir_json(request))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub fn query_ir_capabilities(&self) -> Value {
        self.inner.query_ir_capabilities()
    }
    /// Executes an HQL query and returns the command result as JSON.
    /// Supports SEARCH, TRAVERSE, MATCH graph patterns, MATCH ... SIMILAR
    /// hybrid search, and CONTEXT retrieval forms.
    /// SEARCH/MATCH hybrid may omit `SIMILAR TO [vector]` to search by the
    /// target node's stored embedding.
    /// Hybrid MATCH accepts `K <n>`; SEARCH and hybrid accept `EF <n>` and
    /// `OVERSAMPLE <n>` tuning clauses.
    /// TRAVERSE supports `DIRECTION in|out|both` and `REL a|b` alternation.
    /// Seed and target ids may contain unquoted colons, such as `user:5`.
    #[napi]
    pub async fn execute_hql(&self, query: String) -> Result<Value> {
        let i = Arc::clone(&self.inner);
        let res = tokio::task::spawn_blocking(move || i.execute_hql(&query))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))??;
        Ok(serde_json::to_value(res).map_err(|e| Error::from_reason(e.to_string()))?)
    }
    #[napi]
    pub async fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.hybrid_search(args))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn neighbors(
        &self,
        seed: String,
        args: NeighborInput,
    ) -> Result<Vec<NeighborOutput>> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.neighbors(seed, args, false))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn save_state(&self) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.save_state())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn compact(&self) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.compact())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn create_collection(
        &self,
        name: String,
        model: String,
        dim: u32,
        metric: Option<String>,
        quant: Option<String>,
        ef_search: Option<u32>,
        rerank: Option<bool>,
    ) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            i.create_collection(name, model, dim, metric, quant, ef_search, rerank)
        })
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub fn list_collections(&self) -> Vec<CollectionInfo> {
        self.inner.list_collections()
    }
    #[napi]
    pub async fn add_vector(
        &self,
        node_id: String,
        collection: String,
        embedding: Vec<f64>,
    ) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.add_vector(node_id, collection, embedding))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn flush_index(&self) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.flush_index())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }
    #[napi]
    pub fn index_lag(&self) -> u32 {
        self.inner.index_lag()
    }
    #[napi]
    pub fn set_language_centroid(&self, lang: String, vector: Vec<f64>) {
        self.inner.set_language_centroid(lang, vector);
    }
    #[napi]
    pub fn set_index_params(&self, ef_construction: u32, ef_search: u32) {
        self.inner.set_index_params(ef_construction, ef_search);
    }
    #[napi]
    pub async fn detect_communities(&self) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.detect_communities())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn calculate_structural_gaps(&self) -> Result<Vec<GapSuggestion>> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.calculate_structural_gaps())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn generate_meta_graph(&self) -> Result<()> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.generate_meta_graph())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn get_meta_history(&self, cluster_id: u32) -> Result<Vec<SuperNode>> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || Ok(i.get_meta_history(cluster_id)))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn reconcile_state(&self, events_json: String) -> Result<()> {
        let i = Arc::clone(&self.inner);
        let events = serde_json::from_str::<Vec<SignedEvent>>(&events_json)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        tokio::task::spawn_blocking(move || i.reconcile_state(events))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    /// Anti-entropy source side: JSON of the signed events newer than `from_clock`
    /// (sorted by logical time). Pair with `reconcile_state` on the puller.
    #[napi]
    pub async fn events_since(&self, from_clock: u32) -> Result<String> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            serde_json::to_string(&i.events_since(from_clock))
                .map_err(|e| Error::from_reason(e.to_string()))
        })
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn semantic_verify(&self, event_json: String) -> Result<bool> {
        let i = Arc::clone(&self.inner);
        let event = serde_json::from_str::<Event>(&event_json)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        tokio::task::spawn_blocking(move || i.semantic_verify(&event))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn propose_consensus(
        &self,
        event_json: String,
        signature: Vec<u8>,
    ) -> Result<String> {
        let i = Arc::clone(&self.inner);
        let event = serde_json::from_str::<Event>(&event_json)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        tokio::task::spawn_blocking(move || i.propose_consensus(event, signature))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub async fn submit_vote(
        &self,
        proposal_id: String,
        peer_id: String,
        approve: bool,
        signature: Vec<u8>,
    ) -> Result<bool> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.submit_vote(proposal_id, peer_id, approve, signature))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi]
    pub fn sign_vote(&self, proposal_id: String, approve: bool) -> Vec<u8> {
        self.inner.sign_vote(proposal_id, approve)
    }
    #[napi]
    pub fn get_local_peer_id(&self) -> String {
        self.inner.local_peer_id.clone()
    }
    #[napi]
    pub fn get_logical_clock(&self) -> u32 {
        self.inner.logical_clock.load(Ordering::SeqCst)
    }
    #[napi]
    pub fn get_merkle_root(&self) -> String {
        self.inner.get_merkle_root()
    }
    #[napi]
    pub fn schema_version_sync(&self) -> u32 {
        SCHEMA_VERSION
    }
    #[napi]
    pub fn version_sync(&self) -> String {
        ENGINE_VERSION.to_string()
    }
    #[napi]
    pub fn status_sync(&self) -> DatabaseStatus {
        self.inner.status_sync()
    }
}
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn engine_name_sync() -> String {
    ENGINE_NAME.to_string()
}
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn schema_version_sync() -> u32 {
    SCHEMA_VERSION
}
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn version_sync() -> String {
    ENGINE_VERSION.to_string()
}
