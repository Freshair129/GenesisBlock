// Collection-name validation (storage-readiness audit: `create_collection`
// unvalidated => path traversal).
//
// A collection name becomes part of six on-disk filenames — vec_<n>.bin,
// meta_<n>.bin, fvec_<n>.bin, bqmean_<n>.bin, sq8scale_<n>.bin — each
// path.join'ed to the database directory. These tests pin the FILESYSTEM
// outcome, not just the error string: the point is that nothing is ever
// written outside the DB root, so they assert on a sentinel directory next to
// the database that must stay empty.

use genesis_block_native::{NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::{Path, PathBuf};

fn fresh(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if p.exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    fs::create_dir_all(&p).unwrap();
    p
}

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_str().unwrap().to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

/// Every traversal shape is rejected, and — the assertion that actually
/// matters — the directory beside the database stays empty afterwards.
#[test]
fn traversal_names_are_rejected_and_write_nothing_outside_the_db() {
    let root = fresh("collname_traversal");
    let db_dir = root.join("db");
    let outside = root.join("outside");
    fs::create_dir_all(&db_dir).unwrap();
    fs::create_dir_all(&outside).unwrap();

    let s = open(&db_dir);
    let hostile = [
        "../outside/pwned",
        "../../outside/pwned",
        r"..\outside\pwned",
        "/etc/passwd",
        r"C:\Windows\Temp\pwned",
        "nested/sub",
        "..",
        ".",
        "",
        "trailing space ",
        "semi;colon",
        "null\0byte",
        "-leading-dash",
    ];
    for name in hostile {
        let err = s
            .create_collection(
                name.to_string(),
                "test".to_string(),
                4,
                None,
                None,
                None,
                Some(true), // rerank => would eagerly create fvec_<name>.bin
            )
            .expect_err(&format!("collection name {name:?} must be rejected"));
        let msg = err.to_string();
        assert!(
            msg.contains("collection name"),
            "rejection should name the offending input, got: {msg}"
        );
    }

    // Force a checkpoint so any accepted name would materialize its files.
    s.save_state().unwrap();
    let leaked: Vec<_> = fs::read_dir(&outside)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name()))
        .collect();
    assert!(
        leaked.is_empty(),
        "traversal wrote outside the database root: {leaked:?}"
    );
}

/// The rule must not be so strict that ordinary names stop working — including
/// the engine's own `default` collection, which is created internally.
#[test]
fn ordinary_names_still_work() {
    let db_dir = fresh("collname_ok");
    let s = open(&db_dir);
    for name in ["text", "code_v2", "bge-m3", "A1", "x".repeat(64).as_str()] {
        s.create_collection(
            name.to_string(),
            "test".to_string(),
            4,
            None,
            None,
            None,
            None,
        )
        .unwrap_or_else(|e| panic!("ordinary name {name:?} must be accepted: {e}"));
    }
    // 65 chars is over the bound.
    assert!(s
        .create_collection(
            "y".repeat(65),
            "test".to_string(),
            4,
            None,
            None,
            None,
            None
        )
        .is_err());

    // `default` exists and still serves writes — validation did not break the
    // internally-created collection.
    s.add_node(NodeInput {
        id: Some("n1".to_string()),
        labels: vec!["T".to_string()],
        props: None,
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
    s.flush_index();
    s.save_state().unwrap();
    assert!(db_dir.join("meta_default.bin").exists());
}

/// The remote-triggered path: a CRDT/WAL vector event naming a traversal
/// collection must be INERT (dropped) rather than either auto-provisioning it
/// or aborting replay — one malformed peer event cannot be allowed to stop
/// recovery, and it cannot be allowed to place a file outside the DB either.
#[test]
fn hostile_peer_collection_name_is_inert() {
    let root = fresh("collname_peer");
    let db_dir = root.join("db");
    let outside = root.join("outside");
    fs::create_dir_all(&db_dir).unwrap();
    fs::create_dir_all(&outside).unwrap();

    let s = open(&db_dir);
    // add_vector routes through the same auto-provisioning resolve path that a
    // replayed/synced Event::Vector uses.
    s.add_node(NodeInput {
        id: Some("host".to_string()),
        labels: vec!["T".to_string()],
        props: None,
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
    let hostile = "../outside/peer_pwned";
    let _ = s.add_vector(
        "host".to_string(),
        hostile.to_string(),
        vec![0.0, 1.0, 0.0, 0.0],
    );

    // Whatever the call returned, the engine must still be usable...
    s.flush_index();
    s.save_state().unwrap();
    // ...and nothing may have escaped the database root.
    let leaked: Vec<_> = fs::read_dir(&outside)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name()))
        .collect();
    assert!(
        leaked.is_empty(),
        "peer-supplied collection name wrote outside the DB root: {leaked:?}"
    );
}
