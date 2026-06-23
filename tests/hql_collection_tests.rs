// HQL `IN <collection>` clause (formerly P-D deferred): `SEARCH ... IN "code"`
// and `MATCH ... IN "code"` scope the query to a named vector collection. Without
// the clause, queries route to `default` (back-compat). Grammar SSOT is
// `src/query/hql.pest`; this exercises the full parse -> execute_hql -> scoped
// hybrid_search path.

use genesis_block_native::{Storage, OpenOptions, NodeInput};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("G:/GenesisBlock_Dev/GenesisBlock/tests/{}", name);
    if Path::new(&p).exists() { fs::remove_dir_all(&p).unwrap(); }
    p
}

fn open_dim(path: &str, dim: u32) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(), page_cache_mb: Some(64), read_only: Some(false),
        vector_dim: Some(dim),
    }).unwrap()
}

fn add(s: &Storage, id: &str, emb: Vec<f64>, collection: Option<&str>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()), labels: vec![], props: None,
        embedding: Some(emb), lang: None, valid_from: None, caused_by: None, ttl: None,
        collection: collection.map(|c| c.to_string()),
    }).unwrap();
}

/// Extract the node ids out of an `execute_hql` search/hybrid result value.
fn ids(v: &serde_json::Value) -> Vec<String> {
    v.as_array().unwrap().iter()
        .map(|n| n["node"]["id"].as_str().unwrap().to_string())
        .collect()
}

fn two_collections(name: &str) -> Storage {
    let s = open_dim(&fresh(name), 4);
    s.create_collection("code".to_string(), "jina-code".to_string(), 4, Some("L2".to_string()), None, None, None).unwrap();
    add(&s, "code-A", vec![1.0, 0.0, 0.0, 0.0], Some("code"));
    add(&s, "code-B", vec![0.0, 1.0, 0.0, 0.0], Some("code"));
    add(&s, "def-A", vec![1.0, 0.0, 0.0, 0.0], None); // -> default
    s.flush_index();
    s
}

/// `SEARCH ... IN "code"` (quoted) scopes the search to the code collection and
/// never surfaces a default-collection vector.
#[test]
fn search_in_quoted_collection_scopes() {
    let s = two_collections("test_hql_coll_quoted");
    let v = s.execute_hql("SEARCH q SIMILAR TO [1.0,0.0,0.0,0.0] K 10 IN \"code\"").unwrap();
    let hits = ids(&v);
    assert!(hits.contains(&"code-A".to_string()), "code search returns its own vector");
    assert!(!hits.contains(&"def-A".to_string()), "default vector must not appear in code search");
}

/// The collection name may also be a bare identifier (`IN code`, no quotes).
#[test]
fn search_in_bare_identifier_collection() {
    let s = two_collections("test_hql_coll_bare");
    let v = s.execute_hql("SEARCH q SIMILAR TO [1.0,0.0,0.0,0.0] K 10 IN code").unwrap();
    let hits = ids(&v);
    assert!(hits.contains(&"code-A".to_string()));
    assert!(!hits.contains(&"def-A".to_string()));
}

/// Without an `IN` clause the search routes to `default` (back-compat).
#[test]
fn search_without_in_uses_default() {
    let s = two_collections("test_hql_coll_default");
    let v = s.execute_hql("SEARCH q SIMILAR TO [1.0,0.0,0.0,0.0] K 10").unwrap();
    let hits = ids(&v);
    assert!(hits.contains(&"def-A".to_string()), "default search returns the default vector");
    assert!(!hits.contains(&"code-A".to_string()), "code vector must not appear in default search");
}

/// `MATCH ... ALPHA ... IN "code"` scopes the hybrid command the same way.
#[test]
fn match_in_collection_scopes() {
    let s = two_collections("test_hql_coll_match");
    let v = s.execute_hql("MATCH q SIMILAR TO [1.0,0.0,0.0,0.0] ALPHA 0.0 IN \"code\"").unwrap();
    let hits = ids(&v);
    assert!(hits.contains(&"code-A".to_string()));
    assert!(!hits.contains(&"def-A".to_string()));
}

/// `IN` composes with `LANGUAGE` and `AS OF` in the documented order.
#[test]
fn search_in_with_language_and_as_of_parses() {
    let s = two_collections("test_hql_coll_langasof");
    let v = s.execute_hql(
        "SEARCH q SIMILAR TO [1.0,0.0,0.0,0.0] K 5 IN \"code\" LANGUAGE \"en\" AS OF \"2030-01-01T00:00:00Z\""
    ).unwrap();
    let hits = ids(&v);
    assert!(hits.contains(&"code-A".to_string()));
    assert!(!hits.contains(&"def-A".to_string()));
}

/// Searching an unknown collection via HQL surfaces the typed "not found" error.
#[test]
fn search_in_unknown_collection_errors() {
    let s = two_collections("test_hql_coll_unknown");
    let r = s.execute_hql("SEARCH q SIMILAR TO [1.0,0.0,0.0,0.0] K 5 IN \"nope\"");
    assert!(r.is_err(), "unknown collection must error");
    assert!(r.unwrap_err().to_string().contains("not found"));
}
