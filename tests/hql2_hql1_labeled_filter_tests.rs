use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2, QueryResultV2},
    uee_v2::QueryRequestV2,
    AccessAction, AccessContext, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource,
    NodeInput, OpenOptions, PolicyAdminActor, Storage,
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

fn install_policy(storage: &Storage) {
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

fn add_node(storage: &Storage, id: &str, labels: &[&str]) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: labels.iter().map(|label| (*label).into()).collect(),
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

fn request(language: &str, query: &str) -> QueryRequestV2 {
    serde_json::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": "hql1-labeled-filter",
        "namespace": "default",
        "language_version": language,
        "hql": query,
        "params": {}
    }))
    .unwrap()
}

fn ids(result: &QueryResultV2, column: &str) -> Vec<String> {
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

fn legacy_ids(storage: &Storage, query: &str) -> Vec<String> {
    storage
        .execute_hql(query)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["p.id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn d7_labeled_zero_hop_exact_id_matches_legacy_and_canonical_hql2() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "people:ada", &["Person"]);
    add_node(&storage, "people:grace", &["Person", "Engineer"]);
    add_node(&storage, "people:\\東京", &["Person"]);
    add_node(&storage, "places:lab", &["Place"]);

    let cases = [
        (
            "people:ada",
            "MATCH (p:Person) WHERE p.id = \"people:ada\" RETURN p.id",
        ),
        (
            "people:missing",
            "MATCH (p:Person) WHERE p.id = \"people:missing\" RETURN p.id",
        ),
        (
            "people:\\東京",
            r#"MATCH (p:Person) WHERE p.id = "people:\東京" RETURN p.id"#,
        ),
        (
            "places:lab",
            "MATCH (p:Person) WHERE p.id = \"places:lab\" RETURN p.id",
        ),
    ];
    let legacy = cases
        .iter()
        .map(|(_, query)| legacy_ids(&storage, query))
        .collect::<Vec<_>>();
    install_policy(&storage);

    for ((id, hql1), mut legacy) in cases.into_iter().zip(legacy) {
        let encoded_id = serde_json::to_string(id).unwrap();
        let hql2 = format!(
            "USE default FROM NODES Person AS __hql1_node |> FILTER __hql1_node.id = {encoded_id} |> RETURN __hql1_node.id AS id"
        );
        let QueryOutcomeV2::Rows(canonical) = storage
            .query_v2(actor("legacy-reader"), request("hql.v2", &hql2))
            .unwrap()
        else {
            panic!("expected canonical HQL2 rows")
        };
        let QueryOutcomeV2::Rows(adapted) = storage
            .query_v2(actor("legacy-reader"), request("hql.v1", hql1))
            .unwrap()
        else {
            panic!("expected actor-scoped HQL1 rows")
        };

        let mut canonical = ids(&canonical, "id");
        let mut adapted = ids(&adapted, "p.id");
        legacy.sort();
        canonical.sort();
        adapted.sort();
        assert_eq!(canonical, legacy, "canonical differential for {id}");
        assert_eq!(adapted, legacy, "HQL1 differential for {id}");
    }
}

#[test]
fn d7_labeled_zero_hop_filter_rejects_unproven_shapes() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    install_policy(&storage);

    for query in [
        "MATCH (p:Person) WHERE p.id = 1 RETURN p.id",
        "MATCH (p:Person) WHERE other.id = \"people:ada\" RETURN p.id",
    ] {
        let error = storage
            .query_v2(actor("legacy-reader"), request("hql.v1", query))
            .unwrap_err();
        assert_eq!(error.code, "CAPABILITY_UNSUPPORTED", "{query}");
        assert_eq!(error.stage, "bind", "{query}");
    }
}
