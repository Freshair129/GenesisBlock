#[path = "support/hql2_rank_reference.rs"]
mod reference;

use genesis_block_native::{
    query::hql2::{QueryOutcomeV2, QueryResultV2},
    uee_v2::QueryRequestV2,
    AccessContext, NodeInput, OpenOptions, Storage,
};
use reference::{
    knn as oracle_knn, Batch, Candidate, Document, Fixture, Lineage, Metric, OriginalVector, Scope,
    Snapshot, VectorRequest, Visibility,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
use tempfile::TempDir;

const VECTOR_FIXTURE: [(&str, Option<[f64; 2]>); 4] = [
    ("A", Some([0.1, 0.0])),
    ("B", Some([-0.1, 0.0])),
    ("C", Some([0.3, 0.0])),
    ("missing", None),
];

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn access() -> AccessContext {
    AccessContext {
        principal: "vector-oracle-reader".into(),
        namespace: "default".into(),
    }
}

fn add_node(storage: &Storage, id: &str, embedding: Option<Vec<f64>>) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: vec!["Document".into()],
            props: None,
            embedding,
            lang: Some("en".into()),
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: Some("default".into()),
        })
        .unwrap();
}

fn add_fixture_nodes(storage: &Storage, include_missing: bool) {
    for &(id, vector) in &VECTOR_FIXTURE {
        if vector.is_none() && !include_missing {
            continue;
        }
        add_node(storage, id, vector.map(|values| values.to_vec()));
    }
}

fn space_id(storage: &Storage) -> String {
    let collection = storage
        .list_collections()
        .into_iter()
        .find(|collection| collection.name == "default")
        .unwrap();
    let tuple = (
        collection.name,
        collection.model,
        u16::try_from(collection.dim).unwrap(),
        collection.metric,
        collection.quant,
    );
    let mut hasher = Sha256::new();
    hasher.update(b"genesis.hql2.vector-space.v1:");
    hasher.update(serde_json::to_vec(&tuple).unwrap());
    hex::encode(hasher.finalize())
}

fn vector_param(space_id: &str) -> Value {
    json!({
        "type":"vector",
        "space_id":space_id,
        "values":[0.0, 0.0]
    })
}

fn oracle(space_id: &str, top: usize, rerank: bool) -> Vec<(String, f64)> {
    let vectors = &VECTOR_FIXTURE;
    let documents = vectors
        .iter()
        .map(|(id, values)| Document {
            id: (*id).into(),
            revision: "r1".into(),
            visibility: Visibility {
                tx_from: 1,
                tx_to: None,
                valid_from: 0,
                valid_to: None,
                authorized: true,
            },
            original: values.as_ref().map(|values| OriginalVector {
                fingerprint: space_id.into(),
                values: values.to_vec(),
            }),
            text: String::new(),
            source_hash: format!("fixture-{id}"),
            analyzer_fingerprint: "oracle-analyzer-v1".into(),
            tokens: vec![],
        })
        .collect();
    let fixture = Fixture {
        space: reference::Space {
            fingerprint: space_id.into(),
            dimension: 2,
            metric: Metric::L2Squared,
        },
        analyzer_fingerprint: "oracle-analyzer-v1".into(),
        documents,
    };
    let input = Batch {
        snapshot: Snapshot {
            transaction: 10,
            valid: 5,
        },
        rows: vectors
            .iter()
            .map(|(id, _)| Candidate {
                row_key: (*id).into(),
                owner_id: (*id).into(),
                revision: "r1".into(),
                language: "en".into(),
                hit: None,
                source_ranks: BTreeMap::new(),
            })
            .collect(),
        lineage: Lineage {
            approximate_sources: vec![],
            scope: Scope::WholeInput,
        },
    };
    let request = VectorRequest {
        query: OriginalVector {
            fingerprint: space_id.into(),
            values: vec![0.0, 0.0],
        },
        top,
        source: "dense".into(),
    };
    let outcome = if rerank {
        let mut candidates = input;
        candidates
            .rows
            .retain(|candidate| candidate.owner_id != "missing");
        reference::rerank(candidates, &fixture, &request)
    } else {
        oracle_knn(input, &fixture, &request)
    };
    outcome
        .unwrap()
        .rows
        .into_iter()
        .map(|candidate| {
            (
                candidate.owner_id,
                candidate.hit.expect("KNN assigns a hit").value,
            )
        })
        .collect()
}

fn actual(result: &QueryResultV2) -> Vec<(String, f64)> {
    result
        .rows
        .iter()
        .map(|row| {
            let id = match &row["id"] {
                genesis_block_native::query::hql2::value::QueryValueV2::Utf8(id) => id.clone(),
                other => panic!("id must be Utf8, got {other:?}"),
            };
            let distance = match row["distance"] {
                genesis_block_native::query::hql2::value::QueryValueV2::F64(value) => value,
                ref other => panic!("distance must be F64, got {other:?}"),
            };
            (id, distance)
        })
        .collect()
}

fn assert_matches_oracle(actual: &[(String, f64)], expected: &[(String, f64)]) {
    assert_eq!(
        actual.iter().map(|(id, _)| id).collect::<Vec<_>>(),
        expected.iter().map(|(id, _)| id).collect::<Vec<_>>()
    );
    for ((_, actual), (_, expected)) in actual.iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "distance mismatch: {actual} != {expected}"
        );
    }
}

#[test]
fn exact_knn_hql_and_ir_match_the_independent_p7_rank_oracle() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_fixture_nodes(&storage, true);

    let space_id = space_id(&storage);
    let expected = oracle(&space_id, 4, false);
    let params = json!({"q":vector_param(&space_id)});

    let hql_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"exact-knn-oracle-hql",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS d |> KNN d IN default USING $q TOP 4 EXACT AS hit |> RETURN d.id AS id, hit.distance AS distance",
        "params":params
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql_request).unwrap() else {
        panic!("HQL KNN must execute")
    };

    let ir_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"exact-knn-oracle-ir",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"d","label":"Document"}},
                {"id":"rank","op":"Knn","inputs":["scan"],"config":{
                    "entity":"d",
                    "collection":"default",
                    "query":{"param":"q"},
                    "k":4,
                    "mode":"exact",
                    "as":"hit"
                }},
                {"id":"project","op":"Project","inputs":["rank"],"config":{"fields":[
                    {"as":"id","expression":{"field":{"alias":"d","path":["id"]}}},
                    {"as":"distance","expression":{"field":{"alias":"hit","path":["distance"]}}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{"q":format!("Vector<{space_id},2,f64>")}
        },
        "params":params
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir_request).unwrap() else {
        panic!("typed-IR KNN must execute")
    };

    assert_matches_oracle(&actual(&hql_result), &expected);
    assert_matches_oracle(&actual(&ir_result), &expected);
}

#[test]
fn original_rerank_hql_and_ir_match_the_independent_p7_rank_oracle() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_fixture_nodes(&storage, false);

    let space_id = space_id(&storage);
    let expected = oracle(&space_id, 2, true);
    let params = json!({"q":vector_param(&space_id)});

    let hql_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"original-rerank-oracle-hql",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS d |> RERANK d IN default USING $q TOP 2 EXACT AS fine |> RETURN d.id AS id, fine.distance AS distance",
        "params":params
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql_request).unwrap() else {
        panic!("HQL Original Rerank must execute")
    };

    let ir_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"original-rerank-oracle-ir",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"d","label":"Document"}},
                {"id":"rank","op":"Rerank","inputs":["scan"],"config":{
                    "entity":"d",
                    "collection":"default",
                    "query":{"param":"q"},
                    "k":2,
                    "fidelity":"original",
                    "as":"fine"
                }},
                {"id":"project","op":"Project","inputs":["rank"],"config":{"fields":[
                    {"as":"id","expression":{"field":{"alias":"d","path":["id"]}}},
                    {"as":"distance","expression":{"field":{"alias":"fine","path":["distance"]}}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{"q":format!("Vector<{space_id},2,f64>")}
        },
        "params":params
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir_request).unwrap() else {
        panic!("typed-IR Original Rerank must execute")
    };

    assert_matches_oracle(&actual(&hql_result), &expected);
    assert_matches_oracle(&actual(&ir_result), &expected);
}
