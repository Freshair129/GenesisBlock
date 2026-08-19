//! Schema-version compatibility gate. A database written by a NEWER engine must
//! refuse to open (forward-incompat protection); an older / pre-versioned
//! snapshot must still open (migration path). Pairs with the on-load migrations
//! in `try_load_state`.

use std::fs;

use genesis_block_native::{NodeInput, OpenOptions, Storage, SCHEMA_VERSION};
use tempfile::TempDir;

fn open(path: &str) -> Result<Storage, String> {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .map_err(|e| e.to_string())
}

fn add_and_save(path: &str) {
    let s = open(path).unwrap();
    s.add_node(NodeInput {
        id: Some("n1".into()),
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
    s.save_state().unwrap();
}

/// Rewrite the on-disk `schema_version` field in state.json to `v`.
fn set_ondisk_schema_version(dir: &str, v: u64) {
    let p = format!("{}/state.json", dir);
    let mut val: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&p).unwrap()).unwrap();
    val["schema_version"] = serde_json::json!(v);
    fs::write(&p, val.to_string()).unwrap();
}

#[test]
fn fresh_database_opens() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    // No snapshot yet → no gate → opens fine.
    assert!(open(path).is_ok(), "a fresh database must open");
}

#[test]
fn newer_schema_is_refused() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap().to_string();
    add_and_save(&path);
    // Pretend the snapshot was written by a future engine.
    set_ondisk_schema_version(&path, SCHEMA_VERSION as u64 + 1);
    let err = open(&path)
        .err()
        .expect("opening a newer-schema DB must error");
    assert!(
        err.contains("newer engine"),
        "error must explain the forward-incompat: got {:?}",
        err
    );
}

#[test]
fn older_and_current_schema_open() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap().to_string();
    add_and_save(&path);

    // Current version: opens.
    assert!(open(&path).is_ok(), "current-schema DB must open");

    // Pre-versioned (legacy) snapshot: opens via the migration path.
    set_ondisk_schema_version(&path, 0);
    assert!(
        open(&path).is_ok(),
        "legacy/older-schema DB must open (migration)"
    );
}
