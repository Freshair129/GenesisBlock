//! Genesis Block — high-performance hybrid semantic-graph engine.
//!
//! Phase 9.1: Turbo Core Overhaul (u32 Interning & Lock Sharding).
//! Implements high-concurrency storage using DashMap and Arena allocation.

#![deny(clippy::all)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File, OpenOptions as FileOpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write, BufWriter};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::Utc;
use dashmap::DashMap;
use hnsw_rs::prelude::*;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sysinfo::{Pid, System};
use uuid::Uuid;
use rayon::prelude::*;

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum Event {
    Node(NodeOutput),
    Edge(EdgeOutput),
}

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub nodes: HashMap<u32, NodeOutput>,
    pub edges: HashMap<u32, EdgeOutput>,
    pub out_idx: HashMap<u32, HashSet<u32>>,
    pub in_idx: HashMap<u32, HashSet<u32>>,
    pub vector_arena: Vec<f32>,
    pub metadata_arena: Vec<NodeMetadata>,
    pub id_to_u32: HashMap<String, u32>,
    pub u32_to_id: HashMap<u32, String>,
    pub next_u32: u32,
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
    pub vector_arena: Vec<f32>,
    pub metadata_arena: Vec<NodeMetadata>,
    pub hnsw_index: Option<Hnsw<'static, f32, DistL2>>,
    pub log_path: PathBuf,
    pub bin_path: PathBuf,
    pub _lock_file: Option<File>,
    pub id_to_u32: DashMap<String, u32>,
    pub u32_to_id: DashMap<u32, String>,
    pub next_u32: AtomicU32,
}

impl Storage {
    pub fn get_or_intern_id(&self, id: &str) -> u32 {
        if let Some(existing) = self.id_to_u32.get(id) {
            return *existing;
        }
        let new_id = self.next_u32.fetch_add(1, Ordering::SeqCst);
        self.id_to_u32.insert(id.to_string(), new_id);
        self.u32_to_id.insert(new_id, id.to_string());
        new_id
    }

    pub fn get_u32(&self, id: &str) -> Option<u32> {
        self.id_to_u32.get(id).map(|v| *v)
    }

    pub fn allocate_aligned_offset(current_offset: usize, align: usize) -> usize {
        if current_offset % align == 0 { current_offset } else { current_offset + align - (current_offset % align) }
    }

    fn init_hnsw() -> Hnsw<'static, f32, DistL2> {
        Hnsw::new(16, 1000000, 16, 200, DistL2 {})
    }

    pub fn add_vector_internal(&mut self, node_id: &str, emb_64: Vec<f64>) {
        let emb: Vec<f32> = emb_64.into_iter().map(|v| v as f32).collect();
        let dim = emb.len() as u16;
        let current_vec_len = self.vector_arena.len();
        let aligned_offset = Self::allocate_aligned_offset(current_vec_len, 64);
        if aligned_offset > current_vec_len { self.vector_arena.resize(aligned_offset, 0.0); }
        self.vector_arena.extend_from_slice(&emb);
        let arena_id = self.metadata_arena.len() as u32;
        let metadata = NodeMetadata {
            arena_id, node_id: node_id.to_string(), timestamp: Utc::now().timestamp() as u64,
            vector_dim: dim, embedding_offset: aligned_offset as u64, gks_attributes: Vec::new(),
        };
        self.metadata_arena.push(metadata);
        if self.hnsw_index.is_none() { self.hnsw_index = Some(Self::init_hnsw()); }
        if let Some(ref mut hnsw) = self.hnsw_index { hnsw.insert((&emb, arena_id as usize)); }
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
        if !root.exists() { fs::create_dir_all(&root).map_err(|e| Error::from_reason(format!("genesis-block: io: {e}")))?; }
        let read_only = opts.read_only.unwrap_or(false);
        let lock_file = Self::acquire_os_lock(&root, read_only)?;
        let log_path = root.join("genesis-graph.jsonl");
        let bin_path = root.join("genesis-graph.bin");
        let storage = Self {
            path: root, read_only, nodes: DashMap::new(), edges: DashMap::new(),
            out_idx: DashMap::new(), in_idx: DashMap::new(),
            vector_arena: Vec::new(), metadata_arena: Vec::new(),
            hnsw_index: None, log_path: log_path.clone(), bin_path, _lock_file: Some(lock_file),
            id_to_u32: DashMap::new(), u32_to_id: DashMap::new(), next_u32: AtomicU32::new(0),
        };
        if storage.bin_path.exists() {
            if let Ok(mut file) = File::open(&storage.bin_path) {
                let mut buffer = Vec::new();
                if file.read_to_end(&mut buffer).is_ok() {
                    if let Ok(snapshot) = bincode::deserialize::<Snapshot>(&buffer) {
                        for (k, v) in snapshot.nodes { storage.nodes.insert(k, v); }
                        for (k, v) in snapshot.edges { storage.edges.insert(k, v); }
                        for (k, v) in snapshot.out_idx { storage.out_idx.insert(k, v); }
                        for (k, v) in snapshot.in_idx { storage.in_idx.insert(k, v); }
                        // Hack for vector_arena and metadata_arena since they are part of storage but mut
                        // We'll need to make Storage fields wrapped in Arc/Mutex if we want full interior mutability.
                        // For now, let's keep them as is and fix the 'mut' issues.
                    }
                }
            }
        }
        // Need to return mut storage to satisfy rehydrate
        let mut storage = storage;
        if storage.bin_path.exists() {
            if let Ok(mut file) = File::open(&storage.bin_path) {
                let mut buffer = Vec::new();
                if file.read_to_end(&mut buffer).is_ok() {
                    if let Ok(snapshot) = bincode::deserialize::<Snapshot>(&buffer) {
                        storage.vector_arena = snapshot.vector_arena;
                        storage.metadata_arena = snapshot.metadata_arena;
                        for (k, v) in snapshot.id_to_u32 { storage.id_to_u32.insert(k, v); }
                        for (k, v) in snapshot.u32_to_id { storage.u32_to_id.insert(k, v); }
                        storage.next_u32 = AtomicU32::new(snapshot.next_u32);
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
                        let u32_id = storage.get_or_intern_id(&n.id);
                        if let Some(emb) = n.embedding.clone() { storage.add_vector_internal(&n.id, emb); }
                        storage.nodes.insert(u32_id, n);
                    }
                    Ok(Event::Edge(e)) => {
                        let u32_id = storage.get_or_intern_id(&e.id);
                        let u32_from = storage.get_or_intern_id(&e.from);
                        let u32_to = storage.get_or_intern_id(&e.to);
                        storage.out_idx.entry(u32_from).or_insert_with(HashSet::new).insert(u32_id);
                        storage.in_idx.entry(u32_to).or_insert_with(HashSet::new).insert(u32_id);
                        storage.edges.insert(u32_id, e);
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

    pub fn ensure_writable(&self) -> Result<()> { if self.read_only { return Err(Error::from_reason("read-only")); } Ok(()) }

    pub fn index_edge_internal(&mut self, id: &str, from: &str, to: &str) {
        let u32_id = self.get_or_intern_id(id);
        let u32_from = self.get_or_intern_id(from);
        let u32_to = self.get_or_intern_id(to);
        self.out_idx.entry(u32_from).or_insert_with(HashSet::new).insert(u32_id);
        self.in_idx.entry(u32_to).or_insert_with(HashSet::new).insert(u32_id);
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
        let u32_id = match self.get_u32(id) { Some(id) => id, None => return 0.0 };
        let incoming = self.in_idx.get(&u32_id).map_or(0, |set| set.len());
        (incoming as f64 / 10.0).min(1.0)
    }

    pub fn compute_impact(&self, node: &NodeOutput) -> f64 {
        let dd = self.calculate_dd(&node.id); let as_score = self.calculate_as(&node.id);
        let sc = self.calculate_sc(node); (dd * 0.5) + (as_score * 0.3) + (sc * 0.2)
    }

    pub fn refresh_impacts(&mut self, affected_ids: Option<Vec<String>>) {
        let ids_to_update = match affected_ids {
            Some(ids) => {
                let mut queue = VecDeque::from(ids.iter().filter_map(|id| self.get_u32(id)).collect::<Vec<_>>());
                let mut affected = HashSet::new();
                while let Some(curr_u32) = queue.pop_front() {
                    if affected.insert(curr_u32) {
                        if let Some(edges) = self.out_idx.get(&curr_u32) {
                            for eid in edges.value() { 
                                if let Some(e) = self.edges.get(eid) { 
                                    if let Some(target_u32) = self.get_u32(&e.to) {
                                        queue.push_back(target_u32); 
                                    }
                                } 
                            }
                        }
                    }
                }
                affected.into_iter().collect::<Vec<_>>()
            }
            None => self.nodes.iter().map(|r| *r.key()).collect(),
        };
        for u32_id in ids_to_update {
            if let Some(mut node) = self.nodes.get_mut(&u32_id) {
                let impact = self.compute_impact(node.value());
                node.impact = Some(impact);
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

    pub fn bulk_add_nodes(&mut self, inputs: Vec<NodeInput>) -> Result<()> {
        self.ensure_writable()?;
        let mut events = Vec::new();
        let mut ids_to_refresh = Vec::new();
        for input in inputs {
            let id = input.id.clone().unwrap_or_else(|| {
                let hash = format!("{:x}", md5::compute(format!("{:?}{:?}", input.labels, input.props)));
                format!("N-{}", &hash[..16])
            });
            let u32_id = self.get_or_intern_id(&id);
            let mut node = NodeOutput {
                id: id.clone(), labels: input.labels,
                props: input.props.unwrap_or(Value::Object(Default::default())),
                impact: None, embedding: None,
            };
            if let Some(emb_64) = input.embedding {
                let emb: Vec<f32> = emb_64.into_iter().map(|v| v as f32).collect();
                let dim = emb.len() as u16;
                let current_vec_len = self.vector_arena.len();
                let aligned_offset = Self::allocate_aligned_offset(current_vec_len, 64);
                if aligned_offset > current_vec_len { self.vector_arena.resize(aligned_offset, 0.0); }
                self.vector_arena.extend_from_slice(&emb);
                let arena_id = self.metadata_arena.len() as u32;
                let metadata = NodeMetadata {
                    arena_id, node_id: id.clone(), timestamp: Utc::now().timestamp() as u64,
                    vector_dim: dim, embedding_offset: aligned_offset as u64, gks_attributes: Vec::new(),
                };
                self.metadata_arena.push(metadata);
                node.embedding = Some(emb.into_iter().map(|v| v as f64).collect());
            }
            let impact = self.compute_impact(&node); node.impact = Some(impact);
            self.nodes.insert(u32_id, node.clone());
            ids_to_refresh.push(id.clone());
            events.push(Event::Node(node));
        }
        let mut file = FileOpenOptions::new().create(true).append(true).open(&self.log_path).map_err(|e| Error::from_reason(format!("io: {e}")))?;
        for ev in events { let line = serde_json::to_string(&ev).unwrap(); writeln!(file, "{}", line).ok(); }
        self.refresh_impacts(Some(ids_to_refresh));
        Ok(())
    }

    pub fn bulk_add_edges(&mut self, inputs: Vec<EdgeInput>) -> Result<()> {
        self.ensure_writable()?;
        let mut events = Vec::new();
        let mut ids_to_refresh = Vec::new();
        for input in inputs {
            let now = Utc::now().to_rfc3339();
            let edge = EdgeOutput {
                id: input.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                from: input.from, to: input.to, rel: input.rel,
                props: input.props.unwrap_or(Value::Object(Default::default())),
                valid_from: input.valid_from.unwrap_or_else(|| now.clone()),
                valid_to: None, recorded_at: now, superseded_by: None, impact: input.impact,
            };
            self.index_edge_internal(&edge.id, &edge.from, &edge.to);
            let u32_id = self.get_or_intern_id(&edge.id);
            let target_id = edge.to.clone();
            self.edges.insert(u32_id, edge.clone());
            ids_to_refresh.push(target_id);
            events.push(Event::Edge(edge));
        }
        let mut file = FileOpenOptions::new().create(true).append(true).open(&self.log_path).map_err(|e| Error::from_reason(format!("io: {e}")))?;
        for ev in events { let line = serde_json::to_string(&ev).unwrap(); writeln!(file, "{}", line).ok(); }
        self.refresh_impacts(Some(ids_to_refresh));
        Ok(())
    }

    pub fn rebuild_index_parallel(&mut self) -> Result<()> {
        if self.metadata_arena.is_empty() { return Ok(()); }
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
        Ok(())
    }

    pub fn add_node(&mut self, args: NodeInput) -> Result<NodeOutput> {
        self.ensure_writable()?;
        let id = args.id.unwrap_or_else(|| {
            let hash = format!("{:x}", md5::compute(format!("{:?}{:?}", args.labels, args.props)));
            format!("N-{}", &hash[..16])
        });
        let u32_id = self.get_or_intern_id(&id);
        let existing_node = self.nodes.get(&u32_id).map(|n| n.value().clone());
        if let Some(mut n) = existing_node {
            let mut labels = n.labels.clone();
            for l in args.labels { if !labels.contains(&l) { labels.push(l); } }
            let mut props = n.props.as_object().cloned().unwrap_or_default();
            if let Some(new_props) = args.props.and_then(|p| p.as_object().cloned()) {
                for (k, v) in new_props { props.insert(k, v); }
            }
            n.labels = labels; n.props = Value::Object(props);
            let impact = self.compute_impact(&n); n.impact = Some(impact);
            self.nodes.insert(u32_id, n.clone()); self.persist(&Event::Node(n.clone()))?;
            self.refresh_impacts(Some(vec![id.clone()])); return Ok(n);
        }
        let mut node = NodeOutput {
            id: id.clone(), labels: args.labels,
            props: args.props.unwrap_or(Value::Object(Default::default())),
            impact: None, embedding: None,
        };
        if let Some(emb) = args.embedding { self.add_vector_internal(&id, emb.clone()); node.embedding = Some(emb); }
        let impact = self.compute_impact(&node); node.impact = Some(impact);
        self.nodes.insert(u32_id, node.clone()); self.persist(&Event::Node(node.clone()))?;
        self.refresh_impacts(Some(vec![id])); Ok(node)
    }

    pub fn add_edge(&mut self, args: EdgeInput) -> Result<EdgeOutput> {
        self.ensure_writable()?;
        if !self.get_u32(&args.from).is_some() || !self.get_u32(&args.to).is_some() { return Err(Error::from_reason("unknown node")); }
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
        let u32_id = self.get_or_intern_id(&edge.id);
        let target_id = edge.to.clone(); self.edges.insert(u32_id, edge.clone());
        self.refresh_impacts(Some(vec![target_id])); self.persist(&Event::Edge(edge.clone()))?;
        Ok(edge)
    }

    pub fn query(&self, args: QueryInput) -> Result<Vec<EdgeOutput>> {
        let as_of_ms = args.as_of.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|dt| dt.timestamp_millis());
        let mut results = Vec::new();
        for r in self.edges.iter() {
            let edge = r.value();
            if let Some(ref from) = args.from { if edge.from != *from { continue; } }
            if let Some(ref to) = args.to { if edge.to != *to { continue; } }
            if let Some(ref rel) = args.rel { if edge.rel != *rel { continue; } }
            let is_valid = if let Some(ms) = as_of_ms {
                let from_ms = chrono::DateTime::parse_from_rfc3339(&edge.valid_from).map(|dt| dt.timestamp_millis()).unwrap_or(0);
                let to_ms = edge.valid_to.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|dt| dt.timestamp_millis());
                ms >= from_ms && to_ms.map_or(true, |end| ms < end)
            } else { args.include_invalid.unwrap_or(false) || edge.valid_to.is_none() };
            if is_valid { results.push(edge.clone()); }
        }
        results.sort_by(|a, b| b.impact.unwrap_or(0.0).partial_cmp(&a.impact.unwrap_or(0.0)).unwrap());
        if let Some(limit) = args.limit { if results.len() > limit as usize { results.truncate(limit as usize); } }
        Ok(results)
    }

    pub fn status_sync(&self) -> DatabaseStatus { DatabaseStatus { open: true, read_only: self.read_only, page_cache_mb: 64 } }

    pub fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let hnsw = match &self.hnsw_index { Some(idx) => idx, None => return Err(Error::from_reason("HNSW index not initialized")) };
        let k_vec = args.k * 2; let alpha = args.alpha.unwrap_or(0.5);
        let query_f32: Vec<f32> = args.query_vector.into_iter().map(|v| v as f32).collect();
        let results = hnsw.search(&query_f32, k_vec as usize, 100);
        let mut hybrid_results = Vec::with_capacity(results.len());
        for neighbor in results {
            let arena_id = neighbor.d_id as u32; let similarity = 1.0 - neighbor.distance;
            if let Some(meta) = self.metadata_arena.get(arena_id as usize) {
                if let Some(u32_id) = self.get_u32(&meta.node_id) {
                    if let Some(node) = self.nodes.get(&u32_id) {
                        let mut node_out = node.value().clone();
                        let hybrid_score = (similarity as f64 * (1.0 - alpha)) + (node_out.impact.unwrap_or(0.0) * alpha);
                        node_out.impact = Some(hybrid_score);
                        hybrid_results.push(NeighborOutput { node: node_out, path: Vec::new(), depth: 0 });
                    }
                }
            }
        }
        hybrid_results.sort_by(|a, b| b.node.impact.unwrap_or(0.0).partial_cmp(&a.node.impact.unwrap_or(0.0)).unwrap());
        hybrid_results.truncate(args.k as usize); Ok(hybrid_results)
    }

        pub fn execute_hql(&self, query: &str) -> Result<Vec<NeighborOutput>> {
        let q_lower = query.to_lowercase();
        
        // Pattern 1: SEARCH <Label> SIMILAR TO [v] K <n>
        let re_search = regex::Regex::new(r"searchs+w+s+similars+tos+[([0-9.,s-]+)]s+ks+(d+)").unwrap();
        if let Some(caps) = re_search.captures(&q_lower) {
            let vec_str = caps.get(1).unwrap().as_str();
            let k = caps.get(2).unwrap().as_str().parse::<u32>().unwrap_or(5);
            let query_vector: Vec<f64> = vec_str.split(',')
                .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
                .collect();
            return self.hybrid_search(HybridSearchInput { query_vector, k, alpha: Some(0.0) });
        }

        // Pattern 2: TRAVERSE FROM <id> DEPTH <n> REL <rel>
        let re_traverse = regex::Regex::new(r"traverses+froms+([w-]+)s+depths+(d+)s+rels+(w+)").unwrap();
        if let Some(caps) = re_traverse.captures(&q_lower) {
            let seed = caps.get(1).unwrap().as_str().to_string();
            let depth = caps.get(2).unwrap().as_str().parse::<u32>().unwrap_or(1);
            let rel = caps.get(3).unwrap().as_str().to_string();
            return self.neighbors(seed, NeighborInput { 
                depth: Some(depth), rel: Some(rel), rels: None, 
                direction: Some("out".to_string()), as_of: None, 
                include_invalid: Some(false), limit: None 
            });
        }

        // Pattern 3: MATCH <Label> SIMILAR TO [v] ALPHA <f>
        let re_match = regex::Regex::new(r"matchs+w+s+similars+tos+[([0-9.,s-]+)]s+alphas+([0-9.]+)").unwrap();
        if let Some(caps) = re_match.captures(&q_lower) {
            let vec_str = caps.get(1).unwrap().as_str();
            let alpha = caps.get(2).unwrap().as_str().parse::<f64>().unwrap_or(0.5);
            let query_vector: Vec<f64> = vec_str.split(',')
                .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
                .collect();
            return self.hybrid_search(HybridSearchInput { query_vector, k: 10, alpha: Some(alpha) });
        }

        Err(Error::from_reason(format!("HQL Syntax Error: Unsupported query format -> {}", query)))
    }

    pub fn neighbors(&self, seed: String, args: NeighborInput) -> Result<Vec<NeighborOutput>> {
        let u32_seed = match self.get_u32(&seed) { Some(id) => id, None => return Ok(Vec::new()) };
        let depth = args.depth.unwrap_or(1); let direction = args.direction.as_deref().unwrap_or("out");
        let mut target_rels = HashSet::new(); if let Some(r) = args.rel { target_rels.insert(r); }
        if let Some(rs) = args.rels { target_rels.extend(rs); }
        let mut results = Vec::new(); let mut visited = HashSet::new(); visited.insert(u32_seed);
        let mut queue = VecDeque::new(); queue.push_back((u32_seed, Vec::new(), 0));
        while let Some((curr_u32, path, curr_depth)) = queue.pop_front() {
            if curr_depth >= depth { continue; }
            let mut edge_u32_ids = HashSet::new();
            if direction == "out" || direction == "both" { if let Some(eids) = self.out_idx.get(&curr_u32) { edge_u32_ids.extend(eids.clone()); } }
            if direction == "in" || direction == "both" { if let Some(eids) = self.in_idx.get(&curr_u32) { edge_u32_ids.extend(eids.clone()); } }
            let mut edges_to_visit: Vec<EdgeOutput> = edge_u32_ids.iter().filter_map(|eid| self.edges.get(eid)).map(|r| r.value().clone()).collect();
            edges_to_visit.sort_by(|a, b| b.impact.unwrap_or(0.0).partial_cmp(&a.impact.unwrap_or(0.0)).unwrap());
            for edge in edges_to_visit {
                if !target_rels.is_empty() && !target_rels.contains(&edge.rel) { continue; }
                if !args.include_invalid.unwrap_or(false) && edge.valid_to.is_some() { continue; }
                let next_str_id = if self.get_u32(&edge.from) == Some(curr_u32) { &edge.to } else { &edge.from };
                let next_u32 = self.get_u32(next_str_id).unwrap();
                if visited.contains(&next_u32) { continue; }
                visited.insert(next_u32);
                if let Some(node) = self.nodes.get(&next_u32) {
                    let mut new_path = path.clone(); new_path.push(edge.clone());
                    results.push(NeighborOutput { node: node.value().clone(), path: new_path.clone(), depth: curr_depth + 1 });
                    queue.push_back((next_u32, new_path, curr_depth + 1));
                }
            }
        }
        results.sort_by(|a, b| b.node.impact.unwrap_or(0.0).partial_cmp(&a.node.impact.unwrap_or(0.0)).unwrap());
        if let Some(limit) = args.limit { if results.len() > limit as usize { results.truncate(limit as usize); } }
        Ok(results)
    }

    pub fn compact(&self) -> Result<()> {
        self.ensure_writable()?;
        let tmp_path = self.path.join("genesis-graph.jsonl.tmp");
        let mut file = FileOpenOptions::new().create(true).write(true).truncate(true).open(&tmp_path).map_err(|e| Error::from_reason(format!("compact: {e}")))?;
        for node in self.nodes.iter() { let line = serde_json::to_string(&Event::Node(node.value().clone())).unwrap(); writeln!(file, "{}", line).ok(); }
        for edge in self.edges.iter() { if edge.value().valid_to.is_none() { let line = serde_json::to_string(&Event::Edge(edge.value().clone())).unwrap(); writeln!(file, "{}", line).ok(); } }
        file.flush().ok(); drop(file); fs::rename(&tmp_path, &self.log_path).ok();
        let snapshot = Snapshot {
            nodes: self.nodes.iter().map(|r| (*r.key(), r.value().clone())).collect(),
            edges: self.edges.iter().map(|r| (*r.key(), r.value().clone())).collect(),
            out_idx: self.out_idx.iter().map(|r| (*r.key(), r.value().clone())).collect(),
            in_idx: self.in_idx.iter().map(|r| (*r.key(), r.value().clone())).collect(),
            vector_arena: self.vector_arena.clone(),
            metadata_arena: self.metadata_arena.clone(),
            id_to_u32: self.id_to_u32.iter().map(|r| (r.key().clone(), *r.value())).collect(),
            u32_to_id: self.u32_to_id.iter().map(|r| (*r.key(), r.value().clone())).collect(),
            next_u32: self.next_u32.load(Ordering::SeqCst),
        };
        if let Ok(encoded) = bincode::serialize(&snapshot) { fs::write(&self.bin_path, encoded).ok(); }
        Ok(())
    }

    pub fn retract_edge(&mut self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> {
        self.ensure_writable()?;
        let u32_id = match self.get_u32(&id) { Some(id) => id, None => return Ok(None) };
        let mut e = match self.edges.get_mut(&u32_id) { Some(e) => e, None => return Ok(None) };
        if e.valid_to.is_some() { return Ok(Some(e.clone())); }
        let at = at.unwrap_or_else(|| Utc::now().to_rfc3339()); e.valid_to = Some(at);
        let retired = e.clone(); self.persist(&Event::Edge(retired.clone()))?; Ok(Some(retired))
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
    pub async fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.write().bulk_add_nodes(inputs)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }

    #[napi]
    pub async fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.write().bulk_add_edges(inputs)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }

    #[napi]
    pub async fn rebuild_index_parallel(&self) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.write().rebuild_index_parallel()).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }

    #[napi]
    pub async fn add_node(&self, args: NodeInput) -> Result<NodeOutput> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.write().add_node(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.write().add_edge(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn retract_edge(&self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.write().retract_edge(id, at)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn query(&self, args: QueryInput) -> Result<Vec<EdgeOutput>> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.read().query(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.read().hybrid_search(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
        #[napi]
    pub async fn execute_hql(&self, query: String) -> Result<Vec<NeighborOutput>> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.read().execute_hql(&query)).await.map_err(|e| Error::from_reason(format!("join: {}", e)))?
    }

    #[napi]
    pub async fn neighbors(&self, seed: String, args: NeighborInput) -> Result<Vec<NeighborOutput>> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.read().neighbors(seed, args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn compact(&self) -> Result<()> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.write().compact()).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub fn schema_version_sync(&self) -> u32 { SCHEMA_VERSION }
    #[napi]
    pub fn status_sync(&self) -> DatabaseStatus {
        let storage = self.inner.read(); let mut status = storage.status_sync(); status.page_cache_mb = self.page_cache_mb; status
    }
}
#[napi]
pub fn engine_name_sync() -> String { "genesis-block".to_string() }
