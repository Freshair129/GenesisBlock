// P-C/P-D (ADR--GENESISDB-MULTI-COLLECTION / SPEC--MULTI-COLLECTION-VECTOR-SPACE):
// per-model/per-dim isolated vector spaces. A node's embedding routes to its
// collection; search is scoped + dim-validated; collections survive reload;
// custom collections referenced on WAL replay are auto-provisioned.

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open_dim(path: &str, dim: u32) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(dim),
        retention: None,
    })
    .unwrap()
}

fn add(s: &Storage, id: &str, emb: Vec<f64>, collection: Option<&str>) -> Result<(), String> {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: Some(emb),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: collection.map(|c| c.to_string()),
    })
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn search(
    s: &Storage,
    q: Vec<f64>,
    k: u32,
    collection: Option<&str>,
) -> Result<Vec<String>, String> {
    // Indexing is asynchronous (eventually-searchable); flush so the vector is
    // guaranteed in the HNSW before we assert on results.
    s.flush_index();
    s.hybrid_search(HybridSearchInput {
        query_vector: q,
        k,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: collection.map(|c| c.to_string()),
        ef_search: None,
        oversample: None,
    })
    .map(|v| v.into_iter().map(|n| n.node.id).collect())
    .map_err(|e| e.to_string())
}

/// Two collections of different dims coexist; a search in one never surfaces
/// vectors from the other (no cross-space contamination).
#[test]
fn two_collections_are_isolated() {
    let s = open_dim(&fresh("test_mc_isolated"), 4);
    s.create_collection(
        "code".to_string(),
        "jina-code".to_string(),
        4,
        Some("L2".to_string()),
        None,
        None,
        None,
    )
    .unwrap();
    s.create_collection(
        "text".to_string(),
        "bge-m3".to_string(),
        3,
        Some("Cosine".to_string()),
        None,
        None,
        None,
    )
    .unwrap();

    add(&s, "code-A", vec![1.0, 0.0, 0.0, 0.0], Some("code")).unwrap();
    add(&s, "code-B", vec![0.0, 1.0, 0.0, 0.0], Some("code")).unwrap();
    add(&s, "text-A", vec![1.0, 0.0, 0.0], Some("text")).unwrap();

    let code_hits = search(&s, vec![1.0, 0.0, 0.0, 0.0], 10, Some("code")).unwrap();
    assert!(code_hits.contains(&"code-A".to_string()));
    assert!(
        !code_hits.contains(&"text-A".to_string()),
        "text vector must not appear in code search"
    );

    let text_hits = search(&s, vec![1.0, 0.0, 0.0], 10, Some("text")).unwrap();
    assert_eq!(
        text_hits,
        vec!["text-A".to_string()],
        "text search returns only its own space"
    );
}

/// Inserting or querying with the wrong dim returns a typed error, not garbage.
#[test]
fn dim_mismatch_is_rejected() {
    let s = open_dim(&fresh("test_mc_dim"), 4);
    s.create_collection(
        "code".to_string(),
        "m".to_string(),
        4,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    let bad_insert = add(&s, "x", vec![1.0, 2.0, 3.0], Some("code")); // 3 != 4
    assert!(bad_insert.is_err(), "insert with wrong dim must error");
    assert!(bad_insert.unwrap_err().contains("dim"));

    add(&s, "ok", vec![1.0, 0.0, 0.0, 0.0], Some("code")).unwrap();
    let bad_query = search(&s, vec![1.0, 0.0], 5, Some("code")); // 2 != 4
    assert!(bad_query.is_err(), "query with wrong dim must error");
}

/// A search against an unknown collection errors clearly.
#[test]
fn unknown_collection_errors() {
    let s = open_dim(&fresh("test_mc_unknown"), 4);
    let r = search(&s, vec![1.0, 0.0, 0.0, 0.0], 5, Some("nope"));
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("not found"));
}

/// A node added without a collection routes to `default`; list_collections
/// reflects per-collection counts.
#[test]
fn default_routing_and_listing() {
    let s = open_dim(&fresh("test_mc_default"), 4);
    s.create_collection(
        "code".to_string(),
        "m".to_string(),
        4,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    add(&s, "d1", vec![1.0, 0.0, 0.0, 0.0], None).unwrap(); // -> default
    add(&s, "c1", vec![0.0, 1.0, 0.0, 0.0], Some("code")).unwrap(); // -> code

    let infos = s.list_collections();
    let get = |n: &str| infos.iter().find(|c| c.name == n).cloned();
    assert_eq!(get("default").unwrap().count, 1);
    assert_eq!(get("code").unwrap().count, 1);
    assert_eq!(get("code").unwrap().dim, 4);

    // default search finds the default node, not the code node.
    let hits = search(&s, vec![1.0, 0.0, 0.0, 0.0], 10, None).unwrap();
    assert!(hits.contains(&"d1".to_string()));
    assert!(!hits.contains(&"c1".to_string()));
}

/// Collections + their vectors survive a snapshot save/reopen, preserving the
/// declared model/metric/dim and remaining searchable.
#[test]
fn collections_survive_snapshot_reload() {
    let path = fresh("test_mc_snapshot");
    {
        let s = open_dim(&path, 4);
        s.create_collection(
            "code".to_string(),
            "jina-code".to_string(),
            4,
            Some("Cosine".to_string()),
            None,
            None,
            None,
        )
        .unwrap();
        add(&s, "c1", vec![1.0, 0.0, 0.0, 0.0], Some("code")).unwrap();
        s.save_state().unwrap();
    }
    let s2 = open_dim(&path, 4);
    let info = s2
        .list_collections()
        .into_iter()
        .find(|c| c.name == "code")
        .expect("code collection survives");
    assert_eq!(info.dim, 4);
    assert_eq!(info.model, "jina-code");
    assert_eq!(info.metric, "Cosine");
    assert_eq!(info.count, 1);
    let hits = search(&s2, vec![1.0, 0.0, 0.0, 0.0], 5, Some("code")).unwrap();
    assert_eq!(
        hits,
        vec!["c1".to_string()],
        "vector searchable after reload"
    );
}

/// Pure WAL replay (no snapshot) recovers a custom collection by inferring its
/// dim from the embedding, and routes the node's vector back into it.
#[test]
fn wal_replay_recovers_custom_collection() {
    let path = fresh("test_mc_wal");
    {
        let s = open_dim(&path, 4);
        s.create_collection(
            "code".to_string(),
            "jina-code".to_string(),
            4,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        add(&s, "c1", vec![0.0, 0.0, 1.0, 0.0], Some("code")).unwrap();
        // NOTE: no save_state() -> reopen must replay the WAL.
    }
    let s2 = open_dim(&path, 4);
    // Collection auto-provisioned during replay (dim inferred = 4).
    let info = s2
        .list_collections()
        .into_iter()
        .find(|c| c.name == "code")
        .expect("code recovered on replay");
    assert_eq!(info.dim, 4);
    let hits = search(&s2, vec![0.0, 0.0, 1.0, 0.0], 5, Some("code")).unwrap();
    assert_eq!(
        hits,
        vec!["c1".to_string()],
        "replayed vector searchable in recovered collection"
    );
}
