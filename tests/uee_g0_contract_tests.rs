use genesis_block_native::uee_v2::{
    CommitReceiptV2, QueryIrV2, QueryRequestV2, ReceiptStateV2, TransactionV2,
};
use serde_json::json;
use uuid::Uuid;

fn query_ir(nodes: serde_json::Value, root: &str) -> serde_json::Value {
    json!({
        "contract_version": "query-ir.v2",
        "nodes": nodes,
        "root": root
    })
}

#[test]
fn g0_query_request_requires_one_v2_source() {
    let hql = QueryRequestV2::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": "g0-query",
        "namespace": "default",
        "hql": "MATCH (n)",
        "language_version": "hql.v2",
        "params": {}
    }))
    .unwrap();
    assert_eq!(hql.namespace, "default");

    let ir = QueryRequestV2::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": "g0-ir",
        "namespace": "default",
        "ir": query_ir(json!([
            {
                "id": "nodes",
                "op": "NodeScan",
                "inputs": [],
                "config": {"as": "n"}
            }
        ]), "nodes"),
        "params": {}
    }))
    .unwrap();
    assert!(ir.hql.is_none());
    assert!(ir.ir.is_some());

    let both = QueryRequestV2::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": "g0-both",
        "namespace": "default",
        "hql": "MATCH (n)",
        "language_version": "hql.v2",
        "ir": query_ir(json!([
            {
                "id": "nodes",
                "op": "NodeScan",
                "inputs": [],
                "config": {"as": "n"}
            }
        ]), "nodes"),
        "params": {}
    }));
    assert!(both.is_err());
}

#[test]
fn g0_query_ir_rejects_unknown_inputs_and_cycles() {
    let unknown = QueryIrV2::from_value(query_ir(
        json!([{
            "id": "filter",
            "op": "Filter",
            "inputs": ["missing"],
            "config": {"predicate": {"literal": true, "type": "bool"}}
        }]),
        "filter",
    ));
    assert!(unknown.is_err());

    let cycle = QueryIrV2::from_value(query_ir(
        json!([
            {"id": "a", "op": "Filter", "inputs": ["b"], "config": {}},
            {"id": "b", "op": "Filter", "inputs": ["a"], "config": {}}
        ]),
        "a",
    ));
    assert!(cycle.is_err());
}

#[test]
fn g0_transaction_rejects_invalid_frontier_interval_and_vector() {
    let transaction = TransactionV2::from_value(json!({
        "contract_version": "genesis.tx.v2",
        "transaction_id": Uuid::new_v4(),
        "namespace": "default",
        "expected_frontier": "01",
        "mutations": [{
            "op": "vector.put",
            "owner_id": "node-1",
            "collection": "default",
            "space_id": "space-1",
            "values": [1.0, "not-a-number"]
        }]
    }));
    assert!(transaction.is_err());

    let interval = TransactionV2::from_value(json!({
        "contract_version": "genesis.tx.v2",
        "transaction_id": Uuid::new_v4(),
        "namespace": "default",
        "mutations": [{
            "op": "node.put",
            "id": "node-1",
            "labels": [],
            "props": {},
            "valid": {
                "from": "2026-09-22T02:00:00Z",
                "to": "2026-09-22T01:00:00Z"
            }
        }]
    }));
    assert!(interval.is_err());
}

#[test]
fn g0_transaction_rejects_unknown_fields_and_accepts_valid_mutation() {
    let unknown = TransactionV2::from_value(json!({
        "contract_version": "genesis.tx.v2",
        "transaction_id": Uuid::new_v4(),
        "namespace": "default",
        "mutations": [{
            "op": "node.put",
            "id": "node-1",
            "labels": [],
            "props": {},
            "not_in_contract": true
        }]
    }));
    assert!(unknown.is_err());

    let valid = TransactionV2::from_value(json!({
        "contract_version": "genesis.tx.v2",
        "transaction_id": Uuid::new_v4(),
        "namespace": "default",
        "mutations": [{
            "op": "node.put",
            "id": "node-1",
            "labels": ["Thing"],
            "props": {"name": "value"}
        }],
        "durability": "durable_published"
    }))
    .unwrap();
    assert_eq!(valid.namespace, "default");
}

#[test]
fn g0_receipt_validates_frontiers_without_claiming_commit_support() {
    let receipt = CommitReceiptV2::from_value(json!({
        "transaction_id": Uuid::new_v4(),
        "namespace": "default",
        "database_id": Uuid::new_v4(),
        "state": "durable_published",
        "durable_frontier": "12",
        "published_frontier": "12",
        "index_frontiers": {"default": "12"}
    }))
    .unwrap();
    assert_eq!(receipt.state, ReceiptStateV2::DurablePublished);

    let invalid = CommitReceiptV2::from_value(json!({
        "transaction_id": Uuid::new_v4(),
        "namespace": "default",
        "database_id": Uuid::new_v4(),
        "state": "durable_published",
        "durable_frontier": "0001"
    }));
    assert!(invalid.is_err());
}
