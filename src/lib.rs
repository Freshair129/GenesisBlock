//! Genesis Block — embedded graph engine for GKS.
//!
//! Phase 3.2-3.4 implementation: JSONL Storage, Locking, and Cypher v0.
//! Implements PROTOCOL--GENESIS-GRAPH-FFI.

#![deny(clippy::all)]

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions as FileOpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use hnsw_rs::prelude::*;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sysinfo::{Pid, System};
use uuid::Uuid;

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
#[derive(Debug)]
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
#[derive(Debug)]
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
#[derive(Debug)]
pub struct QueryInput {
    pub from: Option<String>,
    pub to: Option<String>,
    pub rel: Option<String>,
    pub as_of: Option<String>,
    pub include_invalid: Option<bool>,
    pub limit: Option<u32>,
}

#[napi(object)]
#[derive(Debug)]
pub struct NeighborInput {
    pub depth: Option<u32>,
    pub rel: Option<String>,
    pub rels: Option<Vec<String>>,
    pub direction: Option<String>, // 'out' | 'in' | 'both'
    pub as_of: Option<String>,
    pub include_invalid: Option<bool>,
    pub limit: Option<u32>,
}

#[napi(object)]
#[derive(Debug)]
pub struct NeighborOutput {
    pub node: NodeOutput,
    pub path: Vec<EdgeOutput>,
    pub depth: u32,
}

#[napi(object)]
#[derive(Debug)]
pub struct HybridSearchInput {
    pub query_vector: Vec<f64>,
    pub k: u32,
    pub alpha: Option<f64>,
}

#[napi(object)]
#[derive(Debug)]
pub struct DatabaseStatus {
    pub open: bool,
    pub read_only: bool,
    pub page_cache_mb: u32,
}

// --- Internal Storage ---

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum Event {
    Node(NodeOutput),
    Edge(EdgeOutput),
}

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub nodes: HashMap<String, NodeOutput>,
    pub edges: HashMap<String, EdgeOutput>,
    pub out_idx: HashMap<String, HashSet<String>>,
    pub in_idx: HashMap<String, HashSet<String>>,
    pub vector_arena: Vec<f32>,
    pub metadata_arena: Vec<NodeMetadata>,
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
    pub(crate) path: PathBuf,
    pub(crate) read_only: bool,
    pub(crate) nodes: HashMap<String, NodeOutput>,
    pub(crate) edges: HashMap<String, EdgeOutput>,
    pub(crate) out_idx: HashMap<String, HashSet<String>>,
    pub(crate) in_idx: HashMap<String, HashSet<String>>,
    pub(crate) vector_arena: Vec<f32>,
    pub(crate) metadata_arena: Vec<NodeMetadata>,
    pub(crate) hnsw_index: Option<Hnsw<'static, f32, DistL2>>,
    pub(crate) log_path: PathBuf,
    pub(crate) bin_path: PathBuf,
    pub(crate) _lock_file: Option<File>,
}

impl Storage {
    pub(crate) fn allocate_aligned_offset(current_offset: usize, align: usize) -> usize {
        if current_offset % align == 0 {
            current_offset
        } else {
            current_offset + align - (current_offset % align)
        }
    }

    fn init_hnsw() -> Hnsw<'static, f32, DistL2> {
        Hnsw::new(16, 1000000, 16, 200, DistL2 {})
    }

    fn add_vector_internal(&mut self, node_id: &str, emb_64: Vec<f64>) {
        let emb: Vec<f32> = emb_64.into_iter().map(|v| v as f32).collect();
        let dim = emb.len() as u16;
        let current_vec_len = self.vector_arena.len();
        let aligned_offset = Self::allocate_aligned_offset(current_vec_len, 16);
        
        if aligned_offset > current_vec_len {
            self.vector_arena.resize(aligned_offset, 0.0);
        }
        
        self.vector_arena.extend(emb.clone());
        
        let arena_id = self.metadata_arena.len() as u32;
        let metadata = NodeMetadata {
            arena_id,
            node_id: node_id.to_string(),
            timestamp: Utc::now().timestamp() as u64,
            vector_dim: dim,
            embedding_offset: aligned_offset as u64,
            gks_attributes: Vec::new(),
        };
        self.metadata_arena.push(metadata);

        if self.hnsw_index.is_none() {
            self.hnsw_index = Some(Self::init_hnsw());
        }
        if let Some(ref mut hnsw) = self.hnsw_index {
            hnsw.insert((&emb, arena_id as usize));
        }
    }

    fn rehydrate_hnsw_index(&mut self) {
        if self.metadata_arena.is_empty() { return; }
        let mut hnsw = Self::init_hnsw();
        for meta in &self.metadata_arena {
            let start = meta.embedding_offset as usize;
            let end = start + meta.vector_dim as usize;
            if end <= self.vector_arena.len() {
                let vec = &self.vector_arena[start..end];
                hnsw.insert((vec, meta.arena_id as usize));
            }
        }
        self.hnsw_index = Some(hnsw);
    }

    pub fn open(opts: OpenOptions) -> Result<Self> {
        let root = PathBuf::from(opts.path.clone());
        if !root.exists() {
            fs::create_dir_all(&root).map_err(|e| Error::from_reason(format!("genesis-block: io: {e}")))?;
        }
        let read_only = opts.read_only.unwrap_or(false);
        let lock_file = Self::acquire_os_lock(&root, read_only)?;
        let log_path = root.join("genesis-graph.jsonl");
        let bin_path = root.join("genesis-graph.bin");
        let mut storage = Self {
            path: root, read_only, nodes: HashMap::new(), edges: HashMap::new(),
            out_idx: HashMap::new(), in_idx: HashMap::new(),
            vector_arena: Vec::new(), metadata_arena: Vec::new(),
            hnsw_index: None, log_path: log_path.clone(), bin_path, _lock_file: Some(lock_file),
        };

        if storage.bin_path.exists() {
            if let Ok(mut file) = File::open(&storage.bin_path) {
                let mut buffer = Vec::new();
                if file.read_to_end(&mut buffer).is_ok() {
                    if let Ok(snapshot) = bincode::deserialize::<Snapshot>(&buffer) {
                        storage.nodes = snapshot.nodes;
                        storage.edges = snapshot.edges;
                        storage.out_idx = snapshot.out_idx;
                        storage.in_idx = snapshot.in_idx;
                        storage.vector_arena = snapshot.vector_arena;
                        storage.metadata_arena = snapshot.metadata_arena;
                    }
                }
            }
        }
        storage.rehydrate_hnsw_index();

        if log_path.exists() {
            let file = FileOpenOptions::new().read(true).open(&log_path).map_err(|e| Error::from_reason(format!("io: {e}")))?;
            let reader = BufReader::new(file);
            let stream = serde_json::Deserializer::from_reader(reader).into_iter::<Event>();
            for event in stream {
                match event {
                    Ok(Event::Node(n)) => {
                        if let Some(emb) = n.embedding.clone() { storage.add_vector_internal(&n.id, emb); }
                        storage.nodes.insert(n.id.clone(), n);
                    }
                    Ok(Event::Edge(e)) => {
                        storage.index_edge_internal(&e.id, &e.from, &e.to);
                        storage.edges.insert(e.id.clone(), e);
                    }
                    Err(_) => return Err(Error::from_reason("malformed log")),
                }
            }
            storage.refresh_impacts(None);
        }
        Ok(storage)
    }

    fn acquire_os_lock(root: &PathBuf, read_only: bool) -> Result<File> {
        let lock_path = root.join("genesis-graph.lock");
        if !read_only && lock_path.exists() {
            let mut content = String::new();
            if let Ok(mut f) = FileOpenOptions::new().read(true).open(&lock_path) { f.read_to_string(&mut content).ok(); }
            let pid_str = content.trim();
            if !pid_str.is_empty() {
                if let Ok(pid_val) = pid_str.parse::<u32>() {
                    let mut system = System::new(); system.refresh_processes();
                    if system.process(Pid::from(pid_val as usize)).is_some() {
                        if pid_val != std::process::id() { return Err(Error::from_reason("locked")); }
                    }
                }
            }
        }
        let mut file = FileOpenOptions::new().read(true).write(true).create(true).open(&lock_path).map_err(|e| Error::from_reason(format!("lock: {e}")))?;
        if !read_only {
            file.set_len(0).ok(); file.seek(SeekFrom::Start(0)).ok();
            writeln!(file, "{}", std::process::id()).ok(); file.flush().ok();
        }
        Ok(file)
    }

    fn ensure_writable(&self) -> Result<()> {
        if self.read_only { return Err(Error::from_reason("read-only")); }
        Ok(())
    }

    fn index_edge_internal(&mut self, id: &str, from: &str, to: &str) {
        self.out_idx.entry(from.to_string()).or_default().insert(id.to_string());
        self.in_idx.entry(to.to_string()).or_default().insert(id.to_string());
    }

    fn calculate_as(&self, id: &str) -> f64 {
        if id.starts_with("MASTER--") || id.starts_with("FRAME--") { 1.0 }
        else if id.starts_with("CONCEPT--") || id.starts_with("SPEC--") { 0.8 }
        else if id.starts_with("FEAT--") || id.starts_with("ADR--") || id.starts_with("BLUEPRINT--") { 0.6 }
        else { 0.3 }
    }

    fn calculate_sc(&self, node: &NodeOutput) -> f64 {
        let status = node.props.get("status").and_then(|v| v.as_str()).unwrap_or("active");
        match status { "stable" => 1.0, "active" => 0.8, "draft" => 0.4, "deprecated" => 0.1, _ => 0.6 }
    }

    fn calculate_dd(&self, id: &str) -> f64 {
        let incoming = self.in_idx.get(id).map_or(0, |set| set.len());
        (incoming as f64 / 10.0).min(1.0)
    }

    fn compute_impact(&self, node: &NodeOutput) -> f64 {
        let dd = self.calculate_dd(&node.id);
        let as_score = self.calculate_as(&node.id);
        let sc = self.calculate_sc(node);
        (dd * 0.5) + (as_score * 0.3) + (sc * 0.2)
    }

    pub fn refresh_impacts(&mut self, affected_ids: Option<Vec<String>>) {
        let ids_to_update = match affected_ids {
            Some(ids) => {
                let mut queue = std::collections::VecDeque::from(ids);
                let mut affected = HashSet::new();
                while let Some(curr) = queue.pop_front() {
                    if affected.insert(curr.clone()) {
                        if let Some(edges) = self.out_idx.get(&curr) {
                            for eid in edges { if let Some(e) = self.edges.get(eid) { queue.push_back(e.to.clone()); } }
                        }
                    }
                }
                affected.into_iter().collect::<Vec<_>>()
            }
            None => self.nodes.keys().cloned().collect(),
        };
        for id in ids_to_update {
            if let Some(node) = self.nodes.get(&id) {
                let impact = self.compute_impact(node);
                if let Some(node_mut) = self.nodes.get_mut(&id) { node_mut.impact = Some(impact); }
            }
        }
    }

    fn persist(&self, event: &Event) -> Result<()> {
        self.ensure_writable()?;
        let mut file = FileOpenOptions::new().create(true).append(true).open(&self.log_path).map_err(|e| Error::from_reason(format!("io: {e}")))?;
        let line = serde_json::to_string(event).map_err(|e| Error::from_reason(e.to_string()))?;
        writeln!(file, "{}", line).map_err(|e| Error::from_reason(format!("io: {e}")))?;
        Ok(())
    }

    pub fn add_node(&mut self, args: NodeInput) -> Result<NodeOutput> {
        self.ensure_writable()?;
        let id = args.id.unwrap_or_else(|| {
            let hash = format!("{:x}", md5::compute(format!("{:?}{:?}", args.labels, args.props)));
            format!("N-{}", &hash[..16])
        });
        let mut existing_node = self.nodes.get(&id).cloned();
        if let Some(ref mut n) = existing_node {
            let mut labels = n.labels.clone();
            for l in args.labels { if !labels.contains(&l) { labels.push(l); } }
            let mut props = n.props.as_object().cloned().unwrap_or_default();
            if let Some(new_props) = args.props.and_then(|p| p.as_object().cloned()) {
                for (k, v) in new_props { props.insert(k, v); }
            }
            n.labels = labels; n.props = Value::Object(props);
            let impact = self.compute_impact(n); n.impact = Some(impact);
            self.nodes.insert(id.clone(), n.clone());
            self.persist(&Event::Node(n.clone()))?;
            self.refresh_impacts(Some(vec![id.clone()]));
            return Ok(n.clone());
        }
        let mut node = NodeOutput {
            id: id.clone(), labels: args.labels,
            props: args.props.unwrap_or(Value::Object(Default::default())),
            impact: None, embedding: None,
        };
        if let Some(emb) = args.embedding {
            self.add_vector_internal(&id, emb.clone());
            node.embedding = Some(emb);
        }
        let impact = self.compute_impact(&node); node.impact = Some(impact);
        self.nodes.insert(id.clone(), node.clone());
        self.persist(&Event::Node(node.clone()))?;
        self.refresh_impacts(Some(vec![id]));
        Ok(node)
    }

    pub fn add_edge(&mut self, args: EdgeInput) -> Result<EdgeOutput> {
        self.ensure_writable()?;
        if !self.nodes.contains_key(&args.from) || !self.nodes.contains_key(&args.to) { return Err(Error::from_reason("unknown node")); }
        if args.rel == "supersedes" || args.rel == "contradicts" {
            if self.calculate_as(&args.from) < self.calculate_as(&args.to) { return Err(Error::from_reason("axiomatic guard")); }
        }
        let now = Utc::now().to_rfc3339();
        let edge = EdgeOutput {
            id: args.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            from: args.from, to: args.to, rel: args.rel,
            props: args.props.unwrap_or(Value::Object(Default::default())),
            valid_from: args.valid_from.unwrap_or_else(|| now.clone()),
            valid_to: None, recorded_at: now, superseded_by: None, impact: args.impact,
        };
        self.index_edge_internal(&edge.id, &edge.from, &edge.to);
        let target_id = edge.to.clone();
        self.edges.insert(edge.id.clone(), edge.clone());
        self.refresh_impacts(Some(vec![target_id]));
        self.persist(&Event::Edge(edge.clone()))?;
        Ok(edge)
    }

    pub fn compact(&self) -> Result<()> {
        self.ensure_writable()?;
        let tmp_path = self.path.join("genesis-graph.jsonl.tmp");
        let mut file = FileOpenOptions::new().create(true).write(true).truncate(true).open(&tmp_path).map_err(|e| Error::from_reason(format!("compact: {e}")))?;
        for node in self.nodes.values() {
            let line = serde_json::to_string(&Event::Node(node.clone())).unwrap();
            writeln!(file, "{}", line).ok();
        }
        for edge in self.edges.values() {
            if edge.valid_to.is_none() {
                let line = serde_json::to_string(&Event::Edge(edge.clone())).unwrap();
                writeln!(file, "{}", line).ok();
            }
        }
        file.flush().ok(); drop(file); fs::rename(&tmp_path, &self.log_path).ok();
        let snapshot = Snapshot {
            nodes: self.nodes.clone(), edges: self.edges.clone(),
            out_idx: self.out_idx.clone(), in_idx: self.in_idx.clone(),
            vector_arena: self.vector_arena.clone(), metadata_arena: self.metadata_arena.clone(),
        };
        if let Ok(encoded) = bincode::serialize(&snapshot) { fs::write(&self.bin_path, encoded).ok(); }
        Ok(())
    }

    pub fn retract_edge(&mut self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> {
        self.ensure_writable()?;
        let e = match self.edges.get_mut(&id) { Some(e) => e, None => return Ok(None) };
        if e.valid_to.is_some() { return Ok(Some(e.clone())); }
        let at = at.unwrap_or_else(|| Utc::now().to_rfc3339());
        e.valid_to = Some(at); let retired = e.clone();
        self.persist(&Event::Edge(retired.clone()))?; Ok(Some(retired))
    }

    pub fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let hnsw = match &self.hnsw_index { Some(idx) => idx, None => return Err(Error::from_reason("HNSW index not initialized")) };
        let k_vec = args.k * 2; let alpha = args.alpha.unwrap_or(0.5);
        let query_f32: Vec<f32> = args.query_vector.into_iter().map(|v| v as f32).collect();
        let results = hnsw.search(&query_f32, k_vec as usize, 100);
        let mut hybrid_results = Vec::new();
        for neighbor in results {
            let arena_id = neighbor.d_id as u32; let similarity = 1.0 - neighbor.distance;
            if let Some(meta) = self.metadata_arena.get(arena_id as usize) {
                if let Some(node) = self.nodes.get(&meta.node_id) {
                    let mut node_out = node.clone();
                    let hybrid_score = (similarity as f64 * (1.0 - alpha)) + (node_out.impact.unwrap_or(0.0) * alpha);
                    node_out.impact = Some(hybrid_score);
                    hybrid_results.push(NeighborOutput { node: node_out, path: Vec::new(), depth: 0 });
                }
            }
        }
        hybrid_results.sort_by(|a, b| b.node.impact.unwrap_or(0.0).partial_cmp(&a.node.impact.unwrap_or(0.0)).unwrap());
        hybrid_results.truncate(args.k as usize); Ok(hybrid_results)
    }

    pub fn neighbors(&self, seed: String, args: NeighborInput) -> Result<Vec<NeighborOutput>> {
        if !self.nodes.contains_key(&seed) { return Ok(Vec::new()); }
        let depth = args.depth.unwrap_or(1); let direction = args.direction.as_deref().unwrap_or("out");
        let mut target_rels = HashSet::new(); if let Some(r) = args.rel { target_rels.insert(r); }
        if let Some(rs) = args.rels { target_rels.extend(rs); }
        let mut results = Vec::new(); let mut visited = HashSet::new(); visited.insert(seed.clone());
        let mut queue = std::collections::VecDeque::new(); queue.push_back((seed.clone(), Vec::new(), 0));
        while let Some((curr_id, path, curr_depth)) = queue.pop_front() {
            if curr_depth >= depth { continue; }
            let mut edge_ids = HashSet::new();
            if direction == "out" || direction == "both" { if let Some(eids) = self.out_idx.get(&curr_id) { edge_ids.extend(eids.clone()); } }
            if direction == "in" || direction == "both" { if let Some(eids) = self.in_idx.get(&curr_id) { edge_ids.extend(eids.clone()); } }
            let mut edges_to_visit: Vec<EdgeOutput> = edge_ids.iter().filter_map(|eid| self.edges.get(eid)).cloned().collect();
            edges_to_visit.sort_by(|a, b| b.impact.unwrap_or(0.0).partial_cmp(&a.impact.unwrap_or(0.0)).unwrap());
            for edge in edges_to_visit {
                if !target_rels.is_empty() && !target_rels.contains(&edge.rel) { continue; }
                if !args.include_invalid.unwrap_or(false) && edge.valid_to.is_some() { continue; }
                let next_id = if edge.from == curr_id { edge.to.clone() } else { edge.from.clone() };
                if visited.contains(&next_id) { continue; }
                visited.insert(next_id.clone());
                if let Some(node) = self.nodes.get(&next_id) {
                    let mut new_path = path.clone(); new_path.push(edge.clone());
                    results.push(NeighborOutput { node: node.clone(), path: new_path.clone(), depth: curr_depth + 1 });
                    queue.push_back((next_id, new_path, curr_depth + 1));
                }
            }
        }
        results.sort_by(|a, b| b.node.impact.unwrap_or(0.0).partial_cmp(&a.node.impact.unwrap_or(0.0)).unwrap());
        if let Some(limit) = args.limit { if results.len() > limit as usize { results.truncate(limit as usize); } }
        Ok(results)
    }
}

#[napi]
pub struct GenesisDatabase { inner: Arc<RwLock<Storage>>, page_cache_mb: u32 }

#[napi]
impl GenesisDatabase {
    #[napi(factory)]
    pub fn open(opts: OpenOptions) -> Result<Self> {
        let storage = Storage::open(opts.clone())?;
        Ok(Self { inner: Arc::new(RwLock::new(storage)), page_cache_mb: opts.page_cache_mb.unwrap_or(64) })
    }
    #[napi]
    pub async fn add_node(&self, args: NodeInput) -> Result<NodeOutput> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.write().add_node(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.write().add_edge(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn retract_edge(&self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.write().retract_edge(id, at)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn query(&self, args: QueryInput) -> Result<Vec<EdgeOutput>> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let storage = inner.read();
            let as_of_ms = args.as_of.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|dt| dt.timestamp_millis());
            let mut results = Vec::new();
            for e in storage.edges.values() {
                if let Some(ref from) = args.from { if e.from != *from { continue; } }
                if let Some(ref to) = args.to { if e.to != *to { continue; } }
                if let Some(ref rel) = args.rel { if e.rel != *rel { continue; } }
                let is_valid = if let Some(ms) = as_of_ms {
                    let from_ms = chrono::DateTime::parse_from_rfc3339(&e.valid_from).map(|dt| dt.timestamp_millis()).unwrap_or(0);
                    let to_ms = e.valid_to.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|dt| dt.timestamp_millis());
                    ms >= from_ms && to_ms.map_or(true, |end| ms < end)
                } else { args.include_invalid.unwrap_or(false) || e.valid_to.is_none() };
                if is_valid { results.push(e.clone()); }
            }
            results.sort_by(|a, b| b.impact.unwrap_or(0.0).partial_cmp(&a.impact.unwrap_or(0.0)).unwrap());
            if let Some(limit) = args.limit { if results.len() > limit as usize { results.truncate(limit as usize); } }
            Ok(results)
        }).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.read().hybrid_search(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn neighbors(&self, seed: String, args: NeighborInput) -> Result<Vec<NeighborOutput>> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.read().neighbors(seed, args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn compact(&self) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.write().compact()).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub fn schema_version_sync(&self) -> u32 { SCHEMA_VERSION }
    #[napi]
    pub fn status_sync(&self) -> DatabaseStatus {
        let storage = self.inner.read();
        DatabaseStatus { open: true, read_only: storage.read_only, page_cache_mb: self.page_cache_mb }
    }
}
#[napi]
pub fn engine_name_sync() -> String { "genesis-block".to_string() }
