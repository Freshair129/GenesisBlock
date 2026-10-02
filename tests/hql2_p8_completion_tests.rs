#[path = "support/hql2_rank_reference.rs"]
#[allow(dead_code)]
mod rank_reference;

use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryErrorV2, QueryOutcomeV2},
    uee_v2::{ExplainV2, QueryRequestV2},
    AccessContext, EdgeInput, NodeInput, OpenOptions, Storage,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use tempfile::TempDir;

fn lexical_reference_fixture(
    texts: &[(&str, &str)],
) -> (rank_reference::Fixture, rank_reference::Batch) {
    let analyzer_fingerprint = "unicode-whitespace-bm25-v1".to_owned();
    let documents = texts
        .iter()
        .map(|(id, text)| rank_reference::Document {
            id: (*id).into(),
            revision: "r1".into(),
            visibility: rank_reference::Visibility {
                tx_from: 1,
                tx_to: None,
                valid_from: 0,
                valid_to: None,
                authorized: true,
            },
            original: None,
            text: (*text).into(),
            source_hash: String::new(),
            analyzer_fingerprint: analyzer_fingerprint.clone(),
            tokens: match *text {
                "red\u{2003}red blue" => vec!["red".into(), "red".into(), "blue".into()],
                "" => vec![],
                other => vec![other.into()],
            },
        })
        .collect::<Vec<_>>();
    let fixture = rank_reference::Fixture {
        space: rank_reference::Space {
            fingerprint: "unused".into(),
            dimension: 1,
            metric: rank_reference::Metric::L2Squared,
        },
        analyzer_fingerprint,
        documents,
    };
    let batch = rank_reference::Batch {
        snapshot: rank_reference::Snapshot {
            transaction: 10,
            valid: 1,
        },
        rows: texts
            .iter()
            .map(|(id, _)| rank_reference::Candidate {
                row_key: (*id).into(),
                owner_id: (*id).into(),
                revision: "r1".into(),
                language: String::new(),
                hit: None,
                source_ranks: BTreeMap::new(),
            })
            .collect(),
        lineage: rank_reference::Lineage {
            approximate_sources: vec![],
            scope: rank_reference::Scope::WholeInput,
        },
    };
    (fixture, batch)
}

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
        principal: "p8-reader".into(),
        namespace: "default".into(),
    }
}

fn request(id: &str, hql: &str) -> QueryRequestV2 {
    request_with_params(id, hql, json!({}))
}

fn request_with_params(id: &str, hql: &str, params: Value) -> QueryRequestV2 {
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":id,
        "namespace":"default",
        "hql":hql,
        "language_version":"hql.v2",
        "params":params
    }))
    .unwrap()
}

fn request_with_memory_budget(id: &str, hql: &str, max_memory_bytes: u64) -> QueryRequestV2 {
    let mut request = request(id, hql);
    request.budget =
        Some(serde_json::from_value(json!({"max_memory_bytes":max_memory_bytes})).unwrap());
    request
}

fn run(storage: &Storage, id: &str, hql: &str) -> Result<QueryOutcomeV2, QueryErrorV2> {
    storage.query_v2(access(), request(id, hql))
}

fn run_with_params(
    storage: &Storage,
    id: &str,
    hql: &str,
    params: Value,
) -> Result<QueryOutcomeV2, QueryErrorV2> {
    storage.query_v2(access(), request_with_params(id, hql, params))
}

fn run_ir(storage: &Storage, id: &str, ir: Value) -> Result<QueryOutcomeV2, QueryErrorV2> {
    run_ir_with_params(storage, id, ir, json!({}))
}

fn run_ir_with_params(
    storage: &Storage,
    id: &str,
    ir: Value,
    params: Value,
) -> Result<QueryOutcomeV2, QueryErrorV2> {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":id,
        "namespace":"default",
        "params":params,
        "ir":ir
    }))
    .unwrap();
    storage.query_v2(access(), request)
}

fn add_node(storage: &Storage, id: &str, props: Value) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: vec!["Document".into()],
            props: Some(props),
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn add_edge(storage: &Storage, id: &str, from: &str, to: &str, props: Value) {
    storage
        .add_edge(EdgeInput {
            id: Some(id.into()),
            from: from.into(),
            to: to.into(),
            rel: "LINK".into(),
            props: Some(props),
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
}

#[test]
fn lexical_match_uses_registered_profile_bm25_and_exact_ties() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:a", json!({"body":"red\u{2003}red blue"}));
    add_node(&storage, "doc:b", json!({"body":"red\u{2003}red blue"}));
    add_node(&storage, "doc:c", json!({"body":""}));
    add_node(&storage, "doc:missing", json!({"title":"not indexed"}));

    let QueryOutcomeV2::Rows(result) = run(
        &storage,
        "lexical-bm25",
        "USE default FROM NODES AS n |> LEXICAL n FIELD body QUERY \"red\" USING INDEX `unicode-whitespace-bm25-v1` TOP 10 AS score |> RETURN n.id AS id, score AS score",
    )
    .unwrap()
    else {
        panic!("query must execute")
    };

    assert_eq!(result.rows.len(), 2);
    assert_eq!(
        result.rows[0]["id"],
        genesis_block_native::query::hql2::value::QueryValueV2::Utf8("doc:a".into())
    );
    assert_eq!(
        result.rows[1]["id"],
        genesis_block_native::query::hql2::value::QueryValueV2::Utf8("doc:b".into())
    );
    let first = serde_json::to_value(&result.rows[0]["score"]).unwrap();
    let second = serde_json::to_value(&result.rows[1]["score"]).unwrap();
    assert_eq!(first["type"], "Score");
    assert_eq!(first["value"]["metric"], "unicode-whitespace-bm25-v1");
    assert_eq!(first["value"]["value"], second["value"]["value"]);
    let (fixture, batch) = lexical_reference_fixture(&[
        ("doc:a", "red\u{2003}red blue"),
        ("doc:b", "red\u{2003}red blue"),
        ("doc:c", ""),
    ]);
    let expected = rank_reference::lexical_rank(
        batch,
        &fixture,
        &rank_reference::LexicalRequest {
            analyzer_fingerprint: "unicode-whitespace-bm25-v1".into(),
            terms: vec!["red".into()],
            k1: 1.2,
            b: 0.75,
            top: 10,
            source: "lexical".into(),
        },
    )
    .unwrap();
    assert_eq!(
        result
            .rows
            .iter()
            .map(|row| match &row["id"] {
                QueryValueV2::Utf8(id) => id.as_str(),
                _ => panic!("lexical id must be UTF-8"),
            })
            .collect::<Vec<_>>(),
        expected
            .rows
            .iter()
            .map(|row| row.owner_id.as_str())
            .collect::<Vec<_>>()
    );
    for (actual, expected) in result.rows.iter().zip(expected.rows.iter()) {
        let QueryValueV2::Score(score) = &actual["score"] else {
            panic!("lexical output must be Score")
        };
        assert!((score.value - expected.hit.as_ref().unwrap().value).abs() < 1e-12);
    }

    let QueryOutcomeV2::Rows(typed) = run_ir_with_params(
        &storage,
        "lexical-ir-parity",
        json!({
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"n"}},
                {"id":"lex","op":"LexicalMatch","inputs":["scan"],"config":{
                    "entity":"n","field":"body","index":"unicode-whitespace-bm25-v1",
                    "query":{"param":"q"},"k":10,"as":"score"
                }},
                {"id":"project","op":"Project","inputs":["lex"],"config":{"fields":[
                    {"as":"id","expression":{"field":{"alias":"n","path":["id"]}}},
                    {"as":"score","expression":{"field":{"alias":"score","path":[]}}}
                ]}}
            ],
            "root":"project","parameter_types":{"q":"Utf8"}
        }),
        json!({"q":{"type":"Utf8","value":"red"}}),
    )
    .unwrap() else {
        panic!("typed lexical query must execute")
    };
    assert_eq!(typed.rows, result.rows);
}

#[test]
fn lexical_unknown_profile_fails_even_for_an_empty_source() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let error = run(
        &storage,
        "lexical-unknown",
        "USE default FROM NODES AS n |> LEXICAL n FIELD body QUERY \"red\" USING INDEX `unknown-v1` TOP 0 AS score |> RETURN n",
    )
    .unwrap_err();
    assert_eq!(error.code, "CAPABILITY_UNSUPPORTED");
    assert_eq!(error.stage, "bind");
    assert_eq!(error.detail.unwrap()["reason"], "unknown_analyzer");
}

#[test]
fn context_pack_keeps_exact_text_evidence_and_unicode_scalar_offsets() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:context", json!({"body":"é猫😀"}));

    let QueryOutcomeV2::Rows(result) = run(
        &storage,
        "context-pack",
        "USE default FROM NODES AS n |> PACK CONTEXT TEXT prop(n, \"body\") EVIDENCE n TOKENS 5 TOKENIZER \"unicode-scalar-v1\" AS ctx |> RETURN ctx",
    )
    .unwrap()
    else {
        panic!("query must execute")
    };

    assert_eq!(result.rows.len(), 1);
    let context = serde_json::to_value(&result.rows[0]["ctx"]).unwrap();
    assert_eq!(context["type"], "Context");
    assert_eq!(context["value"]["rendered_context"], "é猫[1]");
    assert_eq!(context["value"]["token_count"], 5);
    assert_eq!(context["value"]["fragments"][0]["text"], "é猫");
    assert_eq!(
        context["value"]["fragments"][0]["evidence"]["start_scalar"],
        0
    );
    assert_eq!(
        context["value"]["fragments"][0]["evidence"]["end_scalar"],
        2
    );
    assert_eq!(context["value"]["omitted_refs"][0]["start_scalar"], 2);
    assert_eq!(context["value"]["omitted_refs"][0]["end_scalar"], 3);

    let (mut fixture, batch) = lexical_reference_fixture(&[("doc:context", "é猫😀")]);
    let source_hash = hex::encode(Sha256::digest("é猫😀".as_bytes()));
    fixture.documents[0].source_hash = source_hash.clone();
    let oracle = rank_reference::pack_context(
        &batch,
        &fixture,
        &rank_reference::TokenizerRegistry {
            entries: BTreeMap::from([(
                "unicode-scalar-v1".into(),
                rank_reference::FixtureTokenizer::UnicodeScalarV1,
            )]),
        },
        "unicode-scalar-v1",
        5,
    )
    .unwrap();
    let QueryValueV2::Context(actual) = &result.rows[0]["ctx"] else {
        panic!("context output must be Context")
    };
    assert_eq!(actual.rendered_context, oracle.rendered_context);
    assert_eq!(actual.token_count as usize, oracle.token_count);
    assert_eq!(actual.fragments[0].text, oracle.fragments[0].text);
    assert_eq!(actual.fragments[0].citation, oracle.fragments[0].citation);
    assert_eq!(
        actual.fragments[0].evidence.source_hash,
        oracle.fragments[0].evidence.source_hash
    );
    assert_eq!(
        actual.fragments[0].evidence.start_scalar as usize,
        oracle.fragments[0].evidence.start_scalar
    );
    assert_eq!(
        actual.fragments[0].evidence.end_scalar as usize,
        oracle.fragments[0].evidence.end_scalar
    );
    assert_eq!(
        actual.omitted_refs[0].start_scalar as usize,
        oracle.omitted_refs[0].start_scalar
    );
    assert_eq!(
        actual.omitted_refs[0].end_scalar as usize,
        oracle.omitted_refs[0].end_scalar
    );
    let QueryOutcomeV2::Rows(source_result) = run(
        &storage,
        "context-source-identity",
        "USE default FROM NODES AS n |> RETURN n AS source",
    )
    .unwrap() else {
        panic!("source identity query must execute")
    };
    let QueryValueV2::Entity(source) = &source_result.rows[0]["source"] else {
        panic!("source identity must be Entity")
    };
    assert_eq!(&actual.fragments[0].evidence.source, source);
    assert_eq!(actual.fragments[0].evidence.source_hash, source_hash);

    let QueryOutcomeV2::Rows(typed) = run_ir(
        &storage,
        "context-pack-ir-parity",
        json!({
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"n"}},
                {"id":"pack","op":"ContextPack","inputs":["scan"],"config":{
                    "text":{"call":"prop","args":[
                        {"field":{"alias":"n","path":[]}},
                        {"literal":"body","type":"Utf8"}
                    ]},
                    "evidence":{"field":{"alias":"n","path":[]}},
                    "tokens":5,"tokenizer":"unicode-scalar-v1","as":"ctx"
                }}
            ],
            "root":"pack","parameter_types":{}
        }),
    )
    .unwrap() else {
        panic!("typed ContextPack must execute")
    };
    assert_eq!(typed.rows, result.rows);
}

#[test]
fn context_unknown_tokenizer_fails_even_for_an_empty_source() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let error = run(
        &storage,
        "context-unknown",
        "USE default FROM NODES AS n |> PACK CONTEXT TEXT prop(n, \"body\") EVIDENCE n TOKENS 5 TOKENIZER \"unknown-v1\" AS ctx |> RETURN ctx",
    )
    .unwrap_err();
    assert_eq!(error.code, "CAPABILITY_UNSUPPORTED");
    assert_eq!(error.stage, "bind");
    assert_eq!(error.detail.unwrap()["reason"], "unknown_tokenizer");
}

#[test]
fn lexical_and_context_memory_budget_failures_return_no_partial_rows() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let text = "red ".repeat(2048);
    for index in 0..32 {
        add_node(
            &storage,
            &format!("doc:budget:{index:02}"),
            json!({"body":text}),
        );
    }

    for (id, hql) in [
        (
            "lexical-memory-budget",
            "USE default FROM NODES AS n |> LEXICAL n FIELD body QUERY \"red\" USING INDEX `unicode-whitespace-bm25-v1` TOP 10 AS score |> RETURN n.id AS id, score AS score",
        ),
        (
            "context-memory-budget",
            "USE default FROM NODES AS n |> PACK CONTEXT TEXT prop(n, \"body\") EVIDENCE n TOKENS 5 TOKENIZER \"unicode-scalar-v1\" AS ctx |> RETURN ctx",
        ),
    ] {
        let mut explain = request_with_memory_budget(id, hql, 6 * 1024 * 1024);
        explain.explain = Some(ExplainV2::Plan);
        assert!(matches!(
            storage.query_v2(access(), explain),
            Ok(QueryOutcomeV2::Plan(_))
        ));

        let error = storage
            .query_v2(
                access(),
                request_with_memory_budget(&format!("{id}-execute"), hql, 6 * 1024 * 1024),
            )
            .unwrap_err();
        assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED", "{id}");
        assert_eq!(error.stage, "execute", "{id}");
    }
}

#[test]
fn sequence_node_and_edge_properties_use_exact_json_values() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "node:a", json!({"role":"source"}));
    add_node(
        &storage,
        "node:b",
        json!({"enabled":true,"explicit_null":null,"meta":{"flags":[true,null],"score":1}}),
    );
    add_node(&storage, "node:c", json!({"enabled":true}));
    add_node(
        &storage,
        "node:d",
        json!({"enabled":true,"explicit_null":null}),
    );
    add_edge(
        &storage,
        "edge:match",
        "node:a",
        "node:b",
        json!({"weight":7}),
    );
    add_edge(
        &storage,
        "edge:missing-null",
        "node:a",
        "node:c",
        json!({"weight":7}),
    );
    add_edge(
        &storage,
        "edge:other",
        "node:a",
        "node:d",
        json!({"weight":"7"}),
    );

    let QueryOutcomeV2::Rows(result) = run(
        &storage,
        "sequence-properties",
        "USE default MATCH (a {id: \"node:a\", role: \"source\"})-[e:LINK {weight: 7}]->(b {enabled: true, explicit_null: NULL, meta: {flags: [true, null], score: 1}}) |> RETURN b.id AS id",
    )
    .unwrap()
    else {
        panic!("query must execute")
    };

    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0]["id"], QueryValueV2::Utf8("node:b".into()));

    let QueryOutcomeV2::Rows(typed) = run_ir(
        &storage,
        "sequence-properties-ir-parity",
        json!({
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"match","op":"Match","inputs":[],"config":{
                    "pattern":{
                        "form":"sequence",
                        "start":{"alias":"a","id":{"literal":"node:a","type":"Utf8"},"labels":[],"properties":{
                            "role":{"literal":"source","type":"Json"}
                        }},
                        "steps":[{
                            "edge":{"alias":"e","relations":["LINK"],"direction":"out","min_hops":1,"max_hops":1,"properties":{
                                "weight":{"literal":7,"type":"Json"}
                            }},
                            "node":{"alias":"b","labels":[],"properties":{
                                "enabled":{"literal":true,"type":"Json"},
                                "explicit_null":{"literal":null,"type":"Json"},
                                "meta":{"literal":{"flags":[true,null],"score":1},"type":"Json"}
                            }}
                        }],
                        "mode":"trail"
                    },
                    "anchors":{},"shortest":false
                }},
                {"id":"project","op":"Project","inputs":["match"],"config":{"fields":[
                    {"as":"id","expression":{"field":{"alias":"b","path":["id"]}}}
                ]}}
            ],
            "root":"project","parameter_types":{}
        }),
    )
    .unwrap()
    else {
        panic!("typed sequence query must execute")
    };
    assert_eq!(typed.rows, result.rows);
}

#[test]
fn sequence_property_work_budget_exhaustion_returns_no_partial_rows() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "node:budget:root", json!({}));
    for index in 0..2 {
        let target = format!("node:budget:{index}");
        add_node(&storage, &target, json!({"enabled":true}));
        add_edge(
            &storage,
            &format!("edge:budget:{index}"),
            "node:budget:root",
            &target,
            json!({"weight":7}),
        );
    }

    let mut request = request(
        "sequence-property-budget",
        "USE default MATCH (a {id: \"node:budget:root\"})-[:LINK {weight: 7}]->(b {enabled: true}) |> RETURN b.id AS id",
    );
    request.budget = Some(serde_json::from_value(json!({"max_expanded_edges":1})).unwrap());
    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
    assert_eq!(error.stage, "execute");
}

#[test]
fn sequence_edge_property_filters_candidates_before_shortest_selection() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "node:start", json!({}));
    add_node(&storage, "node:middle", json!({}));
    add_node(&storage, "node:end", json!({}));
    add_edge(
        &storage,
        "edge:short",
        "node:start",
        "node:end",
        json!({"weight":1}),
    );
    add_edge(
        &storage,
        "edge:long-1",
        "node:start",
        "node:middle",
        json!({"weight":7}),
    );
    add_edge(
        &storage,
        "edge:long-2",
        "node:middle",
        "node:end",
        json!({"weight":7}),
    );

    let QueryOutcomeV2::Rows(unconstrained) = run(
        &storage,
        "shortest-edge-property-baseline",
        "USE default MATCH SHORTEST (a {id: \"node:start\"})-[e:LINK*1..2]->(c {id: \"node:end\"}) AS p WALK |> RETURN e AS edges, p AS path",
    )
    .unwrap()
    else {
        panic!("unconstrained shortest query must execute")
    };
    let QueryValueV2::List(unconstrained_edges) = &unconstrained.rows[0]["edges"] else {
        panic!("variable-hop edge alias must project as an edge list")
    };
    assert_eq!(unconstrained_edges.len(), 1);

    let QueryOutcomeV2::Rows(constrained) = run(
        &storage,
        "shortest-edge-property-filtered",
        "USE default MATCH SHORTEST (a {id: \"node:start\"})-[e:LINK {weight: 7}*1..2]->(c {id: \"node:end\"}) AS p WALK |> RETURN c.id AS end, e AS edges, p AS path",
    )
    .unwrap()
    else {
        panic!("property-constrained shortest query must execute")
    };
    assert_eq!(constrained.rows.len(), 1);
    assert_eq!(
        constrained.rows[0]["end"],
        QueryValueV2::Utf8("node:end".into())
    );
    let QueryValueV2::List(constrained_edges) = &constrained.rows[0]["edges"] else {
        panic!("variable-hop edge alias must project as an edge list")
    };
    assert_eq!(constrained_edges.len(), 2);
    assert!(matches!(
        &constrained.rows[0]["path"],
        QueryValueV2::Path(path) if path.edges.len() == 2
    ));

    let QueryOutcomeV2::Rows(typed) = run_ir(
        &storage,
        "shortest-edge-property-ir",
        json!({
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"match","op":"Match","inputs":[],"config":{
                    "pattern":{
                        "form":"sequence",
                        "start":{"alias":"a","id":{"literal":"node:start","type":"Utf8"},"labels":[],"properties":{}},
                        "steps":[{
                            "edge":{"alias":"e","relations":["LINK"],"direction":"out","min_hops":1,"max_hops":2,"properties":{
                                "weight":{"literal":7,"type":"Json"}
                            }},
                            "node":{"alias":"c","id":{"literal":"node:end","type":"Utf8"},"labels":[],"properties":{}}
                        }],
                        "mode":"walk"
                    },
                    "anchors":{},"shortest":true
                }},
                {"id":"project","op":"Project","inputs":["match"],"config":{"fields":[
                    {"as":"end","expression":{"field":{"alias":"c","path":["id"]}}}
                ]}}
            ],
            "root":"project","parameter_types":{}
        }),
    )
    .unwrap()
    else {
        panic!("typed-IR property-constrained shortest query must execute")
    };
    assert_eq!(typed.rows.len(), 1);
    assert_eq!(typed.rows[0]["end"], constrained.rows[0]["end"]);
}

#[test]
fn contextual_null_lists_and_json_objects_keep_their_exact_types() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());

    let QueryOutcomeV2::Rows(result) = run_with_params(
        &storage,
        "contextual-literals",
        "USE default VALUES $xs AS n |> FILTER n IN [NULL, 2] |> RETURN n, {\"kept\": [true, null], \"label\": \"x\", \"type\": \"Entity\", \"value\": {\"id\": \"node:b\"}} AS obj",
        json!({"xs":{"type":"List<Nullable<I64>>","value":["1","2"]}}),
    )
    .unwrap()
    else {
        panic!("query must execute")
    };

    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        serde_json::to_value(&result.rows[0]["n"]).unwrap()["value"],
        "2"
    );
    let object = serde_json::to_value(&result.rows[0]["obj"]).unwrap();
    assert_eq!(object["type"], "Json");
    assert_eq!(object["value"]["kept"], json!([true, null]));
    assert_eq!(object["value"]["label"], "x");
    assert_eq!(object["value"]["type"], "Entity");
    assert_eq!(object["value"]["value"]["id"], "node:b");
}

#[test]
fn ambiguous_null_and_heterogeneous_lists_reject_while_empty_list_is_contextual() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let params = json!({"xs":{"type":"List<I64>","value":["1","2"]}});

    let QueryOutcomeV2::Rows(empty) = run_with_params(
        &storage,
        "empty-contextual-list",
        "USE default VALUES $xs AS n |> FILTER n IN [] |> RETURN n",
        params.clone(),
    )
    .unwrap() else {
        panic!("empty contextual list must execute")
    };
    assert!(empty.rows.is_empty());

    let ambiguous = run_with_params(
        &storage,
        "ambiguous-null",
        "USE default VALUES $xs AS n |> RETURN NULL AS value",
        params.clone(),
    )
    .unwrap_err();
    assert_eq!(ambiguous.code, "BIND_ERROR");
    assert_eq!(
        ambiguous.detail.unwrap()["reason"],
        "ambiguous_null_literal"
    );

    let heterogeneous = run_with_params(
        &storage,
        "heterogeneous-list",
        "USE default VALUES $xs AS n |> FILTER n IN [1, \"1\"] |> RETURN n",
        params,
    )
    .unwrap_err();
    assert_eq!(heterogeneous.code, "BIND_ERROR");

    let nested = format!(
        "USE default VALUES $xs AS n |> RETURN {{\"nested\": {}true{}}} AS value",
        "[".repeat(129),
        "]".repeat(129),
    );
    let nested_error = run_with_params(
        &storage,
        "nested-json-limit",
        &nested,
        json!({"xs":{"type":"List<I64>","value":[]}}),
    )
    .unwrap_err();
    assert!(
        nested_error.code == "BIND_ERROR" || nested_error.code == "HQL_PARSE_ERROR",
        "deep JSON literal must fail closed: {nested_error:?}"
    );
}
