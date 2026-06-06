use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use csv::{Reader, ReaderBuilder};
use serde::{Deserialize, Serialize};
use genesis_block_native::{Storage, NodeInput, EdgeInput};

#[derive(Serialize, Deserialize)]
struct PersonCsv {
    id: String,
    name: String,
    gender: String,
}

#[derive(Serialize, Deserialize)]
struct KnowsCsv {
    source_id: String,
    target_id: String,
}

#[derive(Serialize, Deserialize)]
struct PostCsv {
    id: String,
    content: String,
}

pub fn load_snb_dataset(storage: &mut Storage, path: &str) {
    let person_path = Path::new(path).join("person.csv");
    let knows_path = Path::new(path).join("knows.csv");
    let post_path = Path::new(path).join("post.csv");

    if person_path.exists() {
        let file = File::open(person_path).unwrap();
        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(BufReader::new(file));
        for result in reader.deserialize() {
            let person: PersonCsv = result.unwrap();
            let mut props = serde_json::Map::new();
            props.insert("name".to_string(), serde_json::Value::String(person.name));
            props.insert("gender".to_string(), serde_json::Value::String(person.gender));
            storage.add_node(NodeInput { 
                id: Some(person.id),
                labels: vec!["Person".to_string()],
                props: Some(serde_json::Value::Object(props)),
                embedding: None,
             valid_from: None, caused_by: None, }).unwrap();
        }
    }

    if post_path.exists() {
        let file = File::open(post_path).unwrap();
        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(BufReader::new(file));
        for result in reader.deserialize() {
            let post: PostCsv = result.unwrap();
            let mut props = serde_json::Map::new();
            props.insert("content".to_string(), serde_json::Value::String(post.content));
            let mut embedding = vec![0.1; 768]; // Simplified mock embedding
            storage.add_node(NodeInput { 
                id: Some(post.id),
                labels: vec!["Post".to_string()],
                props: Some(serde_json::Value::Object(props)),
                embedding: Some(embedding),
             valid_from: None, caused_by: None, }).unwrap();
        }
    }

    if knows_path.exists() {
        let file = File::open(knows_path).unwrap();
        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(BufReader::new(file));
        for result in reader.deserialize() {
            let knows: KnowsCsv = result.unwrap();
            storage.add_edge(EdgeInput { 
                id: None, from: knows.source_id, to: knows.target_id, rel: "knows".to_string(),
                props: None, valid_from: None, supersede: None, impact: None,
             caused_by: None, }).unwrap();
        }
    }
}
