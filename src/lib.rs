//! Genesis Block — high-performance hybrid semantic-graph engine.
//!
//! Mark VI: Collective Intelligence & Autonomic Substrate

#![deny(clippy::all)]

use std::collections::{HashSet, VecDeque, HashMap};
use std::fs::{self, File, OpenOptions as FileOpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use sha2::{Sha256, Digest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use rand::Rng;

use chrono::Utc;
use dashmap::DashMap;
use roaring::RoaringBitmap;
use hnsw_rs::prelude::*;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use crossbeam_channel::{unbounded, bounded, Sender, Receiver};

pub mod query;
use query::HqlCommand;

pub const SCHEMA_VERSION: u32 = 1;

// --- Types (PROTOCOL §3) ---

#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OpenOptions {
    pub path: String,
    pub page_cache_mb: Option<u32>,
    pub read_only: Option<bool>,
    pub vector_dim: Option<u32>,
}

#[napi(object)]
#[derive(Serialize, Deserialize, Debug)]
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

#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalClock {
    pub time: u32,
    pub peer_id: String,
}

#[napi(object)]
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

#[napi(object)]
#[derive(Serialize, Deserialize, Debug)]
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

#[napi(object)]
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

#[napi]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalingTier {
    H0 = 0,
    H1 = 1,
    H2 = 2,
    H3 = 3,
    H4 = 4,
    H5 = 5,
}

impl ScalingTier {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "H0" => ScalingTier::H0,
            "H1" => ScalingTier::H1,
            "H2" => ScalingTier::H2,
            "H3" => ScalingTier::H3,
            "H4" => ScalingTier::H4,
            "H5" => ScalingTier::H5,
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
        }
    }
}

#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContextPackage {
    pub nodes: Vec<NodeOutput>,
    pub edges: Vec<EdgeOutput>,
    pub super_nodes: Vec<SuperNode>,
    pub token_estimate: u32,
    pub reasoning_path: String,
}

#[napi(object)]
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryInput {
    pub from: Option<String>,
    pub to: Option<String>,
    pub rel: Option<String>,
    pub as_of: Option<String>,
    pub include_invalid: Option<bool>,
    pub limit: Option<u32>,
}

#[napi(object)]
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

#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NeighborOutput {
    pub node: NodeOutput,
    pub path: Vec<EdgeOutput>,
    pub depth: u32,
}

#[napi(object)]
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
}

#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DatabaseStatus {
    pub open: bool,
    pub read_only: bool,
    pub page_cache_mb: u32,
}

#[napi(object)]
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
}

#[napi(object)]
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
        verifying_key: Vec<u8>
    },
    PullRequest { 
        from_clock: u32,
        target_peer_id: String 
    },
    PushDelta { 
        events: Vec<SignedEvent> 
    },
    ConsensusPropose {
        proposal: ConsensusProposal,
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
    ProposeMutation(Event),
    AcknowledgeMutation(String), 
    RequestFragment(String),
}

#[napi(object)]
#[derive(Serialize, Deserialize, Debug)]
pub struct BatchInput {
    pub nodes: Vec<NodeInput>,
    pub edges: Vec<EdgeInput>,
}

#[napi(object)]
#[derive(Serialize, Deserialize, Debug)]
pub struct BatchOutput {
    pub nodes: Vec<NodeOutput>,
    pub edges: Vec<EdgeOutput>,
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

#[napi(object)]
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

#[napi(object)]
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
pub enum Metric { L2, Cosine }

impl Metric {
    fn parse(s: &str) -> Self {
        if s.eq_ignore_ascii_case("cosine") { Metric::Cosine } else { Metric::L2 }
    }
    fn as_str(&self) -> &'static str {
        match self { Metric::L2 => "L2", Metric::Cosine => "Cosine" }
    }
}

/// Per-collection vector quantization (ADR--GENESISDB-VECTOR-QUANTIZATION).
/// `None` keeps lossless f32 end-to-end — byte-identical to pre-quant DBs.
/// `ScalarU8` (SQ8) stores BOTH the resident arena and the HNSW as u8 (the
/// "full resident cut", ~4× vector RAM); symmetric quant.
/// `Binary` (BQ) packs each dim to one sign bit (u64 words) with a popcount-
/// Hamming HNSW — ~32× vector RAM, lossy.
/// Either quantized mode may opt into an **f32-sidecar rerank** (per-collection
/// `rerank` flag): the exact f32 vectors are kept in a `fvec_<name>.bin` sidecar
/// and used to re-score an over-fetched candidate set, recovering recall lost to
/// quantization (ADR--GENESISDB-VECTOR-QUANTIZATION). `None` collections never
/// allocate a sidecar — the arena is already exact f32.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Quant { None, ScalarU8, Binary }

impl Quant {
    fn parse(s: &str) -> Self {
        if s.eq_ignore_ascii_case("sq8") || s.eq_ignore_ascii_case("scalaru8") { Quant::ScalarU8 }
        else if s.eq_ignore_ascii_case("bq") || s.eq_ignore_ascii_case("binary") { Quant::Binary }
        else { Quant::None }
    }
    fn as_str(&self) -> &'static str {
        match self { Quant::None => "none", Quant::ScalarU8 => "sq8", Quant::Binary => "bq" }
    }
}

// SQ8 maps [-1,1] -> [0,255] with a FIXED affine scale, so concurrent async
// inserts agree without any of them having seen the whole data distribution.
// Cosine collections are already unit-normalized into [-1,1] (clean); L2 values
// outside [-1,1] clamp (documented limitation of the first cut).
const SQ8_SCALE: f32 = 127.5;
const SQ8_BIAS: f32 = 127.5;

#[inline]
fn sq8_q(v: f32) -> u8 {
    let q = (v * SQ8_SCALE + SQ8_BIAS).round();
    if q <= 0.0 { 0 } else if q >= 255.0 { 255 } else { q as u8 }
}
#[inline]
fn sq8_dq(q: u8) -> f32 { (q as f32 - SQ8_BIAS) / SQ8_SCALE }

// Binary quantization (BQ): one sign bit per dim, packed into u64 words. Distance
// is bit Hamming via popcount.
#[inline]
fn bq_words(dim: usize) -> usize { (dim + 63) / 64 }

/// Pack a prepared f32 vector to sign-bit codes (bit set iff component > 0).
#[inline]
fn bq_pack(emb: &[f32]) -> Vec<u64> {
    let mut w = vec![0u64; bq_words(emb.len())];
    for (i, &x) in emb.iter().enumerate() {
        if x > 0.0 { w[i >> 6] |= 1u64 << (i & 63); }
    }
    w
}

/// Expand `dim` sign bits back to ±1.0 f32 (for the heuristic f32 readers only —
/// meta-graph / clustering; never for search). Lossless on sign, not magnitude.
#[inline]
fn bq_unpack(words: &[u64], dim: usize) -> Vec<f32> {
    (0..dim).map(|i| if words[i >> 6] & (1u64 << (i & 63)) != 0 { 1.0 } else { -1.0 }).collect()
}

/// Default over-fetch multiplier for f32-sidecar rerank: pull `k * this` quantized
/// candidates from the HNSW, then re-score them exactly and keep the best `k*2`.
/// Bigger = better recall, more f32 distance work. BQ (1 bit/dim) benefits most.
const RERANK_OVERFETCH: usize = 8;

/// Exact Euclidean distance between two prepared f32 vectors. Used by the rerank
/// stage; reuses the same geometry as the F32 HNSW (`DistL2`). For Cosine
/// collections both the query and the sidecar vector are unit-normalized, so this
/// ranks by cosine (L2 on the unit sphere is monotonic in 1-cos). Exact match ⇒ 0.
#[inline]
fn exact_l2(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| { let d = x - y; d * d }).sum::<f32>().sqrt()
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
        for i in 0..a.len() { d += (a[i] ^ b[i]).count_ones(); }
        d as f32
    }
}

/// The resident vector arena, element type chosen by the collection's `Quant`.
/// Offsets/lengths are in scalar-component units (== dim), identical across
/// variants — so `NodeMetadata.embedding_offset`/`vector_dim` mean the same in
/// every mode; only the on-disk byte width differs (4 for f32, 1 for u8).
pub enum ArenaStore {
    F32(Vec<f32>),
    U8(Vec<u8>),
    /// BQ: bit-packed sign codes — `n` vectors × `bq_words(dim)` u64 words.
    /// `len()` still reports logical components (`n*dim`), so `embedding_offset`
    /// stays in component units exactly like the f32/u8 variants and the shared
    /// `start + len <= arena.len()` bounds checks remain valid.
    Binary { data: Vec<u64>, dim: usize, n: usize },
}

impl ArenaStore {
    fn new(q: Quant, dim: usize) -> Self {
        match q {
            Quant::None => ArenaStore::F32(Vec::new()),
            Quant::ScalarU8 => ArenaStore::U8(Vec::new()),
            Quant::Binary => ArenaStore::Binary { data: Vec::new(), dim, n: 0 },
        }
    }
    /// Number of scalar components stored (NOT bytes).
    pub fn len(&self) -> usize {
        match self {
            ArenaStore::F32(v) => v.len(),
            ArenaStore::U8(v) => v.len(),
            ArenaStore::Binary { n, dim, .. } => n * dim,
        }
    }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// Append a prepared f32 vector, quantizing per mode.
    fn push_f32(&mut self, emb: &[f32]) {
        match self {
            ArenaStore::F32(v) => v.extend_from_slice(emb),
            ArenaStore::U8(v) => v.extend(emb.iter().map(|&x| sq8_q(x))),
            ArenaStore::Binary { data, n, .. } => { data.extend(bq_pack(emb)); *n += 1; }
        }
    }
    /// Read a vector back as f32 (dequantizing per mode). For the heuristic
    /// f32-value readers (meta-graph / clustering) only — never for search.
    fn f32_at(&self, start: usize, len: usize) -> Vec<f32> {
        match self {
            ArenaStore::F32(v) => v[start..start + len].to_vec(),
            ArenaStore::U8(v) => v[start..start + len].iter().map(|&q| sq8_dq(q)).collect(),
            ArenaStore::Binary { data, dim, .. } => {
                let wpv = bq_words(*dim);
                let ws = (start / *dim) * wpv;
                bq_unpack(&data[ws..ws + wpv], len)
            }
        }
    }
    /// Append elements [start, start+len) from `src` (same variant) — compaction.
    fn append_range(&mut self, src: &ArenaStore, start: usize, len: usize) {
        match (self, src) {
            (ArenaStore::F32(d), ArenaStore::F32(s)) => d.extend_from_slice(&s[start..start + len]),
            (ArenaStore::U8(d), ArenaStore::U8(s)) => d.extend_from_slice(&s[start..start + len]),
            (ArenaStore::Binary { data: d, n, .. }, ArenaStore::Binary { data: s, dim, .. }) => {
                let wpv = bq_words(*dim);
                let ws = (start / *dim) * wpv;
                d.extend_from_slice(&s[ws..ws + wpv]);
                *n += 1;
            }
            _ => {} // a collection never mixes variants
        }
    }
    /// Raw little-endian bytes for the `vec_<name>.bin` snapshot.
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            ArenaStore::F32(v) => unsafe {
                std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4)
            }.to_vec(),
            ArenaStore::U8(v) => v.clone(),
            ArenaStore::Binary { data, .. } => data.iter().flat_map(|w| w.to_le_bytes()).collect(),
        }
    }
    fn from_bytes(data: &[u8], q: Quant, dim: usize) -> Self {
        match q {
            Quant::None => ArenaStore::F32(
                data.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect()
            ),
            Quant::ScalarU8 => ArenaStore::U8(data.to_vec()),
            Quant::Binary => {
                let words: Vec<u64> = data.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
                let wpv = bq_words(dim).max(1);
                ArenaStore::Binary { n: words.len() / wpv, data: words, dim }
            }
        }
    }
}

/// HNSW index, element type chosen by `Quant`. `search` distances are always f32
/// (the `Distance::eval` contract), so callers stay mode-agnostic.
enum VecIndex {
    F32(Hnsw<'static, f32, DistL2>),
    U8(Hnsw<'static, u8, DistL2>),
    Binary(Hnsw<'static, u64, DistBinaryHamming>),
}

impl VecIndex {
    fn build(q: Quant, ef_c: usize, cap: usize) -> Self {
        let cap = cap.max(VectorCollection::HNSW_MIN_CAP);
        match q {
            Quant::None => VecIndex::F32(Hnsw::new(16, cap, 16, ef_c, DistL2 {})),
            Quant::ScalarU8 => VecIndex::U8(Hnsw::new(16, cap, 16, ef_c, DistL2 {})),
            Quant::Binary => VecIndex::Binary(Hnsw::new(16, cap, 16, ef_c, DistBinaryHamming {})),
        }
    }
    fn insert_f32(&self, emb: &[f32], id: usize) {
        match self {
            VecIndex::F32(h) => h.insert((emb, id)),
            VecIndex::U8(h) => {
                let q: Vec<u8> = emb.iter().map(|&x| sq8_q(x)).collect();
                h.insert((&q, id));
            }
            VecIndex::Binary(h) => {
                let c = bq_pack(emb);
                h.insert((&c, id));
            }
        }
    }
    fn parallel_insert_f32(&self, items: &[(Vec<f32>, u32)]) {
        match self {
            VecIndex::F32(h) => {
                let refs: Vec<(&Vec<f32>, usize)> = items.iter().map(|(v, id)| (v, *id as usize)).collect();
                h.parallel_insert(&refs);
            }
            VecIndex::U8(h) => {
                let q: Vec<(Vec<u8>, usize)> = items.iter()
                    .map(|(v, id)| (v.iter().map(|&x| sq8_q(x)).collect(), *id as usize)).collect();
                let refs: Vec<(&Vec<u8>, usize)> = q.iter().map(|(v, id)| (v, *id)).collect();
                h.parallel_insert(&refs);
            }
            VecIndex::Binary(h) => {
                let q: Vec<(Vec<u64>, usize)> = items.iter()
                    .map(|(v, id)| (bq_pack(v), *id as usize)).collect();
                let refs: Vec<(&Vec<u64>, usize)> = q.iter().map(|(v, id)| (v, *id)).collect();
                h.parallel_insert(&refs);
            }
        }
    }
    /// Search, returning `(arena_id, distance_f32)` so callers never name the
    /// hnsw_rs `Neighbour` type or branch on element type.
    fn search_f32(&self, query: &[f32], k: usize, ef: usize) -> Vec<(usize, f32)> {
        match self {
            VecIndex::F32(h) => h.search(query, k, ef).into_iter().map(|n| (n.d_id, n.distance)).collect(),
            VecIndex::U8(h) => {
                let q: Vec<u8> = query.iter().map(|&x| sq8_q(x)).collect();
                h.search(&q, k, ef).into_iter().map(|n| (n.d_id, n.distance)).collect()
            }
            VecIndex::Binary(h) => {
                // Normalize Hamming (0..dim) to [0,1] so `1 - distance` stays a
                // sane similarity for the hybrid score blend.
                let c = bq_pack(query);
                let dim = query.len().max(1) as f32;
                h.search(&c, k, ef).into_iter().map(|n| (n.d_id, n.distance / dim)).collect()
            }
        }
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
    pub arena: RwLock<ArenaStore>,          // element type per `quant`; offsets in components
    pub metadata: RwLock<Vec<NodeMetadata>>,
    hnsw: RwLock<Option<VecIndex>>,
    pub node_to_arena: DashMap<u32, u32>,   // node u32 -> arena_id (this collection)
    pub count: AtomicUsize,
    /// Per-collection default HNSW `ef_search`. Set at creation, immutable.
    /// `None` ⇒ fall back to the engine-global default. Resolution order in
    /// `hybrid_search`: per-query override → this → engine-global.
    pub ef_search: Option<u32>,
    /// Optional exact-f32 sidecar for rerank: a flat `Vec<f32>` parallel to the
    /// quantized arena (vector at arena_id `i` occupies `[i*dim .. (i+1)*dim]`,
    /// i.e. the same `embedding_offset` units as the arena). `Some` only for a
    /// quantized collection created with `rerank = true`; `None` collections and
    /// non-rerank quantized collections leave it `None`. Persisted as
    /// `fvec_<name>.bin`.
    pub f32_sidecar: Option<RwLock<Vec<f32>>>,
}

impl VectorCollection {
    fn new(name: String, model: String, dim: u16, metric: Metric, quant: Quant, ef_search: Option<u32>, rerank: bool) -> Self {
        // Rerank only makes sense for a lossy (quantized) arena; a `None`
        // collection already stores exact f32, so never allocate a sidecar there.
        let f32_sidecar = if rerank && quant != Quant::None {
            Some(RwLock::new(Vec::new()))
        } else {
            None
        };
        Self {
            name, model, dim, metric, quant, ef_search, f32_sidecar,
            arena: RwLock::new(ArenaStore::new(quant, dim as usize)),
            metadata: RwLock::new(Vec::new()),
            hnsw: RwLock::new(None),
            node_to_arena: DashMap::new(),
            count: AtomicUsize::new(0),
        }
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

    fn ensure_hnsw(&self, ef_construction: usize) {
        if self.hnsw.read().is_none() {
            let mut w = self.hnsw.write();
            // Lazy create: final element count is unknown here, so reserve the
            // floor and let inserts grow it. Rehydrate (count known) sizes exactly.
            if w.is_none() { *w = Some(VecIndex::build(self.quant, ef_construction, Self::HNSW_MIN_CAP)); }
        }
    }

    /// f64 -> f32, normalizing for Cosine collections so DistL2 ranks by cosine.
    fn prep(&self, emb_64: Vec<f64>) -> Vec<f32> {
        let mut emb: Vec<f32> = emb_64.into_iter().map(|v| v as f32).collect();
        if self.metric == Metric::Cosine {
            let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 { for x in emb.iter_mut() { *x /= norm; } }
        }
        emb
    }

    /// Push a prepared vector into the arena/metadata, return its arena_id.
    /// Does NOT touch HNSW. Short critical section: only the Vec pushes.
    fn stage(&self, node_u32: u32, emb: &[f32], lang: String) -> u32 {
        let arena_id = {
            let mut meta = self.metadata.write();
            let mut arena = self.arena.write();
            let off = arena.len();
            arena.push_f32(emb);
            // Rerank sidecar: keep the exact f32 in lock-step with the arena.
            // Same offset units (components), so `embedding_offset` indexes both.
            // Lock order is meta → arena → sidecar everywhere (stage / compaction).
            if let Some(sidecar) = &self.f32_sidecar {
                sidecar.write().extend_from_slice(emb);
            }
            let arena_id = meta.len() as u32;
            meta.push(NodeMetadata {
                arena_id, node_u32, timestamp: Utc::now().timestamp() as u64,
                vector_dim: self.dim, embedding_offset: off as u64, gks_attributes: Vec::new(),
                lang, cluster_id: arena_id,
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
        if meta.is_empty() { return; }
        let index = VecIndex::build(self.quant, ef_c, meta.len());
        let arena = self.arena.read();
        for m in meta.iter() {
            let start = m.embedding_offset as usize;
            let len = m.vector_dim as usize;
            if start + len <= arena.len() {
                let v = arena.f32_at(start, len);
                index.insert_f32(&v, m.arena_id as usize);
            }
        }
        *self.hnsw.write() = Some(index);
    }

    fn info(&self) -> CollectionInfo {
        CollectionInfo {
            name: self.name.clone(),
            model: self.model.clone(),
            dim: self.dim as u32,
            metric: self.metric.as_str().to_string(),
            quant: self.quant.as_str().to_string(),
            count: self.count.load(Ordering::Relaxed) as u32,
            ef_search: self.ef_search,
            rerank: self.f32_sidecar.is_some(),
        }
    }
}

/// A unit of deferred HNSW indexing, processed off the write hot path by the
/// per-Storage indexing thread (ADR--GENESISDB-ASYNC-INDEXING). The vector is
/// already staged in the collection's arena (durable) before the job is sent;
/// only the HNSW graph insert is deferred.
enum IndexJob {
    One { coll: Arc<VectorCollection>, arena_id: u32, emb: Vec<f32>, ef_c: usize },
    Batch { coll: Arc<VectorCollection>, items: Vec<(Vec<f32>, u32)>, ef_c: usize },
    Flush(Sender<()>),
}

pub struct Storage {
    pub path: PathBuf,
    pub read_only: bool,
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
    pub wal_sender: Sender<(SignedEvent, Sender<bool>)>,
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
        if labels.iter().any(|l| l.to_uppercase() == "MASTER") { Tier::MASTER }
        else if labels.iter().any(|l| l.to_uppercase() == "SPEC") { Tier::SPEC }
        else if labels.iter().any(|l| l.to_uppercase() == "ADR") { Tier::ADR }
        else { Tier::USER }
    }
}

impl Storage {
    pub fn validate_governance(&self, labels: &[String], is_system: bool) -> Result<()> {
        let tier = Tier::from_labels(labels);
        if tier == Tier::MASTER && !is_system {
            return Err(Error::from_reason("403 Forbidden: MASTER tier is immutable for external agents"));
        }
        Ok(())
    }

    fn tokenize_id(id: &str) -> Vec<String> {
        let base_chars: String = id.chars().filter(|c| {
            let cat = unicode_general_category::get_general_category(*c);
            !matches!(cat, unicode_general_category::GeneralCategory::NonspacingMark | unicode_general_category::GeneralCategory::SpacingMark | unicode_general_category::GeneralCategory::EnclosingMark)
        }).collect();

        let mut tokens: Vec<String> = id.chars().map(|c| c.to_lowercase().to_string()).collect();
        if id != base_chars { tokens.extend(base_chars.chars().map(|c| c.to_lowercase().to_string())); }
        tokens.extend(id.chars().collect::<Vec<_>>().windows(2).map(|w| w.iter().collect::<String>().to_lowercase()));
        tokens
    }

    pub fn get_or_intern_id(&self, id: &str) -> u32 {
        if let Some(existing) = self.id_to_u32.get(id) { return *existing; }
        let new_id = self.next_u32.fetch_add(1, Ordering::SeqCst);
        self.id_to_u32.insert(id.to_string(), new_id);
        // No reverse-map insert: id string is recoverable via `nodes[u32].id`
        // (ADR--GENESISDB-NODE-ID-INTERNING, Layer A).

        for trigram in Self::tokenize_id(id) {
            self.trigram_index.entry(trigram).or_insert_with(RoaringBitmap::new).insert(new_id);
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

    pub fn get_u32(&self, id: &str) -> Option<u32> { self.id_to_u32.get(id).map(|v| *v) }

    /// Tune HNSW build/search effort. Call before bulk load to trade recall for
    /// speed (e.g. 100/100 = fast, 200/100 = quality default). Affects future
    /// inserts and the next rebuild; not a retroactive re-index. Applies to all
    /// collections (the tunables are global).
    pub fn set_index_params(&self, ef_construction: u32, ef_search: u32) {
        self.ef_construction.store(ef_construction as usize, Ordering::Relaxed);
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
        let n = name.clone().unwrap_or_else(|| self.default_collection.clone());
        self.collections
            .get(&n)
            .map(|r| Arc::clone(r.value()))
            .ok_or_else(|| Error::from_reason(format!("collection '{}' not found", n)))
    }

    /// Create an isolated vector collection. Idempotent-erroring: fails if a
    /// collection with this name already exists.
    pub fn create_collection(&self, name: String, model: String, dim: u32, metric: Option<String>, quant: Option<String>, ef_search: Option<u32>, rerank: Option<bool>) -> Result<()> {
        self.ensure_writable()?;
        if self.collections.contains_key(&name) {
            return Err(Error::from_reason(format!("collection '{}' already exists", name)));
        }
        let m = metric.as_deref().map(Metric::parse).unwrap_or(Metric::L2);
        let q = quant.as_deref().map(Quant::parse).unwrap_or(Quant::None);
        self.collections.insert(name.clone(), Arc::new(VectorCollection::new(name, model, dim as u16, m, q, ef_search, rerank.unwrap_or(false))));
        Ok(())
    }

    pub fn list_collections(&self) -> Vec<CollectionInfo> {
        self.collections.iter().map(|c| c.value().info()).collect()
    }

    /// Insert one vector into the named (or default) collection. Validates the
    /// embedding length against the collection dim, stages it into the arena
    /// (durable, immediately in-memory), and defers the HNSW insert to the
    /// indexing thread (ADR--GENESISDB-ASYNC-INDEXING).
    fn add_vector_internal(&self, collection: &Option<String>, node_id: &str, emb_64: Vec<f64>, lang: String) -> Result<()> {
        let coll = self.resolve_collection(collection)?;
        if emb_64.len() != coll.dim as usize {
            return Err(Error::from_reason(format!(
                "embedding dim {} != collection '{}' dim {}", emb_64.len(), coll.name, coll.dim
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
    pub fn add_vector(&self, node_id: String, collection: String, embedding: Vec<f64>) -> Result<()> {
        self.ensure_writable()?;
        // A vector attaches to a node — the node must exist.
        let exists = self.get_u32(&node_id).map_or(false, |u| self.nodes.contains_key(&u));
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
            node_id, collection: coll, embedding, lang: Some(lang), clock,
        }))?;
        Ok(())
    }

    fn enqueue_one(&self, coll: &Arc<VectorCollection>, arena_id: u32, emb: Vec<f32>) {
        self.index_pending.fetch_add(1, Ordering::Relaxed);
        let _ = self.index_tx.send(IndexJob::One {
            coll: Arc::clone(coll), arena_id, emb,
            ef_c: self.ef_construction.load(Ordering::Relaxed),
        });
    }

    fn enqueue_batch(&self, coll: &Arc<VectorCollection>, items: Vec<(Vec<f32>, u32)>) {
        if items.is_empty() { return; }
        self.index_pending.fetch_add(items.len(), Ordering::Relaxed);
        let _ = self.index_tx.send(IndexJob::Batch {
            coll: Arc::clone(coll), items,
            ef_c: self.ef_construction.load(Ordering::Relaxed),
        });
    }

    /// Block until every queued vector has been inserted into its HNSW index.
    /// Use before asserting searchability, and before any operation that
    /// reassigns arena ids (compaction / index rebuild) so a pending insert
    /// never targets a stale arena id.
    pub fn flush_index(&self) {
        let (tx, rx) = bounded(1);
        if self.index_tx.send(IndexJob::Flush(tx)).is_ok() { let _ = rx.recv(); }
    }

    /// Vectors staged but not yet inserted into HNSW (eventually-searchable lag).
    pub fn index_lag(&self) -> u32 { self.index_pending.load(Ordering::Relaxed) as u32 }

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
    fn replay_vector(&self, collection: &Option<String>, node_id: &str, emb: Vec<f64>, lang: String, index: bool) {
        let name = collection.clone().unwrap_or_else(|| self.default_collection.clone());
        if !self.collections.contains_key(&name) {
            self.collections.insert(
                name.clone(),
                Arc::new(VectorCollection::new(name.clone(), "recovered".to_string(), emb.len() as u16, Metric::L2, Quant::None, None, false)),
            );
        }
        if let Ok(coll) = self.resolve_collection(&Some(name)) {
            if emb.len() != coll.dim as usize { return; }
            let node_u32 = self.get_or_intern_id(node_id);
            let e = coll.prep(emb);
            let arena_id = coll.stage(node_u32, &e, lang);
            if index { self.enqueue_one(&coll, arena_id, e); }
        }
    }

    /// Rebuild every collection's HNSW from its arena (both load paths).
    fn rehydrate_hnsw_index(&self) {
        let ef_c = self.ef_construction.load(Ordering::Relaxed);
        for c in self.collections.iter() { c.value().rehydrate(ef_c); }
    }

    pub fn open(opts: OpenOptions) -> Result<Self> {
        let root = PathBuf::from(opts.path.clone());
        if !root.exists() { fs::create_dir_all(&root).ok(); }
        let read_only = opts.read_only.unwrap_or(false);
        let vector_dim = opts.vector_dim.unwrap_or(1536) as u16;

        // --- Cryptographic Identity (Mark X) ---
        let identity_path = root.join("identity.bin");
        let signing_key = if identity_path.exists() {
            let bytes = fs::read(&identity_path).map_err(|e| Error::from_reason(e.to_string()))?;
            SigningKey::from_bytes(bytes.as_slice().try_into().map_err(|_| Error::from_reason("invalid identity key length"))?)
        } else {
            
            let key = SigningKey::from_bytes(&OsRng.gen::<[u8; 32]>());
            if !read_only {
                fs::write(&identity_path, key.to_bytes()).map_err(|e| Error::from_reason(e.to_string()))?;
            }
            key
        };
        let verifying_key = signing_key.verifying_key();
        let local_peer_id = hex::encode(Sha256::digest(verifying_key.as_bytes()))[..16].to_string();

        let log_path = root.join("genesis-graph.wal");
        let (wal_sender, wal_receiver): (Sender<(SignedEvent, Sender<bool>)>, Receiver<(SignedEvent, Sender<bool>)>) = unbounded();
        let log_path_clone = log_path.clone();

        let wal_handle = std::thread::spawn(move || {
            if let Ok(file) = FileOpenOptions::new().append(true).create(true).open(&log_path_clone) {
                let mut writer = std::io::BufWriter::with_capacity(128 * 1024, file);
                let mut batch: Vec<crossbeam_channel::Sender<bool>> = Vec::with_capacity(1024);
                loop {
                    match wal_receiver.recv() {
                        Ok((signed_event, ack_tx)) => {
                            batch.push(ack_tx);
                            if let Ok(json) = serde_json::to_string(&signed_event) {
                                let _ = writer.write_all(json.as_bytes());
                                let _ = writer.write_all(b"\n");
                            }
                            let timeout = Duration::from_millis(5);
                            let start = Instant::now();
                            while batch.len() < 1024 && start.elapsed() < timeout {
                                if let Ok((se, tx)) = wal_receiver.try_recv() {
                                    batch.push(tx);
                                    if let Ok(j) = serde_json::to_string(&se) {
                                        let _ = writer.write_all(j.as_bytes());
                                        let _ = writer.write_all(b"\n");
                                    }
                                } else { break; }
                            }
                            let _ = writer.flush();
                            let _ = writer.get_mut().sync_all();
                            for ack in batch.drain(..) { let _ = ack.send(true); }
                        },
                        Err(_) => break,
                    }
                }
            }
        });

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
                    IndexJob::One { coll, arena_id, emb, ef_c } => {
                        coll.ensure_hnsw(ef_c);
                        // insert(&self): hnsw_rs is internally synchronized -> read lock.
                        // The job ships f32; VecIndex quantizes per mode before insert.
                        if let Some(ref idx) = *coll.hnsw.read() { idx.insert_f32(&emb, arena_id as usize); }
                        index_pending_thread.fetch_sub(1, Ordering::Relaxed);
                    }
                    IndexJob::Batch { coll, items, ef_c } => {
                        coll.ensure_hnsw(ef_c);
                        if let Some(ref idx) = *coll.hnsw.read() { idx.parallel_insert_f32(&items); }
                        index_pending_thread.fetch_sub(items.len(), Ordering::Relaxed);
                    }
                    IndexJob::Flush(ack) => { let _ = ack.send(()); }
                }
            }
        });

        // The `default` collection always exists; legacy single-space data and
        // any node added without an explicit collection routes here. Its dim is
        // the OpenOptions vector_dim (back-compat with the old global space).
        let collections: DashMap<String, Arc<VectorCollection>> = DashMap::new();
        collections.insert(
            "default".to_string(),
            Arc::new(VectorCollection::new("default".to_string(), "default".to_string(), vector_dim, Metric::L2, Quant::None, None, false)),
        );

        let storage = Self {
            path: root, read_only, nodes: DashMap::new(), edges: DashMap::new(),
            out_idx: DashMap::new(), in_idx: DashMap::new(),
            collections, default_collection: "default".to_string(),
            log_path, bin_path: PathBuf::from(""), _lock_file: None,
            id_to_u32: DashMap::new(), next_u32: AtomicU32::new(0),
            is_rebuilding: AtomicBool::new(false), trigram_index: DashMap::new(),
            lang_centroids: DashMap::new(), peers: DashMap::new(),
            proposals: DashMap::new(), meta_nodes: DashMap::new(), meta_edges: DashMap::new(),
            meta_history: DashMap::new(),
            wal_sender,
            index_tx,
            index_pending,
            wal_handle: Some(wal_handle),
            index_handle: Some(index_handle),
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

        if !storage.try_load_state() {
            if storage.log_path.exists() {
                if let Ok(file) = File::open(&storage.log_path) {
                    let reader = std::io::BufReader::new(file);
                    use std::io::BufRead;
                    for line_res in reader.lines() {
                        if let Ok(line) = line_res {
                            if let Ok(signed_event) = serde_json::from_str::<SignedEvent>(&line) {
                                let event = signed_event.event;
                                match event {
                                    Event::Node(n) => {
                                        let u32_id = storage.get_or_intern_id(&n.id);
                                        if let Some(emb) = n.embedding.clone() {
                                            storage.replay_vector(&n.collection, &n.id, emb, n.lang.clone().unwrap_or("en".to_string()), false);
                                        }
                                        storage.insert_node_lean(u32_id, n);
                                    }
                                    Event::Edge(e) => {
                                        let u32_id = storage.index_edge_internal(&e.id, &e.from, &e.to);
                                        storage.edges.insert(u32_id, e);
                                    }
                                    Event::Vector(v) => {
                                        // Stage only (index=false): rehydrate_hnsw_index
                                        // after load builds every index once.
                                        storage.replay_vector(&v.collection, &v.node_id, v.embedding, v.lang.clone().unwrap_or_else(|| "en".to_string()), false);
                                    }
                                    Event::Batch(events) => {
                                        for batch_event in events {
                                            match batch_event {
                                                Event::Node(n) => {
                                                    let u32_id = storage.get_or_intern_id(&n.id);
                                                    if let Some(emb) = n.embedding.clone() {
                                                        storage.replay_vector(&n.collection, &n.id, emb, n.lang.clone().unwrap_or("en".to_string()), false);
                                                    }
                                                    storage.insert_node_lean(u32_id, n);
                                                }
                                                Event::Edge(e) => {
                                                    let u32_id = storage.index_edge_internal(&e.id, &e.from, &e.to);
                                                    storage.edges.insert(u32_id, e);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Rebuild the HNSW index for BOTH load paths: WAL replay and the
        // instant snapshot load (try_load_state populates the vector/metadata
        // arenas but never rehydrates HNSW, leaving semantic search broken
        // until a manual rebuild).
        storage.rehydrate_hnsw_index();
        Ok(storage)
    }

    pub fn start_autonomic_loop(storage: Arc<Self>) {
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(3600)); 
                if !storage.read_only {
                    let _ = storage.perform_autonomic_optimization();
                    let _ = storage.save_state();
                }
            }
        });
    }

    pub fn ensure_writable(&self) -> Result<()> { if self.read_only { return Err(Error::from_reason("read-only")); } Ok(()) }

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

    pub fn persist(&self, event: &Event) -> Result<()> {
        let (ack_tx, ack_rx) = unbounded();
        let signed_event = self.sign_event(event);
        self.wal_sender.send((signed_event, ack_tx)).map_err(|_| Error::from_reason("wal disconnected"))?;
        let _ = ack_rx.recv(); Ok(())
    }

    pub fn find_fuzzy_id(&self, id: &str) -> Option<String> {
        // 1. Exact Match
        if self.get_u32(id).is_some() { return Some(id.to_string()); }

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

        if max_lexical_sim > 0.85 { return best_lexical_id; }

        // 3. Neural Fuzzy (Vector Fallback)
        // Relaxed threshold for Thai characters. 
        if max_lexical_sim > 0.20 { return best_lexical_id; }


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
                    })?;
                    
                    for neighbor in context {
                        if neighbor.node.impact.unwrap_or(0.0) > 0.8 {
                            if node.labels != neighbor.node.labels && neighbor.node.labels.contains(&"MASTER".to_string()) {
                                return Ok(false);
                            }
                        }
                    }
                }
                Ok(true)
            }
            Event::Edge(_) => Ok(true),
            // A vector attachment carries no governance/axiom implication.
            Event::Vector(_) => Ok(true),
            Event::Batch(events) => {
                for e in events {
                    if !self.semantic_verify(e)? { return Ok(false); }
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
            return Err(Error::from_reason("proposal rejected by semantic_verify (conflicts with a governing axiom)"));
        }
        let proposal_id = Uuid::new_v4().to_string();
        let event_data = serde_json::to_vec(&event).map_err(|e| Error::from_reason(e.to_string()))?;
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

    pub fn submit_vote(&self, proposal_id: String, peer_id: String, approve: bool, signature: Vec<u8>) -> Result<bool> {
        // Verify the vote is authentically signed by `peer_id` before recording
        // it — otherwise any caller could forge votes on another peer's behalf and
        // drive a proposal to quorum. Unknown peers, malformed or non-matching
        // signatures are rejected (the vote is not counted).
        let vkey = self.peer_verifying_key(&peer_id)
            .ok_or_else(|| Error::from_reason(format!("unknown voter peer '{}' (no verifying key)", peer_id)))?;
        let payload = Self::vote_payload(&proposal_id, &peer_id, approve);
        let sig = Signature::from_slice(&signature)
            .map_err(|_| Error::from_reason("malformed vote signature"))?;
        vkey.verify(&payload, &sig)
            .map_err(|_| Error::from_reason("invalid vote signature"))?;

        if let Some(mut proposal_ref) = self.proposals.get_mut(&proposal_id) {
            let proposal = proposal_ref.value_mut();
            // Already-committed guard: once quorum is crossed and the event applied,
            // later approving votes must not re-apply or re-persist it.
            if proposal.committed {
                return Ok(true);
            }
            proposal.votes.insert(peer_id.clone(), approve);
            // Retain the verified signature as proof of the vote (quorum_signatures).
            proposal.quorum_signatures.insert(peer_id.clone(), signature);

            let approvals = proposal.votes.values().filter(|&&v| v).count();

            // Quorum is a strict majority of the swarm. `self.peers` excludes this
            // node, so the membership denominator is peers + self.
            if approvals <= (self.peers.len() + 1) / 2 {
                return Ok(false);
            }

            // Last gate before the event becomes durable state: re-verify the
            // proposal's own event signature (a gossiped proposal is verified on
            // receipt, but defense-in-depth) and re-run the governance check.
            let signed_event = proposal.signed_event.clone();
            if !self.verify_event_signature(&signed_event) {
                return Err(Error::from_reason("proposal event signature invalid at commit"));
            }
            if !self.semantic_verify(&signed_event.event)? {
                return Err(Error::from_reason("proposal rejected by semantic_verify at commit"));
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
                                if !n_axiom.labels.contains(&"MASTER".to_string()) { n_axiom.labels.push("MASTER".to_string()); }
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
                    self.replay_vector(&v.collection, &v.node_id, v.embedding.clone(), v.lang.clone().unwrap_or_else(|| "en".to_string()), true);
                    self.persist_signed(signed_event.clone())?;
                }
            }
            proposal.committed = true;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn calculate_sc(&self, node: &NodeOutput) -> f64 {
        let stability = node.props.get("stability").and_then(|v| v.as_str()).unwrap_or("active");
        match stability {
            "stable" => 1.0, "active" => 0.8, "draft" => 0.4, "deprecated" => 0.1, _ => 0.8,
        }
    }

    pub fn compute_impact(&self, node: &NodeOutput) -> f64 {
        let u32_id = match self.get_u32(&node.id) { Some(id) => id, None => return 0.7 };
        let incoming_count = self.in_idx.get(&u32_id).map(|edges| edges.len()).unwrap_or(0);
        let dd = (incoming_count as f64 / 10.0).min(1.0);
        let tier = Tier::from_labels(&node.labels);
        let as_score = match tier {
            Tier::MASTER => 1.0, Tier::SPEC => 0.8, Tier::ADR => 0.6, Tier::USER => 0.3,
        };
        let sc = self.calculate_sc(node);
        (dd * 0.5) + (as_score * 0.3) + (sc * 0.2)
    }

    pub fn refresh_impacts(&self, affected_ids: Option<Vec<String>>) {
        let ids_to_process = match affected_ids {
            Some(ids) => ids,
            None => self.nodes.iter().map(|entry| entry.value().id.clone()).collect(),
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
        self.out_idx.entry(u32_from).or_insert_with(HashSet::new).insert(ekey);
        self.in_idx.entry(u32_to).or_insert_with(HashSet::new).insert(ekey);
        ekey
    }

    fn next_clock(&self) -> LogicalClock {
        let time = self.logical_clock.fetch_add(1, Ordering::SeqCst) + 1;
        LogicalClock { time, peer_id: self.local_peer_id.clone() }
    }

    /// Insert a node into the in-memory store without its embedding.
    /// The f32 vector lives in `vector_arena` + the HNSW index (the source of
    /// truth for search); keeping a third f64 copy on every node wastes ~12 KB
    /// per node (the largest avoidable per-node cost). The full embedding is
    /// still persisted in the WAL `Event::Node` for replay/arena rebuild.
    fn insert_node_lean(&self, u32_id: u32, mut node: NodeOutput) {
        node.embedding = None;
        self.nodes.insert(u32_id, node);
    }

    pub fn add_node(&self, args: NodeInput) -> Result<NodeOutput> {
        self.ensure_writable()?;
        self.validate_governance(&args.labels, false)?; 
        let id = args.id.unwrap_or_else(|| format!("N-{}", Uuid::new_v4()));
        let u32_id = self.get_or_intern_id(&id);
        let lang = args.lang.clone().unwrap_or("en".to_string());
        
        let now = Utc::now();
        let expires_at = args.ttl.map(|s| (now + chrono::Duration::seconds(s as i64)).to_rfc3339());

        let mut node = NodeOutput {
            id: id.clone(), labels: args.labels,
            props: args.props.unwrap_or(Value::Object(Default::default())),
            impact: Some(0.7), embedding: None,
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
            node.collection = Some(args.collection.clone().unwrap_or_else(|| self.default_collection.clone()));
        }
        self.insert_node_lean(u32_id, node.clone());
        self.persist(&Event::Node(node.clone()))?;
        Ok(node)
    }

    pub fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> {
        self.ensure_writable()?;
        let edge = EdgeOutput {
            id: args.id.unwrap_or_else(|| Uuid::new_v4().to_string()), from: args.from, to: args.to, rel: args.rel,
            props: args.props.unwrap_or(Value::Object(Default::default())), 
            valid_from: args.valid_from.unwrap_or_else(|| Utc::now().to_rfc3339()), 
            valid_to: None, recorded_at: Utc::now().to_rfc3339(),
            superseded_by: None, impact: args.impact,
            caused_by: args.caused_by,
            clock: self.next_clock(),
        };
        let u32_id = self.index_edge_internal(&edge.id, &edge.from, &edge.to);
        self.edges.insert(u32_id, edge.clone());
        self.refresh_impacts(Some(vec![edge.to.clone()]));
        self.persist(&Event::Edge(edge.clone()))?;
        Ok(edge)
    }

    pub fn supersede_node(&self, id: String, new_props: Option<Value>, caused_by: Option<String>) -> Result<NodeOutput> {
        self.ensure_writable()?;
        let u32_id = match self.get_u32(&id) {
            Some(i) => i,
            None => return Err(Error::from_reason(format!("Node {} not found", id))),
        };

        let now = Utc::now().to_rfc3339();

        let mut old_node = match self.nodes.get(&u32_id) {
            Some(node) => node.value().clone(),
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
        self.is_rebuilding.store(true, Ordering::SeqCst);
        self.flush_index();
        let result = (|| { self.rehydrate_hnsw_index(); Ok(()) })();
        self.is_rebuilding.store(false, Ordering::SeqCst);
        result
    }

    pub fn execute_hql(&self, query: &str) -> Result<serde_json::Value> {
        let command = HqlCommand::try_from(query).map_err(|e| Error::from_reason(e))?;
        match command {
            HqlCommand::Search { vector, k, fuzzy, target, lang, as_of, collection } => {
                let _resolved = if fuzzy { self.find_fuzzy_id(&target) } else { Some(target) };
                let res = self.hybrid_search(HybridSearchInput { query_vector: vector, k, alpha: Some(0.0), lang, as_of, collection, ef_search: None })?;
                Ok(serde_json::to_value(res).unwrap())
            }
            HqlCommand::Traverse { seed, depth, rel, fuzzy, as_of } => {
                let resolved_seed = if fuzzy { self.find_fuzzy_id(&seed).unwrap_or(seed) } else { seed };
                let (target_rel, is_inferred) = match rel {
                    query::ast::HqlRel::Physical(r) => (r, false),
                    query::ast::HqlRel::Inferred(r) => (r, true),
                };
                let res = self.neighbors(resolved_seed, NeighborInput { 
                    depth: Some(depth), rel: Some(target_rel), rels: None, direction: Some("out".to_string()), as_of, include_invalid: Some(false), limit: None 
                }, is_inferred)?;
                Ok(serde_json::to_value(res).unwrap())
            }
            HqlCommand::Hybrid { vector, alpha, fuzzy, target, lang, as_of, collection } => {
                let _resolved = if fuzzy { self.find_fuzzy_id(&target) } else { Some(target) };
                let res = self.hybrid_search(HybridSearchInput { query_vector: vector, k: 10, alpha: Some(alpha), lang, as_of, collection, ef_search: None })?;
                Ok(serde_json::to_value(res).unwrap())
            }
            HqlCommand::Context { target, tier, budget, fuzzy } => {
                let res = self.retrieve_context(&target, &tier, budget, fuzzy)?;
                Ok(serde_json::to_value(res).unwrap())
            }
        }
    }

    fn is_valid_as_of(valid_from: &str, valid_to: &Option<String>, as_of: &Option<String>) -> bool {
        if let Some(as_of_str) = as_of {
            if valid_from > as_of_str.as_str() { return false; }
            if let Some(to) = valid_to {
                if as_of_str.as_str() >= to.as_str() { return false; }
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
                "query dim {} != collection '{}' dim {}", args.query_vector.len(), coll.name, coll.dim
            )));
        }
        let mut query_f32: Vec<f32> = args.query_vector.into_iter().map(|v| v as f32).collect();
        if let Some(lang) = args.lang {
            if let Some(centroid) = self.lang_centroids.get(&lang) {
                for (i, val) in query_f32.iter_mut().enumerate() { if i < centroid.len() { *val += centroid[i]; } }
            }
        }
        // Cosine collections store normalized vectors; normalize the query too.
        if coll.metric == Metric::Cosine {
            let norm: f32 = query_f32.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 { for x in query_f32.iter_mut() { *x /= norm; } }
        }
        // VecIndex quantizes the query per mode and returns (arena_id, distance_f32).
        // ef_search resolution: per-query override → per-collection default → engine-global.
        let ef = args.ef_search.map(|e| e as usize)
            .or_else(|| coll.ef_search.map(|e| e as usize))
            .unwrap_or_else(|| self.ef_search.load(Ordering::Relaxed));
        // Over-fetch more quantized candidates when a rerank sidecar is present, so
        // the exact re-score has a wider pool to recover recall from.
        let k2 = (args.k * 2) as usize;
        let fetch = if coll.f32_sidecar.is_some() {
            (args.k as usize).saturating_mul(RERANK_OVERFETCH).max(k2)
        } else { k2 };
        let mut results = {
            let hnsw_lock = coll.hnsw.read();
            match &*hnsw_lock {
                Some(idx) => idx.search_f32(&query_f32, fetch, ef),
                None => return Err(Error::from_reason("HNSW not init")),
            }
        };
        // f32-sidecar rerank: replace each candidate's quantized distance with the
        // exact f32 distance, re-sort ascending, and keep the best k*2 for the
        // hybrid blend below. The arena_id (d_id) indexes the sidecar at d_id*dim.
        if let Some(sidecar) = &coll.f32_sidecar {
            let sc = sidecar.read();
            let dim = coll.dim as usize;
            results = results.into_iter().filter_map(|(d_id, _)| {
                let start = d_id * dim;
                sc.get(start..start + dim).map(|v| (d_id, exact_l2(&query_f32, v)))
            }).collect();
            results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(k2);
        }
        let mut hybrid_results = Vec::new();
        let meta_arena = coll.metadata.read();
        let alpha = args.alpha.unwrap_or(0.5);

        for (d_id, distance) in results {
            if let Some(meta) = meta_arena.get(d_id) {
                { let u32_id = meta.node_u32; // A2: id interned in metadata
                    if let Some(node) = self.nodes.get(&u32_id) {
                        let mut node_out = node.value().clone();

                        if !Self::is_valid_as_of(&node_out.valid_from, &node_out.valid_to, &args.as_of) {
                            continue;
                        }

                        let similarity = 1.0 - distance as f64;
                        let reasoning_score = (similarity * (1.0 - alpha)) + (node_out.impact.unwrap_or(0.0) * alpha);
                        node_out.impact = Some(reasoning_score);
                        hybrid_results.push(NeighborOutput { node: node_out, path: Vec::new(), depth: 0 });
                    }
                }
            }
        }
        hybrid_results.sort_by(|a, b| b.node.impact.partial_cmp(&a.node.impact).unwrap());
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

    pub fn neighbors(&self, seed: String, args: NeighborInput, is_inferred: bool) -> Result<Vec<NeighborOutput>> {
        let u32_seed = match self.get_u32(&seed) { Some(id) => id, None => return Ok(Vec::new()) };
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
            match &rels_filter { None => true, Some(s) => s.contains(rel) }
        };

        // Direction: "out" (default, back-compat), "in", "both". Case-insensitive.
        let dir = args.direction.as_deref().map(|s| s.to_ascii_lowercase()).unwrap_or_else(|| "out".to_string());
        let walk_out = dir == "out" || dir == "both";
        let walk_in  = dir == "in"  || dir == "both";

        let lim = args.limit.map(|l| l as usize);
        // Retraction visibility: by default a retracted edge (valid_to passed) is
        // hidden from the current view; `include_invalid = true` surfaces it.
        let include_invalid = args.include_invalid.unwrap_or(false);
        let mut results = Vec::new(); let mut visited = HashSet::new(); visited.insert(u32_seed);
        let mut queue = VecDeque::new(); queue.push_back((u32_seed, Vec::new(), 0));
        while let Some((curr_u32, path, curr_depth)) = queue.pop_front() {
            if curr_depth >= depth && !is_inferred { continue; }

            // Collect candidate edge ids from chosen directions, dedup by eid.
            let mut eid_set: HashSet<u128> = HashSet::new();
            if walk_out {
                if let Some(out_eids) = self.out_idx.get(&curr_u32) { eid_set.extend(out_eids.iter().copied()); }
            }
            if walk_in {
                if let Some(in_eids) = self.in_idx.get(&curr_u32) { eid_set.extend(in_eids.iter().copied()); }
            }

            for eid in eid_set.iter() {
                if let Some(edge_ref) = self.edges.get(eid) {
                    let edge = edge_ref.value();

                    // Time-travel check for Edges
                    if !Self::is_valid_as_of(&edge.valid_from, &edge.valid_to, &args.as_of) {
                        continue;
                    }
                    // Retraction filter for the current view: `is_valid_as_of` only
                    // bounds valid_to when `as_of` is set, so with no as_of an edge
                    // retracted in the past is still "valid" there. Hide it unless
                    // the caller opted into invalidated edges.
                    if args.as_of.is_none() && !include_invalid {
                        if let Some(to) = &edge.valid_to {
                            if Utc::now().to_rfc3339().as_str() >= to.as_str() { continue; }
                        }
                    }
                    if !rel_allowed(&edge.rel) { continue; }

                    // Pick the far endpoint by u32 identity rather than by
                    // reverse-mapping curr_u32 back to its string: the near
                    // endpoint is whichever of from/to interns to curr_u32. This
                    // needs no u32->id reverse map and avoids a per-edge string
                    // clone (ADR--GENESISDB-NODE-ID-INTERNING, Layer A).
                    let next_id = if self.get_u32(&edge.from) == Some(curr_u32) { &edge.to } else { &edge.from };
                    if let Some(next_u32) = self.get_u32(next_id) {
                        if !visited.contains(&next_u32) {
                            visited.insert(next_u32);
                            if let Some(node_ref) = self.nodes.get(&next_u32) {
                                let node = node_ref.value();

                                // Time-travel check for Nodes
                                if !Self::is_valid_as_of(&node.valid_from, &node.valid_to, &args.as_of) {
                                    continue;
                                }

                                let mut new_path = path.clone(); new_path.push(edge.clone());
                                results.push(NeighborOutput { node: node.clone(), path: new_path.clone(), depth: curr_depth + 1 });
                                if let Some(l) = lim { if results.len() >= l { return Ok(results); } }
                                if is_inferred || (curr_depth + 1 < depth) { queue.push_back((next_u32, new_path, curr_depth + 1)); }
                            }
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn query(&self, args: QueryInput) -> Result<Vec<EdgeOutput>> {
        let mut res = Vec::new();
        for r in self.edges.iter() {
            let e = r.value();
            if let Some(ref f) = args.from { if e.from != *f { continue; } }
            if let Some(ref t) = args.to { if e.to != *t { continue; } }
            res.push(e.clone());
        }
        Ok(res)
    }

    pub fn detect_communities(&self) -> Result<()> {
        // Community detection runs over the default collection's vector space.
        let coll = self.default_coll();
        let mut meta_arena = coll.metadata.write();
        let mut new_clusters = Vec::with_capacity(meta_arena.len());
        for meta in meta_arena.iter() {
            let mut freq = HashMap::new();
            { let u32_id = meta.node_u32; // A2: id interned in metadata
                let out_eids = self.out_idx.get(&u32_id).map(|v| v.value().clone()).unwrap_or_default();
                for eid in out_eids {
                    if let Some(edge) = self.edges.get(&eid) {
                        let other_id = if self.get_u32(&edge.from) == Some(meta.node_u32) { &edge.to } else { &edge.from };
                        if let Some(to_u32) = self.get_u32(other_id) {
                            if let Some(a_id) = coll.node_to_arena.get(&to_u32) {
                                if let Some(other_meta) = meta_arena.get(*a_id as usize) {
                                    *freq.entry(other_meta.cluster_id).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
                let in_eids = self.in_idx.get(&u32_id).map(|v| v.value().clone()).unwrap_or_default();
                for eid in in_eids {
                    if let Some(edge) = self.edges.get(&eid) {
                        let other_id = if self.get_u32(&edge.from) == Some(meta.node_u32) { &edge.to } else { &edge.from };
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
        if v1.len() != v2.len() || v1.is_empty() { return 0.0; }
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;
        for i in 0..v1.len() {
            dot += v1[i] * v2[i];
            norm_a += v1[i].powi(2);
            norm_b += v2[i].powi(2);
        }
        if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }

    pub fn generate_meta_graph(&self) -> Result<()> {
        // Meta-graph is built over the default collection's vector space.
        let coll = self.default_coll();
        let dim = coll.dim as usize;
        let mut cluster_groups: HashMap<u32, Vec<u32>> = HashMap::new();
        let meta_arena = coll.metadata.read();
        for meta in meta_arena.iter() {
            cluster_groups.entry(meta.cluster_id).or_insert_with(Vec::new).push(meta.node_u32);
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
                                    if i < dim { centroid[i] += *val as f64; }
                                }
                                count += 1;
                            }
                        }
                    }
                }
            }
            if count > 0 {
                for val in centroid.iter_mut() { *val /= count as f64; }
                
                let mut drift = None;
                if let Some(history) = self.meta_history.get(c_id) {
                    if let Some(prev) = history.last() {
                        let sim = Self::cosine_similarity(&centroid, &prev.centroid);
                        drift = Some(1.0 - sim);
                    }
                }

                let sn = SuperNode {
                    cluster_id: *c_id, theme: format!("Theme-{}", c_id),
                    member_count: members.len() as u32, impact: total_impact / members.len() as f64,
                    centroid: centroid.clone(),
                    timestamp: now.clone(),
                    drift,
                };

                self.meta_nodes.insert(*c_id, sn.clone());
                self.meta_history.entry(*c_id).or_insert_with(Vec::new).push(sn);
            }
        }
        for entry in self.edges.iter() {
            let edge = entry.value();
            if let (Some(from_u32), Some(to_u32)) = (self.get_u32(&edge.from), self.get_u32(&edge.to)) {
                if let (Some(from_cid), Some(to_id)) = (coll.node_to_arena.get(&from_u32), coll.node_to_arena.get(&to_u32)) {
                    let c1 = meta_arena[*from_cid as usize].cluster_id;
                    let c2 = meta_arena[*to_id as usize].cluster_id;
                    if c1 != c2 {
                        let key = format!("{}:{}", c1, c2);
                        let mut meta_edge = self.meta_edges.entry(key.clone()).or_insert(MetaEdge { from_cluster: c1, to_cluster: c2, weight: 0 });
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
        let u32_id = match self.get_u32(id) {
            Some(i) => i,
            None => return Ok(()),
        };

        // 1. Collect all edges to remove
        let mut edges_to_remove = Vec::new();
        if let Some(eids) = self.out_idx.get(&u32_id) {
            for eid in eids.iter() { edges_to_remove.push(*eid); }
        }
        if let Some(eids) = self.in_idx.get(&u32_id) {
            for eid in eids.iter() { edges_to_remove.push(*eid); }
        }

        // 2. Comprehensive bi-directional index cleanup
        for eid in edges_to_remove {
            if let Some(edge_ref) = self.edges.get(&eid) {
                let edge = edge_ref.value();
                if let (Some(from_u32), Some(to_u32)) = (self.get_u32(&edge.from), self.get_u32(&edge.to)) {
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
        for c in self.collections.iter() { c.value().node_to_arena.remove(&u32_id); }
        self.nodes.remove(&u32_id);

        Ok(())
    }

    pub fn reconcile_state(&self, signed_events: Vec<SignedEvent>) -> Result<()> {
        self.ensure_writable()?;
        for signed_event in signed_events {
            let event = &signed_event.event;
            let signer_id = &signed_event.signer_peer_id;
            
            // 1. Verify Signature (local events are self-trusted; remote events
            // must carry an authentic signature from their registered peer key).
            if signer_id != &self.local_peer_id && !self.verify_event_signature(&signed_event) {
                println!("Mark X: REJECTED event from {}. Invalid signature or unknown peer.", signer_id);
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
                            match self.logical_clock.compare_exchange_weak(current, remote_node.clock.time, Ordering::SeqCst, Ordering::SeqCst) {
                                Ok(_) => break,
                                Err(actual) => current = actual,
                            }
                        }
                        
                        if let Some(emb) = &remote_node.embedding {
                            self.replay_vector(&remote_node.collection, &remote_node.id, emb.clone(), remote_node.lang.clone().unwrap_or("en".to_string()), true);
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
                            match self.logical_clock.compare_exchange_weak(current, remote_edge.clock.time, Ordering::SeqCst, Ordering::SeqCst) {
                                Ok(_) => break,
                                Err(actual) => current = actual,
                            }
                        }
                        let u32_id = self.index_edge_internal(&remote_edge.id, &remote_edge.from, &remote_edge.to);
                        self.edges.insert(u32_id, remote_edge.clone());
                        self.persist_signed(signed_event.clone())?;
                    }
                }
                Event::Vector(remote_vec) => {
                    // Advance the local clock to the vector's time (like Node/Edge),
                    // so a peer that pulls and re-emits keeps logical time monotone.
                    let mut current = self.logical_clock.load(Ordering::SeqCst);
                    while remote_vec.clock.time > current {
                        match self.logical_clock.compare_exchange_weak(current, remote_vec.clock.time, Ordering::SeqCst, Ordering::SeqCst) {
                            Ok(_) => break,
                            Err(actual) => current = actual,
                        }
                    }
                    // Runtime sync: stage AND enqueue (index=true) — no rehydrate
                    // follows. Auto-provisions the collection if this peer lacks it.
                    // Vectors are append-applied (no LWW): a node holds at most one
                    // vector per collection, deduped at query time.
                    self.replay_vector(&remote_vec.collection, &remote_vec.node_id, remote_vec.embedding.clone(), remote_vec.lang.clone().unwrap_or_else(|| "en".to_string()), true);
                    self.persist_signed(signed_event.clone())?;
                }
                Event::Batch(inner_events) => {
                    // Recursive call needs SignedEvent wrapping, but for now we handle batches as single signed units
                    // To keep it simple, we wrap inner events or just apply them since the batch itself is verified.
                    let wrapped_inners: Vec<SignedEvent> = inner_events.iter().map(|e| SignedEvent {
                        event: e.clone(),
                        signature: signed_event.signature.clone(), // Reuse batch signature
                        signer_peer_id: signer_id.clone(),
                    }).collect();
                    let _ = self.reconcile_state(wrapped_inners);
                }
            }
        }
        Ok(())
    }

    pub fn persist_signed(&self, signed_event: SignedEvent) -> Result<()> {
        let (ack_tx, ack_rx) = unbounded();
        self.wal_sender.send((signed_event, ack_tx)).map_err(|_| Error::from_reason("wal disconnected"))?;
        let _ = ack_rx.recv(); Ok(())
    }

    pub fn get_logical_clock(&self) -> u32 {
        self.logical_clock.load(Ordering::SeqCst)
    }

    pub fn retrieve_context(&self, target_id: &str, tier_str: &str, budget: Option<u32>, fuzzy: bool) -> Result<ContextPackage> {
        let tier = ScalingTier::from_str(tier_str);
        let hops = tier.hops();
        let target_id_resolved = if fuzzy { self.find_fuzzy_id(target_id).unwrap_or(target_id.to_string()) } else { target_id.to_string() };
        
        // 1. Graph Expansion (BFS)
        let mut nodes = HashMap::new();
        let mut edges = Vec::new();
        let mut queue = VecDeque::new();
        
        if let Some(u32_id) = self.get_u32(&target_id_resolved) {
            queue.push_back((u32_id, 0));
            if let Some(node) = self.nodes.get(&u32_id) {
                nodes.insert(u32_id, node.value().clone());
            }
        }

        let mut visited = HashSet::new();
        while let Some((curr_u32, curr_depth)) = queue.pop_front() {
            if curr_depth >= hops || visited.contains(&curr_u32) { continue; }
            visited.insert(curr_u32);

            if let Some(eids) = self.out_idx.get(&curr_u32) {
                for eid in eids.iter() {
                    if let Some(edge_ref) = self.edges.get(eid) {
                        let edge = edge_ref.value();
                        edges.push(edge.clone());
                        if let Some(next_u32) = self.get_u32(&edge.to) {
                            if !nodes.contains_key(&next_u32) {
                                if let Some(node) = self.nodes.get(&next_u32) {
                                    nodes.insert(next_u32, node.value().clone());
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
                            if !nodes.contains_key(&prev_u32) {
                                if let Some(node) = self.nodes.get(&prev_u32) {
                                    nodes.insert(prev_u32, node.value().clone());
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

        if let Some(b) = budget {
            if token_estimate > b {
                // Compression: Switch to SuperNodes for high-level context
                println!("GRL: Budget exceeded ({} > {}). Compressing to SuperNodes.", token_estimate, b);
                for entry in self.meta_nodes.iter() {
                    super_nodes.push(entry.value().clone());
                }
                final_nodes.clear(); // Prune atoms
                edges.clear();
            }
        }

        Ok(ContextPackage {
            nodes: final_nodes,
            edges,
            super_nodes,
            token_estimate,
            reasoning_path: format!("Resolved {} as of Tier {} ({} hops)", target_id_resolved, tier_str, hops),
        })
    }

    pub fn start_gossip_manager(storage: Arc<Self>) {
        let _peer_id = storage.local_peer_id.clone();
        let _verifying_key_bytes = storage.verifying_key.to_bytes().to_vec();

        tokio::spawn(async move {
            let socket = match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => {
                    let addr = s.local_addr().unwrap();
                    storage.gossip_port.store(addr.port() as u32, Ordering::SeqCst);
                    println!("Gossip: Bound to UDP port {}", addr.port());
                    s
                }
                Err(e) => {
                    println!("Gossip: Failed to bind UDP socket: {}", e);
                    return;
                }
            };
            socket.set_broadcast(true).unwrap();

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
                                                };
                                                if let Ok(data) = serde_json::to_vec(&req) {
                                                    let _ = socket.send_to(&data, peer_addr).await;
                                                }
                                            }
                                        }
                                    }
                                    GossipMessage::PullRequest { from_clock, target_peer_id } => {
                                        // Anti-entropy: reply with our events newer than the
                                        // requester's clock, addressed to its registered gossip
                                        // addr (the heartbeat port, not the ephemeral src port).
                                        // Bound the reply to one UDP datagram; the requester's
                                        // clock advances on apply, so the next heartbeat round
                                        // pulls the remainder until the roots stop differing.
                                        if let Some(reply_addr) = storage.peers.get(&target_peer_id).map(|p| p.addr.clone()) {
                                            let mut batch = Vec::new();
                                            let mut bytes = 0usize;
                                            for ev in storage.events_since(from_clock) {
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
                                    GossipMessage::ConsensusPropose { proposal } => {
                                        // Verify the proposal's event is authentically signed by
                                        // its claimed originator before storing it. An unsigned or
                                        // forged proposal could otherwise be driven to quorum and
                                        // applied as a MASTER axiom, bypassing governance.
                                        if storage.verify_event_signature(&proposal.signed_event) {
                                            storage.proposals.insert(proposal.proposal_id.clone(), proposal);
                                        } else {
                                            println!("Mark X: REJECTED proposal {} — invalid event signature or unknown signer.", proposal.proposal_id);
                                        }
                                    }
                                    GossipMessage::ConsensusVote { proposal_id, voter_peer_id, approve, signature } => {
                                        // submit_vote verifies the signature against the
                                        // voter's registered key; forged votes are dropped.
                                        let _ = storage.submit_vote(proposal_id, voter_peer_id, approve, signature);
                                        // TODO: verify signature of the vote itself if needed
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
            socket.set_broadcast(true).unwrap();
            let mut buf = [0u8; 65535];
            loop {
                if let Ok((len, _addr)) = socket.recv_from(&mut buf).await {
                    if let Ok(GossipMessage::Heartbeat { .. }) = serde_json::from_slice::<GossipMessage>(&buf[..len]) {
                        // Discovery logic handled via broadcast in main loop
                    }
                }
            }
        });
    }

    pub fn save_state(&self) -> Result<()> {
        self.ensure_writable()?;
        let temp_dir = self.path.join("temp_save");
        if temp_dir.exists() { let _ = fs::remove_dir_all(&temp_dir); }
        fs::create_dir_all(&temp_dir).ok();

        // 1. Per-collection arenas + metadata + a manifest. HNSW is NOT dumped —
        //    it rehydrates cheaply from each arena on load (the arena is the
        //    source of truth). state.json's `collections` array drives reload.
        let mut manifest: Vec<serde_json::Value> = Vec::new();
        for c in self.collections.iter() {
            let coll = c.value();
            let arena = coll.arena.read();
            fs::write(temp_dir.join(format!("vec_{}.bin", coll.name)), arena.to_bytes())
                .map_err(|e| Error::from_reason(e.to_string()))?;
            let meta = coll.metadata.read();
            let meta_data = bincode::serialize(&*meta).map_err(|e| Error::from_reason(e.to_string()))?;
            fs::write(temp_dir.join(format!("meta_{}.bin", coll.name)), meta_data).map_err(|e| Error::from_reason(e.to_string()))?;
            // Rerank sidecar (exact f32) — only when the collection carries one.
            if let Some(sidecar) = &coll.f32_sidecar {
                let s = sidecar.read();
                let bytes: Vec<u8> = s.iter().flat_map(|f| f.to_le_bytes()).collect();
                fs::write(temp_dir.join(format!("fvec_{}.bin", coll.name)), bytes)
                    .map_err(|e| Error::from_reason(e.to_string()))?;
            }
            manifest.push(serde_json::json!({
                "name": coll.name, "model": coll.model, "dim": coll.dim,
                "metric": coll.metric.as_str(), "quant": coll.quant.as_str(),
                // Per-collection default ef_search; absent ⇒ None (engine-global).
                "ef_search": coll.ef_search,
                // f32-sidecar rerank; absent ⇒ false (no sidecar loaded).
                "rerank": coll.f32_sidecar.is_some(),
                // meta format version: 1 = NodeMetadata.node_u32 (A2). Absent ⇒ 0
                // (pre-A2 String layout), migrated on load.
                "mv": 1
            }));
        }

        // 2. Save DashMaps (Partial state for instant load)
        let nodes: Vec<(u32, NodeOutput)> = self.nodes.iter().map(|e| (*e.key(), e.value().clone())).collect();
        let edges: Vec<(u128, EdgeOutput)> = self.edges.iter().map(|e| (*e.key(), e.value().clone())).collect();
        fs::write(temp_dir.join("nodes.bin"), serde_json::to_vec(&nodes).unwrap()).ok();
        fs::write(temp_dir.join("edges.bin"), serde_json::to_vec(&edges).unwrap()).ok();

        // 3. Save Global Metadata (incl. collections manifest)
        let state = serde_json::json!({
            "logical_clock": self.get_logical_clock(),
            "peer_id": self.local_peer_id,
            "collections": manifest,
            "schema_version": SCHEMA_VERSION,
            "timestamp": Utc::now().to_rfc3339(),
        });
        fs::write(temp_dir.join("state.json"), state.to_string()).ok();

        // Atomic-ish swap: per-collection filenames are dynamic, so move every
        // file produced into the db root instead of a fixed list.
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.path().file_name() {
                    fs::rename(entry.path(), self.path.join(name)).ok();
                }
            }
        }
        let _ = fs::remove_dir_all(&temp_dir);

        println!("Mark IX: State persisted successfully to {}", self.path.display());
        Ok(())
    }

    fn try_load_state(&self) -> bool {
        let state_path = self.path.join("state.json");
        if !state_path.exists() { return false; }

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
                let coll = VectorCollection::new(name.clone(), model, dim, metric, quant, ef_search, rerank);
                if let Ok(data) = fs::read(self.path.join(format!("vec_{}.bin", name))) {
                    *coll.arena.write() = ArenaStore::from_bytes(&data, quant, dim as usize);
                }
                // Rerank sidecar: exact f32 vectors, parallel to the arena. Only
                // present when the collection opted into rerank (and is quantized).
                if let Some(sidecar) = &coll.f32_sidecar {
                    if let Ok(data) = fs::read(self.path.join(format!("fvec_{}.bin", name))) {
                        *sidecar.write() = data.chunks_exact(4)
                            .map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
                    }
                }
                if let Ok(data) = fs::read(self.path.join(format!("meta_{}.bin", name))) {
                    if cm["mv"].as_u64().unwrap_or(0) >= 1 {
                        if let Ok(meta) = bincode::deserialize::<Vec<NodeMetadata>>(&data) {
                            coll.count.store(meta.len(), Ordering::Relaxed);
                            *coll.metadata.write() = meta;
                        }
                    } else if let Ok(v0) = bincode::deserialize::<Vec<NodeMetadataV0>>(&data) {
                        // Pre-A2 String layout — migrate to interned u32 in step 3.
                        coll.count.store(v0.len(), Ordering::Relaxed);
                        legacy_meta.insert(name.clone(), v0);
                    }
                }
                self.collections.insert(name, Arc::new(coll));
            }
        } else if let Ok(data) = fs::read(self.path.join("vector.bin")) {
            // Legacy single-space DB -> wrap as the `default` collection.
            let dim = state_val["vector_dim"].as_u64().unwrap_or(1536) as u16;
            let coll = VectorCollection::new("default".to_string(), "default".to_string(), dim, Metric::L2, Quant::None, None, false);
            *coll.arena.write() = ArenaStore::from_bytes(&data, Quant::None, dim as usize);
            if let Ok(md) = fs::read(self.path.join("meta.bin")) {
                // Legacy single-space snapshots always predate A2 (String layout).
                if let Ok(v0) = bincode::deserialize::<Vec<NodeMetadataV0>>(&md) {
                    coll.count.store(v0.len(), Ordering::Relaxed);
                    legacy_meta.insert("default".to_string(), v0);
                }
            }
            self.collections.insert("default".to_string(), Arc::new(coll));
        } else {
            return false;
        }
        // Guarantee the default collection always exists post-load.
        if !self.collections.contains_key(&self.default_collection) {
            self.collections.insert(
                self.default_collection.clone(),
                Arc::new(VectorCollection::new(self.default_collection.clone(), "default".to_string(), 1536, Metric::L2, Quant::None, None, false)),
            );
        }

        // 2. Load Maps
        if let Ok(data) = fs::read(self.path.join("nodes.bin")) {
            match serde_json::from_slice::<Vec<(u32, NodeOutput)>>(&data) {
                Ok(nodes) => {
                    println!("Mark IX: Loading {} nodes from snapshot", nodes.len());
                    let mut max_u32 = 0;
                    for (k, v) in nodes {
                        if k > max_u32 { max_u32 = k; }
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
                            self.trigram_index.entry(trigram).or_insert_with(RoaringBitmap::new).insert(k);
                        }
                        self.insert_node_lean(k, v);
                    }
                    self.next_u32.store(max_u32 + 1, Ordering::SeqCst);
                }
                Err(e) => { println!("Mark IX: Failed to deserialize nodes: {}", e); return false; }
            }
        } else { println!("Mark IX: nodes.bin not found"); }

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
                    self.out_idx.entry(from_u32).or_insert_with(HashSet::new).insert(k);
                    self.in_idx.entry(to_u32).or_insert_with(HashSet::new).insert(k);
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
                    let nu = self.get_u32(&v0.node_id).unwrap_or_else(|| self.get_or_intern_id(&v0.node_id));
                    coll.node_to_arena.insert(nu, v0.arena_id);
                    migrated.push(NodeMetadata {
                        arena_id: v0.arena_id, node_u32: nu, timestamp: v0.timestamp,
                        vector_dim: v0.vector_dim, embedding_offset: v0.embedding_offset,
                        gks_attributes: v0.gks_attributes, lang: v0.lang, cluster_id: v0.cluster_id,
                    });
                }
                *coll.metadata.write() = migrated;
            } else {
                let meta = coll.metadata.read();
                for m in meta.iter() { coll.node_to_arena.insert(m.node_u32, m.arena_id); }
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
            let expires_at = args.ttl.map(|s| (now + chrono::Duration::seconds(s as i64)).to_rfc3339());

            // Resolve + dim-validate the target collection up-front so a bad
            // vector fails the whole batch BEFORE the WAL write (all-or-nothing).
            let coll_name = if let Some(emb) = &args.embedding {
                let coll = self.resolve_collection(&args.collection)?;
                if emb.len() != coll.dim as usize {
                    return Err(Error::from_reason(format!(
                        "embedding dim {} != collection '{}' dim {}", emb.len(), coll.name, coll.dim
                    )));
                }
                Some(coll.name.clone())
            } else { None };

            let node = NodeOutput {
                id: id.clone(), labels: args.labels,
                props: args.props.unwrap_or(Value::Object(Default::default())),
                impact: Some(0.7), embedding: args.embedding.clone(),
                lang: Some(lang.clone()),
                valid_from: args.valid_from.unwrap_or_else(|| now.to_rfc3339()),
                valid_to: None, caused_by: args.caused_by, expires_at,
                clock: self.next_clock(),
                collection: coll_name,
            };

            events.push(Event::Node(node.clone()));
            output_nodes.push(node);
        }

        for args in input.edges {
            let edge = EdgeOutput {
                id: args.id.unwrap_or_else(|| Uuid::new_v4().to_string()), 
                from: args.from, to: args.to, rel: args.rel,
                props: args.props.unwrap_or(Value::Object(Default::default())), 
                valid_from: Utc::now().to_rfc3339(), valid_to: None, recorded_at: Utc::now().to_rfc3339(),
                superseded_by: None, impact: args.impact, caused_by: args.caused_by,
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
        let mut by_coll: HashMap<String, Vec<(u32, String, Vec<f64>, String)>> = HashMap::new();
        for event in events {
            match event {
                Event::Node(n) => {
                    let u32_id = self.get_or_intern_id(&n.id);
                    if let Some(emb) = n.embedding.clone() {
                        let cn = n.collection.clone().unwrap_or_else(|| self.default_collection.clone());
                        by_coll.entry(cn).or_default().push((u32_id, n.id.clone(), emb, n.lang.clone().unwrap_or_else(|| "en".to_string())));
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

        Ok(BatchOutput { nodes: output_nodes, edges: output_edges })
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
            // → sidecar). It uses the same `embedding_offset`/`len` component units
            // as the arena, so its slices move identically.
            let mut sidecar_guard = coll.f32_sidecar.as_ref().map(|s| s.write());

            let mut new_meta = Vec::with_capacity(live_nodes.len());
            let mut new_vec = ArenaStore::new(coll.quant, coll.dim as usize);
            let mut new_sidecar: Vec<f32> = Vec::new();
            coll.node_to_arena.clear();

            for meta in meta_arena.iter() {
                { let u32_id = meta.node_u32; // A2: id interned in metadata
                    if live_nodes.contains(&u32_id) {
                        let start_off = meta.embedding_offset as usize;
                        let len = meta.vector_dim as usize;
                        if start_off + len <= vec_arena.len() {
                            let new_offset = new_vec.len() as u64;
                            new_vec.append_range(&vec_arena, start_off, len);
                            if let Some(old_sidecar) = sidecar_guard.as_deref() {
                                if let Some(slice) = old_sidecar.get(start_off..start_off + len) {
                                    new_sidecar.extend_from_slice(slice);
                                }
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
            if let Some(g) = sidecar_guard.as_mut() { **g = new_sidecar; }
        }

        // 3. Rebuild every collection's HNSW from its compacted arena.
        self.rehydrate_hnsw_index();

        // 4. Prune Adjacency Indices
        let mut orphaned_indices = Vec::new();
        for entry in self.out_idx.iter() {
            if !live_nodes.contains(entry.key()) { orphaned_indices.push(*entry.key()); }
        }
        for k in orphaned_indices { self.out_idx.remove(&k); }

        let mut orphaned_in = Vec::new();
        for entry in self.in_idx.iter() {
            if !live_nodes.contains(entry.key()) { orphaned_in.push(*entry.key()); }
        }
        for k in orphaned_in { self.in_idx.remove(&k); }

        println!("Mark IX: Index Compaction complete in {:?}. Arenas resized to {} nodes.", start.elapsed(), live_nodes.len());
        Ok(())
    }

    pub fn set_language_centroid(&self, lang: String, vector: Vec<f64>) {
        let v_f32: Vec<f32> = vector.into_iter().map(|v| v as f32).collect();
        self.lang_centroids.insert(lang, v_f32);
    }

    pub fn get_merkle_root(&self) -> String {
        if !self.log_path.exists() { return "0".repeat(64); }
        let mut hasher = Sha256::new();
        if let Ok(file) = File::open(&self.log_path) {
            let reader = std::io::BufReader::new(file);
            use std::io::BufRead;
            for line_res in reader.lines() {
                if let Ok(line) = line_res { hasher.update(line.as_bytes()); }
            }
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
        if !self.log_path.exists() { return Vec::new(); }
        let mut out: Vec<(u32, SignedEvent)> = Vec::new();
        if let Ok(file) = File::open(&self.log_path) {
            let reader = std::io::BufReader::new(file);
            use std::io::BufRead;
            for line in reader.lines().map_while(|r| r.ok()) {
                if let Ok(se) = serde_json::from_str::<SignedEvent>(&line) {
                    if let Some(t) = Self::event_time(&se.event) {
                        if t > from_clock { out.push((t, se)); }
                    }
                }
            }
        }
        out.sort_by_key(|(t, _)| *t);
        out.into_iter().map(|(_, se)| se).collect()
    }

    pub fn compact(&self) -> Result<()> {
        self.ensure_writable()?;
        let new_log_path = self.path.join("genesis-graph.wal.new");
        let mut writer = std::io::BufWriter::new(File::create(&new_log_path).map_err(|e| Error::from_reason(e.to_string()))?);
        
        let now = Utc::now().to_rfc3339();
        let mut count = 0;

        // 1. Write current live nodes
        for entry in self.nodes.iter() {
            let node = entry.value();
            if let Some(exp) = &node.expires_at {
                if now > *exp { continue; }
            }
            if node.valid_to.is_none() {
                if let Ok(json) = serde_json::to_string(&self.sign_event(&Event::Node(node.clone()))) {
                    let _ = writer.write_all(json.as_bytes());
                    let _ = writer.write_all(b"\n");
                    count += 1;
                }
            }
        }

        // 2. Write current live edges
        for entry in self.edges.iter() {
            let edge = entry.value();
            if edge.valid_to.is_none() {
                if let Ok(json) = serde_json::to_string(&self.sign_event(&Event::Edge(edge.clone()))) {
                    let _ = writer.write_all(json.as_bytes());
                    let _ = writer.write_all(b"\n");
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
            use std::io::BufRead;
            let mut latest: HashMap<(String, Option<String>), VectorEvent> = HashMap::new();
            if let Ok(file) = File::open(&self.log_path) {
                let reader = std::io::BufReader::new(file);
                for line in reader.lines().map_while(|r| r.ok()) {
                    if let Ok(se) = serde_json::from_str::<SignedEvent>(&line) {
                        if let Event::Vector(v) = se.event {
                            let live = self.get_u32(&v.node_id).map_or(false, |u| self.nodes.contains_key(&u));
                            if !live { continue; }
                            let key = (v.node_id.clone(), v.collection.clone());
                            match latest.get(&key) {
                                Some(prev) if prev.clock.time >= v.clock.time => {}
                                _ => { latest.insert(key, v); }
                            }
                        }
                    }
                }
            }
            for v in latest.into_values() {
                if let Ok(json) = serde_json::to_string(&self.sign_event(&Event::Vector(v))) {
                    let _ = writer.write_all(json.as_bytes());
                    let _ = writer.write_all(b"\n");
                    count += 1;
                }
            }
        }

        writer.flush().ok();
        fs::rename(&new_log_path, &self.log_path).ok();
        println!("Mark IX: WAL Compacted. {} live events preserved.", count);
        Ok(())
    }
    /// Bitemporal retraction (soft-delete): set the edge's `valid_to` so it is no
    /// longer live in the current view, while preserving the relationship for
    /// time-travel queries (`as_of` before `at`, or `include_invalid = true`).
    /// `at` defaults to now. Returns the retracted edge, or `None` if no edge with
    /// `id` exists. Idempotent on the live/retracted distinction — re-retracting
    /// just moves `valid_to`.
    pub fn retract_edge(&self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> {
        self.ensure_writable()?;
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
    pub fn status_sync(&self) -> DatabaseStatus { DatabaseStatus { open: true, read_only: self.read_only, page_cache_mb: 512 } }
    // Bulk paths route through execute_batch so each chunk is ONE Event::Batch =
    // ONE WAL fsync (vs one fsync per item). Chunked to bound the size of a
    // single serialized batch / fsync.
    const BULK_CHUNK: usize = 1024;
    pub fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> {
        let mut it = inputs.into_iter();
        loop {
            let chunk: Vec<NodeInput> = it.by_ref().take(Self::BULK_CHUNK).collect();
            if chunk.is_empty() { break; }
            self.execute_batch(BatchInput { nodes: chunk, edges: Vec::new() })?;
        }
        Ok(())
    }
    pub fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> {
        let mut it = inputs.into_iter();
        loop {
            let chunk: Vec<EdgeInput> = it.by_ref().take(Self::BULK_CHUNK).collect();
            if chunk.is_empty() { break; }
            self.execute_batch(BatchInput { nodes: Vec::new(), edges: chunk })?;
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
                let entry = cluster_centroids.entry(c_id).or_insert_with(|| vec![0.0; meta.vector_dim as usize]);
                for (i, val) in vec.iter().enumerate() { entry[i] += val; }
                *cluster_member_count.entry(c_id).or_insert(0) += 1;
                { let u32_id = meta.node_u32; // A2: id interned in metadata
                    if let Some(node) = self.nodes.get(&u32_id) { *cluster_impact.entry(c_id).or_insert(0.0) += node.value().impact.unwrap_or(0.0); }
                }
            }
        }
        for (c_id, centroid) in cluster_centroids.iter_mut() {
            let count = cluster_member_count[c_id] as f32;
            for val in centroid.iter_mut() { *val /= count; }
        }
        let cluster_ids: Vec<u32> = cluster_centroids.keys().cloned().collect();
        for i in 0..cluster_ids.len() {
            for j in i + 1..cluster_ids.len() {
                let id_a = cluster_ids[i]; let id_b = cluster_ids[j];
                let avg_impact_a = cluster_impact[&id_a] / cluster_member_count[&id_a] as f64;
                let avg_impact_b = cluster_impact[&id_b] / cluster_member_count[&id_b] as f64;
                if avg_impact_a < 0.5 || avg_impact_b < 0.5 { continue; }
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
        self.meta_history.get(&cluster_id).map(|v| v.value().clone()).unwrap_or_default()
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
        if let Some(h) = self.wal_handle.take() { let _ = h.join(); }
        if let Some(h) = self.index_handle.take() { let _ = h.join(); }
        if !self.read_only {
            println!("Mark IX: Graceful shutdown. State saved.");
        }
    }
}

#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GapSuggestion {
    pub cluster_a: u32,
    pub cluster_b: u32,
    pub similarity: f64,
    pub reason: String,
}

#[napi]
pub struct GenesisDatabase { inner: Arc<Storage> }

#[napi]
impl GenesisDatabase {
    #[napi(factory)]
    pub fn open(opts: OpenOptions) -> Result<Self> { 
        let storage = Arc::new(Storage::open(opts)?);
        Storage::start_autonomic_loop(Arc::clone(&storage));
        Storage::start_gossip_manager(Arc::clone(&storage));
        Ok(Self { inner: storage }) 
    }
    #[napi] pub async fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.bulk_add_nodes(inputs)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.bulk_add_edges(inputs)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn rebuild_index_parallel(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.rebuild_index_parallel()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn add_node(&self, args: NodeInput) -> Result<NodeOutput> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.add_node(args)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.add_edge(args)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn supersede_node(&self, id: String, new_props: Option<serde_json::Value>, caused_by: Option<String>) -> Result<NodeOutput> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.supersede_node(id, new_props, caused_by)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn retract_edge(&self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.retract_edge(id, at)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn retrieve_context(&self, target_id: String, tier: String, budget: Option<u32>, fuzzy: bool) -> Result<ContextPackage> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || i.retrieve_context(&target_id, &tier, budget, fuzzy)).await.map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi] pub async fn execute_hql(&self, query: String) -> Result<Value> { 
        let i = Arc::clone(&self.inner); 
        let res = tokio::task::spawn_blocking(move || i.execute_hql(&query)).await.map_err(|e| Error::from_reason(e.to_string()))??;
        Ok(serde_json::to_value(res).map_err(|e| Error::from_reason(e.to_string()))?)
    }
    #[napi] pub async fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.hybrid_search(args)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn neighbors(&self, seed: String, args: NeighborInput) -> Result<Vec<NeighborOutput>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.neighbors(seed, args, false)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn save_state(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.save_state()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn compact(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.compact()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn create_collection(&self, name: String, model: String, dim: u32, metric: Option<String>, quant: Option<String>, ef_search: Option<u32>, rerank: Option<bool>) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.create_collection(name, model, dim, metric, quant, ef_search, rerank)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub fn list_collections(&self) -> Vec<CollectionInfo> { self.inner.list_collections() }
    #[napi] pub async fn add_vector(&self, node_id: String, collection: String, embedding: Vec<f64>) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.add_vector(node_id, collection, embedding)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn flush_index(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.flush_index()).await.map_err(|e| Error::from_reason(e.to_string())) }
    #[napi] pub fn index_lag(&self) -> u32 { self.inner.index_lag() }
    #[napi] pub fn set_language_centroid(&self, lang: String, vector: Vec<f64>) { self.inner.set_language_centroid(lang, vector); }
    #[napi] pub fn set_index_params(&self, ef_construction: u32, ef_search: u32) { self.inner.set_index_params(ef_construction, ef_search); }
    #[napi] pub async fn detect_communities(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.detect_communities()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn calculate_structural_gaps(&self) -> Result<Vec<GapSuggestion>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.calculate_structural_gaps()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn generate_meta_graph(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.generate_meta_graph()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn get_meta_history(&self, cluster_id: u32) -> Result<Vec<SuperNode>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || Ok(i.get_meta_history(cluster_id))).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn reconcile_state(&self, events_json: String) -> Result<()> {
        let i = Arc::clone(&self.inner);
        let events = serde_json::from_str::<Vec<SignedEvent>>(&events_json).map_err(|e| Error::from_reason(e.to_string()))?;
        tokio::task::spawn_blocking(move || i.reconcile_state(events)).await.map_err(|e| Error::from_reason(e.to_string()))?
    }
    /// Anti-entropy source side: JSON of the signed events newer than `from_clock`
    /// (sorted by logical time). Pair with `reconcile_state` on the puller.
    #[napi] pub async fn events_since(&self, from_clock: u32) -> Result<String> {
        let i = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || serde_json::to_string(&i.events_since(from_clock)).map_err(|e| Error::from_reason(e.to_string())))
            .await.map_err(|e| Error::from_reason(e.to_string()))?
    }
    #[napi] pub async fn semantic_verify(&self, event_json: String) -> Result<bool> { 
        let i = Arc::clone(&self.inner); 
        let event = serde_json::from_str::<Event>(&event_json).map_err(|e| Error::from_reason(e.to_string()))?;
        tokio::task::spawn_blocking(move || i.semantic_verify(&event)).await.map_err(|e| Error::from_reason(e.to_string()))? 
    }
    #[napi] pub async fn propose_consensus(&self, event_json: String, signature: Vec<u8>) -> Result<String> { 
        let i = Arc::clone(&self.inner); 
        let event = serde_json::from_str::<Event>(&event_json).map_err(|e| Error::from_reason(e.to_string()))?;
        tokio::task::spawn_blocking(move || i.propose_consensus(event, signature)).await.map_err(|e| Error::from_reason(e.to_string()))? 
    }
    #[napi] pub async fn submit_vote(&self, proposal_id: String, peer_id: String, approve: bool, signature: Vec<u8>) -> Result<bool> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.submit_vote(proposal_id, peer_id, approve, signature)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub fn sign_vote(&self, proposal_id: String, approve: bool) -> Vec<u8> { self.inner.sign_vote(proposal_id, approve) }
    #[napi] pub fn get_local_peer_id(&self) -> String { self.inner.local_peer_id.clone() }
    #[napi] pub fn get_logical_clock(&self) -> u32 { self.inner.logical_clock.load(Ordering::SeqCst) }
    #[napi] pub fn get_merkle_root(&self) -> String { self.inner.get_merkle_root() }
    #[napi] pub fn schema_version_sync(&self) -> u32 { SCHEMA_VERSION }
    #[napi] pub fn status_sync(&self) -> DatabaseStatus { self.inner.status_sync() }
}
#[napi] pub fn engine_name_sync() -> String { "genesis-block".to_string() }
#[napi] pub fn schema_version_sync() -> u32 { SCHEMA_VERSION }
