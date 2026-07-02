// Per-collection default HNSW `ef_search` (MARK XIV P3). A collection may carry
// its own default ef; resolution order in `hybrid_search` is per-query override →
// per-collection default → engine-global. These tests pin DETERMINISTIC invariants
// only — HNSW is approximate, so recall-vs-ef behaviour is validated by benchmark
// (the Recall@500k frontier), not here.
//
// Guarded: (a) the default is exposed on CollectionInfo and is None when unset;
// (b) it survives save_state + reopen (manifest round-trip), and an absent field
// loads as None (back-compat); (c) a collection that carries a default still
// returns its exact match (the resolution chain is wired and search works);
// (d) a per-query override still wins over the collection default.

use genesis_block_native::{CollectionInfo, HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(4),
    })
    .unwrap()
}

fn add(s: &Storage, id: &str, emb: Vec<f64>, coll: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: Some(emb),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: Some(coll.to_string()),
    })
    .unwrap();
}

fn info(s: &Storage, name: &str) -> CollectionInfo {
    s.list_collections()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("collection '{}' not found", name))
}

fn top1(s: &Storage, q: Vec<f64>, coll: &str, ef: Option<u32>) -> Option<String> {
    s.flush_index();
    s.hybrid_search(HybridSearchInput {
        query_vector: q,
        k: 1,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: Some(coll.to_string()),
        ef_search: ef,
        oversample: None,
    })
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .next()
}

fn seed(s: &Storage, coll: &str) {
    add(s, "A", vec![1.0, 0.0, 0.0, 0.0], coll);
    add(s, "B", vec![0.0, 1.0, 0.0, 0.0], coll);
    add(s, "C", vec![0.0, 0.0, 1.0, 0.0], coll);
    add(s, "D", vec![0.0, 0.0, 0.0, 1.0], coll);
    add(s, "E", vec![0.9, 0.1, 0.0, 0.0], coll);
}

/// CollectionInfo exposes the per-collection ef default; it is `Some` when set at
/// creation and `None` when omitted.
#[test]
fn info_exposes_ef_default() {
    let s = open(&fresh("test_cef_info"));
    s.create_collection("withef".into(), "m".into(), 4, None, None, Some(77), None)
        .unwrap();
    s.create_collection("noef".into(), "m".into(), 4, None, None, None, None)
        .unwrap();
    assert_eq!(info(&s, "withef").ef_search, Some(77));
    assert_eq!(info(&s, "noef").ef_search, None);
}

/// The per-collection default survives save_state + reopen (manifest round-trip).
/// The `None` collection round-trips to `None` — the same code path an old
/// snapshot (manifest with no `ef_search` field) takes on load.
#[test]
fn ef_default_survives_reload() {
    let path = fresh("test_cef_reload");
    {
        let s = open(&path);
        s.create_collection("withef".into(), "m".into(), 4, None, None, Some(77), None)
            .unwrap();
        s.create_collection("noef".into(), "m".into(), 4, None, None, None, None)
            .unwrap();
        seed(&s, "withef");
        s.flush_index();
        s.save_state().unwrap();
    }
    let s = open(&path); // same path, no fresh() -> instant-load from snapshot
    assert_eq!(
        info(&s, "withef").ef_search,
        Some(77),
        "ef default persisted across reopen"
    );
    assert_eq!(
        info(&s, "noef").ef_search,
        None,
        "absent ef_search loads as None"
    );
}

/// A collection that carries a default still returns its exact match — the
/// resolution chain (per-query None -> collection default) is wired and the value
/// is honoured by HNSW search without breaking it.
#[test]
fn collection_default_search_finds_exact() {
    let s = open(&fresh("test_cef_search"));
    s.create_collection("withef".into(), "m".into(), 4, None, None, Some(64), None)
        .unwrap();
    seed(&s, "withef");
    // per-query ef = None -> falls through to the collection's default (64)
    assert_eq!(
        top1(&s, vec![1.0, 0.0, 0.0, 0.0], "withef", None).as_deref(),
        Some("A")
    );
    assert_eq!(
        top1(&s, vec![0.0, 0.0, 1.0, 0.0], "withef", None).as_deref(),
        Some("C")
    );
}

/// A per-query override still wins over the collection default.
#[test]
fn per_query_override_beats_collection_default() {
    let s = open(&fresh("test_cef_override"));
    // Collection default is a tiny ef; the per-query override raises it.
    s.create_collection("withef".into(), "m".into(), 4, None, None, Some(1), None)
        .unwrap();
    seed(&s, "withef");
    assert_eq!(
        top1(&s, vec![1.0, 0.0, 0.0, 0.0], "withef", Some(64)).as_deref(),
        Some("A")
    );
}
