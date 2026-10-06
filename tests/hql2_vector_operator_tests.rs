use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::{ExplainV2, QueryRequestV2},
    AccessContext, NodeInput, OpenOptions, Storage,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use tempfile::TempDir;

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
        principal: "vector-reader".into(),
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

fn space_id(storage: &Storage) -> String {
    space_id_for(storage, "default")
}

fn space_id_for(storage: &Storage, name: &str) -> String {
    let collection = storage
        .list_collections()
        .into_iter()
        .find(|collection| collection.name == name)
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

fn ir_request(request_id: &str, space_id: &str, rank_op: Value) -> QueryRequestV2 {
    let rank = rank_op;
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"d","label":"Document"}},
                rank,
                {"id":"project","op":"Project","inputs":["rank"],"config":{"fields":[
                    {"as":"id","expression":{"field":{"alias":"d","path":["id"]}}},
                    {"as":"distance","expression":{"field":{"alias":"hit","path":["distance"]}}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{"q":format!("Vector<{space_id},2,f64>")}
        },
        "params":{"q":vector_param(space_id)}
    }))
    .unwrap()
}

#[test]
fn exact_knn_reads_original_vectors_and_drops_missing_candidates() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "a", Some(vec![0.1, 0.0]));
    add_node(&storage, "b", Some(vec![0.2, 0.0]));
    add_node(&storage, "missing", None);
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":2,"mode":"exact","as":"hit"}
    });
    let request = ir_request("exact-knn", &space_id, rank);

    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("KNN must execute")
    };
    assert_eq!(
        result
            .rows
            .iter()
            .map(|row| row["id"].clone())
            .collect::<Vec<_>>(),
        [
            QueryValueV2::Utf8("a".into()),
            QueryValueV2::Utf8("b".into())
        ]
    );
    let QueryValueV2::F64(first) = result.rows[0]["distance"] else {
        panic!("distance must be a finite float")
    };
    let QueryValueV2::F64(second) = result.rows[1]["distance"] else {
        panic!("distance must be a finite float")
    };
    assert!((first - 0.01).abs() < 1e-12);
    assert!((second - 0.04).abs() < 1e-12);
    assert_eq!(
        result.semantics.candidate_search,
        genesis_block_native::query::hql2::result::CandidateSearchV2::Exact
    );
    assert_eq!(
        result.semantics.distance_fidelity,
        genesis_block_native::query::hql2::result::DistanceFidelityV2::Original
    );
    assert_eq!(
        result.semantics.scope,
        genesis_block_native::query::hql2::result::ResultScopeV2::WholeInput
    );
}

#[test]
fn analyze_reports_exact_vector_distance_evaluations_and_budget_errors() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "a", Some(vec![0.1, 0.0]));
    add_node(&storage, "b", Some(vec![0.2, 0.0]));
    add_node(&storage, "missing", None);
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":2,"mode":"exact","as":"hit"}
    });

    let mut request = ir_request("analyze-exact-knn", &space_id, rank.clone());
    request.explain = Some(ExplainV2::Analyze);
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("ANALYZE KNN must execute")
    };
    assert_eq!(result.rows.len(), 2);
    let rank_node = result
        .explain
        .unwrap()
        .plan
        .into_iter()
        .find(|node| node.id == "rank")
        .unwrap();
    let actual = serde_json::to_value(rank_node.actual.unwrap()).unwrap();
    assert_eq!(
        actual["distance_evaluations"]["unit"],
        "distance_evaluations"
    );
    assert_eq!(
        actual["distance_evaluations"]["measurement"]["status"],
        "measured"
    );
    assert_eq!(actual["distance_evaluations"]["measurement"]["value"], 2);

    let mut limited =
        serde_json::to_value(ir_request("analyze-exact-knn-budget", &space_id, rank)).unwrap();
    limited["explain"] = json!("analyze");
    limited["budget"] = json!({"max_distance_evaluations":1});
    let limited: QueryRequestV2 = serde_json::from_value(limited).unwrap();
    assert_eq!(
        storage.query_v2(access(), limited).unwrap_err().code,
        "QUERY_BUDGET_EXCEEDED"
    );
}

#[test]
fn exact_knn_fails_closed_below_the_selected_vector_history_floor() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "a", None);
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let selected_tx: i64 = projection
        .query_row(
            "SELECT tx_from FROM hql2_record_revisions
             WHERE kind='node' AND record_id='a' AND operation='upsert'
             ORDER BY tx_from LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    storage
        .add_vector("a".into(), "default".into(), vec![0.1, 0.0])
        .unwrap();
    let vector_id = serde_json::to_string(&("a", "default")).unwrap();
    let vector_tx: i64 = projection
        .query_row(
            "SELECT tx_from FROM hql2_record_revisions
             WHERE kind='vector' AND record_id=?1 ORDER BY tx_from LIMIT 1",
            [&vector_id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(vector_tx > selected_tx);
    assert!(u64::try_from(vector_tx).unwrap() <= storage.stable_frontier());
    projection
        .execute(
            "UPDATE hql2_source_history_floors SET history_floor=?1
             WHERE namespace='default' AND source='vector'",
            [vector_tx],
        )
        .unwrap();

    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":1,"mode":"exact","as":"hit"}
    });
    let mut request =
        serde_json::to_value(ir_request("historical-knn-vector-floor", &space_id, rank)).unwrap();
    request["temporal"] = json!({"tx_as_of":selected_tx.to_string()});
    let request: QueryRequestV2 = serde_json::from_value(request).unwrap();
    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "HISTORY_UNAVAILABLE");
}

#[test]
fn hql_knn_and_rerank_accept_decimal_u64_k_parameters() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "a", Some(vec![0.1, 0.0]));
    add_node(&storage, "b", Some(vec![0.2, 0.0]));
    let space_id = space_id(&storage);
    let query = "USE default FROM NODES Document AS d |> KNN d IN default USING $q TOP $knn_k EXACT AS hit |> RERANK d IN default USING $q TOP $rerank_k EXACT AS fine |> RETURN d.id AS id, fine.distance AS distance";
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"hql-vector-u64-parameters",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":query,
        "params":{
            "q":vector_param(&space_id),
            "knn_k":{"type":"DecimalU64","value":"2"},
            "rerank_k":{"type":"DecimalU64","value":"1"}
        }
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("HQL KNN/RERANK must execute")
    };
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0]["id"], QueryValueV2::Utf8("a".into()));

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"hql-vector-u64-overflow",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS d |> KNN d IN default USING $q TOP $knn_k EXACT AS hit |> RETURN d.id AS id",
        "params":{
            "q":vector_param(&space_id),
            "knn_k":{"type":"DecimalU64","value":"4294967296"}
        }
    }))
    .unwrap();
    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "ranking_bound");

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"hql-rerank-u64-wrong-type",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":query,
        "params":{
            "q":vector_param(&space_id),
            "knn_k":{"type":"DecimalU64","value":"2"},
            "rerank_k":{"type":"I64","value":"1"}
        }
    }))
    .unwrap();
    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "parameter_type_mismatch");

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"hql-rerank-u64-overflow",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS d |> RERANK d IN default USING $q TOP $rerank_k EXACT AS fine |> RETURN d.id AS id",
        "params":{
            "q":vector_param(&space_id),
            "rerank_k":{"type":"DecimalU64","value":"4294967296"}
        }
    }))
    .unwrap();
    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "ranking_bound");
}

#[test]
fn hql_and_ir_exact_knn_have_identical_results() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "z", Some(vec![0.1, 0.0]));
    add_node(&storage, "a", Some(vec![-0.1, 0.0]));
    add_node(&storage, "missing", None);
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":1,"mode":"exact","as":"hit"}
    });
    let ir = ir_request("ir-knn-parity", &space_id, rank);
    let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir).unwrap() else {
        panic!("IR KNN must execute")
    };
    let hql: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"hql-knn-parity",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS d |> KNN d IN default USING $q TOP 1 EXACT AS hit |> RETURN d.id AS id, hit.distance AS distance",
        "params":{"q":vector_param(&space_id)}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql).unwrap() else {
        panic!("HQL KNN must execute")
    };
    assert_eq!(hql_result.rows, ir_result.rows);
    assert_eq!(hql_result.semantics, ir_result.semantics);
    assert_eq!(hql_result.rows[0]["id"], QueryValueV2::Utf8("a".into()));
}

#[test]
fn original_rerank_fails_closed_when_any_candidate_lacks_original_vector() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "present", Some(vec![0.1, 0.0]));
    add_node(&storage, "missing", None);
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Rerank","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":2,"fidelity":"original","as":"hit"}
    });
    let request = ir_request("rerank-original", &space_id, rank);

    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "EXACT_ORIGINAL_UNAVAILABLE");
    assert_eq!(error.stage, "execute");
}

#[test]
fn original_rerank_returns_top_candidates_with_original_score_semantics() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "b", Some(vec![0.2, 0.0]));
    add_node(&storage, "a", Some(vec![0.1, 0.0]));
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Rerank","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":1,"fidelity":"original","as":"hit"}
    });
    let request = ir_request("rerank-original-success", &space_id, rank);

    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("Rerank must execute")
    };
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0]["id"], QueryValueV2::Utf8("a".into()));
    let QueryValueV2::F64(distance) = result.rows[0]["distance"] else {
        panic!("score distance must be exposed as a finite float")
    };
    assert!((distance - 0.01).abs() < 1e-12);
    assert_eq!(
        result.semantics.distance_fidelity,
        genesis_block_native::query::hql2::result::DistanceFidelityV2::Original
    );
    assert_eq!(
        result.semantics.scope,
        genesis_block_native::query::hql2::result::ResultScopeV2::RerankCandidates
    );
}

#[test]
fn approximate_knn_is_rejected_until_candidate_coverage_is_implemented() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":1,"mode":"approx","as":"hit"}
    });
    let error = storage
        .query_v2(access(), ir_request("approx-knn", &space_id, rank))
        .unwrap_err();
    assert_eq!(error.code, "CAPABILITY_UNSUPPORTED");
    assert_eq!(error.stage, "bind");
    assert_eq!(error.detail.unwrap()["reason"], "knn_approx_unavailable");
}

#[test]
fn vector_type_must_match_the_resolved_collection_space() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":1,"mode":"exact","as":"hit"}
    });
    let mut request = ir_request("collection-space-mismatch", &space_id, rank);
    let other_space = "f".repeat(64);
    request
        .ir
        .as_mut()
        .unwrap()
        .parameter_types
        .insert("q".into(), format!("Vector<{other_space},2,f64>"));
    request
        .params
        .insert("q".into(), vector_param(&other_space));

    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "COLLECTION_SPACE_MISMATCH");
    assert_eq!(error.stage, "bind");
}

#[test]
fn vector_candidate_budget_fails_before_returning_partial_rows() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "a", Some(vec![0.1, 0.0]));
    add_node(&storage, "b", Some(vec![0.2, 0.0]));
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":1,"mode":"exact","as":"hit"}
    });
    let mut request = ir_request("candidate-budget", &space_id, rank);
    request.budget = Some(genesis_block_native::uee_v2::QueryBudgetV2 {
        max_memory_bytes: None,
        max_spill_bytes: None,
        max_elapsed_ms: None,
        max_expanded_nodes: None,
        max_expanded_edges: None,
        max_vector_candidates: Some(1),
        max_distance_evaluations: None,
        max_result_rows: None,
        max_result_bytes: None,
    });

    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
    assert_eq!(error.stage, "execute");
}

#[test]
fn exact_cosine_knn_uses_the_original_collection_metric() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .create_collection(
            "cosine".into(),
            "cosine-model".into(),
            2,
            Some("Cosine".into()),
            None,
            None,
            None,
        )
        .unwrap();
    add_node(&storage, "same", None);
    add_node(&storage, "orthogonal", None);
    storage
        .add_vector("same".into(), "cosine".into(), vec![1.0, 0.0])
        .unwrap();
    storage
        .add_vector("orthogonal".into(), "cosine".into(), vec![0.0, 1.0])
        .unwrap();
    let space_id = space_id_for(&storage, "cosine");
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"cosine","query":{"param":"q"},"k":1,"mode":"exact","as":"hit"}
    });
    let mut request = ir_request("cosine-knn", &space_id, rank);
    request.params.insert(
        "q".into(),
        json!({"type":"vector","space_id":space_id,"values":[1.0,0.0]}),
    );

    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("cosine KNN must execute")
    };
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0]["id"], QueryValueV2::Utf8("same".into()));
    assert_eq!(result.rows[0]["distance"], QueryValueV2::F64(0.0));
}

#[test]
fn zero_cosine_query_is_rejected_even_when_the_input_is_empty() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .create_collection(
            "cosine".into(),
            "cosine-model".into(),
            2,
            Some("Cosine".into()),
            None,
            None,
            None,
        )
        .unwrap();
    let space_id = space_id_for(&storage, "cosine");
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"cosine","query":{"param":"q"},"k":0,"mode":"exact","as":"hit"}
    });
    let mut request = ir_request("cosine-zero-query", &space_id, rank);
    request.params.insert(
        "q".into(),
        json!({"type":"vector","space_id":space_id,"values":[0.0,0.0]}),
    );

    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "COLLECTION_SPACE_MISMATCH");
    assert_eq!(error.stage, "bind");
}

#[test]
fn knn_exposes_a_typed_score_value_when_the_score_is_projected_whole() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "a", Some(vec![0.1, 0.0]));
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":1,"mode":"exact","as":"hit"}
    });
    let mut raw = serde_json::to_value(ir_request("score-value", &space_id, rank)).unwrap();
    raw["ir"]["nodes"].as_array_mut().unwrap().pop();
    raw["ir"]["root"] = json!("rank");
    let request = serde_json::from_value(raw).unwrap();

    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("KNN must execute")
    };
    assert_eq!(result.columns[1].data_type, "Score");
    let QueryValueV2::Score(score) = &result.rows[0]["hit"] else {
        panic!("whole hit must preserve its typed Score value")
    };
    assert!((score.value - 0.01).abs() < 1e-12);
    let encoded = serde_json::to_value(&result.rows[0]["hit"]).unwrap();
    assert_eq!(encoded["type"], "Score");
    assert_eq!(encoded["value"]["metric"], "L2");
}

#[test]
fn vector_parameter_space_must_match_its_ir_type_declaration() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":1,"mode":"exact","as":"hit"}
    });
    let mut request = ir_request("vector-space-mismatch", &space_id, rank);
    request.params.insert(
        "q".into(),
        json!({"type":"vector","space_id":"different-space","values":[0.0,0.0]}),
    );

    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "COLLECTION_SPACE_MISMATCH");
    assert_eq!(error.stage, "bind");
}

#[test]
fn vector_parameter_dimension_must_match_its_ir_type_declaration() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let space_id = space_id(&storage);
    let rank = json!({
        "id":"rank","op":"Knn","inputs":["scan"],
        "config":{"entity":"d","collection":"default","query":{"param":"q"},"k":1,"mode":"exact","as":"hit"}
    });
    let mut request = ir_request("vector-dimension-mismatch", &space_id, rank);
    request.params.insert(
        "q".into(),
        json!({"type":"vector","space_id":space_id,"values":[0.0]}),
    );

    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "COLLECTION_SPACE_MISMATCH");
    assert_eq!(error.stage, "bind");
}
