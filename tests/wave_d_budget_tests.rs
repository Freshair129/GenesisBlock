use genesis_block_native::{EdgeInput, NeighborInput, NodeInput, OpenOptions, Storage};
use serde_json::json;
use tempfile::tempdir;

fn storage() -> (Storage, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: None,
    })
    .unwrap();
    (storage, dir)
}

fn add_node(storage: &Storage, id: &str) {
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

#[test]
fn zero_neighbor_limit_is_rejected_before_traversal() {
    let (storage, _dir) = storage();
    add_node(&storage, "A");
    add_node(&storage, "B");
    storage
        .add_edge(EdgeInput {
            id: Some("ab".to_string()),
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
    let error = storage
        .neighbors(
            "A".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: None,
                limit: Some(0),
            },
            false,
        )
        .expect_err("zero limit must fail before traversal");
    assert!(error
        .to_string()
        .starts_with("QUERY_RESOURCE_LIMIT_EXCEEDED:"));
}

#[test]
fn query_budget_rejects_zero_before_work() {
    let (storage, _dir) = storage();
    let error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "budget-zero",
            "budget": { "max_expanded_nodes": 0 },
            "operation": {
                "kind": "traverse",
                "seed_id": "missing",
                "depth": 1,
                "relations": ["LINK"],
                "direction": "out"
            }
        }))
        .expect_err("zero budget must fail before touching the graph");
    assert!(error
        .to_string()
        .starts_with("QUERY_BUDGET_INVALID: max_expanded_nodes"));
}

#[test]
fn query_budget_stops_graph_expansion_with_typed_reason() {
    let (storage, _dir) = storage();
    add_node(&storage, "A");
    add_node(&storage, "B");
    add_node(&storage, "C");
    storage
        .add_edge(EdgeInput {
            id: Some("ab-budget".to_string()),
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
    storage
        .add_edge(EdgeInput {
            id: Some("ac-budget".to_string()),
            from: "A".to_string(),
            to: "C".to_string(),
            rel: "LINK".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    let error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "budget-nodes",
            "budget": { "max_expanded_nodes": 0 },
            "operation": {
                "kind": "traverse",
                "seed_id": "A",
                "depth": 1,
                "relations": ["LINK"],
                "direction": "out"
            }
        }))
        .expect_err("zero expansion budget must be rejected before traversal");
    assert!(error.to_string().starts_with("QUERY_BUDGET_INVALID:"));

    let error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "budget-nodes-live",
            "budget": { "max_expanded_nodes": 1 },
            "operation": {
                "kind": "traverse",
                "seed_id": "A",
                "depth": 1,
                "relations": ["LINK"],
                "direction": "out"
            }
        }))
        .expect_err("one-node budget must stop before the first neighbor row");
    assert_eq!(error.to_string(), "QUERY_BUDGET_EXCEEDED: reason=nodes");
}

#[test]
fn query_budget_reports_candidate_and_byte_exhaustion() {
    let (storage, _dir) = storage();
    let error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "budget-candidates",
            "budget": { "max_vector_candidates": 1 },
            "operation": {
                "kind": "search",
                "mode": "vector",
                "query_vector": [1.0, 0.0],
                "k": 2
            }
        }))
        .expect_err("candidate budget must be checked before ANN work");
    assert_eq!(
        error.to_string(),
        "QUERY_BUDGET_EXCEEDED: reason=candidates"
    );

    let error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "budget-bytes",
            "budget": { "max_serialized_bytes": 1 },
            "operation": {
                "kind": "traverse",
                "seed_id": "missing",
                "depth": 1,
                "relations": ["LINK"],
                "direction": "out"
            }
        }))
        .expect_err("response byte budget must be checked before returning");
    assert_eq!(error.to_string(), "QUERY_BUDGET_EXCEEDED: reason=bytes");
}

#[test]
fn hql_match_uses_the_same_budget_and_stops_dense_frontiers() {
    let (storage, _dir) = storage();
    for id in ["A", "B", "C"] {
        add_node(&storage, id);
    }
    for (edge_id, to) in [("ab-hql-budget", "B"), ("ac-hql-budget", "C")] {
        storage
            .add_edge(EdgeInput {
                id: Some(edge_id.to_string()),
                from: "A".to_string(),
                to: to.to_string(),
                rel: "LINK".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            })
            .unwrap();
    }

    let error = storage
        .execute_hql_with_budget(
            "MATCH (a {id:\"A\"})-[:LINK]->(b) RETURN b.id",
            Some(genesis_block_native::QueryBudget {
                max_result_rows: Some(1),
                ..Default::default()
            }),
        )
        .expect_err("MATCH must stop before materializing an unbounded frontier");
    assert_eq!(error.to_string(), "QUERY_BUDGET_EXCEEDED: reason=rows");
}
