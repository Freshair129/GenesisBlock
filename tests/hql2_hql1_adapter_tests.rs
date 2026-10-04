use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::QueryRequestV2,
    AccessAction, AccessContext, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource,
    EdgeInput, NodeInput, OpenOptions, PolicyAdminActor, Storage,
};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn actor(principal: &str) -> AccessContext {
    AccessContext {
        principal: principal.into(),
        namespace: "default".into(),
    }
}

fn install_policy(storage: &Storage, include_reader: bool) {
    let mut grants = vec![AccessGrant {
        principal: "local-owner".into(),
        action: AccessAction::ManagePolicy,
        resource: AccessResource::Namespace("default".into()),
    }];
    if include_reader {
        grants.push(AccessGrant {
            principal: "legacy-reader".into(),
            action: AccessAction::Read,
            resource: AccessResource::Namespace("default".into()),
        });
    }
    storage
        .replace_access_policy(
            PolicyAdminActor {
                access: actor("local-owner"),
            },
            0,
            AccessPolicy {
                revision: 1,
                mode: AccessPolicyMode::Enforced,
                grants,
            },
        )
        .unwrap();
}

fn add_node(storage: &Storage, id: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
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

fn add_edge(storage: &Storage, id: &str, from: &str, to: &str, relation: &str) {
    storage
        .add_edge(EdgeInput {
            id: Some(id.into()),
            from: from.into(),
            to: to.into(),
            rel: relation.into(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
}

fn request(query: &str) -> QueryRequestV2 {
    serde_json::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": "hql1-adapter",
        "namespace": "default",
        "language_version": "hql.v1",
        "hql": query,
        "params": {}
    }))
    .unwrap()
}

fn hql2_request(query: &str) -> QueryRequestV2 {
    serde_json::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": "hql2-adapter-baseline",
        "namespace": "default",
        "language_version": "hql.v2",
        "hql": query,
        "params": {}
    }))
    .unwrap()
}

fn projected_ids(
    result: &genesis_block_native::query::hql2::QueryResultV2,
    column: &str,
) -> Vec<String> {
    assert_eq!(result.columns.len(), 1);
    assert_eq!(result.columns[0].name, column);
    result
        .rows
        .iter()
        .map(|row| match row.get(column).unwrap() {
            QueryValueV2::Utf8(id) => id.clone(),
            value => panic!("expected UTF-8 node ID, got {value:?}"),
        })
        .collect()
}

#[test]
fn actor_scoped_hql1_zero_hop_match_matches_legacy_and_hql2() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "node:a");
    add_node(&storage, "node:b");
    let legacy = storage
        .execute_hql("MATCH (a) RETURN a.id")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["a.id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    install_policy(&storage, true);

    let QueryOutcomeV2::Rows(hql2) = storage
        .query_v2(
            actor("legacy-reader"),
            serde_json::from_value(json!({
                "contract_version": "genesis.api.v2",
                "request_id": "hql2-baseline",
                "namespace": "default",
                "language_version": "hql.v2",
                "hql": "USE default FROM NODES AS a |> RETURN a.id AS id",
                "params": {}
            }))
            .unwrap(),
        )
        .unwrap()
    else {
        panic!("expected HQL2 rows")
    };
    let QueryOutcomeV2::Rows(hql1) = storage
        .query_v2(actor("legacy-reader"), request("MATCH (a) RETURN a.id"))
        .unwrap()
    else {
        panic!("expected actor-scoped HQL1 rows")
    };

    let mut legacy = legacy;
    let mut hql2 = projected_ids(&hql2, "id");
    let mut hql1 = projected_ids(&hql1, "a.id");
    legacy.sort();
    hql2.sort();
    hql1.sort();
    assert_eq!(hql1, legacy);
    assert_eq!(hql1, hql2);
}

#[test]
fn actor_scoped_hql1_zero_hop_label_match_matches_legacy_and_hql2() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for (id, labels) in [
        ("people:ada", vec!["Person"]),
        ("people:grace", vec!["Person", "Engineer"]),
        ("places:lab", vec!["Place"]),
        ("plain", vec![]),
    ] {
        storage
            .add_node(NodeInput {
                id: Some(id.into()),
                labels: labels.into_iter().map(str::to_owned).collect(),
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

    let legacy_ids = |query: &str| {
        storage
            .execute_hql(query)
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["p.id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    let legacy_people = legacy_ids("MATCH (p:Person) RETURN p.id");
    let legacy_missing = legacy_ids("MATCH (p:Missing) RETURN p.id");
    install_policy(&storage, true);

    for (label, mut legacy) in [("Person", legacy_people), ("Missing", legacy_missing)] {
        let QueryOutcomeV2::Rows(hql2) = storage
            .query_v2(
                actor("legacy-reader"),
                hql2_request(&format!(
                    "USE default FROM NODES {label} AS __hql1_node |> RETURN __hql1_node.id AS id"
                )),
            )
            .unwrap()
        else {
            panic!("expected canonical HQL2 rows")
        };

        let QueryOutcomeV2::Rows(hql1) = storage
            .query_v2(
                actor("legacy-reader"),
                request(&format!("MATCH (p:{label}) RETURN p.id")),
            )
            .unwrap()
        else {
            panic!("expected actor-scoped HQL1 rows")
        };

        let mut hql2 = projected_ids(&hql2, "id");
        let mut hql1 = projected_ids(&hql1, "p.id");
        legacy.sort();
        hql2.sort();
        hql1.sort();
        assert_eq!(hql2, legacy, "canonical differential for {label}");
        assert_eq!(hql1, legacy, "actor-scoped HQL1 differential for {label}");
    }
}

#[test]
fn actor_scoped_hql1_zero_hop_id_equality_matches_legacy_and_hql2() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for id in ["a", "b"] {
        add_node(&storage, id);
    }

    let cases = [
        (
            "MATCH (a) WHERE a.id = \"a\" RETURN a.id",
            "USE default FROM NODES AS __hql1_node |> FILTER __hql1_node.id = \"a\" |> RETURN __hql1_node.id AS id",
            vec!["a".to_owned()],
        ),
        (
            "MATCH (a) WHERE a.id = \"missing\" RETURN a.id",
            "USE default FROM NODES AS __hql1_node |> FILTER __hql1_node.id = \"missing\" |> RETURN __hql1_node.id AS id",
            Vec::new(),
        ),
    ];

    let legacy_results = cases
        .iter()
        .map(|(legacy_query, _, _)| {
            storage
                .execute_hql(legacy_query)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["a.id"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    install_policy(&storage, true);

    for (index, (legacy_query, canonical, expected)) in cases.iter().enumerate() {
        let QueryOutcomeV2::Rows(hql2) = storage
            .query_v2(actor("legacy-reader"), hql2_request(canonical))
            .unwrap()
        else {
            panic!("expected canonical HQL2 rows")
        };
        let QueryOutcomeV2::Rows(adapted) = storage
            .query_v2(actor("legacy-reader"), request(legacy_query))
            .unwrap()
        else {
            panic!("expected actor-scoped HQL1 rows")
        };
        let mut legacy = legacy_results[index].clone();
        let mut hql2 = projected_ids(&hql2, "id");
        let mut adapted = projected_ids(&adapted, "a.id");
        legacy.sort();
        hql2.sort();
        adapted.sort();
        assert_eq!(legacy, *expected, "legacy semantics for {}", cases[index].0);
        assert_eq!(hql2, *expected, "HQL2 semantics for {}", cases[index].0);
        assert_eq!(adapted, *expected, "adapter semantics for {legacy_query}");
    }
}

#[test]
fn actor_scoped_hql1_directed_one_hop_match_preserves_parallel_row_multiplicity() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for id in ["a", "b", "c"] {
        add_node(&storage, id);
    }
    add_edge(&storage, "ab-1", "a", "b", "LINK");
    add_edge(&storage, "ab-2", "a", "b", "LINK");
    add_edge(&storage, "ac", "a", "c", "OTHER");

    let legacy = storage
        .execute_hql("MATCH (a)-[:LINK]->(b) RETURN b.id")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["b.id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(legacy, ["b", "b"]);
    let legacy_source = storage
        .execute_hql("MATCH (a)-[:LINK]->(b) RETURN a.id")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["a.id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    install_policy(&storage, true);

    let QueryOutcomeV2::Rows(adapted) = storage
        .query_v2(
            actor("legacy-reader"),
            request("MATCH (a)-[:LINK]->(b) RETURN b.id"),
        )
        .unwrap()
    else {
        panic!("expected actor-scoped HQL1 rows")
    };

    let hql2_request = serde_json::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": "hql2-one-hop-baseline",
        "namespace": "default",
        "language_version": "hql.v2",
        "hql": "USE default MATCH (__hql1_source)-[:LINK]->(__hql1_target) AS __hql1_path WALK |> RETURN __hql1_target.id AS id",
        "params": {}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(hql2) = storage
        .query_v2(actor("legacy-reader"), hql2_request)
        .unwrap()
    else {
        panic!("expected HQL2 baseline rows")
    };

    let mut legacy = legacy;
    let mut adapted = projected_ids(&adapted, "b.id");
    let mut hql2 = projected_ids(&hql2, "id");
    legacy.sort();
    adapted.sort();
    hql2.sort();
    assert_eq!(adapted, legacy);
    assert_eq!(adapted, hql2);

    let QueryOutcomeV2::Rows(adapted_source) = storage
        .query_v2(
            actor("legacy-reader"),
            request("MATCH (a)-[:LINK]->(b) RETURN a.id"),
        )
        .unwrap()
    else {
        panic!("expected actor-scoped HQL1 source-projection rows")
    };
    let hql2_source_request = serde_json::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": "hql2-one-hop-source-baseline",
        "namespace": "default",
        "language_version": "hql.v2",
        "hql": "USE default MATCH (__hql1_source)-[:LINK]->(__hql1_target) AS __hql1_path WALK |> RETURN __hql1_source.id AS id",
        "params": {}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(hql2_source) = storage
        .query_v2(actor("legacy-reader"), hql2_source_request)
        .unwrap()
    else {
        panic!("expected HQL2 source-projection rows")
    };
    let mut legacy_source = legacy_source;
    let mut adapted_source = projected_ids(&adapted_source, "a.id");
    let mut hql2_source = projected_ids(&hql2_source, "id");
    legacy_source.sort();
    adapted_source.sort();
    hql2_source.sort();
    assert_eq!(adapted_source, legacy_source);
    assert_eq!(adapted_source, hql2_source);
}

#[test]
fn actor_scoped_hql1_one_hop_directions_and_untyped_edges_match_legacy_and_hql2() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for id in ["a", "b", "c"] {
        add_node(&storage, id);
    }
    add_edge(&storage, "ab", "a", "b", "LINK");
    add_edge(&storage, "bc", "b", "c", "OTHER");
    add_edge(&storage, "ca", "c", "a", "LINK");

    let cases = [
        (
            "MATCH (x)<-[:LINK]-(y) RETURN x.id",
            "x.id",
            "USE default MATCH (__hql1_source)<-[:LINK]-(__hql1_target) AS __hql1_path WALK |> RETURN __hql1_source.id AS id",
        ),
        (
            "MATCH (a)-[:LINK]-(b) RETURN b.id",
            "b.id",
            "USE default MATCH (__hql1_source)-[:LINK]-(__hql1_target) AS __hql1_path WALK |> RETURN __hql1_target.id AS id",
        ),
        (
            "MATCH (a)-->(b) RETURN b.id",
            "b.id",
            "USE default MATCH (__hql1_source)-->(__hql1_target) AS __hql1_path WALK |> RETURN __hql1_target.id AS id",
        ),
        (
            "MATCH (x)<--(y) RETURN x.id",
            "x.id",
            "USE default MATCH (__hql1_source)<--(__hql1_target) AS __hql1_path WALK |> RETURN __hql1_source.id AS id",
        ),
        (
            "MATCH (a)--(b) RETURN b.id",
            "b.id",
            "USE default MATCH (__hql1_source)--(__hql1_target) AS __hql1_path WALK |> RETURN __hql1_target.id AS id",
        ),
    ];
    let expected = cases
        .iter()
        .map(|(hql, column, _)| {
            storage
                .execute_hql(hql)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row[*column].as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    install_policy(&storage, true);

    for (index, (hql, legacy_column, canonical)) in cases.into_iter().enumerate() {
        let QueryOutcomeV2::Rows(adapted) = storage
            .query_v2(actor("legacy-reader"), request(hql))
            .unwrap()
        else {
            panic!("expected actor-scoped HQL1 rows")
        };
        let QueryOutcomeV2::Rows(hql2) = storage
            .query_v2(actor("legacy-reader"), hql2_request(canonical))
            .unwrap()
        else {
            panic!("expected HQL2 baseline rows")
        };

        let mut legacy = expected[index].clone();
        let mut adapted = projected_ids(&adapted, legacy_column);
        let mut hql2 = projected_ids(&hql2, "id");
        legacy.sort();
        adapted.sort();
        hql2.sort();
        assert_eq!(adapted, legacy, "legacy differential for {hql}");
        assert_eq!(adapted, hql2, "HQL2 parity for {hql}");
    }
}

#[test]
fn actor_scoped_hql1_one_hop_id_equality_filter_matches_legacy_and_hql2() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for id in ["a", "b", "c"] {
        add_node(&storage, id);
    }
    add_edge(&storage, "ab-1", "a", "b", "LINK");
    add_edge(&storage, "ab-2", "a", "b", "LINK");
    add_edge(&storage, "ac", "a", "c", "LINK");

    let cases = [
        (
            "MATCH (a)-[:LINK]->(b) WHERE b.id = \"b\" RETURN b.id",
            "b.id",
            "USE default MATCH (__hql1_source)-[:LINK]->(__hql1_target) AS __hql1_path WALK |> FILTER __hql1_target.id = \"b\" |> RETURN __hql1_target.id AS id",
        ),
        (
            "MATCH (a)-[:LINK]->(b) WHERE a.id = \"a\" RETURN b.id",
            "b.id",
            "USE default MATCH (__hql1_source)-[:LINK]->(__hql1_target) AS __hql1_path WALK |> FILTER __hql1_source.id = \"a\" |> RETURN __hql1_target.id AS id",
        ),
        (
            "MATCH (a)-[:LINK]->(b) WHERE b.id = \"b\" RETURN a.id",
            "a.id",
            "USE default MATCH (__hql1_source)-[:LINK]->(__hql1_target) AS __hql1_path WALK |> FILTER __hql1_target.id = \"b\" |> RETURN __hql1_source.id AS id",
        ),
    ];
    let expected = cases
        .iter()
        .map(|(query, column, _)| {
            storage
                .execute_hql(query)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row[*column].as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(expected[0], ["b", "b"]);
    let mut start_filter_rows = expected[1].clone();
    start_filter_rows.sort();
    assert_eq!(start_filter_rows, ["b", "b", "c"]);
    assert_eq!(expected[2], ["a", "a"]);
    install_policy(&storage, true);

    for (index, (legacy_query, legacy_column, canonical)) in cases.into_iter().enumerate() {
        let QueryOutcomeV2::Rows(hql2) = storage
            .query_v2(actor("legacy-reader"), hql2_request(canonical))
            .unwrap()
        else {
            panic!("expected canonical HQL2 rows")
        };
        let QueryOutcomeV2::Rows(adapted) = storage
            .query_v2(actor("legacy-reader"), request(legacy_query))
            .unwrap()
        else {
            panic!("expected actor-scoped HQL1 rows")
        };
        let mut legacy = expected[index].clone();
        let mut hql2 = projected_ids(&hql2, "id");
        let mut adapted = projected_ids(&adapted, legacy_column);
        legacy.sort();
        hql2.sort();
        adapted.sort();
        assert_eq!(hql2, legacy, "canonical differential for {legacy_query}");
        assert_eq!(adapted, legacy, "adapter differential for {legacy_query}");
    }
}

#[test]
fn hql1_namespace_read_denial_precedes_legacy_parse() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    install_policy(&storage, false);

    let error = storage
        .query_v2(actor("intruder"), request("MATCH ("))
        .unwrap_err();
    assert_eq!(error.code, "FORBIDDEN");
    assert_eq!(error.stage, "authorize");
}

#[test]
fn hql1_request_namespace_mismatch_precedes_legacy_parse() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    install_policy(&storage, true);
    let mut request = request("MATCH (");
    request.namespace = "other".into();

    let error = storage
        .query_v2(actor("legacy-reader"), request)
        .unwrap_err();
    assert_eq!(error.code, "FORBIDDEN");
    assert_eq!(error.stage, "authorize");
}

#[test]
fn authorized_malformed_hql1_reports_parse_error() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    install_policy(&storage, true);

    let error = storage
        .query_v2(actor("legacy-reader"), request("MATCH ("))
        .unwrap_err();
    assert_eq!(error.code, "HQL_PARSE_ERROR");
    assert_eq!(error.stage, "parse");
}

#[test]
fn valid_unlisted_hql1_form_fails_closed() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    install_policy(&storage, true);

    let error = storage
        .query_v2(
            actor("legacy-reader"),
            request("TRAVERSE FROM node:a DEPTH 1 REL LINK"),
        )
        .unwrap_err();
    assert_eq!(error.code, "CAPABILITY_UNSUPPORTED");

    for query in [
        "MATCH (a:Person) WHERE a.id = \"a\" RETURN a.id",
        "MATCH (a) WHERE a.id != \"a\" RETURN a.id",
        "MATCH (a) WHERE a.id = 1 RETURN a.id",
        "MATCH (a) WHERE b.id = \"a\" RETURN a.id",
        "MATCH (a) WHERE a.id = \"a\" AND a.id = \"b\" RETURN a.id",
        "MATCH (a) WHERE a.id = \"a\" ORDER BY a.id RETURN a.id",
        "MATCH (a) WHERE a.id = \"a\" RETURN a.label",
        "MATCH (a)-[r:LINK]->(b) RETURN b.id",
        "MATCH (a:Person)-[:LINK]->(b) RETURN b.id",
        "MATCH (a)-[:LINK]->(b:Person) RETURN b.id",
        "MATCH (a)-[:LINK]->(b)-[:LINK]->(c) RETURN c.id",
        "MATCH (a)-[:LINK]->(b) RETURN b.label",
        "MATCH (a)-[:LINK]->(b) RETURN a.id, b.id",
        "MATCH (a)-[:LINK]->(b) WHERE b.id != \"b\" RETURN b.id",
        "MATCH (a)-[:LINK]->(b) WHERE b.label = \"Person\" RETURN b.id",
        "MATCH (a)-[:LINK]->(b) WHERE b.id = 1 RETURN b.id",
        "MATCH (a)-[:LINK]->(b) WHERE c.id = \"b\" RETURN b.id",
        "MATCH (a)-[:LINK]->(b) WHERE b.id = \"b\" AND a.id = \"a\" RETURN b.id",
    ] {
        let error = storage
            .query_v2(actor("legacy-reader"), request(query))
            .unwrap_err();
        assert_eq!(error.code, "CAPABILITY_UNSUPPORTED", "{query}");
    }
}

#[test]
fn broad_unlisted_hql1_pattern_is_bounded_before_legacy_ast_allowlist() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    install_policy(&storage, true);
    let source = format!("MATCH (a){} RETURN a.id", "-[:LINK]->(b)".repeat(1_000));

    let error = storage
        .query_v2(actor("legacy-reader"), request(&source))
        .unwrap_err();
    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
    assert_eq!(error.stage, "parse");
}
