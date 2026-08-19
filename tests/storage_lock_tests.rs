use std::{collections::BTreeMap, fs, path::Path};

use genesis_block_native::{NodeInput, OpenOptions, Storage};
use tempfile::TempDir;

fn options(dir: &TempDir) -> OpenOptions {
    OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(8),
        retention: None,
    }
}

fn data_root_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, current: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, files);
            } else if !matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("genesis.lock" | "projection.sqlite-shm" | "projection.sqlite-wal")
            ) {
                // Lock/SHM/WAL sidecars are coordination artifacts. The durable
                // Genesis WAL, projection database, snapshots, and identity must
                // remain byte-identical during a read-only session.
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                files.insert(relative, fs::read(path).unwrap());
            }
        }
    }

    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn a_data_root_has_exactly_one_storage_owner() {
    let dir = TempDir::new().unwrap();
    let first = Storage::open(options(&dir)).expect("first owner must acquire the data root");

    let error = match Storage::open(options(&dir)) {
        Ok(_) => panic!("a second owner must not open the same data root"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("already open"),
        "contention error must be actionable: {error}"
    );

    drop(first);

    Storage::open(options(&dir)).expect("dropping the first owner must release the data root");
}

#[test]
fn read_only_open_still_requires_exclusive_process_ownership() {
    let dir = TempDir::new().unwrap();
    let first = Storage::open(options(&dir)).expect("first owner must acquire the data root");
    let mut read_only = options(&dir);
    read_only.read_only = Some(true);

    assert!(
        Storage::open(read_only).is_err(),
        "read-only mode still owns native WAL, graph, and vector lifecycle"
    );

    drop(first);
}

#[test]
fn read_only_open_does_not_modify_durable_database_payloads() {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(options(&dir)).unwrap();
    storage
        .add_node(NodeInput {
            id: Some("read-only-proof".to_string()),
            labels: vec!["Evidence".to_string()],
            props: Some(serde_json::json!({"status": "stable"})),
            embedding: Some(vec![0.25; 8]),
            lang: Some("en".to_string()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    storage.flush_index();
    drop(storage);

    let before = data_root_bytes(dir.path());
    let mut read_only = options(&dir);
    read_only.read_only = Some(true);
    let storage = Storage::open(read_only).unwrap();
    storage
        .execute_hql_read_only(r#"TRAVERSE FROM "read-only-proof" DEPTH 1 REL ANY"#)
        .unwrap();
    drop(storage);
    let after = data_root_bytes(dir.path());

    if before != after {
        let changed = before
            .keys()
            .chain(after.keys())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter_map(|path| {
                let before_bytes = before.get(path);
                let after_bytes = after.get(path);
                (before_bytes != after_bytes).then(|| {
                    format!(
                        "{path}: {:?} -> {:?}",
                        before_bytes.map(Vec::len),
                        after_bytes.map(Vec::len)
                    )
                })
            })
            .collect::<Vec<_>>();
        panic!("read-only access changed durable database bytes: {changed:?}");
    }
}
