//! Regression for the active-row identity lookup scanning every row revision in
//! the namespace (FUNG #135). Upserting an existing row must not get slower as
//! revisions of rows in *other* tables accumulate in the same namespace.

use genesis_block_native::{
    OpenOptions, RelationalColumn, RelationalColumnType, RelationalMutationBatch,
    RelationalMutationKind, RelationalRowMutation, RelationalSchemaPackage, RelationalTable,
    Storage,
};
use serde_json::json;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "scaling_test".into(),
        schema_version: 1,
        previous_version: None,
        package_id: "550e8400-e29b-41d4-a716-446655440135".into(),
        schema_hash: String::new(),
        tables: ["records", "chunks"]
            .into_iter()
            .map(|name| RelationalTable {
                name: name.into(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::Text),
                    RelationalColumn::required("value", RelationalColumnType::Text),
                ],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            })
            .collect(),
        named_queries: vec![],
    }
}

fn apply(
    storage: &Storage,
    table: &str,
    kind: RelationalMutationKind,
    id: String,
    value: String,
) -> Duration {
    let batch = RelationalMutationBatch {
        mutation_id: uuid::Uuid::new_v4().to_string(),
        namespace: "scaling_test".into(),
        schema_version: 1,
        operations: vec![RelationalRowMutation {
            table: table.into(),
            kind,
            values: json!({"id": id, "value": value}),
            key: None,
        }],
    };
    let started = Instant::now();
    storage.apply_relational_batch(batch).unwrap();
    started.elapsed()
}

/// Times `count` upserts of the same long-lived row.
fn upserts(storage: &Storage, count: usize, round: usize) -> Duration {
    (0..count)
        .map(|i| {
            apply(
                storage,
                "records",
                RelationalMutationKind::Upsert,
                "long-lived".into(),
                format!("r{round}-{i}"),
            )
        })
        .sum()
}

#[test]
fn upsert_of_existing_row_does_not_scale_with_other_table_revisions() {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap();
    storage.register_relational_schema(schema()).unwrap();
    apply(
        &storage,
        "records",
        RelationalMutationKind::Insert,
        "long-lived".into(),
        "v0".into(),
    );
    // Warm up statement caches before the first sample.
    upserts(&storage, 10, 0);

    const SAMPLE: usize = 40;
    const OTHER_ROWS: usize = 4000;
    let early = upserts(&storage, SAMPLE, 1);
    for n in 0..OTHER_ROWS {
        apply(
            &storage,
            "chunks",
            RelationalMutationKind::Insert,
            format!("row-{n}"),
            "x".into(),
        );
    }
    let late = upserts(&storage, SAMPLE, 2);

    let ratio = late.as_secs_f64() / early.as_secs_f64();
    eprintln!("early {early:?} late {late:?} ratio {ratio:.2}");
    assert!(
        ratio < 2.0,
        "upsert latency grew {ratio:.2}x after {OTHER_ROWS} other row revisions \
         (early {early:?}, late {late:?})"
    );
}
