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

fn request(language: &str, query: &str) -> QueryRequestV2 {
    serde_json::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": format!("hql1-order-{language}"),
        "namespace": "default",
        "language_version": language,
        "hql": query,
        "params": {}
    }))
    .unwrap()
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

fn add_edge(storage: &Storage, id: &str, from: &str, to: &str) {
    storage
        .add_edge(EdgeInput {
            id: Some(id.into()),
            from: from.into(),
            to: to.into(),
            rel: "LINK".into(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
}

fn install_reader(storage: &Storage) {
    storage
        .replace_access_policy(
            PolicyAdminActor {
                access: actor("local-owner"),
            },
            0,
            AccessPolicy {
                revision: 1,
                mode: AccessPolicyMode::Enforced,
                grants: vec![
                    AccessGrant {
                        principal: "local-owner".into(),
                        action: AccessAction::ManagePolicy,
                        resource: AccessResource::Namespace("default".into()),
                    },
                    AccessGrant {
                        principal: "legacy-reader".into(),
                        action: AccessAction::Read,
                        resource: AccessResource::Namespace("default".into()),
                    },
                ],
            },
        )
        .unwrap();
}

fn ids(result: &genesis_block_native::query::hql2::QueryResultV2, column: &str) -> Vec<String> {
    assert_eq!(result.columns.len(), 1);
    assert_eq!(result.columns[0].name, column);
    result
        .rows
        .iter()
        .map(|row| match row.get(column).unwrap() {
            QueryValueV2::Utf8(id) => id.clone(),
            value => panic!("expected UTF-8 ID, got {value:?}"),
        })
        .collect()
}

#[test]
fn actor_scoped_hql1_one_hop_order_by_matches_legacy_and_hql2() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for id in ["source:z", "source:a", "target:m", "target:b", "target:z"] {
        add_node(&storage, id);
    }
    add_edge(&storage, "edge-1", "source:z", "target:m");
    add_edge(&storage, "edge-2", "source:a", "target:z");
    add_edge(&storage, "edge-3", "source:z", "target:b");
    add_edge(&storage, "edge-4", "source:a", "target:b");

    let cases = [
        (
            "MATCH (a)-[:LINK]->(b) ORDER BY b.id RETURN b.id",
            "USE default MATCH (__hql1_source)-[:LINK]->(__hql1_target) AS __hql1_path WALK |> ORDER BY __hql1_target.id ASC |> RETURN __hql1_target.id AS id",
            "b.id",
            "id",
            vec!["target:b", "target:b", "target:m", "target:z"],
        ),
        (
            "MATCH (a)-[:LINK]->(b) ORDER BY b.id ASC RETURN b.id",
            "USE default MATCH (__hql1_source)-[:LINK]->(__hql1_target) AS __hql1_path WALK |> ORDER BY __hql1_target.id ASC |> RETURN __hql1_target.id AS id",
            "b.id",
            "id",
            vec!["target:b", "target:b", "target:m", "target:z"],
        ),
        (
            "MATCH (a)-[:LINK]->(b) ORDER BY b.id DESC RETURN b.id",
            "USE default MATCH (__hql1_source)-[:LINK]->(__hql1_target) AS __hql1_path WALK |> ORDER BY __hql1_target.id DESC |> RETURN __hql1_target.id AS id",
            "b.id",
            "id",
            vec!["target:z", "target:m", "target:b", "target:b"],
        ),
        (
            "MATCH (a)-[:LINK]->(b) ORDER BY a.id DESC RETURN a.id",
            "USE default MATCH (__hql1_source)-[:LINK]->(__hql1_target) AS __hql1_path WALK |> ORDER BY __hql1_source.id DESC |> RETURN __hql1_source.id AS id",
            "a.id",
            "id",
            vec!["source:z", "source:z", "source:a", "source:a"],
        ),
    ];

    let expected = cases
        .iter()
        .map(|(legacy, _, column, _, _)| {
            storage
                .execute_hql(legacy)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row[*column].as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    install_reader(&storage);

    for (i, (legacy, canonical, legacy_column, canonical_column, expected_order)) in
        cases.iter().enumerate()
    {
        assert_eq!(expected[i], *expected_order, "legacy fixture: {legacy}");
        let QueryOutcomeV2::Rows(adapted) = storage
            .query_v2(actor("legacy-reader"), request("hql.v1", legacy))
            .unwrap()
        else {
            panic!("expected actor-scoped HQL1 rows")
        };
        let QueryOutcomeV2::Rows(hql2) = storage
            .query_v2(actor("legacy-reader"), request("hql.v2", canonical))
            .unwrap()
        else {
            panic!("expected HQL2 rows")
        };
        assert_eq!(ids(&adapted, legacy_column), expected[i]);
        assert_eq!(ids(&hql2, canonical_column), expected[i]);
    }
}

#[test]
fn unprojected_or_zero_hop_order_by_fails_closed() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    install_reader(&storage);

    for query in [
        "MATCH (a)-[:LINK]->(b) ORDER BY a.id ASC RETURN b.id",
        "MATCH (a) ORDER BY a.id ASC RETURN a.id",
    ] {
        let error = storage
            .query_v2(actor("legacy-reader"), request("hql.v1", query))
            .unwrap_err();
        assert_eq!(error.code, "CAPABILITY_UNSUPPORTED", "{query}");
        assert_eq!(error.stage, "bind", "{query}");
    }
}
