use genesis_block_native::{
    NamedQueryDefinition, OpenOptions, RelationalColumn, RelationalColumnType, RelationalIndex,
    RelationalQuery, RelationalSchemaPackage, RelationalTable, Storage,
};
use std::path::Path;
use tempfile::{Builder, TempDir};

const PACKAGE_ID: &str = "00000000-0000-4000-8000-000000000901";

fn fresh(name: &str) -> TempDir {
    Builder::new()
        .prefix(name)
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap()
}

fn open(path: impl AsRef<Path>) -> Storage {
    Storage::open(OpenOptions {
        path: path.as_ref().to_string_lossy().into_owned(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

fn package(
    namespace: &str,
    tables: Vec<RelationalTable>,
    named_queries: Vec<NamedQueryDefinition>,
) -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: namespace.to_string(),
        schema_version: 1,
        previous_version: None,
        package_id: PACKAGE_ID.to_string(),
        schema_hash: String::new(),
        tables,
        named_queries,
    }
}

fn simple_table(name: String) -> RelationalTable {
    RelationalTable {
        name,
        columns: vec![RelationalColumn::required("id", RelationalColumnType::Text)],
        primary_key: vec!["id".to_string()],
        foreign_keys: vec![],
        indexes: vec![],
    }
}

fn table_package(namespace: &str, table_count: usize) -> RelationalSchemaPackage {
    package(
        namespace,
        (0..table_count)
            .map(|index| simple_table(format!("table_{index}")))
            .collect(),
        vec![],
    )
}

fn named_query_package(count: usize) -> RelationalSchemaPackage {
    let namespace = "named_query_limit";
    let named_queries = (0..count)
        .map(|index| NamedQueryDefinition {
            name: format!("query_{index}"),
            parameters: vec![],
            query: RelationalQuery {
                namespace: namespace.to_string(),
                table: "base".to_string(),
                columns: vec!["base.id".to_string()],
                joins: vec![],
                filters: vec![],
                limit: Some(10),
                offset: None,
            },
            default_limit: 10,
            max_limit: 10,
        })
        .collect();
    package(
        namespace,
        vec![simple_table("base".to_string())],
        named_queries,
    )
}

fn columns_package(count: usize) -> RelationalSchemaPackage {
    let columns = (0..count)
        .map(|index| {
            RelationalColumn::required(&format!("column_{index}"), RelationalColumnType::Text)
        })
        .collect();
    package(
        "column_limit",
        vec![RelationalTable {
            name: "base".to_string(),
            columns,
            primary_key: vec!["column_0".to_string()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        vec![],
    )
}

fn primary_key_package(count: usize) -> RelationalSchemaPackage {
    let names: Vec<String> = (0..count).map(|index| format!("key_{index}")).collect();
    package(
        "primary_key_limit",
        vec![RelationalTable {
            name: "base".to_string(),
            columns: names
                .iter()
                .map(|name| RelationalColumn::required(name, RelationalColumnType::Text))
                .collect(),
            primary_key: names,
            foreign_keys: vec![],
            indexes: vec![],
        }],
        vec![],
    )
}

fn index_package(count: usize) -> RelationalSchemaPackage {
    package(
        "index_limit",
        vec![RelationalTable {
            name: "base".to_string(),
            columns: vec![RelationalColumn::required("id", RelationalColumnType::Text)],
            primary_key: vec!["id".to_string()],
            foreign_keys: vec![],
            indexes: (0..count)
                .map(|index| RelationalIndex {
                    name: format!("index_{index}"),
                    columns: vec!["id".to_string()],
                    unique: false,
                })
                .collect(),
        }],
        vec![],
    )
}

fn assert_rejected(storage: &Storage, schema: RelationalSchemaPackage, expected: &str) {
    let error = storage
        .register_relational_schema(schema)
        .expect_err("schema should be rejected");
    let text = format!("{error:?}");
    assert!(
        text.contains(expected),
        "expected error containing {expected:?}, got {text}"
    );
}

#[test]
fn table_resource_limit_accepts_64_66_128_and_rejects_129_without_commit() {
    for table_count in [64, 66, 128] {
        let path = fresh(&format!("schema_tables_{table_count}"));
        let storage = open(path.path());
        let namespace = format!("schema_tables_{table_count}");
        assert_eq!(
            storage
                .register_relational_schema(table_package(&namespace, table_count))
                .unwrap(),
            1
        );
        assert_eq!(
            storage
                .get_relational_schema(&namespace)
                .unwrap()
                .unwrap()
                .tables
                .len(),
            table_count
        );
    }

    let path = fresh("schema_tables_rejected");
    let storage = open(path.path());
    let namespace = "schema_tables_rejected";
    storage
        .register_relational_schema(table_package(namespace, 66))
        .unwrap();
    let before_schema = storage.get_relational_schema(namespace).unwrap();
    let before_stable = storage.stable_frontier();
    let before_txn = storage.txn_frontier();

    assert_rejected(
        &storage,
        table_package(namespace, 129),
        "schema resource limit exceeded",
    );

    assert_eq!(
        storage.get_relational_schema(namespace).unwrap(),
        before_schema
    );
    assert_eq!(storage.stable_frontier(), before_stable);
    assert_eq!(storage.txn_frontier(), before_txn);
}

#[test]
fn unchanged_named_query_column_primary_key_and_index_limits_remain_active() {
    for (name, schema) in [
        ("named_queries_128", named_query_package(128)),
        ("columns_128", columns_package(128)),
        ("primary_keys_4", primary_key_package(4)),
        ("indexes_64", index_package(64)),
    ] {
        let path = fresh(name);
        let storage = open(path.path());
        assert_eq!(storage.register_relational_schema(schema).unwrap(), 1);
    }

    for (name, schema) in [
        ("named_queries_129", named_query_package(129)),
        ("columns_129", columns_package(129)),
        ("primary_keys_5", primary_key_package(5)),
        ("indexes_65", index_package(65)),
    ] {
        let path = fresh(name);
        let storage = open(path.path());
        assert_rejected(&storage, schema, "resource limit");
    }
}

#[test]
fn rejected_schema_registration_preserves_sequencing_additive_and_hash_guards() {
    let path = fresh("schema_guard_integrity");
    let storage = open(path.path());
    let namespace = "schema_guard_integrity";
    let base = table_package(namespace, 1);
    storage.register_relational_schema(base.clone()).unwrap();
    let before_schema = storage.get_relational_schema(namespace).unwrap();
    let before_stable = storage.stable_frontier();
    let before_txn = storage.txn_frontier();

    let mut skipped = base.clone();
    skipped.schema_version = 3;
    skipped.previous_version = Some(1);
    assert_rejected(
        &storage,
        skipped,
        "schema version must advance exactly once",
    );

    let mut required_column = base.clone();
    required_column.schema_version = 2;
    required_column.previous_version = Some(1);
    required_column.tables[0]
        .columns
        .push(RelationalColumn::required(
            "required",
            RelationalColumnType::Text,
        ));
    assert_rejected(
        &storage,
        required_column,
        "new relational columns must be nullable",
    );

    let mut tampered_hash = base;
    tampered_hash.schema_hash = "0".repeat(64);
    assert_rejected(&storage, tampered_hash, "schema_hash mismatch");

    assert_eq!(
        storage.get_relational_schema(namespace).unwrap(),
        before_schema
    );
    assert_eq!(storage.stable_frontier(), before_stable);
    assert_eq!(storage.txn_frontier(), before_txn);
}

#[test]
fn valid_expanded_schema_persists_and_reopens() {
    let path = fresh("schema_tables_persisted");
    let namespace = "schema_tables_persisted";
    {
        let storage = open(path.path());
        storage
            .register_relational_schema(table_package(namespace, 66))
            .unwrap();
        assert_eq!(
            storage
                .get_relational_schema(namespace)
                .unwrap()
                .unwrap()
                .tables
                .len(),
            66
        );
    }

    let reopened = open(path.path());
    let schema = reopened
        .get_relational_schema(namespace)
        .unwrap()
        .expect("persisted schema should reopen");
    assert_eq!(schema.tables.len(), 66);
    assert!(!schema.schema_hash.is_empty());
}
