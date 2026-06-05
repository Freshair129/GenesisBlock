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
}

#[napi(object)]
#[derive(Serialize, Deserialize, Debug)]
pub struct NodeInput {
    pub id: Option<String>,
    pub labels: Vec<String>,
    pub props: Option<serde_json::Value>,
    pub embedding: Option<Vec<f64>>,
    pub lang: Option<String>,
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
    pub public_key: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SyncEvent {
    ProposeMutation(Event),
    AcknowledgeMutation(String), 
    RequestFragment(String),
}

// --- Internal Storage ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    Node(NodeOutput),
    Edge(EdgeOutput),
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
    pub event: Event,
    pub signature: Vec<u8>,
    pub votes: HashMap<String, bool>, // PeerID -> Vote
}

#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SuperNode {
    pub cluster_id: u32,
    pub theme: String,
    pub member_count: u32,
    pub impact: f64,
    pub centroid: Vec<f64>,
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
    pub wal_sender: Sender<(Event, Sender<bool>)>,
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

    pub fn get_or_intern_id(&self, id: &str) -> u32 {
        if let Some(existing) = self.id_to_u32.get(id) { return *existing; }
        let new_id = self.next_u32.fetch_add(1, Ordering::SeqCst);
        self.id_to_u32.insert(id.to_string(), new_id);
        self.u32_to_id.insert(new_id, id.to_string());
        if id.len() >= 3 {
            for i in 0..id.len() - 2 {
                let trigram = id[i..i+3].to_lowercase();
                self.trigram_index.entry(trigram).or_insert_with(HashSet::new).insert(new_id);
            }
        }
        new_id
    }

    pub fn get_u32(&self, id: &str) -> Option<u32> { self.id_to_u32.get(id).map(|v| *v) }

    fn init_hnsw() -> Hnsw<'static, f32, DistL2> { Hnsw::new(16, 1000000, 16, 200, DistL2 {}) }

    fn add_vector_internal(&self, node_id: &str, emb_64: Vec<f64>, lang: String) {
        let emb: Vec<f32> = emb_64.into_iter().map(|v| v as f32).collect();
        let dim = emb.len() as u16;
        let mut vec_arena = self.vector_arena.write();
        let current_vec_len = vec_arena.len();
        vec_arena.extend_from_slice(&emb);
        let mut meta_arena = self.metadata_arena.write();
        let arena_id = meta_arena.len() as u32;
        meta_arena.push(NodeMetadata {
            arena_id, node_id: node_id.to_string(), timestamp: Utc::now().timestamp() as u64,
            vector_dim: dim, embedding_offset: current_vec_len as u64, gks_attributes: Vec::new(),
            lang, cluster_id: arena_id,
        });
        if let Some(u32_id) = self.get_u32(node_id) { self.u32_to_arena_id.insert(u32_id, arena_id); }
        if self.hnsw_index.read().is_none() { *self.hnsw_index.write() = Some(Self::init_hnsw()); }
        if let Some(ref mut hnsw) = *self.hnsw_index.write() { hnsw.insert((&emb, arena_id as usize)); }
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
        let log_path = root.join("genesis-graph.wal");
        let (wal_sender, wal_receiver): (Sender<(Event, Sender<bool>)>, Receiver<(Event, Sender<bool>)>) = unbounded();
        let log_path_clone = log_path.clone();

        std::thread::spawn(move || {
            if let Ok(file) = FileOpenOptions::new().append(true).create(true).open(&log_path_clone) {
                let mut writer = std::io::BufWriter::with_capacity(128 * 1024, file);
                let mut batch: Vec<crossbeam_channel::Sender<bool>> = Vec::with_capacity(1024);
                loop {
                    match wal_receiver.recv() {
                        Ok((event, ack_tx)) => {
                            batch.push(ack_tx);
                            if let Ok(json) = serde_json::to_string(&event) {
                                let _ = writer.write_all(json.as_bytes());
                                let _ = writer.write_all(b"\n");
                            }
                            let timeout = Duration::from_millis(5);
                            let start = Instant::now();
                            while batch.len() < 1024 && start.elapsed() < timeout {
                                if let Ok((e, tx)) = wal_receiver.try_recv() {
                                    batch.push(tx);
                                    if let Ok(j) = serde_json::to_string(&e) {
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
            wal_sender,
        };

        if storage.log_path.exists() {
            if let Ok(file) = File::open(&storage.log_path) {
                let reader = std::io::BufReader::new(file);
                use std::io::BufRead;
                for line_res in reader.lines() {
                    if let Ok(line) = line_res {
                        if let Ok(event) = serde_json::from_str::<Event>(&line) {
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
                            }
                        }
                    }
                }
            }
        }
        storage.rehydrate_hnsw_index();
        Ok(storage)
    }

    pub fn start_autonomic_loop(storage: Arc<Self>) {
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(3600)); 
                if !storage.read_only {
                    let _ = storage.perform_autonomic_optimization();
                }
            }
        });
    }

    pub fn ensure_writable(&self) -> Result<()> { if self.read_only { return Err(Error::from_reason("read-only")); } Ok(()) }

    pub fn persist(&self, event: &Event) -> Result<()> {
        let (ack_tx, ack_rx) = unbounded();
        self.wal_sender.send((event.clone(), ack_tx)).map_err(|_| Error::from_reason("wal disconnected"))?;
        let _ = ack_rx.recv(); Ok(())
    }

    pub fn find_fuzzy_id(&self, id: &str) -> Option<String> {
        if self.get_u32(id).is_some() { return Some(id.to_string()); }
        let mut candidates = HashSet::new();
        let id_lower = id.to_lowercase();
        if id.len() >= 3 {
            for i in 0..id.len() - 2 {
                if let Some(nodes) = self.trigram_index.get(&id_lower[i..i+3]) { candidates.extend(nodes.clone()); }
            }
        } else {
            for entry in self.u32_to_id.iter() { candidates.insert(*entry.key()); }
        }
        let mut best_id = None; let mut max_sim = 0.0;
        for u32_id in candidates {
            if let Some(candidate_id) = self.u32_to_id.get(&u32_id) {
                let sim = strsim::jaro_winkler(id, candidate_id.value());
                if sim > max_sim { max_sim = sim; best_id = Some(candidate_id.value().clone()); }
            }
        }
        if max_sim > 0.85 { best_id } else { None }
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
        }
    }

    pub fn propose_consensus(&self, event: Event, signature: Vec<u8>) -> Result<String> {
        let proposal_id = Uuid::new_v4().to_string();
        let proposal = ConsensusProposal {
            proposal_id: proposal_id.clone(),
            event,
            signature,
            votes: HashMap::new(),
        };
        self.proposals.insert(proposal_id.clone(), proposal);
        Ok(proposal_id)
    }

    pub fn submit_vote(&self, proposal_id: String, peer_id: String, approve: bool) -> Result<bool> {
        if let Some(mut proposal_ref) = self.proposals.get_mut(&proposal_id) {
            proposal_ref.value_mut().votes.insert(peer_id, approve);
            let approvals = proposal_ref.value().votes.values().filter(|&&v| v).count();
            
            if approvals > (self.peers.len() / 2) {
                let event = proposal_ref.value().event.clone();
                match event {
                    Event::Node(mut n) => {
                        if !n.labels.contains(&"MASTER".to_string()) { n.labels.push("MASTER".to_string()); }
                        let u32_id = self.get_or_intern_id(&n.id);
                        self.nodes.insert(u32_id, n.clone());
                        self.persist(&Event::Node(n))?;
                    }
                    Event::Edge(e) => {
                        let u32_id = self.get_or_intern_id(&e.id);
                        self.edges.insert(u32_id, e.clone());
                        self.persist(&Event::Edge(e))?;
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

    pub fn add_node(&self, args: NodeInput) -> Result<NodeOutput> {
        self.ensure_writable()?;
        self.validate_governance(&args.labels, false)?; 
        let id = args.id.unwrap_or_else(|| format!("N-{}", Uuid::new_v4()));
        let u32_id = self.get_or_intern_id(&id);
        let lang = args.lang.clone().unwrap_or("en".to_string());
        let mut node = NodeOutput { 
            id: id.clone(), labels: args.labels, 
            props: args.props.unwrap_or(Value::Object(Default::default())), 
            impact: Some(0.7), embedding: None,
            lang: Some(lang.clone()),
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
        };
        self.index_edge_internal(&edge.id, &edge.from, &edge.to);
        self.edges.insert(self.get_or_intern_id(&edge.id), edge.clone());
        self.refresh_impacts(Some(vec![edge.to.clone()]));
        self.persist(&Event::Edge(edge.clone()))?;
        Ok(edge)
    }

    pub fn rebuild_index_parallel(&self) -> Result<()> {
        self.is_rebuilding.store(true, Ordering::SeqCst);
        let result = (|| { self.rehydrate_hnsw_index(); Ok(()) })();
        self.is_rebuilding.store(false, Ordering::SeqCst);
        result
    }

    pub fn execute_hql(&self, query: &str) -> Result<Vec<NeighborOutput>> {
        let command = HqlCommand::try_from(query).map_err(|e| Error::from_reason(e))?;
        match command {
            HqlCommand::Search { vector, k, fuzzy, target, lang } => {
                let _resolved = if fuzzy { self.find_fuzzy_id(&target) } else { Some(target) };
                self.hybrid_search(HybridSearchInput { query_vector: vector, k, alpha: Some(0.0), lang })
            }
            HqlCommand::Traverse { seed, depth, rel, fuzzy } => {
                let resolved_seed = if fuzzy { self.find_fuzzy_id(&seed).unwrap_or(seed) } else { seed };
                let (target_rel, is_inferred) = match rel {
                    query::ast::HqlRel::Physical(r) => (r, false),
                    query::ast::HqlRel::Inferred(r) => (r, true),
                };
                self.neighbors(resolved_seed, NeighborInput { 
                    depth: Some(depth), rel: Some(target_rel), rels: None, direction: Some("out".to_string()), as_of: None, include_invalid: Some(false), limit: None 
                }, is_inferred)
            }
            HqlCommand::Hybrid { vector, alpha, fuzzy, target, lang } => {
                let _resolved = if fuzzy { self.find_fuzzy_id(&target) } else { Some(target) };
                self.hybrid_search(HybridSearchInput { query_vector: vector, k: 10, alpha: Some(alpha), lang })
            }
        }
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
                        if target_rel == "ANY" || edge.rel == target_rel {
                            let curr_id = self.u32_to_id.get(&curr_u32).unwrap().value().clone();
                            let next_id = if edge.from == curr_id { &edge.to } else { &edge.from };
                            if let Some(next_u32) = self.get_u32(next_id) {
                                if !visited.contains(&next_u32) {
                                    visited.insert(next_u32);
                                    if let Some(node) = self.nodes.get(&next_u32) {
                                        let mut new_path = path.clone(); new_path.push(edge.clone());
                                        results.push(NeighborOutput { node: node.value().clone(), path: new_path.clone(), depth: curr_depth + 1 });
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
        for meta in meta_arena.iter_mut() {
            let mut freq = HashMap::new();
            if let Some(u32_id) = self.get_u32(&meta.node_id) {
                if let Some(eids) = self.out_idx.get(&u32_id) {
                    for eid in eids.iter() {
                        if let Some(edge) = self.edges.get(eid) {
                            if let Some(to_u32) = self.get_u32(&edge.to) {
                                if let Some(a_id) = self.u32_to_arena_id.get(&to_u32) {
                                    *freq.entry(*a_id).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
            if let Some((&best_cluster, _)) = freq.iter().max_by_key(|&(_, count)| count) {
                meta.cluster_id = best_cluster;
            }
        }
        Ok(())
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
        for (c_id, members) in cluster_groups.iter() {
            let mut centroid = vec![0.0; 1536];
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
                                    if i < 1536 { centroid[i] += *val as f64; }
                                }
                                count += 1;
                            }
                        }
                    }
                }
            }
            if count > 0 {
                for val in centroid.iter_mut() { *val /= count as f64; }
                self.meta_nodes.insert(*c_id, SuperNode {
                    cluster_id: *c_id, theme: format!("Theme-{}", c_id),
                    member_count: members.len() as u32, impact: total_impact / members.len() as f64,
                    centroid,
                });
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
        for entry in self.nodes.iter() {
            let u32_id = entry.key();
            let is_master = entry.value().labels.contains(&"MASTER".to_string());
            if !is_master {
                let has_in = self.in_idx.contains_key(u32_id);
                let has_out = self.out_idx.contains_key(u32_id);
                if !has_in && !has_out {
                    to_delete.push(entry.value().id.clone());
                }
            }
        }
        for id in to_delete {
            if let Some(u32_id) = self.get_u32(&id) {
                self.nodes.remove(&u32_id);
                self.id_to_u32.remove(&id);
                self.u32_to_id.remove(&u32_id);
                println!("Mark VI: Pruned orphaned node '{}'", id);
            }
        }
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

    pub fn compact(&self) -> Result<()> { Ok(()) }
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
        Ok(Self { inner: storage }) 
    }
    #[napi] pub async fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.bulk_add_nodes(inputs)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.bulk_add_edges(inputs)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn rebuild_index_parallel(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.rebuild_index_parallel()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn add_node(&self, args: NodeInput) -> Result<NodeOutput> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.add_node(args)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.add_edge(args)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn retract_edge(&self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.retract_edge(id, at)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn query(&self, args: QueryInput) -> Result<Vec<EdgeOutput>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.query(args)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn execute_hql(&self, query: String) -> Result<Vec<NeighborOutput>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.execute_hql(&query)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.hybrid_search(args)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn neighbors(&self, seed: String, args: NeighborInput) -> Result<Vec<NeighborOutput>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.neighbors(seed, args, false)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn compact(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.compact()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub fn set_language_centroid(&self, lang: String, vector: Vec<f64>) { self.inner.set_language_centroid(lang, vector); }
    #[napi] pub async fn detect_communities(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.detect_communities()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn calculate_structural_gaps(&self) -> Result<Vec<GapSuggestion>> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.calculate_structural_gaps()).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn generate_meta_graph(&self) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.generate_meta_graph()).await.map_err(|e| Error::from_reason(e.to_string()))? }
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
    #[napi] pub fn get_merkle_root(&self) -> String { self.inner.get_merkle_root() }
    #[napi] pub fn schema_version_sync(&self) -> u32 { SCHEMA_VERSION }
    #[napi] pub fn status_sync(&self) -> DatabaseStatus { self.inner.status_sync() }
}
#[napi] pub fn engine_name_sync() -> String { "genesis-block".to_string() }
#[napi] pub fn schema_version_sync() -> u32 { SCHEMA_VERSION }
