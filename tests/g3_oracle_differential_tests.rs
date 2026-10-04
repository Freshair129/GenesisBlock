use genesis_block_native::{
    BatchInput, EdgeInput, GenesisTransaction, NodeInput, OpenOptions, RelationalMutationGroup,
    RelationalMutationKind, RelationalQuery, RelationalRowMutation, RelationalSchemaPackage,
    Storage,
};
use serde_json::{json, Value};
use std::{collections::HashMap, fs, path::Path};
use tempfile::TempDir;

const FIXTURES: &str = include_str!("fixtures/g3_oracle_cases.json");

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn case(id: &str) -> Value {
    let document: Value = serde_json::from_str(FIXTURES).unwrap();
    document["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["id"] == id)
        .cloned()
        .unwrap()
}

fn remove_materialized_state(path: &Path) {
    for name in [
        "state.json",
        "nodes.bin",
        "edges.bin",
        "edges_retired.bin",
        "projection.sqlite",
        "projection.sqlite-wal",
        "projection.sqlite-shm",
        "vec_default.bin",
        "meta_default.bin",
        "fvec_default.bin",
        "bqmean_default.bin",
        "sq8scale_default.bin",
    ] {
        let _ = fs::remove_file(path.join(name));
    }
}

fn add_fixture_node(storage: &Storage, operation: &Value) {
    storage
        .add_node(NodeInput {
            id: Some(operation["id"].as_str().unwrap().into()),
            labels: vec!["G3".into()],
            props: Some(operation["props"].clone()),
            embedding: None,
            lang: Some("en".into()),
            valid_from: Some(operation["valid_from"].as_str().unwrap().into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn canonical_temporal(response: Value) -> Value {
    Value::Array(
        response["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                json!({
                    "node_id": row["node"]["id"],
                    "props": row["node"]["props"],
                    "valid_window": if row["node"]["valid_to"].is_null() { "open" } else { "closed" },
                })
            })
            .collect(),
    )
}

fn query_temporal(storage: &Storage, query: &Value, markers: &HashMap<String, u64>) -> Value {
    let tx = match query["tx"].as_str().unwrap() {
        "latest" => None,
        marker => Some(*markers.get(marker).unwrap()),
    };
    let valid_at = query["valid_at"].as_str();
    let request = json!({
        "contract_version": "query-ir.v1",
        "request_id": query["id"],
        "operation": {
            "kind": "traverse",
            "seed_id": "hub",
            "depth": 1,
            "relations": ["KNOWS"],
            "direction": "out"
        }
    });
    let request = if valid_at.is_some() || tx.is_some() {
        let mut request = request;
        request["temporal"] = json!({"valid_at": valid_at, "tx_as_of": tx});
        request
    } else {
        request
    };
    canonical_temporal(storage.execute_query_ir_json(request).unwrap())
}

fn run_temporal_case(case: &Value, path: &Path) -> Value {
    let storage = open(path);
    let mut markers = HashMap::new();
    for operation in case["operations"].as_array().unwrap() {
        match operation["op"].as_str().unwrap() {
            "add_node" => add_fixture_node(&storage, operation),
            "add_edge" => {
                storage
                    .add_edge(EdgeInput {
                        id: Some(operation["id"].as_str().unwrap().into()),
                        from: operation["from"].as_str().unwrap().into(),
                        to: operation["to"].as_str().unwrap().into(),
                        rel: operation["rel"].as_str().unwrap().into(),
                        props: None,
                        valid_from: Some(operation["valid_from"].as_str().unwrap().into()),
                        supersede: None,
                        impact: None,
                        caused_by: None,
                    })
                    .unwrap();
            }
            "mark" => {
                markers.insert(
                    operation["name"].as_str().unwrap().to_string(),
                    storage.stable_frontier(),
                );
            }
            "supersede" => {
                storage
                    .supersede_node(
                        operation["id"].as_str().unwrap().into(),
                        Some(operation["props"].clone()),
                        None,
                    )
                    .unwrap();
            }
            other => panic!("unknown temporal fixture operation {other}"),
        }
    }

    let mut observed = serde_json::Map::new();
    for query in case["queries"].as_array().unwrap() {
        observed.insert(
            query["id"].as_str().unwrap().into(),
            query_temporal(&storage, query, &markers),
        );
    }
    let before_reopen = Value::Object(observed.clone());
    storage.save_state().unwrap();
    drop(storage);
    remove_materialized_state(path);

    let reopened = open(path);
    let mut after_reopen = serde_json::Map::new();
    for query in case["queries"].as_array().unwrap() {
        after_reopen.insert(
            query["id"].as_str().unwrap().into(),
            query_temporal(&reopened, query, &markers),
        );
    }
    assert_eq!(before_reopen, Value::Object(after_reopen.clone()));
    Value::Object(after_reopen)
}

fn relational_schema(case: &Value) -> RelationalSchemaPackage {
    serde_json::from_value(case["schema"].clone()).unwrap()
}

fn run_relational_case(case: &Value, path: &Path) -> Vec<Value> {
    let storage = open(path);
    storage
        .register_relational_schema(relational_schema(case))
        .unwrap();

    let mut groups = Vec::new();
    for table in ["lefts", "rights"] {
        let mutations = case["rows"][table]
            .as_array()
            .unwrap()
            .iter()
            .map(|values| RelationalRowMutation {
                table: table.into(),
                kind: RelationalMutationKind::Insert,
                values: values.clone(),
                key: None,
            })
            .collect();
        groups.push(RelationalMutationGroup {
            namespace: "default".into(),
            mutations,
        });
    }
    storage
        .commit_transaction(GenesisTransaction {
            transaction_id: "g3-relational-fixture".into(),
            expected_frontier: None,
            relational: groups,
            graph: BatchInput {
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            vectors: Vec::new(),
        })
        .unwrap();
    let query: RelationalQuery = serde_json::from_value(case["query"].clone()).unwrap();
    let before_reopen = storage.query_relational(query.clone()).unwrap();
    storage.save_state().unwrap();
    drop(storage);
    remove_materialized_state(path);

    let reopened = open(path);
    let after_reopen = reopened.query_relational(query).unwrap();
    assert_eq!(before_reopen, after_reopen);
    after_reopen
}

#[test]
fn g3_oracle_matches_public_storage_before_and_after_wal_reopen() {
    let fixture = case("temporal_two_axis");
    let dir = TempDir::new().unwrap();
    let observed = run_temporal_case(&fixture, dir.path());
    assert_eq!(observed, fixture["expected"]);
}

#[test]
fn g3_oracle_preserves_relational_null_and_bag_semantics_after_reopen() {
    let fixture = case("relational_null_bag_left_join");
    let dir = TempDir::new().unwrap();
    let observed = run_relational_case(&fixture, dir.path());
    assert_eq!(Value::Array(observed), fixture["expected"]);
}
