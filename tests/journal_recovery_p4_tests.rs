use genesis_block_native::{NodeInput, OpenOptions, Storage};
use std::fs::{self, OpenOptions as FsOpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

fn fresh(name: &str) -> PathBuf {
    let path = PathBuf::from(format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name));
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    path
}

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.display().to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

fn seed(path: &Path) {
    let storage = open(path);
    storage
        .add_node(NodeInput {
            id: Some("p4-seed".into()),
            labels: vec!["P4".into()],
            props: Some(serde_json::json!({"durability": true})),
            embedding: None,
            lang: Some("en".into()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    drop(storage);
}

fn append_active(path: &Path, bytes: &[u8]) {
    let active = path.join("wal").join("active.gwal");
    let mut file = FsOpenOptions::new().append(true).open(active).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn open_error(path: &Path) -> String {
    match Storage::open(OpenOptions {
        path: path.display().to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    }) {
        Ok(_) => panic!("malformed journal unexpectedly opened"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn oversized_active_frame_is_rejected_before_recovery() {
    let path = fresh("p4_oversized_active_frame");
    seed(&path);

    let mut frame = Vec::new();
    frame.extend_from_slice(&((64 * 1024 * 1024 + 1) as u32).to_le_bytes());
    frame.extend_from_slice(&999u64.to_le_bytes());
    frame.extend_from_slice(&0u32.to_le_bytes());
    append_active(&path, &frame);

    let error = open_error(&path);
    assert!(
        error.contains("JOURNAL_PREFLIGHT_FAILED")
            && error.contains("frame payload exceeds reader bound"),
        "unexpected oversized-frame error: {error}"
    );
}

#[test]
fn complete_corrupt_active_frame_is_rejected_but_partial_tail_is_allowed() {
    let corrupt = fresh("p4_complete_corrupt_active_frame");
    seed(&corrupt);
    let mut complete = Vec::new();
    complete.extend_from_slice(&4u32.to_le_bytes());
    complete.extend_from_slice(&1000u64.to_le_bytes());
    complete.extend_from_slice(&1u32.to_le_bytes());
    complete.extend_from_slice(&[0xAA; 4]);
    append_active(&corrupt, &complete);
    let error = open_error(&corrupt);
    assert!(
        error.contains("JOURNAL_PREFLIGHT_FAILED") && error.contains("corrupt journal frame"),
        "unexpected complete-corruption error: {error}"
    );

    let partial = fresh("p4_partial_active_frame");
    seed(&partial);
    let mut torn = Vec::new();
    torn.extend_from_slice(&4u32.to_le_bytes());
    torn.extend_from_slice(&1001u64.to_le_bytes());
    torn.extend_from_slice(&0u32.to_le_bytes());
    torn.extend_from_slice(&[0xBB; 2]);
    append_active(&partial, &torn);
    let recovered = open(&partial);
    assert!(recovered.node_view("p4-seed").is_some());
}
