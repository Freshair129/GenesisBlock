use genesis_block_native::{IndexCoverageReport, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&path).exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(3),
        retention: None,
    })
    .unwrap()
}

fn add(storage: &Storage, id: &str, vector: Vec<f64>) {
    storage
        .add_node(NodeInput {
            id: Some(id.to_string()),
            labels: vec![],
            props: None,
            embedding: Some(vector),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn default_report(storage: &Storage) -> IndexCoverageReport {
    storage
        .list_collections()
        .into_iter()
        .find(|collection| collection.name == "default")
        .unwrap()
        .coverage
}

#[test]
fn coverage_requires_explicit_validation_and_tracks_frontiers() {
    let storage = open(&fresh("index_coverage_validation"));
    storage
        .create_collection(
            "empty".to_string(),
            "test".to_string(),
            3,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    add(&storage, "a", vec![1.0, 0.0, 0.0]);

    let before = default_report(&storage);
    assert!(!before.validated);
    assert_ne!(before.state, "READY");
    assert_eq!(before.source_count, 1);

    storage.flush_index();
    let after_flush = default_report(&storage);
    assert!(!after_flush.validated);
    assert_ne!(after_flush.state, "READY");

    let reports = storage.validate_index_coverage().unwrap();
    let report = reports
        .into_iter()
        .find(|coverage| coverage.collection == "default")
        .unwrap();
    assert_eq!(report.state, "READY", "coverage report: {report:?}");
    assert!(report.validated);
    assert_eq!(report.source_count, 1);
    assert_eq!(report.indexed_count, 1);
    assert_eq!(report.missing_count, 0);
    assert_eq!(report.extra_count, 0);
    assert!(report.source_frontier > 0);
    assert_eq!(report.built_frontier, report.source_frontier);

    let empty = storage
        .list_collections()
        .into_iter()
        .find(|collection| collection.name == "empty")
        .unwrap()
        .coverage;
    assert_eq!(empty.state, "READY");
    assert_eq!(empty.source_count, 0);

    let ready = default_report(&storage);
    assert_eq!(ready.state, "READY");
    assert!(ready.validated);

    add(&storage, "b", vec![0.0, 1.0, 0.0]);
    storage.flush_index();
    let stale = default_report(&storage);
    assert!(!stale.validated);
    assert_ne!(stale.state, "READY");
    assert_eq!(stale.source_count, 2);
}
