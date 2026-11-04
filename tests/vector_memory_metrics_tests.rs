use genesis_block_native::{
    CollectionInfo, HybridSearchInput, NodeInput, OpenOptions, Storage, SIDECAR_CACHE_ROWS,
};
use serde_json::{json, Value};

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.into(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(8),
        retention: None,
    })
    .unwrap()
}

fn info(storage: &Storage) -> Value {
    serde_json::to_value(
        storage
            .list_collections()
            .into_iter()
            .find(|c| c.name == "test")
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn reports_cache_payload_and_arena_capacity_without_counting_disk_as_ram() {
    let dir = tempfile::tempdir().unwrap();
    let storage = open(dir.path().to_str().unwrap());
    storage
        .create_collection(
            "test".into(),
            "m".into(),
            8,
            None,
            Some("sq8".into()),
            None,
            Some(true),
        )
        .unwrap();
    assert_eq!(info(&storage)["sidecar_cache_bytes"], json!(0));
    for id in 0..5 {
        storage
            .add_node(NodeInput {
                id: Some(id.to_string()),
                labels: vec![],
                props: None,
                embedding: Some(vec![0.1 * id as f64; 8]),
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: Some("test".into()),
            })
            .unwrap();
    }
    storage.flush_index();
    let coll = storage.collections.get("test").unwrap();
    let sidecar = coll.f32_sidecar.as_ref().unwrap().read();
    assert!(sidecar.row(0).is_some());
    assert!(sidecar.row(1).is_some());
    assert!(sidecar.row(0).is_some()); // hits do not duplicate cache ownership
    assert!(sidecar.row(100).is_none()); // misses do not allocate retained payload
    drop(sidecar);
    drop(coll);
    let metrics = info(&storage);
    assert_eq!(metrics["sidecar_resident_bytes"], json!(0)); // legacy full-file residency
    assert_eq!(metrics["sidecar_cache_bytes"], json!(2 * 8 * 4));
    assert_eq!(metrics["sidecar_disk_bytes"], json!(5 * 8 * 4));
    assert_eq!(metrics["arena_resident_bytes"], json!(5 * 8)); // legacy logical payload
    assert!(metrics["arena_allocated_bytes"].as_u64().unwrap() >= 5 * 8);
    assert_eq!(metrics["hnsw_vector_bytes"], json!(5 * 8));
    let mut legacy = metrics;
    for key in [
        "sidecar_cache_bytes",
        "arena_allocated_bytes",
        "hnsw_vector_bytes",
    ] {
        legacy.as_object_mut().unwrap().remove(key);
    }
    let decoded: CollectionInfo = serde_json::from_value(legacy).unwrap();
    let decoded = serde_json::to_value(decoded).unwrap();
    assert_eq!(decoded["sidecar_cache_bytes"], json!(0));
}

#[test]
fn cache_metric_is_bounded_under_concurrent_reads_and_reopen_starts_cold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.bin");
    std::fs::write(&path, vec![0u8; (SIDECAR_CACHE_ROWS + 8) * 8]).unwrap();
    let reader = genesis_block_native::SidecarReader::new(std::fs::File::open(&path).unwrap(), 2);
    assert_eq!(reader.cache_bytes(), 0);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let reader = &reader;
            scope.spawn(move || {
                for i in 0..SIDECAR_CACHE_ROWS + 8 {
                    assert!(reader.row(i).is_some());
                }
            });
        }
    });
    assert_eq!(reader.cache_bytes(), SIDECAR_CACHE_ROWS * 2 * 4);
    let reopened = genesis_block_native::SidecarReader::new(std::fs::File::open(&path).unwrap(), 2);
    assert_eq!(reopened.cache_bytes(), 0);
}

#[test]
fn all_modes_report_distinct_arena_and_hnsw_widths_and_rebuild_search() {
    for (quant, arena_width, hnsw_width) in
        [("none", 4, 4), ("sq8", 1, 1), ("bq", 1, 1), ("f16", 2, 4)]
    {
        let dir = tempfile::tempdir().unwrap();
        let storage = open(dir.path().to_str().unwrap());
        storage
            .create_collection(
                "test".into(),
                "m".into(),
                8,
                Some("l2".into()),
                Some(quant.into()),
                None,
                Some(false),
            )
            .unwrap();
        storage
            .add_node(NodeInput {
                id: Some("one".into()),
                labels: vec![],
                props: None,
                embedding: Some(vec![-1.0, 0.5, -0.25, 0.125, -0.5, 1.0, 0.25, -0.125]),
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: Some("test".into()),
            })
            .unwrap();
        storage.flush_index();
        storage.rebuild_index_parallel().unwrap();
        assert_eq!(info(&storage)["hnsw_vector_bytes"], json!(8 * hnsw_width));
        assert_eq!(
            info(&storage)["arena_resident_bytes"],
            json!(8 * arena_width)
        );
        assert_eq!(
            storage
                .validate_index_coverage()
                .unwrap()
                .iter()
                .find(|c| c.collection == "test")
                .unwrap()
                .state,
            "READY"
        );
        let got = storage
            .hybrid_search(HybridSearchInput {
                query_vector: vec![-1.0, 0.5, -0.25, 0.125, -0.5, 1.0, 0.25, -0.125],
                k: 1,
                alpha: None,
                lang: None,
                as_of: None,
                collection: Some("test".into()),
                ef_search: None,
                oversample: None,
            })
            .unwrap();
        assert_eq!(got[0].node.id, "one");
    }
}

#[test]
fn compacting_the_last_vector_releases_the_old_index() {
    let dir = tempfile::tempdir().unwrap();
    let storage = open(dir.path().to_str().unwrap());
    storage
        .add_node(NodeInput {
            id: Some("last".into()),
            labels: vec![],
            props: None,
            embedding: Some(vec![0.25; 8]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    storage.flush_index();
    assert_eq!(storage.list_collections()[0].indexed, 1);
    storage.retract_node("last").unwrap();
    storage.perform_index_compaction().unwrap();
    let collection = storage
        .list_collections()
        .into_iter()
        .find(|c| c.name == "default")
        .unwrap();
    assert_eq!(collection.count, 0);
    assert_eq!(
        collection.indexed, 0,
        "empty rehydrate must drop the old graph"
    );
    assert_eq!(storage.validate_index_coverage().unwrap()[0].state, "READY");
}

#[test]
fn compaction_does_not_reserve_global_node_count_for_empty_collections() {
    let dir = tempfile::tempdir().unwrap();
    let storage = open(dir.path().to_str().unwrap());
    storage
        .create_collection("empty".into(), "m".into(), 8, None, None, None, None)
        .unwrap();
    for id in 0..128 {
        storage
            .add_node(NodeInput {
                id: Some(format!("node{id}")),
                labels: vec![],
                props: None,
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
    }
    storage.perform_index_compaction().unwrap();
    for collection in storage.collections.iter() {
        assert!(collection.metadata.read().is_empty());
        assert_eq!(
            collection.metadata.read().capacity(),
            0,
            "an empty collection must not reserve metadata for unrelated nodes"
        );
    }
}
