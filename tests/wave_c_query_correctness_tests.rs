use chrono::{Duration, Utc};
use genesis_block_native::{
    BatchInput, EdgeInput, HybridSearchInput, NodeInput, OpenOptions, Storage,
};
use tempfile::{tempdir, TempDir};

fn storage(dim: u32) -> (Storage, TempDir) {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(dim),
        retention: None,
    })
    .unwrap();
    (storage, dir)
}

fn node(id: &str, embedding: Vec<f64>, valid_from: Option<String>) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: Some(embedding),
        lang: None,
        valid_from,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

fn search(storage: &Storage, args: HybridSearchInput) -> Vec<String> {
    storage.flush_index();
    storage
        .hybrid_search(args)
        .unwrap()
        .into_iter()
        .map(|hit| hit.node.id)
        .collect()
}

fn collection_query(vector: Vec<f64>, k: u32, collection: &str) -> HybridSearchInput {
    HybridSearchInput {
        collection: Some(collection.to_string()),
        ..query(vector, k)
    }
}

fn query(vector: Vec<f64>, k: u32) -> HybridSearchInput {
    HybridSearchInput {
        query_vector: vector,
        k,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: None,
        ef_search: Some(100),
        oversample: None,
    }
}

#[test]
fn filtered_ann_refills_after_retired_shortlist() {
    let (storage, _dir) = storage(2);

    // Put the live tail in first so the nearest HNSW shortlist is made up of
    // the subsequently retired rows. The current implementation filters those
    // rows only after ANN and returns fewer than k.
    for i in 0..3 {
        storage
            .add_node(node(&format!("live-{i}"), vec![0.0, 10.0 + i as f64], None))
            .unwrap();
    }
    for i in 0..50 {
        storage
            .add_node(node(
                &format!("retired-{i}"),
                vec![1.0, 0.001 * i as f64],
                None,
            ))
            .unwrap();
    }
    for i in 0..50 {
        storage.retract_node(&format!("retired-{i}")).unwrap();
    }

    let ids = search(&storage, query(vec![1.0, 0.0], 3));
    assert_eq!(ids.len(), 3, "filtered ANN must refill to k live results");
    assert!(ids.iter().all(|id| id.starts_with("live-")));
}

#[test]
fn filtered_ann_refills_quantized_rerank_candidates() {
    let (storage, _dir) = storage(2);
    storage
        .create_collection(
            "sq8".to_string(),
            "test-model".to_string(),
            2,
            Some("L2".to_string()),
            Some("sq8".to_string()),
            Some(100),
            Some(true),
        )
        .unwrap();

    for i in 0..3 {
        let mut input = node(&format!("sq8-live-{i}"), vec![0.0, -1.0], None);
        input.collection = Some("sq8".to_string());
        storage.add_node(input).unwrap();
    }
    for i in 0..20 {
        let mut input = node(
            &format!("sq8-retired-{i}"),
            vec![1.0, 0.001 * i as f64],
            None,
        );
        input.collection = Some("sq8".to_string());
        storage.add_node(input).unwrap();
    }
    for i in 0..20 {
        storage.retract_node(&format!("sq8-retired-{i}")).unwrap();
    }

    let ids = search(
        &storage,
        HybridSearchInput {
            oversample: Some(2),
            ..collection_query(vec![1.0, 0.0], 3, "sq8")
        },
    );
    assert_eq!(
        ids.len(),
        3,
        "quantized rerank must refill to k live results"
    );
    assert!(ids.iter().all(|id| id.starts_with("sq8-live-")));
}

#[test]
fn temporal_search_normalizes_rfc3339_and_hides_future_nodes() {
    let (storage, _dir) = storage(2);
    storage
        .add_node(node(
            "offset-equivalent",
            vec![1.0, 0.0],
            Some("2025-01-01T07:00:00+07:00".to_string()),
        ))
        .unwrap();
    storage
        .add_node(node(
            "future",
            vec![0.9, 0.1],
            Some((Utc::now() + Duration::hours(1)).to_rfc3339()),
        ))
        .unwrap();
    let mut expired = node("expired", vec![0.8, 0.2], None);
    expired.ttl = Some(0);
    storage.add_node(expired).unwrap();

    let as_of = HybridSearchInput {
        query_vector: vec![1.0, 0.0],
        k: 2,
        alpha: Some(0.0),
        lang: None,
        as_of: Some("2025-01-01T00:00:00Z".to_string()),
        collection: None,
        ef_search: Some(100),
        oversample: None,
    };
    let historical = search(&storage, as_of);
    assert!(historical.contains(&"offset-equivalent".to_string()));

    let current = search(&storage, query(vec![1.0, 0.0], 2));
    assert!(!current.contains(&"future".to_string()));
    assert!(!current.contains(&"expired".to_string()));
}

#[test]
fn grl_hides_retracted_edges_and_endpoints() {
    let (storage, _dir) = storage(2);
    storage
        .add_node(NodeInput {
            id: Some("A".to_string()),
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
    storage
        .add_node(NodeInput {
            id: Some("B".to_string()),
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
    storage
        .add_edge(EdgeInput {
            id: Some("E".to_string()),
            from: "A".to_string(),
            to: "B".to_string(),
            rel: "LINK".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    storage.retract_edge("E".to_string(), None).unwrap();
    let context = storage.retrieve_context("A", "H1", None, false).unwrap();
    assert!(context.edges.is_empty(), "GRL must hide retracted edges");
    assert!(
        context.nodes.iter().all(|node| node.id != "B"),
        "GRL must not reach a node through a retracted edge"
    );
}

#[test]
fn direct_vector_and_query_controls_fail_closed() {
    let (storage, _dir) = storage(2);
    let before = storage.stable_frontier();
    let error = storage.add_node(node("nan", vec![f64::NAN, 0.0], None));
    assert!(error.is_err());
    assert!(storage.node_view("nan").is_none());
    assert_eq!(storage.stable_frontier(), before);

    storage
        .add_node(node("valid", vec![1.0, 0.0], None))
        .unwrap();
    storage
        .create_collection(
            "code".to_string(),
            "test-model".to_string(),
            2,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    let before_secondary = storage.stable_frontier();
    assert!(storage
        .add_vector("valid".to_string(), "code".to_string(), vec![f64::NAN, 0.0])
        .is_err());
    assert_eq!(storage.stable_frontier(), before_secondary);
    storage.flush_index();

    let invalid_queries = [
        HybridSearchInput {
            query_vector: vec![f64::NAN, 0.0],
            ..query(vec![1.0, 0.0], 1)
        },
        query(vec![1.0, 0.0], 0),
        HybridSearchInput {
            alpha: Some(f64::NAN),
            ..query(vec![1.0, 0.0], 1)
        },
        HybridSearchInput {
            ef_search: Some(0),
            ..query(vec![1.0, 0.0], 1)
        },
        HybridSearchInput {
            oversample: Some(0),
            ..query(vec![1.0, 0.0], 1)
        },
        HybridSearchInput {
            as_of: Some("tomorrow".to_string()),
            ..query(vec![1.0, 0.0], 1)
        },
    ];
    for input in invalid_queries {
        assert!(storage.hybrid_search(input).is_err());
    }
}

#[test]
fn batch_edge_preserves_valid_from() {
    let (storage, _dir) = storage(2);
    for id in ["A", "B"] {
        storage
            .add_node(NodeInput {
                id: Some(id.to_string()),
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
    let valid_from = "2020-01-02T03:04:05+07:00".to_string();
    let output = storage
        .execute_batch(BatchInput {
            nodes: vec![],
            edges: vec![EdgeInput {
                id: Some("batch-edge".to_string()),
                from: "A".to_string(),
                to: "B".to_string(),
                rel: "LINK".to_string(),
                props: None,
                valid_from: Some(valid_from.clone()),
                supersede: None,
                impact: None,
                caused_by: None,
            }],
        })
        .unwrap();

    assert_eq!(output.edges[0].valid_from, valid_from);
    let context = storage.retrieve_context("A", "H1", None, false).unwrap();
    assert_eq!(context.edges[0].valid_from, "2020-01-02T03:04:05+07:00");
}
