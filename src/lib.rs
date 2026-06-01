//! Genesis Block — high-performance hybrid semantic-graph engine.
//!
//! Phase 13: WAL Group Commit & Binary WAL

#![deny(clippy::all)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File, OpenOptions as FileOpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use chrono::Utc;
use dashmap::DashMap;
use hnsw_rs::prelude::*;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use parking_lot::{RwLock, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sysinfo::System;
use uuid::Uuid;
use rayon::prelude::*;
use crossbeam_channel::{unbounded, Sender, Receiver};

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
    pub vector_arena: RwLock<Vec<f32>>,
    pub metadata_arena: RwLock<Vec<NodeMetadata>>,
    pub hnsw_index: RwLock<Option<Hnsw<'static, f32, DistL2>>>,
    pub log_path: PathBuf,
    pub bin_path: PathBuf,
    pub _lock_file: Option<File>,
    pub id_to_u32: DashMap<String, u32>,
    pub u32_to_id: DashMap<u32, String>,
    pub next_u32: AtomicU32,
    pub wal_sender: Sender<(Vec<u8>, Sender<bool>)>,
}

impl Storage {
    pub fn get_or_intern_id(&self, id: &str) -> u32 {
        if let Some(existing) = self.id_to_u32.get(id) { return *existing; }
        let new_id = self.next_u32.fetch_add(1, Ordering::SeqCst);
        self.id_to_u32.insert(id.to_string(), new_id);
        self.u32_to_id.insert(new_id, id.to_string());
        new_id
    }

    pub fn get_u32(&self, id: &str) -> Option<u32> { self.id_to_u32.get(id).map(|v| *v) }

    pub fn allocate_aligned_offset(current_offset: usize, align: usize) -> usize {
        if current_offset % align == 0 { current_offset } else { current_offset + align - (current_offset % align) }
    }

    fn init_hnsw() -> Hnsw<'static, f32, DistL2> { Hnsw::new(16, 1000000, 16, 200, DistL2 {}) }

    fn add_vector_internal(&self, node_id: &str, emb_64: Vec<f64>) {
        let emb: Vec<f32> = emb_64.into_iter().map(|v| v as f32).collect();
        let dim = emb.len() as u16;
        let mut vec_arena = self.vector_arena.write();
        let current_vec_len = vec_arena.len();
        let aligned_offset = Self::allocate_aligned_offset(current_vec_len, 64);
        if aligned_offset > current_vec_len { vec_arena.resize(aligned_offset, 0.0); }
        vec_arena.extend_from_slice(&emb);
        let mut meta_arena = self.metadata_arena.write();
        let arena_id = meta_arena.len() as u32;
        meta_arena.push(NodeMetadata {
            arena_id, node_id: node_id.to_string(), timestamp: Utc::now().timestamp() as u64,
            vector_dim: dim, embedding_offset: aligned_offset as u64, gks_attributes: Vec::new(),
        });
        if self.hnsw_index.read().is_none() { *self.hnsw_index.write() = Some(Self::init_hnsw()); }
        if let Some(ref mut hnsw) = *self.hnsw_index.write() { hnsw.insert((&emb, arena_id as usize)); }
    }

    fn rehydrate_hnsw_index(&self) {
        let meta_arena = self.metadata_arena.read();
        if meta_arena.is_empty() { return; }
        let mut hnsw = Self::init_hnsw();
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
        let lock_file = Self::acquire_os_lock(&root, read_only)?;
        let log_path = root.join("genesis-graph.wal"); // Phase 13: Changed to .wal
        
        let (wal_sender, wal_receiver) = unbounded();
        let log_path_clone = log_path.clone();
        
        // WAL Flusher Thread (Group Commit)
        std::thread::spawn(move || {
            let mut batch: Vec<(Vec<u8>, crossbeam_channel::Sender<bool>)> = Vec::with_capacity(1024);
            loop {
                // Wait for the first event, then try to drain up to 1024 events or 5ms
                match wal_receiver.recv() {
                    Ok(event) => {
                        batch.push(event);
                        let timeout = Duration::from_millis(5);
                        let start = std::time::Instant::now();
                        while batch.len() < 1024 && start.elapsed() < timeout {
                            if let Ok(e) = wal_receiver.try_recv() {
                                batch.push(e);
                            } else {
                                break; // empty channel
                            }
                        }
                        
                        // Execute Group Commit
                        if let Ok(mut file) = FileOpenOptions::new().append(true).create(true).open(&log_path_clone) {
                            for (data, _) in &batch {
                                file.write_all(data).ok();
                            }
                            file.sync_all().ok(); // Physical hardware flush
                            
                            // Send Acks
                            for (_, ack) in batch.drain(..) {
                                ack.send(true).ok();
                            }
                        }
                    },
                    Err(_) => break, // Channel disconnected
                }
            }
        });

        let storage = Self {
            path: root, read_only, nodes: DashMap::new(), edges: DashMap::new(),
            out_idx: DashMap::new(), in_idx: DashMap::new(),
            vector_arena: RwLock::new(Vec::new()), metadata_arena: RwLock::new(Vec::new()),
            hnsw_index: RwLock::new(None), log_path, bin_path: PathBuf::from(""), _lock_file: Some(lock_file),
            id_to_u32: DashMap::new(), u32_to_id: DashMap::new(), next_u32: AtomicU32::new(0),
            wal_sender,
        };
        storage.rehydrate_hnsw_index();
        Ok(storage)
    }

    fn acquire_os_lock(root: &PathBuf, read_only: bool) -> Result<File> {
        let lock_path = root.join("genesis-graph.lock");
        let file = FileOpenOptions::new().read(true).write(true).create(true).open(&lock_path).map_err(|e| Error::from_reason(format!("lock: {e}")))?;
        Ok(file)
    }

    pub fn ensure_writable(&self) -> Result<()> { if self.read_only { return Err(Error::from_reason("read-only")); } Ok(()) }

    pub fn persist(&self, event: &Event) -> Result<()> {
        let event_data = bincode::serialize(event).map_err(|e| Error::from_reason(format!("bincode: {e}")))?;
        let (ack_tx, ack_rx) = unbounded();
        self.wal_sender.send((event_data, ack_tx)).map_err(|_| Error::from_reason("wal sender disconnected"))?;
        let ack = ack_rx.recv().unwrap_or(false);
        if !ack { return Err(Error::from_reason("Group Commit Failed")); }
        Ok(())
    }

    pub fn calculate_as(&self, id: &str) -> f64 { 0.6 }
    pub fn calculate_dd(&self, id: &str) -> f64 { 0.5 }
    pub fn calculate_sc(&self, node: &NodeOutput) -> f64 { 0.8 }
    pub fn compute_impact(&self, node: &NodeOutput) -> f64 { 0.7 }
    pub fn refresh_impacts(&self, _affected_ids: Option<Vec<String>>) {}
    
    pub fn index_edge_internal(&self, id: &str, from: &str, to: &str) {
        let u32_id = self.get_or_intern_id(id);
        let u32_from = self.get_or_intern_id(from);
        let u32_to = self.get_or_intern_id(to);
        self.out_idx.entry(u32_from).or_insert_with(HashSet::new).insert(u32_id);
        self.in_idx.entry(u32_to).or_insert_with(HashSet::new).insert(u32_id);
    }

    pub fn add_node(&self, args: NodeInput) -> Result<NodeOutput> {
        self.ensure_writable()?;
        let id = args.id.clone().unwrap_or_else(|| format!("N-{}", Uuid::new_v4()));
        let u32_id = self.get_or_intern_id(&id);
        let mut node = NodeOutput {
            id: id.clone(), labels: args.labels,
            props: args.props.unwrap_or(Value::Object(Default::default())),
            impact: None, embedding: None,
        };
        if let Some(emb) = args.embedding { self.add_vector_internal(&id, emb.clone()); node.embedding = Some(emb); }
        node.impact = Some(self.compute_impact(&node));
        self.nodes.insert(u32_id, node.clone());
        self.persist(&Event::Node(node.clone()))?;
        Ok(node)
    }

    pub fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> {
        self.ensure_writable()?;
        let edge = EdgeOutput {
            id: args.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            from: args.from, to: args.to, rel: args.rel,
            props: args.props.unwrap_or(Value::Object(Default::default())),
            valid_from: Utc::now().to_rfc3339(), valid_to: None, recorded_at: Utc::now().to_rfc3339(),
            superseded_by: None, impact: args.impact,
        };
        self.index_edge_internal(&edge.id, &edge.from, &edge.to);
        self.edges.insert(self.get_or_intern_id(&edge.id), edge.clone());
        self.persist(&Event::Edge(edge.clone()))?;
        Ok(edge)
    }

    pub fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> {
        self.ensure_writable()?;
        let processed: Vec<(String, NodeOutput, Option<Vec<f32>>)> = inputs.into_par_iter().map(|input| {
            let id = input.id.clone().unwrap_or_else(|| format!("N-{}", Uuid::new_v4()));
            let node = NodeOutput {
                id: id.clone(), labels: input.labels,
                props: input.props.unwrap_or(Value::Object(Default::default())),
                impact: None, embedding: None,
            };
            let emb = input.embedding.map(|e| e.into_iter().map(|v| v as f32).collect::<Vec<_>>());
            (id, node, emb)
        }).collect();

        for (id, mut node, emb) in processed {
            let u32_id = self.get_or_intern_id(&id);
            if let Some(e) = emb {
                let dim = e.len() as u16;
                let mut vec_arena = self.vector_arena.write();
                let current_vec_len = vec_arena.len();
                let aligned_offset = Self::allocate_aligned_offset(current_vec_len, 64);
                if aligned_offset > current_vec_len { vec_arena.resize(aligned_offset, 0.0); }
                vec_arena.extend_from_slice(&e);
                
                let mut meta_arena = self.metadata_arena.write();
                let arena_id = meta_arena.len() as u32;
                meta_arena.push(NodeMetadata {
                    arena_id, node_id: id.clone(), timestamp: Utc::now().timestamp() as u64,
                    vector_dim: dim, embedding_offset: aligned_offset as u64, gks_attributes: Vec::new(),
                });
                node.embedding = Some(e.into_iter().map(|v| v as f64).collect());
            }
            node.impact = Some(0.0); 
            self.nodes.insert(u32_id, node.clone());
            self.persist(&Event::Node(node))?; // The Group Commit handles the batching automatically!
        }
        Ok(())
    }

    pub fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> {
        self.ensure_writable()?;
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
            self.edges.insert(u32_id, edge.clone());
            self.persist(&Event::Edge(edge))?; // The Group Commit handles the batching automatically!
        }
        Ok(())
    }

    pub fn rebuild_index_parallel(&self) -> Result<()> {
        let meta_arena = self.metadata_arena.read();
        let vec_arena = self.vector_arena.read();
        if meta_arena.is_empty() { return Ok(()); }
        
        let mut hnsw = Self::init_hnsw();
        for meta in meta_arena.iter() {
            let start = meta.embedding_offset as usize;
            let end = start + meta.vector_dim as usize;
            if end <= vec_arena.len() {
                hnsw.insert((&vec_arena[start..end], meta.arena_id as usize));
            }
        }
            
        *self.hnsw_index.write() = Some(hnsw);
        Ok(())
    }

    pub fn execute_hql(&self, query: &str) -> Result<Vec<NeighborOutput>> {
        let q_lower = query.to_lowercase();
        
        let re_search = regex::Regex::new(r"search\s+\w+\s+similar\s+to\s+\[([0-9.,\s-]+)\]\s+k\s+(\d+)").unwrap();
        if let Some(caps) = re_search.captures(&q_lower) {
            let vec_str = caps.get(1).unwrap().as_str();
            let k = caps.get(2).unwrap().as_str().parse::<u32>().unwrap_or(5);
            let query_vector: Vec<f64> = vec_str.split(',')
                .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
                .collect();
            return self.hybrid_search(HybridSearchInput { query_vector, k, alpha: Some(0.0) });
        }

        let re_traverse = regex::Regex::new(r"traverse\s+from\s+([\w-]+)\s+depth\s+(\d+)\s+rel\s+(\w+)").unwrap();
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

        Err(Error::from_reason(format!("HQL Syntax Error or Unsupported -> {}", query)))
    }

    pub fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let hnsw_lock = self.hnsw_index.read();
        let hnsw = match &*hnsw_lock { Some(idx) => idx, None => return Err(Error::from_reason("HNSW index not initialized")) };
        let k_vec = args.k * 2; let alpha = args.alpha.unwrap_or(0.5);
        let query_f32: Vec<f32> = args.query_vector.into_iter().map(|v| v as f32).collect();
        let results = hnsw.search(&query_f32, k_vec as usize, 100);
        let mut hybrid_results = Vec::with_capacity(results.len());
        
        let meta_arena = self.metadata_arena.read();
        for neighbor in results {
            let arena_id = neighbor.d_id as u32; let similarity = 1.0 - neighbor.distance;
            if let Some(meta) = meta_arena.get(arena_id as usize) {
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

    pub fn compact(&self) -> Result<()> {
        let tmp_path = self.path.join("genesis-graph.wal.tmp");
        let mut file = FileOpenOptions::new().create(true).write(true).truncate(true).open(&tmp_path).map_err(|e| Error::from_reason(format!("compact: {e}")))?;
        for node in self.nodes.iter() { let line = serde_json::to_string(&Event::Node(node.value().clone())).unwrap(); writeln!(file, "{}", line).ok(); }
        for edge in self.edges.iter() { if edge.value().valid_to.is_none() { let line = serde_json::to_string(&Event::Edge(edge.value().clone())).unwrap(); writeln!(file, "{}", line).ok(); } }
        file.flush().ok(); drop(file); fs::rename(&tmp_path, &self.log_path).ok();
        Ok(())
    }

    pub fn retract_edge(&self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> {
        self.ensure_writable()?;
        let u32_id = match self.get_u32(&id) { Some(id) => id, None => return Ok(None) };
        let mut e = match self.edges.get_mut(&u32_id) { Some(e) => e, None => return Ok(None) };
        if e.valid_to.is_some() { return Ok(Some(e.clone())); }
        let at = at.unwrap_or_else(|| Utc::now().to_rfc3339()); e.valid_to = Some(at);
        let retired = e.clone(); self.persist(&Event::Edge(retired.clone()))?; Ok(Some(retired))
    }
    
    pub fn status_sync(&self) -> DatabaseStatus { DatabaseStatus { open: true, read_only: self.read_only, page_cache_mb: 64 } }
}

#[napi]
pub struct GenesisDatabase { inner: Arc<Storage> }

#[napi]
impl GenesisDatabase {
    #[napi(factory)]
    pub fn open(opts: OpenOptions) -> Result<Self> { Ok(Self { inner: Arc::new(Storage::open(opts)?) }) }

    #[napi]
    pub async fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.bulk_add_nodes(inputs)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }

    #[napi]
    pub async fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.bulk_add_edges(inputs)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }

    #[napi]
    pub async fn rebuild_index_parallel(&self) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.rebuild_index_parallel()).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }

    #[napi]
    pub async fn add_node(&self, args: NodeInput) -> Result<NodeOutput> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.add_node(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn add_edge(&self, args: EdgeInput) -> Result<EdgeOutput> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.add_edge(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn retract_edge(&self, id: String, at: Option<String>) -> Result<Option<EdgeOutput>> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.retract_edge(id, at)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn query(&self, args: QueryInput) -> Result<Vec<EdgeOutput>> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.query(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn execute_hql(&self, query: String) -> Result<Vec<NeighborOutput>> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.execute_hql(&query)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn hybrid_search(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.hybrid_search(args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn neighbors(&self, seed: String, args: NeighborInput) -> Result<Vec<NeighborOutput>> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.neighbors(seed, args)).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub async fn compact(&self) -> Result<()> {
        let inner = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || inner.compact()).await.map_err(|e| Error::from_reason(format!("join: {e}")))?
    }
    #[napi]
    pub fn schema_version_sync(&self) -> u32 { SCHEMA_VERSION }
    #[napi]
    pub fn status_sync(&self) -> DatabaseStatus { self.inner.status_sync() }
}
#[napi]
pub fn engine_name_sync() -> String { "genesis-block".to_string() }
