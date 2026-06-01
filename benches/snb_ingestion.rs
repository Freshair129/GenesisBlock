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
    let root = "G:/GenesisBlock_Dev/GenesisBlock/benches/snb/temp_db";
    let data_path = "G:/GenesisBlock_Dev/GenesisBlock/benches/snb/data";
    
    if Path::new(root).exists() { fs::remove_dir_all(root).unwrap(); }
    
    let mut storage = Storage::open(OpenOptions {
        path: root.to_string(),
        page_cache_mb: Some(128),
        read_only: Some(false),
    }).expect("Failed to open storage");

    println!("🚀 Starting Phase 8: LDBC SNB Ingestion (SF0.1)");
    let start = Instant::now();

    // 1. Load Persons
    let mut person_count = 0;
    let file = File::open(Path::new(data_path).join("person.csv")).unwrap();
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(BufReader::new(file));
    for result in rdr.deserialize() {
        let p: PersonCsv = result.unwrap();
        storage.add_node(NodeInput {
            id: Some(p.id),
            labels: vec!["Person".to_string()],
            props: None,
            embedding: None,
        }).unwrap();
        person_count += 1;
    }

    // 2. Load Posts (with Vectors)
    let mut post_count = 0;
    let file = File::open(Path::new(data_path).join("post.csv")).unwrap();
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(BufReader::new(file));
    for result in rdr.deserialize() {
        let p: PostCsv = result.unwrap();
        storage.add_node(NodeInput {
            id: Some(p.id),
            labels: vec!["Post".to_string()],
            props: None,
            embedding: Some(vec![0.1; 768]),
        }).unwrap();
        post_count += 1;
    }

    // 3. Load Knows
    let mut edge_count = 0;
    let file = File::open(Path::new(data_path).join("knows.csv")).unwrap();
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(BufReader::new(file));
    for result in rdr.deserialize() {
        let k: KnowsCsv = result.unwrap();
        storage.add_edge(EdgeInput {
            id: None, from: k.source_id, to: k.target_id, rel: "knows".to_string(),
            props: None, valid_from: None, supersede: None, impact: None,
        }).unwrap();
        edge_count += 1;
    }

    let duration = start.elapsed();
    let total_ops = person_count + post_count + edge_count;
    
    println!("✅ Ingestion Complete");
    println!("---------------------------------");
    println!("Total Nodes:  {}", person_count + post_count);
    println!("Total Edges:  {}", edge_count);
    println!("Total Ops:    {}", total_ops);
    println!("Duration:     {:?}", duration);
    println!("TPS:          {:.2}", total_ops as f64 / duration.as_secs_f64());
    println!("---------------------------------");
}
