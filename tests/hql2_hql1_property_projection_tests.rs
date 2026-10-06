use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2, QueryResultV2},
    uee_v2::QueryRequestV2,
    AccessContext, NodeInput, OpenOptions, Storage,
};
use serde_json::{json, Value};
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

fn add_node(storage: &Storage, id: &str, labels: &[&str], props: Value) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: labels.iter().map(|label| (*label).to_owned()).collect(),
            props: Some(props),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn actor() -> AccessContext {
    AccessContext {
        principal: "property-reader".into(),
        namespace: "default".into(),
    }
}

fn request(query: &str, language_version: &str) -> QueryRequestV2 {
    serde_json::from_value(json!({
        "contract_version": "genesis.api.v2",
        "request_id": "hql1-property-projection",
        "namespace": "default",
        "language_version": language_version,
        "hql": query,
        "params": {}
    }))
    .unwrap()
}

fn json_values(result: &QueryResultV2, column: &str) -> Vec<Value> {
    assert_eq!(result.columns.len(), 1);
    assert_eq!(result.columns[0].name, column);
    result
        .rows
        .iter()
        .map(|row| match row.get(column).unwrap() {
            QueryValueV2::Json(value) => value.clone(),
            QueryValueV2::Null => Value::Null,
            value => panic!("expected JSON property value, got {value:?}"),
        })
        .collect()
}

fn sorted_json(values: Vec<Value>) -> Vec<String> {
    let mut encoded = values
        .into_iter()
        .map(|value| serde_json::to_string(&value).unwrap())
        .collect::<Vec<_>>();
    encoded.sort();
    encoded
}

#[test]
fn actor_scoped_hql1_zero_hop_property_projection_matches_legacy_and_hql2() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(
        &storage,
        "node:a",
        &[],
        json!({
            "score": 7,
            "name": "Ada",
            "profile": {"language": "th"},
            "explicit_null": null
        }),
    );
    add_node(
        &storage,
        "node:b",
        &[],
        json!({"score": 7, "name": "Ada", "profile": null}),
    );
    add_node(&storage, "node:c", &[], json!({"name": "missing-score"}));

    for (property, expected) in [
        ("score", vec![json!(7), json!(7), Value::Null]),
        (
            "name",
            vec![json!("Ada"), json!("Ada"), json!("missing-score")],
        ),
        (
            "profile",
            vec![json!({"language": "th"}), Value::Null, Value::Null],
        ),
        ("explicit_null", vec![Value::Null, Value::Null, Value::Null]),
    ] {
        let legacy_query = format!("MATCH (n) RETURN n.prop.{property}");
        let legacy = storage
            .execute_hql(&legacy_query)
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row[format!("n.{property}")].clone())
            .collect::<Vec<_>>();

        let key = serde_json::to_string(property).unwrap();
        let hql2_query = format!("USE default FROM NODES AS n |> RETURN prop(n, {key}) AS id");
        let QueryOutcomeV2::Rows(hql2) = storage
            .query_v2(actor(), request(&hql2_query, "hql.v2"))
            .unwrap()
        else {
            panic!("expected canonical HQL2 rows")
        };
        let hql2 = json_values(&hql2, "id");

        assert_eq!(sorted_json(legacy.clone()), sorted_json(expected.clone()));
        assert_eq!(sorted_json(hql2), sorted_json(expected));

        let QueryOutcomeV2::Rows(hql1) = storage
            .query_v2(actor(), request(&legacy_query, "hql.v1"))
            .unwrap()
        else {
            panic!("expected actor-scoped HQL1 rows")
        };
        assert_eq!(
            sorted_json(json_values(&hql1, &format!("n.{property}"))),
            sorted_json(legacy)
        );
    }
}

#[test]
fn actor_scoped_hql1_labeled_zero_hop_property_projection_matches_legacy_and_hql2() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for (id, labels, props) in [
        ("person:a", &["Person"][..], json!({"score": 7})),
        ("person:b", &["Person"][..], json!({"score": 7})),
        ("person:c", &["Person"][..], json!({"name": "missing"})),
        ("person:d", &["Person"][..], json!({"score": null})),
        ("place:a", &["Place"][..], json!({"score": 7})),
    ] {
        add_node(&storage, id, labels, props);
    }

    for (label, expected) in [
        ("Person", vec![json!(7), json!(7), Value::Null, Value::Null]),
        ("Missing", Vec::new()),
    ] {
        let legacy_query = format!("MATCH (n:{label}) RETURN n.prop.score");
        let legacy = storage
            .execute_hql(&legacy_query)
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["n.score"].clone())
            .collect::<Vec<_>>();

        let hql2_query =
            format!("USE default FROM NODES {label} AS n |> RETURN prop(n, \"score\") AS id");
        let QueryOutcomeV2::Rows(hql2) = storage
            .query_v2(actor(), request(&hql2_query, "hql.v2"))
            .unwrap()
        else {
            panic!("expected canonical HQL2 rows")
        };
        let QueryOutcomeV2::Rows(hql1) = storage
            .query_v2(actor(), request(&legacy_query, "hql.v1"))
            .unwrap()
        else {
            panic!("expected actor-scoped HQL1 rows")
        };

        assert_eq!(sorted_json(legacy.clone()), sorted_json(expected.clone()));
        assert_eq!(
            sorted_json(json_values(&hql1, "n.score")),
            sorted_json(legacy)
        );
        assert_eq!(sorted_json(json_values(&hql2, "id")), sorted_json(expected));
    }
}

#[test]
fn actor_scoped_hql1_property_projection_keeps_other_shapes_fail_closed() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "node:a", &[], json!({"score": 7}));

    for query in [
        "MATCH (n) WHERE n.id = \"node:a\" RETURN n.prop.score",
        "MATCH (n) RETURN n.prop.score, n.id",
    ] {
        let error = storage
            .query_v2(actor(), request(query, "hql.v1"))
            .unwrap_err();
        assert_eq!(error.code, "CAPABILITY_UNSUPPORTED", "{query}");
    }
}
