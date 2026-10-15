use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use serde_json::json;
use tempfile::TempDir;

fn storage(dim: u32) -> (Storage, TempDir) {
    let dir = TempDir::new().unwrap();
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

fn node(id: &str, embedding: Option<Vec<f64>>) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

#[test]
fn query_ir_search_returns_versioned_envelope() {
    let (storage, _dir) = storage(3);
    storage
        .add_node(node("query-ir-nearest", Some(vec![1.0, 0.0, 0.0])))
        .unwrap();
    storage.flush_index();

    let response = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-search-1",
            "operation": {
                "kind": "search",
                "mode": "vector",
                "query_vector": [0.9, 0.1, 0.0],
                "k": 1
            }
        }))
        .unwrap();

    assert_eq!(response["contract_version"], "query-ir.v1");
    assert_eq!(response["request_id"], "req-search-1");
    assert_eq!(response["operation_kind"], "search");
    assert_eq!(response["data"][0]["node"]["id"], "query-ir-nearest");
}

#[test]
fn query_ir_traverse_returns_neighbor() {
    let (storage, _dir) = storage(3);
    storage.add_node(node("query-ir-src", None)).unwrap();
    storage.add_node(node("query-ir-dst", None)).unwrap();
    storage
        .add_edge(EdgeInput {
            id: Some("query-ir-edge".to_string()),
            from: "query-ir-src".to_string(),
            to: "query-ir-dst".to_string(),
            rel: "KNOWS".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    let response = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-traverse-1",
            "operation": {
                "kind": "traverse",
                "seed_id": "query-ir-src",
                "depth": 1,
                "relations": ["KNOWS"],
                "direction": "out"
            }
        }))
        .unwrap();

    assert_eq!(response["operation_kind"], "traverse");
    assert_eq!(response["data"][0]["node"]["id"], "query-ir-dst");
}

#[test]
fn hql_and_query_ir_match_path_preserve_typed_path_results() {
    let (storage, _dir) = storage(3);
    storage.add_node(node("query-ir-path-src", None)).unwrap();
    storage.add_node(node("query-ir-path-dst", None)).unwrap();
    storage
        .add_edge(EdgeInput {
            id: Some("query-ir-path-edge".to_string()),
            from: "query-ir-path-src".to_string(),
            to: "query-ir-path-dst".to_string(),
            rel: "KNOWS".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    let hql = storage
        .execute_hql(
            r#"MATCH (a {id:"query-ir-path-src"})-[r:KNOWS]->(b) RETURN a.id, r.label, b.id"#,
        )
        .unwrap();
    let ir = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "query-ir-match-path",
            "operation": {
                "kind": "match_path",
                "pattern": {
                    "start": {
                        "var": "a",
                        "label": null,
                        "props": [{"key": "id", "value": "query-ir-path-src"}]
                    },
                    "hops": [{
                        "edge": {"var": "r", "rel_type": "KNOWS", "direction": "out"},
                        "node": {"var": "b", "label": null, "props": []}
                    }]
                },
                "limit": 10,
                "return": {
                    "kind": "fields",
                    "fields": [
                        {"var": "a", "field": "id"},
                        {"var": "r", "field": "label"},
                        {"var": "b", "field": "id"}
                    ]
                }
            }
        }))
        .unwrap();

    assert_eq!(ir["operation_kind"], "match_path");
    assert_eq!(hql, ir["data"]);
    assert_eq!(ir["data"][0]["a.id"], "query-ir-path-src");
    assert_eq!(ir["data"][0]["r.label"], "KNOWS");
    assert_eq!(ir["data"][0]["b.id"], "query-ir-path-dst");
}

#[test]
fn query_ir_match_path_supports_clauses_and_rejects_tx_time_selectors() {
    let (storage, _dir) = storage(3);
    storage.add_node(node("query-ir-clause-src", None)).unwrap();
    storage
        .add_node(NodeInput {
            props: Some(json!({"n": 1})),
            ..node("query-ir-clause-low", None)
        })
        .unwrap();
    storage
        .add_node(NodeInput {
            props: Some(json!({"n": 3})),
            ..node("query-ir-clause-high", None)
        })
        .unwrap();
    for (id, target) in [
        ("query-ir-clause-edge-low", "query-ir-clause-low"),
        ("query-ir-clause-edge-high", "query-ir-clause-high"),
    ] {
        storage
            .add_edge(EdgeInput {
                id: Some(id.to_string()),
                from: "query-ir-clause-src".to_string(),
                to: target.to_string(),
                rel: "KNOWS".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            })
            .unwrap();
    }

    let response = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "query-ir-match-path-clauses",
            "operation": {
                "kind": "match_path",
                "pattern": {
                    "start": {"props": [{"key": "id", "value": "query-ir-clause-src"}]},
                    "hops": [{
                        "edge": {"rel_type": "KNOWS", "direction": "out"},
                        "node": {"var": "b"}
                    }]
                },
                "where": [{
                    "field": {"var": "b", "field": "prop.n"},
                    "op": "gt",
                    "value": 1
                }],
                "order_by": {
                    "field": {"var": "b", "field": "prop.n"},
                    "descending": true
                },
                "limit": 1,
                "return": {
                    "kind": "fields",
                    "fields": [{"var": "b", "field": "id"}, {"var": "b", "field": "prop.n"}]
                }
            }
        }))
        .unwrap();
    assert_eq!(response["data"].as_array().unwrap().len(), 1);
    assert_eq!(response["data"][0]["b.id"], "query-ir-clause-high");
    assert_eq!(response["data"][0]["b.n"], 3);

    let tx_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "query-ir-match-path-tx",
            "temporal": {"tx_as_of": 0},
            "operation": {
                "kind": "match_path",
                "pattern": {"start": {"var": "n"}},
                "limit": 1
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(tx_error.starts_with("QUERY_CAPABILITY_UNSUPPORTED:"));
}

#[test]
fn query_ir_search_can_use_target_node_embedding() {
    let (storage, _dir) = storage(3);
    storage
        .add_node(node("query-ir-target", Some(vec![1.0, 0.0, 0.0])))
        .unwrap();
    storage
        .add_node(node("query-ir-other", Some(vec![0.0, 1.0, 0.0])))
        .unwrap();
    storage.flush_index();

    let response = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-target-search",
            "operation": {
                "kind": "search",
                "mode": "vector",
                "target_id": "query-ir-target",
                "k": 1
            }
        }))
        .unwrap();

    assert_eq!(response["data"][0]["node"]["id"], "query-ir-target");
}

#[test]
fn query_ir_rejects_unknown_version_and_fields() {
    let (storage, _dir) = storage(3);

    let version_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v2",
            "request_id": "req-version",
            "operation": {
                "kind": "traverse",
                "seed_id": "missing",
                "depth": 1,
                "relations": ["KNOWS"],
                "direction": "out"
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(version_error.starts_with("QUERY_IR_VERSION_UNSUPPORTED:"));

    let validation_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-unknown",
            "unexpected": true,
            "operation": {
                "kind": "traverse",
                "seed_id": "missing",
                "depth": 1,
                "relations": ["KNOWS"],
                "direction": "out"
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(validation_error.starts_with("QUERY_IR_VALIDATION_FAILED:"));
}

#[test]
fn query_ir_rejects_unsupported_filter_and_lexical_modes() {
    let (storage, _dir) = storage(3);

    let filter_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-filter",
            "operation": {
                "kind": "search",
                "mode": "vector",
                "query_vector": [1.0, 0.0, 0.0],
                "filters": {"label": "ENTITY"},
                "k": 1
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(filter_error.starts_with("QUERY_CAPABILITY_UNSUPPORTED:"));

    let lexical_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "req-lexical",
            "operation": {
                "kind": "search",
                "mode": "lexical",
                "query_vector": [1.0, 0.0, 0.0],
                "k": 1
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(lexical_error.starts_with("QUERY_CAPABILITY_UNSUPPORTED:"));
}

#[test]
fn hql_and_query_ir_preserve_search_hybrid_and_traverse_results() {
    let (storage, _dir) = storage(3);
    storage
        .add_node(node("parity-src", Some(vec![1.0, 0.0, 0.0])))
        .unwrap();
    storage
        .add_node(node("parity-dst", Some(vec![0.0, 1.0, 0.0])))
        .unwrap();
    storage
        .add_edge(EdgeInput {
            id: Some("parity-edge".to_string()),
            from: "parity-src".to_string(),
            to: "parity-dst".to_string(),
            rel: "KNOWS".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
    storage.flush_index();

    let hql_search = storage
        .execute_hql("SEARCH parity-src SIMILAR TO [1,0,0] K 1")
        .unwrap();
    let ir_search = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "parity-search",
            "operation": {
                "kind": "search",
                "mode": "vector",
                "query_vector": [1.0, 0.0, 0.0],
                "k": 1
            }
        }))
        .unwrap();
    assert_eq!(
        hql_search[0]["node"]["id"],
        ir_search["data"][0]["node"]["id"]
    );

    let hql_hybrid = storage
        .execute_hql("MATCH parity-src SIMILAR TO [1,0,0] ALPHA 0.5 K 1")
        .unwrap();
    let ir_hybrid = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "parity-hybrid",
            "operation": {
                "kind": "search",
                "mode": "hybrid",
                "query_vector": [1.0, 0.0, 0.0],
                "alpha": 0.5,
                "k": 1
            }
        }))
        .unwrap();
    assert_eq!(
        hql_hybrid[0]["node"]["id"],
        ir_hybrid["data"][0]["node"]["id"]
    );

    let hql_traverse = storage
        .execute_hql("TRAVERSE FROM parity-src DEPTH 1 REL KNOWS")
        .unwrap();
    let ir_traverse = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "parity-traverse",
            "operation": {
                "kind": "traverse",
                "seed_id": "parity-src",
                "depth": 1,
                "relations": ["KNOWS"],
                "direction": "out"
            }
        }))
        .unwrap();
    assert_eq!(
        hql_traverse[0]["node"]["id"],
        ir_traverse["data"][0]["node"]["id"]
    );
}

#[test]
fn hql_and_query_ir_reject_zero_k_with_the_same_resource_error() {
    let (storage, _dir) = storage(3);
    storage
        .add_node(node("zero-k-target", Some(vec![1.0, 0.0, 0.0])))
        .unwrap();
    storage.flush_index();

    let hql_error = storage
        .execute_hql("SEARCH zero-k-target SIMILAR TO [1,0,0] K 0")
        .unwrap_err()
        .to_string();
    let hybrid_error = storage
        .execute_hql("MATCH zero-k-target SIMILAR TO [1,0,0] ALPHA 0.5 K 0")
        .unwrap_err()
        .to_string();
    let ir_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "zero-k-parity",
            "operation": {
                "kind": "search",
                "mode": "vector",
                "query_vector": [1.0, 0.0, 0.0],
                "k": 0
            }
        }))
        .unwrap_err()
        .to_string();

    assert_eq!(hql_error, ir_error);
    assert_eq!(hybrid_error, ir_error);
}

#[test]
fn hql_and_query_ir_reject_missing_context_target_with_the_same_error() {
    let (storage, _dir) = storage(3);
    let hql_error = storage
        .execute_hql("CONTEXT FOR missing-context-target TIER H0")
        .expect_err("HQL context must reject a missing target")
        .to_string();
    let ir_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "missing-context-parity",
            "operation": {
                "kind": "context",
                "target_id": "missing-context-target",
                "tier": "H0"
            }
        }))
        .expect_err("Query IR context must reject a missing target")
        .to_string();

    assert_eq!(hql_error, ir_error);
}

#[test]
fn hql_and_query_ir_preserve_context_result() {
    let (storage, _dir) = storage(3);
    storage
        .add_node(NodeInput {
            props: Some(json!({"text": "context parity"})),
            ..node("context-parity-target", None)
        })
        .unwrap();

    let hql_context = storage
        .execute_hql("CONTEXT FOR context-parity-target TIER H0")
        .unwrap();
    let ir_context = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "context-parity",
            "operation": {
                "kind": "context",
                "target_id": "context-parity-target",
                "tier": "H0"
            }
        }))
        .unwrap();

    assert_eq!(hql_context, ir_context["data"]);
}
