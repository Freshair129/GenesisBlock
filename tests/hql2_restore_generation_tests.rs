use genesis_block_native::{
    AccessContext, BackupExportRequest, BackupRestoreRequest, BatchInput, GenesisTransaction,
    NodeInput, OpenOptions, Storage, TemporalRead,
};
use std::{fs, path::Path, time::Duration};
use tempfile::TempDir;

fn read_u32(bytes: &[u8], offset: &mut usize) -> u32 {
    let value = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    value
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> u64 {
    let value = u64::from_le_bytes(bytes[*offset..*offset + 8].try_into().unwrap());
    *offset += 8;
    value
}

fn rewrite_manifest_frontier(bundle_path: &Path, frontier: u64) {
    let original = fs::read(bundle_path).unwrap();
    let magic = b"GENESIS-BACKUP-V1\0";
    assert!(original.starts_with(magic));
    let mut offset = magic.len();
    let manifest_name_len = read_u32(&original, &mut offset) as usize;
    let manifest_name = &original[offset..offset + manifest_name_len];
    assert_eq!(manifest_name, b"manifest.json");
    offset += manifest_name_len;
    let manifest_len = usize::try_from(read_u64(&original, &mut offset)).unwrap();
    let manifest_end = offset + manifest_len;
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&original[offset..manifest_end]).unwrap();
    manifest["stable_frontier"] = serde_json::Value::from(frontier);
    offset = manifest_end;

    let artifact_count = manifest["artifacts"].as_array().unwrap().len();
    let mut artifacts = Vec::with_capacity(artifact_count);
    for index in 0..artifact_count {
        let path_len = read_u32(&original, &mut offset) as usize;
        let path = std::str::from_utf8(&original[offset..offset + path_len])
            .unwrap()
            .to_string();
        offset += path_len;
        let byte_count = usize::try_from(read_u64(&original, &mut offset)).unwrap();
        let data_end = offset + byte_count;
        let data = original[offset..data_end].to_vec();
        offset = data_end;
        assert_eq!(
            manifest["artifacts"][index]["path"].as_str(),
            Some(path.as_str())
        );
        assert_eq!(
            manifest["artifacts"][index]["byte_count"].as_u64(),
            Some(byte_count as u64)
        );
        artifacts.push((path, data));
    }
    assert_eq!(offset, original.len());

    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    let mut rewritten = Vec::new();
    rewritten.extend_from_slice(magic);
    rewritten.extend_from_slice(&(manifest_name.len() as u32).to_le_bytes());
    rewritten.extend_from_slice(manifest_name);
    rewritten.extend_from_slice(&(manifest_bytes.len() as u64).to_le_bytes());
    rewritten.extend_from_slice(&manifest_bytes);
    for (path, data) in artifacts {
        rewritten.extend_from_slice(&(path.len() as u32).to_le_bytes());
        rewritten.extend_from_slice(path.as_bytes());
        rewritten.extend_from_slice(&(data.len() as u64).to_le_bytes());
        rewritten.extend_from_slice(&data);
    }
    fs::write(bundle_path, rewritten).unwrap();
}

fn commit_node_transaction(storage: &Storage, id: &str) {
    storage
        .commit_transaction(GenesisTransaction {
            transaction_id: format!("tx-{id}"),
            expected_frontier: Some(storage.txn_frontier()),
            relational: vec![],
            graph: BatchInput {
                nodes: vec![NodeInput {
                    id: Some(id.into()),
                    labels: vec!["Document".into()],
                    props: None,
                    embedding: None,
                    lang: None,
                    valid_from: None,
                    caused_by: None,
                    ttl: None,
                    collection: None,
                }],
                edges: vec![],
            },
            vectors: vec![],
        })
        .unwrap();
}

fn open(path: &Path, read_only: bool) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(read_only),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn access() -> AccessContext {
    AccessContext {
        principal: "restore-verifier".into(),
        namespace: "default".into(),
    }
}

#[test]
fn restore_publishes_a_generation_before_exposing_the_target() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source");
    let bundle_path = dir.path().join("backup.gdbbundle");
    let restore_root = dir.path().join("restored");
    let source = open(&source_root, false);
    commit_node_transaction(&source, "restore:node");
    let source_frontier = source.stable_frontier();
    let source_txn_frontier = source.txn_frontier();
    assert!(source_txn_frontier > 0);
    source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone(),
        })
        .unwrap();

    let restored_info = Storage::restore_backup(BackupRestoreRequest {
        bundle_path,
        target_root: restore_root.clone(),
    })
    .unwrap();
    assert_eq!(restored_info.stable_frontier, source_frontier);

    let restored = open(&restore_root, true);
    assert!(restored.stable_frontier() >= source_frontier);
    assert_eq!(restored.txn_frontier(), source_txn_frontier);
    let lease = restored
        .pin_generation(
            access(),
            TemporalRead {
                as_of: None,
                tx_as_of: None,
            },
            Duration::from_secs(30),
        )
        .expect("restored target must have a published generation before success");
    restored.validate_lease(&lease).unwrap();
}

#[test]
fn restore_accepts_a_bundle_whose_last_frame_is_the_generation_receipt() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source");
    let bundle_path = dir.path().join("backup.gdbbundle");
    let restore_root = dir.path().join("restored");
    let source = open(&source_root, false);
    commit_node_transaction(&source, "restore:receipt-last");
    let transaction_frontier = source.txn_frontier();
    assert!(transaction_frontier > 0);
    let generation = source.publish_generation().unwrap();
    assert_eq!(generation.publication_seq, source.stable_frontier());
    assert_eq!(generation.txn_frontier, transaction_frontier);
    source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone(),
        })
        .unwrap();

    let restored_info = Storage::restore_backup(BackupRestoreRequest {
        bundle_path,
        target_root: restore_root.clone(),
    })
    .unwrap();
    assert_eq!(restored_info.stable_frontier, generation.publication_seq);

    let restored = open(&restore_root, true);
    assert_eq!(restored.stable_frontier(), generation.publication_seq);
    assert_eq!(restored.txn_frontier(), transaction_frontier);
    let lease = restored
        .pin_generation(
            access(),
            TemporalRead {
                as_of: None,
                tx_as_of: None,
            },
            Duration::from_secs(30),
        )
        .expect("a valid receipt at the manifest frontier must be usable after restore");
    restored.validate_lease(&lease).unwrap();
}

#[test]
fn restore_rejects_a_manifest_frontier_not_present_in_recovered_wal() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source");
    let bundle_path = dir.path().join("backup.gdbbundle");
    let restore_root = dir.path().join("restored");
    let source = open(&source_root, false);
    commit_node_transaction(&source, "restore:manifest-frontier");
    let exported = source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone(),
        })
        .unwrap();
    rewrite_manifest_frontier(&bundle_path, exported.stable_frontier + 1);

    let error = Storage::restore_backup(BackupRestoreRequest {
        bundle_path,
        target_root: restore_root.clone(),
    })
    .unwrap_err();
    assert!(format!("{error:?}").contains("recovered WAL frontier mismatch"));
    assert!(
        !restore_root.exists(),
        "failed restore must not expose its target"
    );
}
