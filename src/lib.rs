//! Genesis Block — high-performance hybrid semantic-graph engine.
//!
//! Mark IV: Global Scale & Reasoning

#![deny(clippy::all)]

use std::collections::{HashSet, VecDeque};
use std::fs::{self, File, OpenOptions as FileOpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::time::{Duration, Instant};

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
}

#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DatabaseStatus {
    pub open: bool,
    pub read_only: bool,
    pub page_cache_mb: u32,
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
    pub next_u32: AtomicU32,
    pub is_rebuilding: AtomicBool,
    pub trigram_index: DashMap<String, HashSet<u32>>,
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

    fn add_vector_internal(&self, node_id: &str, emb_64: Vec<f64>) {
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
        });
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
            id_to_u32: DashMap::new(), u32_to_id: DashMap::new(), next_u32: AtomicU32::new(0),
            is_rebuilding: AtomicBool::new(false), trigram_index: DashMap::new(), wal_sender,
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
                                    if let Some(emb) = n.embedding.clone() { storage.add_vector_internal(&n.id, emb); }
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

    pub fn calculate_sc(&self, node: &NodeOutput) -> f64 {
        let stability = node.props.get("stability").and_then(|v| v.as_str()).unwrap_or("active");
        match stability {
            "stable" => 1.0,
            "active" => 0.8,
            "draft" => 0.4,
            "deprecated" => 0.1,
            _ => 0.8,
        }
    }

    pub fn compute_impact(&self, node: &NodeOutput) -> f64 {
        let u32_id = match self.get_u32(&node.id) { Some(id) => id, None => return 0.7 };
        let incoming_count = self.in_idx.get(&u32_id).map(|edges| edges.len()).unwrap_or(0);
        let dd = (incoming_count as f64 / 10.0).min(1.0);
        let tier = Tier::from_labels(&node.labels);
        let as_score = match tier {
            Tier::MASTER => 1.0,
            Tier::SPEC => 0.8,
            Tier::ADR => 0.6,
            Tier::USER => 0.3,
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
        let mut node = NodeOutput { id: id.clone(), labels: args.labels, props: args.props.unwrap_or(Value::Object(Default::default())), impact: Some(0.7), embedding: None };
        if let Some(emb) = args.embedding { self.add_vector_internal(&id, emb.clone()); node.embedding = Some(emb); }
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
        
        // Trigger impact refresh for the 'to' node (incoming reference changed)
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
            HqlCommand::Search { vector, k, fuzzy, target, .. } => {
                let _resolved = if fuzzy { self.find_fuzzy_id(&target) } else { Some(target) };
                self.hybrid_search(HybridSearchInput { query_vector: vector, k, alpha: Some(0.0) })
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
            HqlCommand::Hybrid { vector, alpha, fuzzy, target, .. } => {
                let _resolved = if fuzzy { self.find_fuzzy_id(&target) } else { Some(target) };
                self.hybrid_search(HybridSearchInput { query_vector: vector, k: 10, alpha: Some(alpha) })
            }
        }
    }

    pub fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let hnsw_lock = self.hnsw_index.read();
        let hnsw = match &*hnsw_lock { Some(idx) => idx, None => return Err(Error::from_reason("HNSW not init")) };
        let query_f32: Vec<f32> = args.query_vector.into_iter().map(|v| v as f32).collect();
        let results = hnsw.search(&query_f32, (args.k * 2) as usize, 100);
        let mut hybrid_results = Vec::new();
        let meta_arena = self.metadata_arena.read();
        for neighbor in results {
            if let Some(meta) = meta_arena.get(neighbor.d_id as usize) {
                if let Some(u32_id) = self.get_u32(&meta.node_id) {
                    if let Some(node) = self.nodes.get(&u32_id) {
                        hybrid_results.push(NeighborOutput { node: node.value().clone(), path: Vec::new(), depth: 0 });
                    }
                }
            }
        }
        hybrid_results.truncate(args.k as usize); Ok(hybrid_results)
    }

    pub fn neighbors(&self, seed: String, args: NeighborInput, is_inferred: bool) -> Result<Vec<NeighborOutput>> {
        let u32_seed = match self.get_u32(&seed) { Some(id) => id, None => return Ok(Vec::new()) };
        let depth = args.depth.unwrap_or(1);
        let target_rel = args.rel.as_deref().unwrap_or("ANY");

        let mut results = Vec::new(); 
        let mut visited = HashSet::new(); visited.insert(u32_seed);
        let mut queue = VecDeque::new(); queue.push_back((u32_seed, Vec::new(), 0));

        while let Some((curr_u32, path, curr_depth)) = queue.pop_front() {
            // In regular traversal, we stop when we reach requested depth.
            // In inferred traversal, we follow the chain (transitive closure).
            if curr_depth >= depth && !is_inferred { continue; }

            if let Some(eids) = self.out_idx.get(&curr_u32) {
                for eid in eids.iter() {
                    if let Some(edge_ref) = self.edges.get(eid) {
                        let edge = edge_ref.value();
                        let rel_match = target_rel == "ANY" || edge.rel == target_rel;

                        if rel_match {
                            let curr_id = self.u32_to_id.get(&curr_u32).unwrap().value().clone();
                            let next_id = if edge.from == curr_id { &edge.to } else { &edge.from };
                            
                            if let Some(next_u32) = self.get_u32(next_id) {
                                if !visited.contains(&next_u32) {
                                    visited.insert(next_u32);
                                    if let Some(node) = self.nodes.get(&next_u32) {
                                        let mut new_path = path.clone(); new_path.push(edge.clone());
                                        results.push(NeighborOutput { node: node.value().clone(), path: new_path.clone(), depth: curr_depth + 1 });
                                        
                                        // Continue expansion
                                        if is_inferred || (curr_depth + 1 < depth) {
                                            queue.push_back((next_u32, new_path, curr_depth + 1));
                                        }
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

    pub fn compact(&self) -> Result<()> { Ok(()) }
    pub fn retract_edge(&self, _id: String, _at: Option<String>) -> Result<Option<EdgeOutput>> { Ok(None) }
    pub fn status_sync(&self) -> DatabaseStatus { DatabaseStatus { open: true, read_only: self.read_only, page_cache_mb: 512 } }
    pub fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> { for i in inputs { self.add_node(i)?; } Ok(()) }
    pub fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> { for i in inputs { self.add_edge(i)?; } Ok(()) }
}

#[napi]
pub struct GenesisDatabase { inner: Arc<Storage> }

#[napi]
impl GenesisDatabase {
    #[napi(factory)]
    pub fn open(opts: OpenOptions) -> Result<Self> { Ok(Self { inner: Arc::new(Storage::open(opts)?) }) }
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
    #[napi] pub fn schema_version_sync(&self) -> u32 { SCHEMA_VERSION }
    #[napi] pub fn status_sync(&self) -> DatabaseStatus { self.inner.status_sync() }
}
#[napi] pub fn engine_name_sync() -> String { "genesis-block".to_string() }
