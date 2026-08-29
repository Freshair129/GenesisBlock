//! Read-only SQL surface (SPEC--GENESISDB-READONLY-SQL-SURFACE).
//!
//! Every rejection case asserts the database is still readable AND unchanged
//! afterwards. "Refused" and "wrote, then errored" return the same Err unless
//! something checks, and only the second one is a corruption bug.

use genesis_block_native::{
    OpenOptions, RelationalColumn, RelationalColumnType, RelationalMutationKind,
    RelationalRowMutation, RelationalSchemaPackage, RelationalTable, Storage,
};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn fresh(name: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&path).exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

fn schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "shop".to_string(),
        schema_version: 1,
        previous_version: None,
        package_id: "00000000-0000-4000-8000-0000000000aa".to_string(),
        schema_hash: String::new(),
        tables: vec![RelationalTable {
            name: "items".to_string(),
            columns: vec![
                RelationalColumn::required("id", RelationalColumnType::Text),
                RelationalColumn::required("name", RelationalColumnType::Text),
                RelationalColumn::required("price", RelationalColumnType::Integer),
            ],
            primary_key: vec!["id".to_string()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        named_queries: vec![],
    }
}

fn seeded(path: &str) -> Storage {
    let storage = open(path);
    storage.register_relational_schema(schema()).unwrap();
    let rows = [
        ("i1", "espresso machine", 25000),
        ("i2", "coffee grinder", 8000),
        ("i3", "espresso cups", 900),
    ];
    storage
        .apply_relational_rows(
            "shop",
            rows.iter()
                .map(|(id, name, price)| RelationalRowMutation {
                    table: "items".to_string(),
                    kind: RelationalMutationKind::Upsert,
                    values: json!({"id": id, "name": name, "price": price}),
                    key: None,
                })
                .collect(),
        )
        .unwrap();
    storage
}

/// The count the database must still report after every rejected statement.
fn item_count(storage: &Storage) -> i64 {
    let rows = storage
        .query_sql("SELECT count(*) AS n FROM app_shop__items", vec![])
        .expect("counting must keep working");
    rows[0]["n"].as_i64().unwrap()
}

// 1. The whole point: things the relational IR cannot express.
#[test]
fn sql_surface_does_what_the_relational_ir_cannot() {
    let storage = seeded(&fresh("rosql_positive"));

    // `>` and ORDER BY - RelationalFilter only has equal(), and the IR has no
    // ordering at all.
    let dear = storage
        .query_sql(
            "SELECT id FROM app_shop__items WHERE price > ?1 ORDER BY price DESC",
            vec![json!(1000)],
        )
        .unwrap();
    assert_eq!(dear.len(), 2, "two items cost more than 1000");
    assert_eq!(dear[0]["id"], json!("i1"), "ORDER BY DESC puts i1 first");

    // LIKE + aggregates, none of which the IR has.
    let grouped = storage
        .query_sql(
            "SELECT count(*) AS n, sum(price) AS total FROM app_shop__items WHERE name LIKE ?1",
            vec![json!("espresso%")],
        )
        .unwrap();
    assert_eq!(grouped[0]["n"], json!(2));
    assert_eq!(grouped[0]["total"], json!(25900));
}

// 2, 3, 4 and 7 together: every forbidden statement is refused AND leaves the
// database untouched.
#[test]
fn every_write_shaped_statement_is_refused_and_changes_nothing() {
    let path = fresh("rosql_refusals");
    let storage = seeded(&path);
    let before = item_count(&storage);
    assert_eq!(before, 3);

    let forbidden = [
        "INSERT INTO app_shop__items (id, name, price) VALUES ('x', 'x', 1)",
        "UPDATE app_shop__items SET price = 0",
        "DELETE FROM app_shop__items",
        "DROP TABLE app_shop__items",
        "CREATE TABLE sneaky (a TEXT)",
        "CREATE INDEX sneaky_idx ON app_shop__items (name)",
        "ALTER TABLE app_shop__items ADD COLUMN sneaky TEXT",
        "ATTACH DATABASE 'elsewhere.db' AS other",
        "PRAGMA journal_mode = DELETE",
        "VACUUM",
    ];

    for statement in forbidden {
        let outcome = storage.query_sql(statement, vec![]);
        let reason = outcome
            .as_ref()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(
            outcome.is_err(),
            "must be refused, but it succeeded: {statement}"
        );
        // Assert WHY, not merely that it failed. A test that accepts any error
        // starts passing for the wrong reason the moment a table name is
        // mistyped - "no such table" would look exactly like the guard working.
        //
        // The accepted reasons are the three real layers, measured by disabling
        // the authorizer and recording what still caught each statement:
        // writes fall to the read-only connection, ATTACH and VACUUM to
        // SQLITE_LIMIT_ATTACHED = 0. Any OTHER error means this case stopped
        // testing what it claims to test.
        assert!(
            reason.contains("not authorized")
                || reason.contains("authorization denied")
                || reason.contains("readonly database")
                || reason.contains("too many attached databases"),
            "refused for the wrong reason: [{statement}] -> {reason}"
        );
        // The assertion that matters. A statement that wrote and *then* errored
        // reports the same Err as one that never ran.
        assert_eq!(
            item_count(&storage),
            before,
            "database changed after a refused statement: {statement}"
        );
    }

    // Nothing leaked into the schema either.
    let tables = storage
        .query_sql(
            "SELECT count(*) AS n FROM sqlite_master WHERE name = ?1",
            vec![json!("sneaky")],
        )
        .unwrap();
    assert_eq!(tables[0]["n"], json!(0), "a refused CREATE left a table");

    // And the projection survives a reopen - a write that only reached the
    // page cache would show up here.
    drop(storage);
    let reopened = open(&path);
    assert_eq!(item_count(&reopened), before);
}

// A sentinel for the authorizer specifically.
//
// Every statement in the list above is also caught by the read-only connection
// or by SQLITE_LIMIT_ATTACHED, so that test keeps passing even with the
// authorizer removed - measured, not assumed. This one cannot: `PRAGMA
// table_list` only READS, so layers 1 and 3 have no opinion about it and the
// allow-list is the sole thing standing in its way. If this test ever goes
// green after a change to the authorizer, layer 2 is gone.
#[test]
fn a_read_only_pragma_is_denied_by_the_authorizer_alone() {
    let storage = seeded(&fresh("rosql_pragma_sentinel"));

    let reason = storage
        .query_sql("PRAGMA table_list", vec![])
        .err()
        .map(|e| e.to_string())
        .expect("a read-only PRAGMA must still be refused");

    assert!(
        reason.contains("not authorized") || reason.contains("authorization denied"),
        "only the authorizer can refuse this; got: {reason}"
    );
}

// 5. A read-only query can still burn a core forever. Only the deadline stops
//    this one: count(*) emits a single row, so the row cap never fires.
#[test]
fn a_runaway_read_is_interrupted_by_the_deadline() {
    let storage = seeded(&fresh("rosql_runaway"));

    let started = Instant::now();
    let outcome = storage.query_sql(
        "WITH RECURSIVE forever(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM forever) SELECT count(*) AS n FROM forever",
        vec![],
    );
    let elapsed = started.elapsed();

    assert!(outcome.is_err(), "an endless scan must not return a result");
    assert!(
        elapsed.as_secs() < 30,
        "the deadline did not fire; took {elapsed:?}"
    );
}

// 6. Over the row cap is an ERROR, not a quietly shortened list.
#[test]
fn exceeding_the_row_cap_errors_rather_than_truncating() {
    let storage = seeded(&fresh("rosql_rowcap"));

    let outcome = storage.query_sql(
        "WITH RECURSIVE many(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM many WHERE x < 20000) SELECT x FROM many",
        vec![],
    );

    let message = outcome.expect_err("20000 rows exceeds the cap").to_string();
    assert!(
        message.contains("SQL_LIMIT_EXCEEDED"),
        "the error must name the limit, got: {message}"
    );

    // Just under the cap still works, so the cap is a boundary rather than a
    // blanket refusal of recursive queries.
    let ok = storage
        .query_sql(
            "WITH RECURSIVE some(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM some WHERE x < 100) SELECT x FROM some",
            vec![],
        )
        .unwrap();
    assert_eq!(ok.len(), 100);
}

// 8. A parameter carrying SQL is a value, never syntax.
#[test]
fn a_parameter_containing_sql_is_bound_not_executed() {
    let storage = seeded(&fresh("rosql_injection"));
    let before = item_count(&storage);

    let rows = storage
        .query_sql(
            "SELECT id FROM app_shop__items WHERE name = ?1",
            vec![json!("x'; DROP TABLE app_shop__items; --")],
        )
        .unwrap();

    assert!(rows.is_empty(), "no item is named that");
    assert_eq!(
        item_count(&storage),
        before,
        "the table survives a parameter that looks like SQL"
    );
}

// 9. SQLite compiles only the FIRST statement of its input and silently drops
//    the rest, so `SELECT ...; DROP TABLE ...` used to return the SELECT's rows
//    and an Ok. The DROP never ran - measured, with the table still present
//    afterwards - but answering Ok to a question that was only half asked is
//    the same wrong-answer-that-looks-right as a truncated result set.
//
//    The second half of this test matters as much as the first: a scanner that
//    rejects too eagerly would break every legitimate query carrying a `;`
//    inside a string, a comment or a quoted identifier.
#[test]
fn a_second_statement_is_refused_rather_than_silently_dropped() {
    let storage = seeded(&fresh("rosql_multi_statement"));
    let before = item_count(&storage);

    let refused = [
        "SELECT id FROM app_shop__items; DROP TABLE app_shop__items",
        "SELECT id FROM app_shop__items;;",
        "SELECT 1 AS c; SELECT 2 AS c",
    ];
    for statement in refused {
        let reason = storage
            .query_sql(statement, vec![])
            .err()
            .map(|e| e.to_string())
            .unwrap_or_else(|| panic!("a second statement must be refused: {statement}"));
        assert!(
            reason.contains("one statement per call"),
            "refused for the wrong reason: [{statement}] -> {reason}"
        );
    }
    assert_eq!(item_count(&storage), before, "nothing was executed");

    // A terminator with nothing meaningful after it is still one statement.
    for statement in [
        "SELECT id FROM app_shop__items;",
        "SELECT id FROM app_shop__items;   \n  ",
        "SELECT id FROM app_shop__items; -- bye",
        "SELECT id FROM app_shop__items; /* bye */",
    ] {
        assert!(
            storage.query_sql(statement, vec![]).is_ok(),
            "a bare terminator must not be read as a second statement: {statement}"
        );
    }

    // A `;` that is data, an identifier or a comment is not a terminator. The
    // Thai case also covers the byte scan against multi-byte UTF-8.
    let hidden = [
        ("SELECT ';' AS c", json!(";")),
        ("SELECT 'it''s; fine' AS c", json!("it's; fine")),
        ("SELECT /* ; */ ';' AS c", json!(";")),
        ("SELECT 'กาแฟ; เอสเปรสโซ' AS c", json!("กาแฟ; เอสเปรสโซ")),
    ];
    for (statement, expected) in hidden {
        let rows = storage
            .query_sql(statement, vec![])
            .unwrap_or_else(|e| panic!("must not be read as two statements: [{statement}] -> {e}"));
        assert_eq!(rows[0]["c"], expected, "value mangled: {statement}");
    }
    for statement in [
        "SELECT 1 AS c -- ; not a statement",
        "SELECT 1 AS \"we;ird\"",
        "SELECT 1 AS [we;ird]",
    ] {
        assert!(
            storage.query_sql(statement, vec![]).is_ok(),
            "a `;` inside a comment or a quoted identifier is not a terminator: {statement}"
        );
    }
}

// 10. Two result columns sharing a name cannot both survive into a JSON object.
//     Measured before the guard: `SELECT id, price AS id` returned
//     `{"id": 100}` - the PRICE under the name `id`. Not a field the caller
//     notices missing, but the wrong value under an expected name.
#[test]
fn duplicate_result_column_names_are_refused_rather_than_collapsed() {
    let storage = seeded(&fresh("rosql_dup_columns"));

    for statement in [
        "SELECT id, id FROM app_shop__items",
        "SELECT id, price AS id FROM app_shop__items",
    ] {
        let reason = storage
            .query_sql(statement, vec![])
            .err()
            .map(|e| e.to_string())
            .unwrap_or_else(|| panic!("a collapsing result shape must be refused: {statement}"));
        assert!(
            reason.contains("appears more than once"),
            "refused for the wrong reason: [{statement}] -> {reason}"
        );
    }

    // Aliased apart, both columns come back - so this is a guard against an
    // ambiguous shape, not a blanket refusal of repeated columns.
    let rows = storage
        .query_sql("SELECT id, price AS p FROM app_shop__items", vec![])
        .unwrap();
    assert_eq!(rows[0]["id"], json!("i1"));
    assert_eq!(rows[0]["p"], json!(25000));
}

// 11. Input carrying no statement gets a reason that says so. It already
//     failed before this, with SQLite's bare "not an error".
#[test]
fn input_with_no_statement_says_so() {
    let storage = seeded(&fresh("rosql_empty"));

    for statement in ["", "   ", "-- nothing", "/* nothing */", ";"] {
        let reason = storage
            .query_sql(statement, vec![])
            .err()
            .map(|e| e.to_string())
            .unwrap_or_else(|| panic!("empty input must be refused: {statement:?}"));
        assert!(
            reason.contains("no statement to run"),
            "refused for the wrong reason: [{statement:?}] -> {reason}"
        );
    }
}
