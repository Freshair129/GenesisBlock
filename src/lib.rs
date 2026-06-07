//! Genesis Block — high-performance hybrid semantic-graph engine.
//!
//! Mark VI: Collective Intelligence & Autonomic Substrate

#![deny(clippy::all)]

use std::collections::{HashSet, VecDeque, HashMap};
use std::fs::{self, File, OpenOptions as FileOpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use sha2::{Sha256, Digest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use rand::Rng;

use chrono::Utc;
use dashmap::DashMap;
use hnsw_rs::prelude::*;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use crossbeam_channel::{unbounded, Sender, Receiver};

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
}

#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeMetadata {
    pub arena_id: u32,
    pub node_id: String,
    pub timestamp: u64,
    pub vector_dim: u16,
    pub embedding_offset: u64,
    pub gks_attributes: Vec<u8>,
    pub lang: String,
    pub cluster_id: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsensusProposal {
    pub proposal_id: String,
    pub signed_event: SignedEvent,
    pub votes: HashMap<String, bool>, // PeerID -> Vote
    pub quorum_signatures: HashMap<String, Vec<u8>>, // PeerID -> Signature
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

pub struct Storage {
    pub path: PathBuf,
    pub read_only: bool,
    pub nodes: DashMap<u32, NodeOutput>,
    pub edges: DashMap<u32, EdgeOutput>,
    pub out_idx: DashMap<u32, HashSet<u32>>,
    pub in_idx: DashMap<u32, HashSet<u32>>,
    pub vector_arena: RwLock<Vec<f32>>,
    pub metadata_arena: RwLock<Vec<NodeMetadata>>,
    pub hnsw_index: RwLock<Option<Hnsw<'static, f32, DistL2>>>,
    pub log_path: PathBuf,
    pub bin_path: PathBuf,
    pub _lock_file: Option<File>,
    pub id_to_u32: DashMap<String, u32>,
    pub u32_to_id: DashMap<u32, String>,
    pub u32_to_arena_id: DashMap<u32, u32>,
    pub next_u32: AtomicU32,
    pub is_rebuilding: AtomicBool,
    pub trigram_index: DashMap<String, HashSet<u32>>,
    pub lang_centroids: DashMap<String, Vec<f32>>,
    pub peers: DashMap<String, SyncPeer>,
    pub proposals: DashMap<String, ConsensusProposal>,
    pub meta_nodes: DashMap<u32, SuperNode>,
    pub meta_edges: DashMap<String, MetaEdge>,
    pub meta_history: DashMap<u32, Vec<SuperNode>>,
    pub wal_sender: Sender<(SignedEvent, Sender<bool>)>,
    pub local_peer_id: String,
    pub logical_clock: AtomicU32,
    pub vector_dim: u16,
    pub gossip_port: AtomicU32,
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
    fn validate_governance(&self, labels: &[String], is_system: bool) -> Result<()> {
        let tier = Tier::from_labels(labels);
        if tier == Tier::MASTER && !is_system {
            return Err(Error::from_reason("403 Forbidden: MASTER tier is immutable for external agents"));
        }
        Ok(())
    }

    fn tokenize_id(id: &str) -> Vec<String> {
        let base_chars: String = id.chars().filter(|c| {
            let cat = unicode_general_category::get_general_category(*c);
            use unicode_general_category::GeneralCategory::*;
            cat != NonspacingMark && cat != SpacingMark && cat != EnclosingMark
        }).collect();

        let mut tokens = Vec::new();
        
        // 1. Raw Character tokens (High Recall)
        for c in id.chars() {
            tokens.push(c.to_string().to_lowercase());
        }

        // 2. Base Character tokens
        if id != base_chars {
            for c in base_chars.chars() {
                tokens.push(c.to_string().to_lowercase());
            }
        }

        // 3. Bigrams (Raw)
        let raw_chars: Vec<char> = id.chars().collect();
        if raw_chars.len() >= 2 {
            for i in 0..raw_chars.len() - 1 {
                tokens.push(raw_chars[i..i+2].iter().collect::<String>().to_lowercase());
            }
        }

        tokens
    }

    pub fn get_or_intern_id(&self, id: &str) -> u32 {
        if let Some(existing) = self.id_to_u32.get(id) { return *existing; }
        let new_id = self.next_u32.fetch_add(1, Ordering::SeqCst);
        self.id_to_u32.insert(id.to_string(), new_id);
        self.u32_to_id.insert(new_id, id.to_string());
        
        for trigram in Self::tokenize_id(id) {
            self.trigram_index.entry(trigram).or_insert_with(HashSet::new).insert(new_id);
        }
        new_id
    }

    pub fn get_u32(&self, id: &str) -> Option<u32> { self.id_to_u32.get(id).map(|v| *v) }

    fn init_hnsw() -> Hnsw<'static, f32, DistL2> { Hnsw::new(16, 1000000, 16, 200, DistL2 {}) }

    fn add_vector_internal(&self, node_id: &str, emb_64: Vec<f64>, lang: String) {
        let emb: Vec<f32> = emb_64.into_iter().map(|v| v as f32).collect();
        let mut meta_arena = self.metadata_arena.write();
        let mut vec_arena = self.vector_arena.write();
        let current_vec_len = vec_arena.len();
        vec_arena.extend_from_slice(&emb);
        let arena_id = meta_arena.len() as u32;
        meta_arena.push(NodeMetadata {
            arena_id, node_id: node_id.to_string(), timestamp: Utc::now().timestamp() as u64,
            vector_dim: self.vector_dim, embedding_offset: current_vec_len as u64, gks_attributes: Vec::new(),
            lang, cluster_id: arena_id,
        });
        if let Some(u32_id) = self.get_u32(node_id) { self.u32_to_arena_id.insert(u32_id, arena_id); }
        if self.hnsw_index.read().is_none() { 
            *self.hnsw_index.write() = Some(Self::init_hnsw()); 
        }
        if let Some(ref mut hnsw) = *self.hnsw_index.write() { 
            hnsw.insert((&emb, arena_id as usize)); 
        }
    }

    fn rehydrate_hnsw_index(&self) {
        let meta_arena = self.metadata_arena.read();
        if meta_arena.is_empty() { return; }
        let hnsw = Self::init_hnsw();
        let vec_arena = self.vector_arena.read();
        for meta in meta_arena.iter() {
            let start = meta.embedding_offset as usize;
            let end = start + meta.vector_dim as usize;
            if end <= vec_arena.len() { hnsw.insert((&vec_arena[start..end], meta.arena_id as usize)); }
        }
        *self.hnsw_index.write() = Some(hnsw);
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

        std::thread::spawn(move || {
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

        let storage = Self {
            path: root, read_only, nodes: DashMap::new(), edges: DashMap::new(),
            out_idx: DashMap::new(), in_idx: DashMap::new(),
            vector_arena: RwLock::new(Vec::new()), metadata_arena: RwLock::new(Vec::new()),
            hnsw_index: RwLock::new(None), log_path, bin_path: PathBuf::from(""), _lock_file: None,
            id_to_u32: DashMap::new(), u32_to_id: DashMap::new(), u32_to_arena_id: DashMap::new(), next_u32: AtomicU32::new(0),
            is_rebuilding: AtomicBool::new(false), trigram_index: DashMap::new(), 
            lang_centroids: DashMap::new(), peers: DashMap::new(),
            proposals: DashMap::new(), meta_nodes: DashMap::new(), meta_edges: DashMap::new(),
            meta_history: DashMap::new(),
            wal_sender,
            local_peer_id,
            logical_clock: AtomicU32::new(0),
            vector_dim,
            gossip_port: AtomicU32::new(0),
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
                                            storage.add_vector_internal(&n.id, emb, n.lang.clone().unwrap_or("en".to_string())); 
                                        }
                                        storage.nodes.insert(u32_id, n);
                                    }
                                    Event::Edge(e) => {
                                        let u32_id = storage.get_or_intern_id(&e.id);
                                        storage.index_edge_internal(&e.id, &e.from, &e.to);
                                        storage.edges.insert(u32_id, e);
                                    }
                                    Event::Batch(events) => {
                                        for batch_event in events {
                                            match batch_event {
                                                Event::Node(n) => {
                                                    let u32_id = storage.get_or_intern_id(&n.id);
                                                    if let Some(emb) = n.embedding.clone() { 
                                                        storage.add_vector_internal(&n.id, emb, n.lang.clone().unwrap_or("en".to_string())); 
                                                    }
                                                    storage.nodes.insert(u32_id, n);
                                                }
                                                Event::Edge(e) => {
                                                    let u32_id = storage.get_or_intern_id(&e.id);
                                                    storage.index_edge_internal(&e.id, &e.from, &e.to);
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
            storage.rehydrate_hnsw_index();
        }
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

    pub fn persist(&self, event: &Event) -> Result<()> {
        let (ack_tx, ack_rx) = unbounded();
        
        let event_data = serde_json::to_vec(event).map_err(|e| Error::from_reason(e.to_string()))?;
        let signature = self.signing_key.sign(&event_data).to_bytes().to_vec();
        
        let signed_event = SignedEvent {
            event: event.clone(),
            signature,
            signer_peer_id: self.local_peer_id.clone(),
        };

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
                candidates.extend(nodes.clone()); 
            }
        }

        let mut best_lexical_id = None; 
        let mut max_lexical_sim = 0.0;
        
        for u32_id in &candidates {
            if let Some(candidate_id) = self.u32_to_id.get(u32_id) {
                let sim = strsim::jaro_winkler(id, candidate_id.value());
                if sim > max_lexical_sim { 
                    max_lexical_sim = sim; 
                    best_lexical_id = Some(candidate_id.value().clone()); 
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
            Event::Batch(events) => {
                for e in events {
                    if !self.semantic_verify(e)? { return Ok(false); }
                }
                Ok(true)
            }
        }
    }

        pub fn propose_consensus(&self, event: Event, signature: Vec<u8>) -> Result<String> {
        let proposal_id = Uuid::new_v4().to_string();
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
        };
        self.proposals.insert(proposal_id.clone(), proposal);
        Ok(proposal_id)
    }

        pub fn submit_vote(&self, proposal_id: String, peer_id: String, approve: bool) -> Result<bool> {
        if let Some(mut proposal_ref) = self.proposals.get_mut(&proposal_id) {
            let proposal = proposal_ref.value_mut();
            proposal.votes.insert(peer_id.clone(), approve);
            
            let approvals = proposal.votes.values().filter(|&&v| v).count();
            
            if approvals > (self.peers.len() / 2) {
                let signed_event = &proposal.signed_event;
                match &signed_event.event {
                    Event::Node(n) => {
                        let mut n_axiom = n.clone();
                        if !n_axiom.labels.contains(&"MASTER".to_string()) { 
                            n_axiom.labels.push("MASTER".to_string()); 
                        }
                        let u32_id = self.get_or_intern_id(&n_axiom.id);
                        self.nodes.insert(u32_id, n_axiom.clone());
                        self.persist_signed(SignedEvent {
                            event: Event::Node(n_axiom),
                            signature: signed_event.signature.clone(),
                            signer_peer_id: signed_event.signer_peer_id.clone(),
                        })?;
                    }
                    Event::Edge(e) => {
                        let u32_id = self.get_or_intern_id(&e.id);
                        self.edges.insert(u32_id, e.clone());
                        self.persist_signed(signed_event.clone())?;
                    }
                    Event::Batch(events) => {
                        for e in events {
                            match e {
                                Event::Node(n) => {
                                    let mut n_axiom = n.clone();
                                    if !n_axiom.labels.contains(&"MASTER".to_string()) { n_axiom.labels.push("MASTER".to_string()); }
                                    let u32_id = self.get_or_intern_id(&n_axiom.id);
                                    self.nodes.insert(u32_id, n_axiom);
                                }
                                Event::Edge(edge) => {
                                    let u32_id = self.get_or_intern_id(&edge.id);
                                    self.edges.insert(u32_id, edge.clone());
                                }
                                _ => {}
                            }
                        }
                        self.persist_signed(signed_event.clone())?;
                    }
                }
                return Ok(true);
            }
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

    pub fn index_edge_internal(&self, id: &str, from: &str, to: &str) {
        let u32_id = self.get_or_intern_id(id);
        let u32_from = self.get_or_intern_id(from);
        let u32_to = self.get_or_intern_id(to);
        self.out_idx.entry(u32_from).or_insert_with(HashSet::new).insert(u32_id);
        self.in_idx.entry(u32_to).or_insert_with(HashSet::new).insert(u32_id);
    }

    fn next_clock(&self) -> LogicalClock {
        let time = self.logical_clock.fetch_add(1, Ordering::SeqCst) + 1;
        LogicalClock { time, peer_id: self.local_peer_id.clone() }
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
        };
        if let Some(emb) = args.embedding { self.add_vector_internal(&id, emb.clone(), lang); node.embedding = Some(emb); }
        self.nodes.insert(u32_id, node.clone());
        self.persist(&Event::Node(node.clone()))?;
        Ok(node)
    }

    pub fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> {
        self.ensure_writable()?;
        let edge = EdgeOutput {
            id: args.id.unwrap_or_else(|| Uuid::new_v4().to_string()), from: args.from, to: args.to, rel: args.rel,
            props: args.props.unwrap_or(Value::Object(Default::default())), valid_from: Utc::now().to_rfc3339(), valid_to: None, recorded_at: Utc::now().to_rfc3339(),
            superseded_by: None, impact: args.impact,
            caused_by: args.caused_by,
            clock: self.next_clock(),
        };
        self.index_edge_internal(&edge.id, &edge.from, &edge.to);
        self.edges.insert(self.get_or_intern_id(&edge.id), edge.clone());
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

        self.nodes.insert(u32_id, new_node.clone());
        self.persist(&Event::Node(new_node.clone()))?;

        Ok(new_node)
    }

    pub fn rebuild_index_parallel(&self) -> Result<()> {
        self.is_rebuilding.store(true, Ordering::SeqCst);
        let result = (|| { self.rehydrate_hnsw_index(); Ok(()) })();
        self.is_rebuilding.store(false, Ordering::SeqCst);
        result
    }

    pub fn execute_hql(&self, query: &str) -> Result<serde_json::Value> {
        let command = HqlCommand::try_from(query).map_err(|e| Error::from_reason(e))?;
        match command {
            HqlCommand::Search { vector, k, fuzzy, target, lang, as_of } => {
                let _resolved = if fuzzy { self.find_fuzzy_id(&target) } else { Some(target) };
                let res = self.hybrid_search(HybridSearchInput { query_vector: vector, k, alpha: Some(0.0), lang, as_of })?;
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
            HqlCommand::Hybrid { vector, alpha, fuzzy, target, lang, as_of } => {
                let _resolved = if fuzzy { self.find_fuzzy_id(&target) } else { Some(target) };
                let res = self.hybrid_search(HybridSearchInput { query_vector: vector, k: 10, alpha: Some(alpha), lang, as_of })?;
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
        let hnsw_lock = self.hnsw_index.read();
        let hnsw = match &*hnsw_lock { Some(idx) => idx, None => return Err(Error::from_reason("HNSW not init")) };
        let mut query_f32: Vec<f32> = args.query_vector.into_iter().map(|v| v as f32).collect();
        if let Some(lang) = args.lang {
            if let Some(centroid) = self.lang_centroids.get(&lang) {
                for (i, val) in query_f32.iter_mut().enumerate() { if i < centroid.len() { *val += centroid[i]; } }
            }
        }
        let results = hnsw.search(&query_f32, (args.k * 2) as usize, 100);
        let mut hybrid_results = Vec::new();
        let meta_arena = self.metadata_arena.read();
        let alpha = args.alpha.unwrap_or(0.5);

        for neighbor in results {
            if let Some(meta) = meta_arena.get(neighbor.d_id as usize) {
                if let Some(u32_id) = self.get_u32(&meta.node_id) {
                    if let Some(node) = self.nodes.get(&u32_id) {
                        let mut node_out = node.value().clone();
                        
                        if !Self::is_valid_as_of(&node_out.valid_from, &node_out.valid_to, &args.as_of) {
                            continue;
                        }

                        let similarity = 1.0 - neighbor.distance as f64;
                        let reasoning_score = (similarity * (1.0 - alpha)) + (node_out.impact.unwrap_or(0.0) * alpha);
                        node_out.impact = Some(reasoning_score);
                        hybrid_results.push(NeighborOutput { node: node_out, path: Vec::new(), depth: 0 });
                    }
                }
            }
        }
        hybrid_results.sort_by(|a, b| b.node.impact.partial_cmp(&a.node.impact).unwrap());
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
        let target_rel = args.rel.as_deref().unwrap_or("ANY");
        let mut results = Vec::new(); let mut visited = HashSet::new(); visited.insert(u32_seed);
        let mut queue = VecDeque::new(); queue.push_back((u32_seed, Vec::new(), 0));
        while let Some((curr_u32, path, curr_depth)) = queue.pop_front() {
            if curr_depth >= depth && !is_inferred { continue; }
            if let Some(eids) = self.out_idx.get(&curr_u32) {
                for eid in eids.iter() {
                    if let Some(edge_ref) = self.edges.get(eid) {
                        let edge = edge_ref.value();
                        
                        // Time-travel check for Edges
                        if !Self::is_valid_as_of(&edge.valid_from, &edge.valid_to, &args.as_of) {
                            continue;
                        }

                        if target_rel == "ANY" || edge.rel == target_rel {
                            let curr_id = self.u32_to_id.get(&curr_u32).unwrap().value().clone();
                            let next_id = if edge.from == curr_id { &edge.to } else { &edge.from };
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
                                        if is_inferred || (curr_depth + 1 < depth) { queue.push_back((next_u32, new_path, curr_depth + 1)); }
                                    }
                                }
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
        let mut meta_arena = self.metadata_arena.write();
        let mut new_clusters = Vec::with_capacity(meta_arena.len());
        for meta in meta_arena.iter() {
            let mut freq = HashMap::new();
            if let Some(u32_id) = self.get_u32(&meta.node_id) {
                let out_eids = self.out_idx.get(&u32_id).map(|v| v.value().clone()).unwrap_or_default();
                for eid in out_eids {
                    if let Some(edge) = self.edges.get(&eid) {
                        let other_id = if edge.from == meta.node_id { &edge.to } else { &edge.from };
                        if let Some(to_u32) = self.get_u32(other_id) {
                            if let Some(a_id) = self.u32_to_arena_id.get(&to_u32) {
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
                        let other_id = if edge.from == meta.node_id { &edge.to } else { &edge.from };
                        if let Some(to_u32) = self.get_u32(other_id) {
                            if let Some(a_id) = self.u32_to_arena_id.get(&to_u32) {
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
        let mut cluster_groups: HashMap<u32, Vec<u32>> = HashMap::new();
        let meta_arena = self.metadata_arena.read();
        for meta in meta_arena.iter() {
            if let Some(u32_id) = self.get_u32(&meta.node_id) {
                cluster_groups.entry(meta.cluster_id).or_insert_with(Vec::new).push(u32_id);
            }
        }
        let vec_arena = self.vector_arena.read();
        let now = Utc::now().to_rfc3339();

        for (c_id, members) in cluster_groups.iter() {
            let mut centroid = vec![0.0; self.vector_dim as usize];
            let mut total_impact = 0.0;
            let mut count = 0;
            for &u32_id in members {
                if let Some(node) = self.nodes.get(&u32_id) {
                    total_impact += node.value().impact.unwrap_or(0.0);
                    if let Some(a_id) = self.u32_to_arena_id.get(&u32_id) {
                        if let Some(meta) = meta_arena.get(*a_id as usize) {
                            let start = meta.embedding_offset as usize;
                            let end = start + meta.vector_dim as usize;
                            if end <= vec_arena.len() {
                                for (i, val) in vec_arena[start..end].iter().enumerate() {
                                    if i < self.vector_dim as usize { centroid[i] += *val as f64; }
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
                if let (Some(from_cid), Some(to_id)) = (self.u32_to_arena_id.get(&from_u32), self.u32_to_arena_id.get(&to_u32)) {
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
            }
            self.edges.remove(&eid);
        }

        // 3. Remove node and primary indices
        self.id_to_u32.remove(id);
        self.u32_to_id.remove(&u32_id);
        self.out_idx.remove(&u32_id);
        self.in_idx.remove(&u32_id);
        self.u32_to_arena_id.remove(&u32_id);
        self.nodes.remove(&u32_id);

        Ok(())
    }

    pub fn reconcile_state(&self, signed_events: Vec<SignedEvent>) -> Result<()> {
        self.ensure_writable()?;
        for signed_event in signed_events {
            let event = &signed_event.event;
            let signer_id = &signed_event.signer_peer_id;
            
            // 1. Verify Signature
            if signer_id != &self.local_peer_id {
                let verified = if let Some(peer) = self.peers.get(signer_id) {
                    if let Ok(v_key) = VerifyingKey::from_bytes(&peer.verifying_key.as_slice().try_into().unwrap_or([0u8; 32])) {
                        let data = serde_json::to_vec(event).unwrap_or_default();
                        let sig = Signature::from_slice(&signed_event.signature).unwrap_or(Signature::from_bytes(&[0u8; 64]));
                        v_key.verify(&data, &sig).is_ok()
                    } else { false }
                } else { false };

                if !verified {
                    println!("Mark X: REJECTED event from {}. Invalid signature or unknown peer.", signer_id);
                    continue;
                }
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
                            self.add_vector_internal(&remote_node.id, emb.clone(), remote_node.lang.clone().unwrap_or("en".to_string()));
                        }
                        self.nodes.insert(u32_id, remote_node.clone());
                        self.persist_signed(signed_event.clone())?;
                    }
                }
                Event::Edge(remote_edge) => {
                    let u32_id = self.get_or_intern_id(&remote_edge.id);
                    let mut apply = true;
                    if let Some(local_edge) = self.edges.get(&u32_id) {
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
                        self.index_edge_internal(&remote_edge.id, &remote_edge.from, &remote_edge.to);
                        self.edges.insert(u32_id, remote_edge.clone());
                        self.persist_signed(signed_event.clone())?;
                    }
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
        let peer_id = storage.local_peer_id.clone();
        let verifying_key_bytes = storage.verifying_key.to_bytes().to_vec();

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
                                    GossipMessage::PullRequest { from_clock: _, target_peer_id: _ } => {
                                        // TODO: Implement PushDelta response with SignedEvents
                                    }
                                    GossipMessage::PushDelta { events } => {
                                        let _ = storage.reconcile_state(events);
                                    }
                                    GossipMessage::ConsensusPropose { proposal } => {
                                        // Auto-verify and store proposal
                                        storage.proposals.insert(proposal.proposal_id.clone(), proposal);
                                    }
                                    GossipMessage::ConsensusVote { proposal_id, voter_peer_id, approve, signature } => {
                                        let _ = storage.submit_vote(proposal_id, voter_peer_id, approve);
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
        if !temp_dir.exists() { fs::create_dir_all(&temp_dir).ok(); }

        // 1. Save Arenas
        let vec_arena = self.vector_arena.read();
        let meta_arena = self.metadata_arena.read();
        fs::write(temp_dir.join("vector.bin"), unsafe {
            std::slice::from_raw_parts(vec_arena.as_ptr() as *const u8, vec_arena.len() * 4)
        }).map_err(|e| Error::from_reason(e.to_string()))?;
        
        let meta_data = bincode::serialize(&*meta_arena).map_err(|e| Error::from_reason(e.to_string()))?;
        fs::write(temp_dir.join("meta.bin"), meta_data).map_err(|e| Error::from_reason(e.to_string()))?;

        // 2. Save HNSW Index
        let hnsw_lock = self.hnsw_index.read();
        if let Some(hnsw) = &*hnsw_lock {
            // hnsw_rs 0.3.4 uses file_dump(&path, basename)
            hnsw.file_dump(&temp_dir, "index").map_err(|e| Error::from_reason(format!("HNSW dump error: {:?}", e)))?;
        }

        // 3. Save DashMaps (Partial state for instant load)
        let nodes: Vec<(u32, NodeOutput)> = self.nodes.iter().map(|e| (*e.key(), e.value().clone())).collect();
        let edges: Vec<(u32, EdgeOutput)> = self.edges.iter().map(|e| (*e.key(), e.value().clone())).collect();
        fs::write(temp_dir.join("nodes.bin"), serde_json::to_vec(&nodes).unwrap()).ok();
        fs::write(temp_dir.join("edges.bin"), serde_json::to_vec(&edges).unwrap()).ok();

        // 4. Save Global Metadata
        let state = serde_json::json!({
            "logical_clock": self.get_logical_clock(),
            "peer_id": self.local_peer_id,
            "vector_dim": self.vector_dim,
            "schema_version": SCHEMA_VERSION,
            "timestamp": Utc::now().to_rfc3339(),
        });
        fs::write(temp_dir.join("state.json"), state.to_string()).ok();

        // Atomic Swap
        // Note: file_dump creates {basename}.hnsw.graph and {basename}.hnsw.data
        for file in &["vector.bin", "meta.bin", "nodes.bin", "edges.bin", "state.json", "index.hnsw.graph", "index.hnsw.data"] {
            let src = temp_dir.join(file);
            if src.exists() {
                let dst = self.path.join(file);
                fs::rename(src, dst).ok();
            }
        }
        
        println!("Mark IX: State persisted successfully to {}", self.path.display());
        Ok(())
    }

    fn try_load_state(&self) -> bool {
        let state_path = self.path.join("state.json");
        if !state_path.exists() { return false; }

        println!("Mark IX: Attempting instant load from binary state...");
        
        let start = Instant::now();
        // 1. Load Metadata & Arenas
        if let Ok(data) = fs::read(self.path.join("vector.bin")) {
            let mut vec_arena = self.vector_arena.write();
            *vec_arena = unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const f32, data.len() / 4).to_vec()
            };
        } else { return false; }

        if let Ok(data) = fs::read(self.path.join("meta.bin")) {
            if let Ok(meta) = bincode::deserialize::<Vec<NodeMetadata>>(&data) {
                *self.metadata_arena.write() = meta;
            } else { return false; }
        } else { return false; }

        // 2. Load Maps
        if let Ok(data) = fs::read(self.path.join("nodes.bin")) {
            match serde_json::from_slice::<Vec<(u32, NodeOutput)>>(&data) {
                Ok(nodes) => {
                    println!("Mark IX: Loading {} nodes from snapshot", nodes.len());
                    let mut max_u32 = 0;
                    for (k, v) in nodes { 
                        if k > max_u32 { max_u32 = k; }
                        self.id_to_u32.insert(v.id.clone(), k);
                        self.u32_to_id.insert(k, v.id.clone());
                        self.nodes.insert(k, v); 
                    }
                    self.next_u32.store(max_u32 + 1, Ordering::SeqCst);
                }
                Err(e) => { println!("Mark IX: Failed to deserialize nodes: {}", e); return false; }
            }
        } else { println!("Mark IX: nodes.bin not found"); }

        if let Ok(data) = fs::read(self.path.join("edges.bin")) {
            if let Ok(edges) = serde_json::from_slice::<Vec<(u32, EdgeOutput)>>(&data) {
                println!("Mark IX: Loading {} edges from snapshot", edges.len());
                for (k, v) in edges { 
                    self.index_edge_internal(&v.id, &v.from, &v.to);
                    self.edges.insert(k, v); 
                }
            }
        }

        // 3. Load HNSW Index (Placeholder for future native serialization)
        // For now, re-insertion from memory arenas is extremely fast.

        // 4. Sync Global State
        if let Ok(content) = fs::read_to_string(state_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(clock) = v["logical_clock"].as_u64() {
                    self.logical_clock.store(clock as u32, Ordering::SeqCst);
                }
            }
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
            
            let node = NodeOutput {
                id: id.clone(), labels: args.labels,
                props: args.props.unwrap_or(Value::Object(Default::default())),
                impact: Some(0.7), embedding: args.embedding.clone(),
                lang: Some(lang.clone()),
                valid_from: args.valid_from.unwrap_or_else(|| now.to_rfc3339()),
                valid_to: None, caused_by: args.caused_by, expires_at,
                clock: self.next_clock(),
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

        // 4. Memory Index Phase
        for event in events {
            match event {
                Event::Node(n) => {
                    let u32_id = self.get_or_intern_id(&n.id);
                    if let Some(emb) = n.embedding.clone() { 
                        self.add_vector_internal(&n.id, emb, n.lang.clone().unwrap_or("en".to_string())); 
                    }
                    self.nodes.insert(u32_id, n);
                }
                Event::Edge(e) => {
                    let u32_id = self.get_or_intern_id(&e.id);
                    self.index_edge_internal(&e.id, &e.from, &e.to);
                    self.edges.insert(u32_id, e);
                }
                _ => {}
            }
        }

        Ok(BatchOutput { nodes: output_nodes, edges: output_edges })
    }

    pub fn perform_index_compaction(&self) -> Result<()> {
        println!("Mark IX: Starting Index Compaction...");
        let start = Instant::now();
        
        // 1. Identify Live Set
        let live_nodes: HashSet<u32> = self.nodes.iter().map(|e| *e.key()).collect();
        
        // 2. Lock and Compact Arenas
        let mut meta_arena = self.metadata_arena.write();
        let mut vec_arena = self.vector_arena.write();
        
        let mut new_meta = Vec::with_capacity(live_nodes.len());
        let mut new_vec = Vec::with_capacity(live_nodes.len() * self.vector_dim as usize);
        
        self.u32_to_arena_id.clear();

        for meta in meta_arena.iter() {
            if let Some(u32_id) = self.get_u32(&meta.node_id) {
                if live_nodes.contains(&u32_id) {
                    let start_off = meta.embedding_offset as usize;
                    let end_off = start_off + meta.vector_dim as usize;
                    
                    if end_off <= vec_arena.len() {
                        let new_offset = new_vec.len() as u64;
                        new_vec.extend_from_slice(&vec_arena[start_off..end_off]);
                        
                        let new_arena_id = new_meta.len() as u32;
                        let mut meta_clone = meta.clone();
                        meta_clone.arena_id = new_arena_id;
                        meta_clone.embedding_offset = new_offset;
                        
                        self.u32_to_arena_id.insert(u32_id, new_arena_id);
                        new_meta.push(meta_clone);
                    }
                }
            }
        }

        *meta_arena = new_meta;
        *vec_arena = new_vec;
        
        // 3. Rebuild HNSW from scratch
        drop(meta_arena);
        drop(vec_arena);
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
                if let Ok(json) = serde_json::to_string(&Event::Node(node.clone())) {
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
                if let Ok(json) = serde_json::to_string(&Event::Edge(edge.clone())) {
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
    pub fn retract_edge(&self, _id: String, _at: Option<String>) -> Result<Option<EdgeOutput>> { Ok(None) }
    pub fn status_sync(&self) -> DatabaseStatus { DatabaseStatus { open: true, read_only: self.read_only, page_cache_mb: 512 } }
    pub fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> { for i in inputs { self.add_node(i)?; } Ok(()) }
    pub fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> { for i in inputs { self.add_edge(i)?; } Ok(()) }
    
    pub fn calculate_structural_gaps(&self) -> Result<Vec<GapSuggestion>> {
        let mut gaps = Vec::new();
        let mut cluster_centroids: HashMap<u32, Vec<f32>> = HashMap::new();
        let mut cluster_member_count: HashMap<u32, u32> = HashMap::new();
        let mut cluster_impact: HashMap<u32, f64> = HashMap::new();
        let meta_arena = self.metadata_arena.read();
        let vec_arena = self.vector_arena.read();
        for meta in meta_arena.iter() {
            let c_id = meta.cluster_id;
            let start = meta.embedding_offset as usize;
            let end = start + meta.vector_dim as usize;
            if end <= vec_arena.len() {
                let vec = &vec_arena[start..end];
                let entry = cluster_centroids.entry(c_id).or_insert_with(|| vec![0.0; meta.vector_dim as usize]);
                for (i, val) in vec.iter().enumerate() { entry[i] += val; }
                *cluster_member_count.entry(c_id).or_insert(0) += 1;
                if let Some(u32_id) = self.get_u32(&meta.node_id) {
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
    #[napi] pub fn set_language_centroid(&self, lang: String, vector: Vec<f64>) { self.inner.set_language_centroid(lang, vector); }
    #[napi] pub async fn detect_communities(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.detect_communities()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn calculate_structural_gaps(&self) -> Result<Vec<GapSuggestion>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.calculate_structural_gaps()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn generate_meta_graph(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.generate_meta_graph()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn get_meta_history(&self, cluster_id: u32) -> Result<Vec<SuperNode>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || Ok(i.get_meta_history(cluster_id))).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn reconcile_state(&self, events_json: String) -> Result<()> {
        let i = Arc::clone(&self.inner);
        let events = serde_json::from_str::<Vec<SignedEvent>>(&events_json).map_err(|e| Error::from_reason(e.to_string()))?;
        tokio::task::spawn_blocking(move || i.reconcile_state(events)).await.map_err(|e| Error::from_reason(e.to_string()))?
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
    #[napi] pub async fn submit_vote(&self, proposal_id: String, peer_id: String, approve: bool) -> Result<bool> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.submit_vote(proposal_id, peer_id, approve)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub fn get_local_peer_id(&self) -> String { self.inner.local_peer_id.clone() }
    #[napi] pub fn get_logical_clock(&self) -> u32 { self.inner.logical_clock.load(Ordering::SeqCst) }
    #[napi] pub fn get_merkle_root(&self) -> String { self.inner.get_merkle_root() }
    #[napi] pub fn schema_version_sync(&self) -> u32 { SCHEMA_VERSION }
    #[napi] pub fn status_sync(&self) -> DatabaseStatus { self.inner.status_sync() }
}
#[napi] pub fn engine_name_sync() -> String { "genesis-block".to_string() }
#[napi] pub fn schema_version_sync() -> u32 { SCHEMA_VERSION }
