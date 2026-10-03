//! HQL2 table joins against the independent P7 bag interpreter and typed IR.
#[path = "support/hql2_reference.rs"]
mod reference;

use genesis_block_native::{
    query::hql2::{value::QueryValueV2 as V, QueryOutcomeV2},
    uee_v2::QueryRequestV2,
    AccessContext, NodeInput, OpenOptions, RelationalColumn, RelationalColumnType,
    RelationalMutationKind, RelationalRowMutation, RelationalSchemaPackage, RelationalTable,
    Storage,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
use tempfile::TempDir;
use uuid::Uuid;

const P7_JSON_NULL: &str = "__p7_json_null__";
const P7_JSON_NUMBER_PREFIX: &str = "__p7_json_number__:";

#[derive(Clone, Copy)]
enum JoinKey {
    Number(i64),
    Missing,
    JsonNull,
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
        principal: "reader".into(),
        namespace: "default".into(),
    }
}

fn hql_request(request_id: &str, kind: &str) -> QueryRequestV2 {
    let projection = if matches!(kind, "semi" | "anti") {
        "prop(n, \"k\") AS left_key"
    } else {
        "prop(n, \"k\") AS left_key, prop(r, \"k\") AS right_key"
    };
    let query = format!(
        "USE default FROM NODES JoinKey AS n |> {kind} JOIN TABLE right_rows AS r ON prop(n, \"k\") = prop(r, \"k\") |> RETURN {projection}"
    );
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":query,
        "params":{}
    }))
    .unwrap()
}

fn typed_ir_request(request_id: &str, kind: &str) -> QueryRequestV2 {
    let property = |alias| {
        json!({"call":"prop","args":[
            {"field":{"alias":alias,"path":[]}},
            {"type":"Utf8","literal":"k"}
        ]})
    };
    let mut nodes = vec![
        json!({"id":"left","op":"NodeScan","inputs":[],"config":{"as":"n","label":"JoinKey"}}),
        json!({"id":"right","op":"RowScan","inputs":[],"config":{"table":"right_rows","as":"r"}}),
        json!({"id":"join","op":"Join","inputs":["left","right"],"config":{
            "kind":kind,
            "condition":{"binary":"eq","left":property("n"),"right":property("r")}
        }}),
    ];
    let mut fields = vec![json!({"as":"left_key","expression":property("n")})];
    if !matches!(kind, "semi" | "anti") {
        fields.push(json!({"as":"right_key","expression":property("r")}));
    }
    nodes.push(json!({"id":"project","op":"Project","inputs":["join"],"config":{"fields":fields}}));
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":nodes,
            "root":"project",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap()
}

fn from_p7(value: &reference::Value) -> V {
    match value {
        reference::Value::Null => V::Null,
        reference::Value::Text(value) if value == P7_JSON_NULL => V::Json(Value::Null),
        reference::Value::Text(value) if value.starts_with(P7_JSON_NUMBER_PREFIX) => {
            let number = value[P7_JSON_NUMBER_PREFIX.len()..]
                .parse::<i64>()
                .expect("P7 JSON-number sentinel is generated locally");
            V::Json(json!(number))
        }
        other => panic!("unexpected P7 HQL join value: {other:?}"),
    }
}

fn p7_value(key: JoinKey) -> reference::Value {
    match key {
        JoinKey::Number(value) => reference::Value::Text(format!("{P7_JSON_NUMBER_PREFIX}{value}")),
        JoinKey::Missing => reference::Value::Null,
        // P7 has no JSON value variant; reserve a text sentinel for JSON null.
        JoinKey::JsonNull => reference::Value::Text(P7_JSON_NULL.into()),
    }
}

fn expected_rows(
    left: &[JoinKey],
    right: &[JoinKey],
    kind: reference::JoinKind,
) -> Vec<BTreeMap<String, V>> {
    use reference::{Expr as E, Plan as P};

    let values = |field: &str, values: &[JoinKey]| {
        P::Values(
            values
                .iter()
                .map(|value| BTreeMap::from([(field.into(), p7_value(*value))]))
                .collect(),
        )
    };
    let join = P::Join {
        left: Box::new(values("n.k", left)),
        right: Box::new(values("r.k", right)),
        on: E::Eq(
            Box::new(E::Field("n.k".into())),
            Box::new(E::Field("r.k".into())),
        ),
        kind,
        right_fields: vec!["r.k".into()],
    };
    let fields = if matches!(kind, reference::JoinKind::Semi | reference::JoinKind::Anti) {
        vec![("left_key".into(), E::Field("n.k".into()))]
    } else {
        vec![
            ("left_key".into(), E::Field("n.k".into())),
            ("right_key".into(), E::Field("r.k".into())),
        ]
    };
    reference::execute(&P::Project(Box::new(join), fields))
        .unwrap()
        .iter()
        .map(|row| {
            row.iter()
                .map(|(key, value)| (key.clone(), from_p7(value)))
                .collect()
        })
        .collect()
}

fn sorted_rows(rows: Vec<BTreeMap<String, V>>) -> Vec<String> {
    let mut rows = rows
        .into_iter()
        .map(|row| format!("{row:?}"))
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn storage_hql_join_kinds_match_typed_ir_and_independent_p7() {
    let directory = TempDir::new().unwrap();
    let storage = open(directory.path());
    storage
        .register_relational_schema(RelationalSchemaPackage {
            namespace: "default".into(),
            schema_version: 1,
            previous_version: None,
            package_id: Uuid::new_v4().to_string(),
            schema_hash: String::new(),
            tables: vec![RelationalTable {
                name: "right_rows".into(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::Text),
                    RelationalColumn {
                        name: "k".into(),
                        column_type: RelationalColumnType::Json,
                        nullable: true,
                        default: None,
                    },
                ],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            }],
            named_queries: vec![],
        })
        .unwrap();

    let left = [
        JoinKey::Number(1),
        JoinKey::Number(1),
        JoinKey::Missing,
        JoinKey::Number(2),
        JoinKey::JsonNull,
    ];
    let right = [
        JoinKey::Number(1),
        JoinKey::Number(1),
        JoinKey::JsonNull,
        JoinKey::Number(99),
    ];
    for (index, key) in left.iter().enumerate() {
        let props = match key {
            JoinKey::Number(value) => json!({"k":value}),
            JoinKey::Missing => json!({}),
            JoinKey::JsonNull => json!({"k":null}),
        };
        storage
            .add_node(NodeInput {
                id: Some(format!("join-left-{index}")),
                labels: vec!["JoinKey".into()],
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
    storage
        .apply_relational_rows(
            "default",
            right
                .iter()
                .enumerate()
                .map(|(index, value)| RelationalRowMutation {
                    table: "right_rows".into(),
                    kind: RelationalMutationKind::Insert,
                    values: json!({
                        "id":format!("right-{index}"),
                        "k":match value {
                            JoinKey::Number(value) => json!(value),
                            JoinKey::Missing | JoinKey::JsonNull => Value::Null,
                        }
                    }),
                    key: None,
                })
                .collect(),
        )
        .unwrap();

    for (index, query) in [
        "USE default FROM NODES JoinKey AS n |> RETURN prop(n, \"k\") AS k",
        "USE default FROM TABLE right_rows AS r |> RETURN prop(r, \"k\") AS k",
    ]
    .into_iter()
    .enumerate()
    {
        let QueryOutcomeV2::Rows(result) = storage
            .query_v2(
                access(),
                serde_json::from_value(json!({
                    "contract_version":"genesis.api.v2",
                    "request_id":format!("null-source-{index}"),
                    "namespace":"default",
                    "language_version":"hql.v2",
                    "hql":query,
                    "params":{}
                }))
                .unwrap(),
            )
            .unwrap()
        else {
            panic!("source property query should return rows")
        };
        if query.contains("NODES") {
            assert!(result.rows.iter().any(|row| row["k"] == V::Null));
            assert!(result
                .rows
                .iter()
                .any(|row| row["k"] == V::Json(Value::Null)));
        } else {
            assert!(result
                .rows
                .iter()
                .any(|row| row["k"] == V::Json(Value::Null)));
        }
    }

    for (kind, oracle_kind, expected_count) in [
        ("inner", reference::JoinKind::Inner, 5),
        ("left", reference::JoinKind::Left, 7),
        ("semi", reference::JoinKind::Semi, 3),
        ("anti", reference::JoinKind::Anti, 2),
    ] {
        let expected = sorted_rows(expected_rows(&left, &right, oracle_kind));
        let QueryOutcomeV2::Rows(hql) = storage
            .query_v2(access(), hql_request(&format!("hql-join-{kind}"), kind))
            .unwrap()
        else {
            panic!("HQL join should return rows for {kind}");
        };
        let QueryOutcomeV2::Rows(ir) = storage
            .query_v2(access(), typed_ir_request(&format!("ir-join-{kind}"), kind))
            .unwrap()
        else {
            panic!("typed-IR join should return rows for {kind}");
        };

        assert_eq!(hql.rows.len(), expected_count, "{kind} P7 row count");
        assert_eq!(
            sorted_rows(hql.rows.clone()),
            expected,
            "{kind} differs from P7"
        );
        assert_eq!(
            sorted_rows(hql.rows),
            sorted_rows(ir.rows),
            "{kind} differs from typed IR"
        );
    }

    let QueryOutcomeV2::Rows(bare_join) = storage
        .query_v2(access(), hql_request("hql-bare-join", ""))
        .unwrap()
    else {
        panic!("bare JOIN should return rows")
    };
    assert_eq!(
        sorted_rows(bare_join.rows),
        sorted_rows(expected_rows(&left, &right, reference::JoinKind::Inner)),
        "bare JOIN must default to INNER"
    );

    for kind in ["semi", "anti"] {
        let query = format!(
            "USE default FROM NODES JoinKey AS n |> {kind} JOIN TABLE right_rows AS r ON prop(n, \"k\") = prop(r, \"k\") |> RETURN prop(r, \"k\") AS leaked"
        );
        let error = storage
            .query_v2(
                access(),
                serde_json::from_value(json!({
                    "contract_version":"genesis.api.v2",
                    "request_id":format!("{kind}-right-scope"),
                    "namespace":"default",
                    "language_version":"hql.v2",
                    "hql":query,
                    "params":{}
                }))
                .unwrap(),
            )
            .unwrap_err();
        assert_eq!(error.code, "BIND_ERROR", "{kind} must hide right scope");
    }
}
