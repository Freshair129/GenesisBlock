use genesis_block_native::{
    query::hql2::{QueryOutcomeV2, QueryResultV2},
    uee_v2::QueryRequestV2,
    AccessContext, NodeInput, OpenOptions, Storage,
};
use rusqlite::Connection;
use serde_json::{json, Value};
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
        principal: "history-change-reader".into(),
        namespace: "default".into(),
    }
}

fn run_hql(
    storage: &Storage,
    request_id: &str,
    hql: &str,
) -> Result<QueryResultV2, genesis_block_native::query::hql2::QueryErrorV2> {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":hql,
        "params":{}
    }))
    .unwrap();
    match storage.query_v2(access(), request)? {
        QueryOutcomeV2::Rows(result) => Ok(result),
        QueryOutcomeV2::Plan(_) => panic!("read query must return rows"),
    }
}

fn run_hql_with_budget(
    storage: &Storage,
    request_id: &str,
    hql: &str,
    budget: Value,
) -> Result<QueryResultV2, genesis_block_native::query::hql2::QueryErrorV2> {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":hql,
        "params":{},
        "budget":budget
    }))
    .unwrap();
    match storage.query_v2(access(), request)? {
        QueryOutcomeV2::Rows(result) => Ok(result),
        QueryOutcomeV2::Plan(_) => panic!("read query must return rows"),
    }
}

fn run_ir_history(storage: &Storage, request_id: &str, valid_at: &str) -> QueryResultV2 {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "temporal":{"valid_at":valid_at},
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{
                "id":"history",
                "op":"HistoryScan",
                "inputs":[],
                "config":{
                    "kind":"node",
                    "id":{"literal":"history:one","type":"Utf8"},
                    "as":"h"
                }
            }],
            "root":"history",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    match storage.query_v2(access(), request).unwrap() {
        QueryOutcomeV2::Rows(result) => result,
        QueryOutcomeV2::Plan(_) => panic!("read query must return rows"),
    }
}

fn run_ir_changes(storage: &Storage, request_id: &str, after_seq: u64) -> QueryResultV2 {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{
                "id":"changes",
                "op":"ChangeScan",
                "inputs":[],
                "config":{"after_seq":after_seq.to_string(),"as":"c"}
            }],
            "root":"changes",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    match storage.query_v2(access(), request).unwrap() {
        QueryOutcomeV2::Rows(result) => result,
        QueryOutcomeV2::Plan(_) => panic!("read query must return rows"),
    }
}

fn add_node(storage: &Storage) {
    storage
        .add_node(NodeInput {
            id: Some("history:one".into()),
            labels: vec!["Document".into()],
            props: Some(json!({"title":"original"})),
            embedding: None,
            lang: None,
            valid_from: Some("2010-01-01T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn tagged(
    row: &std::collections::BTreeMap<
        String,
        genesis_block_native::query::hql2::value::QueryValueV2,
    >,
    field: &str,
) -> Value {
    serde_json::to_value(&row[field]).unwrap()
}

#[test]
fn history_scan_reads_closed_revisions_and_hydrates_the_exact_revision() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    storage
        .supersede_node(
            "history:one".into(),
            Some(json!({"title":"replacement"})),
            None,
        )
        .unwrap();

    let result = run_hql(
        &storage,
        "history-closed-revision",
        "USE default AT VALID \"2020-01-01T00:00:00Z\" HISTORY NODE \"history:one\" AS h |> ORDER BY h.tx_from ASC NULLS LAST |> RETURN h.tx_from AS tx, h.tx_to AS tx_to, h.operation AS operation, h.revision_id AS revision, prop(h.subject, \"title\") AS title",
    )
    .expect("HistoryScan should enumerate retained closed revisions");

    assert_eq!(result.rows.len(), 2);
    let first = &result.rows[0];
    let second = &result.rows[1];
    for row in [first, second] {
        assert_eq!(tagged(row, "tx")["type"], "DecimalU64");
        assert_eq!(tagged(row, "operation")["value"], "upsert");
        assert_eq!(tagged(row, "title")["value"], "original");
        assert!(!tagged(row, "revision")["value"]
            .as_str()
            .unwrap()
            .is_empty());
    }
    assert_eq!(tagged(first, "title"), tagged(second, "title"));
    assert_ne!(tagged(first, "revision"), tagged(second, "revision"));
    assert!(
        tagged(first, "tx")["value"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            < tagged(second, "tx")["value"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
    );
    assert_ne!(tagged(first, "tx_to")["type"], "Null");
    assert_ne!(tagged(second, "tx_to")["type"], "Null");

    let hql_typed = run_hql(
        &storage,
        "history-hql-ir-parity-hql",
        "USE default AT VALID \"2020-01-01T00:00:00Z\" HISTORY NODE \"history:one\" AS h |> RETURN h",
    )
    .unwrap();
    let ir_typed = run_ir_history(&storage, "history-hql-ir-parity-ir", "2020-01-01T00:00:00Z");
    assert_eq!(hql_typed.columns, ir_typed.columns);
    assert_eq!(hql_typed.rows, ir_typed.rows);
}

#[test]
fn history_scan_includes_retract_tombstones_without_user_properties() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let current = storage
        .supersede_node("history:one".into(), Some(json!({"title":"current"})), None)
        .unwrap();
    storage.retract_node("history:one").unwrap();

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let (from, to): (String, String) = projection
        .query_row(
            "SELECT valid_from, valid_to FROM hql2_record_revisions
             WHERE kind='node' AND record_id='history:one' AND operation='retract'
             ORDER BY tx_from DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let from = chrono::DateTime::parse_from_rfc3339(&from).unwrap();
    let to = chrono::DateTime::parse_from_rfc3339(&to).unwrap();
    let valid_at = (from + (to - from) / 2).to_rfc3339();
    assert!(chrono::DateTime::parse_from_rfc3339(&current.valid_from).unwrap() < to);

    let query = format!(
        "USE default AT VALID \"{valid_at}\" HISTORY NODE \"history:one\" AS h |> RETURN h.operation AS operation, prop(h.subject, \"title\") AS title, has_prop(h.subject, \"title\") AS has_title"
    );
    let result = run_hql(&storage, "history-tombstone", &query)
        .expect("HistoryScan should include the retained retract row");
    let tombstone = result
        .rows
        .iter()
        .find(|row| tagged(row, "operation")["value"] == "retract")
        .expect("retract metadata should be present");
    assert_eq!(tagged(tombstone, "title")["type"], "Null");
    assert_eq!(tagged(tombstone, "has_title")["value"], false);
}

#[test]
fn change_scan_emits_ordered_revision_events_with_typed_sequences_and_operations() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let after = storage.stable_frontier();
    add_node(&storage);
    storage
        .supersede_node(
            "history:one".into(),
            Some(json!({"title":"replacement"})),
            None,
        )
        .unwrap();
    storage.retract_node("history:one").unwrap();

    let query = format!(
        "USE default CHANGES SINCE {after} AS c |> ORDER BY c.sequence ASC NULLS LAST |> RETURN c.subject AS subject, c.sequence AS sequence, c.operation AS operation"
    );
    let result = run_hql(&storage, "change-events", &query)
        .expect("ChangeScan should emit retained revisions after the cursor");

    assert_eq!(result.rows.len(), 4);
    let operations: Vec<_> = result
        .rows
        .iter()
        .map(|row| {
            tagged(row, "operation")["value"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(operations, ["upsert", "correct", "correct", "retract"]);
    let sequences: Vec<_> = result
        .rows
        .iter()
        .map(|row| {
            let value = tagged(row, "sequence");
            assert_eq!(value["type"], "DecimalU64");
            assert_eq!(tagged(row, "subject")["value"]["id"], "history:one");
            value["value"].as_str().unwrap().parse::<u64>().unwrap()
        })
        .collect();
    assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(result
        .rows
        .iter()
        .all(|row| tagged(row, "subject")["type"] == "Entity"));

    let hql_result = run_hql(
        &storage,
        "change-hql-ir-parity-hql",
        &format!("USE default CHANGES SINCE {after} AS c |> RETURN c"),
    )
    .unwrap();
    let ir_result = run_ir_changes(&storage, "change-hql-ir-parity-ir", after);
    assert_eq!(hql_result.columns, ir_result.columns);
    assert_eq!(hql_result.rows, ir_result.rows);
}

#[test]
fn hql_change_scan_accepts_decimal_u64_floor_and_take_parameters() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let floor = storage.stable_frontier();
    add_node(&storage);
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"hql-change-u64-parameters",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default CHANGES SINCE $floor AS c |> TAKE $limit |> RETURN c.sequence AS sequence",
        "params":{
            "floor":{"type":"DecimalU64","value":floor.to_string()},
            "limit":{"type":"DecimalU64","value":"1"}
        }
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("read query must return rows")
    };
    assert_eq!(result.rows.len(), 1);
    assert!(matches!(
        result.rows[0]["sequence"],
        genesis_block_native::query::hql2::value::QueryValueV2::DecimalU64(_)
    ));

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"hql-change-u64-wrong-type",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default CHANGES SINCE $floor AS c |> RETURN c.sequence AS sequence",
        "params":{"floor":{"type":"I64","value":"0"}}
    }))
    .unwrap();
    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "parameter_type_mismatch");
}

#[test]
fn change_scan_floor_is_an_exclusive_cursor_and_equality_is_valid() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let floor = storage.stable_frontier();
    assert!(floor > 0);
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    projection
        .execute(
            "UPDATE hql2_source_history_floors SET history_floor=?1
             WHERE namespace='default' AND source IN ('row','vector','annotation')",
            [i64::try_from(floor).unwrap()],
        )
        .unwrap();

    let below = run_hql(
        &storage,
        "change-below-floor",
        &format!("USE default CHANGES SINCE {} AS c |> RETURN c", floor - 1),
    )
    .unwrap_err();
    assert_eq!(below.code, "HISTORY_UNAVAILABLE");

    let at_floor = run_hql(
        &storage,
        "change-at-floor",
        &format!("USE default CHANGES SINCE {floor} AS c |> RETURN c"),
    )
    .unwrap();
    assert!(at_floor.rows.is_empty());
}

#[test]
fn change_scan_includes_owner_bound_vector_revision_events() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let after = storage.stable_frontier();
    storage
        .add_node(NodeInput {
            id: Some("history:vector-owner".into()),
            labels: vec!["Document".into()],
            props: None,
            embedding: Some(vec![0.125, 0.25]),
            lang: None,
            valid_from: Some("2010-01-01T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: Some("default".into()),
        })
        .unwrap();

    let result = run_hql(
        &storage,
        "change-vector-owner",
        &format!("USE default CHANGES SINCE {after} AS c |> RETURN c"),
    )
    .unwrap();
    let kinds: Vec<_> = result
        .rows
        .iter()
        .map(|row| {
            tagged(row, "c")["value"]["subject"]["kind"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(kinds, ["node", "vector"]);
}

#[test]
fn history_scan_rejects_a_source_floor_beyond_the_pinned_frontier() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let floor = storage.stable_frontier() + 1;
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    projection
        .execute(
            "UPDATE hql2_source_history_floors SET history_floor=?1
             WHERE namespace='default' AND source='annotation'",
            [i64::try_from(floor).unwrap()],
        )
        .unwrap();

    let error = run_hql(
        &storage,
        "history-floor-beyond-snapshot",
        "USE default HISTORY ANNOTATION \"missing:annotation\" AS h |> RETURN h",
    )
    .unwrap_err();
    assert_eq!(error.code, "HISTORY_UNAVAILABLE");
}

#[test]
fn history_artifact_source_and_unknown_change_families_fail_closed() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage);
    let artifact_error = run_hql(
        &storage,
        "history-artifact-unavailable",
        "USE default HISTORY ARTIFACT \"artifact:one\" AS h |> RETURN h",
    )
    .unwrap_err();
    assert_eq!(artifact_error.code, "CAPABILITY_UNSUPPORTED");

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let (database_id, sequence): (String, i64) = projection
        .query_row(
            "SELECT database_id, tx_from FROM hql2_record_revisions
             WHERE kind='node' ORDER BY tx_from LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    projection
        .execute(
            "INSERT INTO hql2_record_revisions(
                 database_id, namespace, kind, record_id, revision_id, predecessor_revision_id,
                 operation, valid_from, valid_to, tx_from, tx_to, schema_ref, schema_version,
                 origin_database_id, payload_json
             ) VALUES (?1, 'default', 'future_family', 'future:one', 'future-revision', NULL,
                       'upsert', '2000-01-01T00:00:00Z', NULL, ?2, NULL, NULL, NULL, ?1, '{}')",
            rusqlite::params![database_id, sequence],
        )
        .unwrap();

    let error = run_hql(
        &storage,
        "change-unknown-family",
        "USE default CHANGES SINCE 0 AS c |> RETURN c",
    )
    .unwrap_err();
    assert_eq!(error.code, "CAPABILITY_UNSUPPORTED");
}

#[test]
fn change_scan_rejects_a_cursor_beyond_the_pinned_frontier() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let error = run_hql(
        &storage,
        "change-invalid-cursor",
        &format!("USE default CHANGES SINCE {} AS c |> RETURN c", u64::MAX),
    )
    .unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.stage, "bind");
    assert_eq!(
        error
            .detail
            .as_ref()
            .and_then(|detail| detail.get("reason")),
        Some(&json!("invalid_change_bounds"))
    );
}

#[test]
fn history_and_change_scans_fail_without_partial_rows_when_source_budget_is_exhausted() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let after = storage.stable_frontier();
    add_node(&storage);
    storage
        .supersede_node(
            "history:one".into(),
            Some(json!({"title":"replacement"})),
            None,
        )
        .unwrap();

    let change_error = run_hql_with_budget(
        &storage,
        "change-source-budget",
        &format!("USE default CHANGES SINCE {after} AS c |> RETURN c"),
        json!({"max_expanded_nodes":1}),
    )
    .unwrap_err();
    assert_eq!(change_error.code, "QUERY_BUDGET_EXCEEDED");

    let history_error = run_hql_with_budget(
        &storage,
        "history-source-budget",
        "USE default HISTORY NODE \"history:one\" AS h |> RETURN h",
        json!({"max_expanded_nodes":1}),
    )
    .unwrap_err();
    assert_eq!(history_error.code, "QUERY_BUDGET_EXCEEDED");
}
