use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};

#[derive(Serialize, Deserialize)]
struct PersonCsv { id: String, name: String, gender: String }
#[derive(Serialize, Deserialize)]
struct KnowsCsv { source_id: String, target_id: String }
#[derive(Serialize, Deserialize)]
struct PostCsv { id: String, content: String }

fn main() {
    let root = "G:/GenesisBlock_Dev/GenesisBlock/benches/snb/temp_db_bulk";
    let data_path = "G:/GenesisBlock_Dev/GenesisBlock/benches/snb/data";
    
    if Path::new(root).exists() { fs::remove_dir_all(root).unwrap(); }
    
    let mut storage = Storage::open(OpenOptions {
        path: root.to_string(),
        page_cache_mb: Some(512),
        read_only: Some(false),
    }).expect("Failed to open storage");

    println!("🚀 Starting Phase 8: LDBC SNB BULK Ingestion (SF0.1 FULL)");
    let global_start = Instant::now();

    // 1. Bulk Load Persons
    let file = File::open(Path::new(data_path).join("person.csv")).unwrap();
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(BufReader::new(file));
    let mut buffer = Vec::new();
    let mut total_nodes = 0;
    for result in rdr.deserialize() {
        let p: PersonCsv = result.unwrap();
        buffer.push(NodeInput {
            id: Some(p.id), labels: vec!["Person".to_string()], props: None, embedding: None,
        });
        if buffer.len() >= 10000 {
            total_nodes += buffer.len();
            storage.bulk_add_nodes(buffer.drain(..).collect()).unwrap();
            print!("Nodes loaded: {}\r", total_nodes);
        }
    }
    total_nodes += buffer.len();
    storage.bulk_add_nodes(buffer).unwrap();

    // 2. Bulk Load Posts (with Vectors)
    let file = File::open(Path::new(data_path).join("post.csv")).unwrap();
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(BufReader::new(file));
    let mut buffer = Vec::new();
    for result in rdr.deserialize() {
        let p: PostCsv = result.unwrap();
        buffer.push(NodeInput {
            id: Some(p.id), labels: vec!["Post".to_string()], props: None, embedding: Some(vec![0.1; 768]),
        });
        if buffer.len() >= 5000 {
            total_nodes += buffer.len();
            storage.bulk_add_nodes(buffer.drain(..).collect()).unwrap();
            print!("Nodes loaded: {}\r", total_nodes);
        }
    }
    total_nodes += buffer.len();
    storage.bulk_add_nodes(buffer).unwrap();
    println!("\n✅ All Nodes Ingested.");

    // 3. Bulk Load Edges
    let file = File::open(Path::new(data_path).join("knows.csv")).unwrap();
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(BufReader::new(file));
    let mut buffer = Vec::new();
    let mut total_edges = 0;
    for result in rdr.deserialize() {
        let k: KnowsCsv = result.unwrap();
        buffer.push(EdgeInput {
            id: None, from: k.source_id, to: k.target_id, rel: "knows".to_string(),
            props: None, valid_from: None, supersede: None, impact: None,
        });
        if buffer.len() >= 50000 {
            total_edges += buffer.len();
            storage.bulk_add_edges(buffer.drain(..).collect()).unwrap();
            print!("Edges loaded: {}\r", total_edges);
        }
    }
    total_edges += buffer.len();
    storage.bulk_add_edges(buffer).unwrap();
    println!("\n✅ All Edges Ingested.");

    let ingest_duration = global_start.elapsed();

    // 4. Parallel Index Rebuild
    println!("⚡ Starting Parallel HNSW Rebuild...");
    let rebuild_start = Instant::now();
    storage.rebuild_index_parallel().unwrap();
    let rebuild_duration = rebuild_start.elapsed();

    let total_duration = global_start.elapsed();
    let total_ops = total_nodes + total_edges;
    
    println!("---------------------------------");
    println!("📊 FINAL BULK PERFORMANCE REPORT");
    println!("---------------------------------");
    println!("Total Nodes:      {}", total_nodes);
    println!("Total Edges:      {}", total_edges);
    println!("Total Operations: {}", total_ops);
    println!("Ingest Time:      {:?}", ingest_duration);
    println!("Index Rebuild:    {:?}", rebuild_duration);
    println!("Total Time:       {:?}", total_duration);
    println!("Bulk TPS:         {:.2}", total_ops as f64 / total_duration.as_secs_f64());
    println!("---------------------------------");
}
