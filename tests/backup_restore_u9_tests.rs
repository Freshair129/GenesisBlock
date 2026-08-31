use genesis_block_native::{
    BackupExportRequest, BackupRestoreRequest, EdgeInput, HybridSearchInput, NodeInput,
    OpenOptions, RelationalColumn, RelationalColumnType, RelationalMutationKind, RelationalQuery,
    RelationalRowMutation, RelationalSchemaPackage, RelationalTable, Storage, ENGINE_VERSION,
    SCHEMA_VERSION,
};
use serde_json::json;
use std::fs;
use std::path::Path;
use uuid::Uuid;

fn fresh(name: &str) -> String {
    let path = format!(
        "{}/{}_{}",
        env!("CARGO_TARGET_TMPDIR"),
        name,
        Uuid::new_v4()
    );
    if Path::new(&path).exists() {
        for _ in 0..50 {
            match fs::remove_dir_all(&path) {
                Ok(()) => break,
                Err(error) if error.raw_os_error() == Some(32) => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(error) => panic!("could not clear test root {path}: {error}"),
            }
        }
        assert!(
            !Path::new(&path).exists(),
            "test root remained locked: {path}"
        );
    }
    path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

fn node(id: &str, embedding: [f64; 4], collection: Option<&str>) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec!["U9".to_string()],
        props: Some(serde_json::json!({"fixture": id})),
        embedding: Some(embedding.to_vec()),
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: collection.map(str::to_string),
    }
}

fn relational_schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "u9".to_string(),
        schema_version: 1,
        previous_version: None,
        package_id: "00000000-0000-4000-8000-000000000901".to_string(),
        schema_hash: String::new(),
        tables: vec![RelationalTable {
            name: "records".to_string(),
            columns: vec![
                RelationalColumn::required("id", RelationalColumnType::Text),
                RelationalColumn::required("value", RelationalColumnType::Text),
            ],
            primary_key: vec!["id".to_string()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        named_queries: vec![],
    }
}

fn manifest_json_range(bytes: &[u8]) -> std::ops::Range<usize> {
    const MAGIC: &[u8] = b"GENESIS-BACKUP-V1\0";
    assert!(bytes.starts_with(MAGIC));
    let mut cursor = MAGIC.len();
    let name_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
    cursor += 4 + name_len;
    let manifest_len = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap()) as usize;
    cursor += 8;
    cursor..cursor + manifest_len
}

fn replace_manifest_value(bytes: &mut [u8], from: &str, to: &str) {
    assert_eq!(from.len(), to.len());
    let range = manifest_json_range(bytes);
    let manifest = std::str::from_utf8(&bytes[range.clone()]).unwrap();
    let offset = manifest.find(from).unwrap();
    bytes[range.start + offset..range.start + offset + from.len()].copy_from_slice(to.as_bytes());
}

fn replace_archive_path(bytes: &mut [u8], from: &str, to: &str) {
    assert_eq!(from.len(), to.len());
    let mut cursor = manifest_json_range(bytes).end;
    while cursor < bytes.len() {
        let path_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        if &bytes[cursor..cursor + path_len] == from.as_bytes() {
            bytes[cursor..cursor + path_len].copy_from_slice(to.as_bytes());
            return;
        }
        cursor += path_len;
        let byte_count = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap()) as usize;
        cursor += 8 + byte_count;
    }
    panic!("archive path was not found: {from}");
}

#[test]
fn export_then_clean_restore_preserves_graph_vector_and_manifest_identity() {
    let source_root = fresh("u9_source");
    let bundle_path = format!("{}/u9-backup.genesis", fresh("u9_bundle"));
    let restore_root = fresh("u9_restore");

    let source = open(&source_root);
    source
        .create_collection(
            "notes".to_string(),
            "fixture-model".to_string(),
            4,
            Some("L2".to_string()),
            None,
            None,
            None,
        )
        .unwrap();
    source
        .add_node(node("note-a", [1.0, 0.0, 0.0, 0.0], Some("notes")))
        .unwrap();
    source
        .add_node(node("note-b", [0.0, 1.0, 0.0, 0.0], Some("notes")))
        .unwrap();
    source
        .register_relational_schema(relational_schema())
        .unwrap();
    source
        .apply_relational_rows(
            "u9",
            vec![RelationalRowMutation {
                table: "records".to_string(),
                kind: RelationalMutationKind::Upsert,
                values: json!({"id": "record-1", "value": "coherent"}),
                key: None,
            }],
        )
        .unwrap();
    source
        .add_edge(EdgeInput {
            id: Some("note-a-links-note-b".to_string()),
            from: "note-a".to_string(),
            to: "note-b".to_string(),
            rel: "LINKS".to_string(),
            props: Some(serde_json::json!({"fixture": true})),
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    let bundle = source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone().into(),
        })
        .unwrap();
    assert_eq!(bundle.stable_frontier, source.stable_frontier());
    assert!(Path::new(&bundle_path).is_file());

    let restored_bundle = Storage::restore_backup(BackupRestoreRequest {
        bundle_path: bundle_path.into(),
        target_root: restore_root.clone().into(),
    })
    .unwrap();
    assert_eq!(restored_bundle.sha256, bundle.sha256);

    let restored = open(&restore_root);
    assert_eq!(restored.stable_frontier(), bundle.stable_frontier);
    assert!(restored.node_view("note-a").is_some());
    assert_eq!(restored.edges.len(), 1);
    assert_eq!(
        restored
            .query_relational(RelationalQuery {
                namespace: "u9".to_string(),
                table: "records".to_string(),
                columns: vec!["records.value".to_string()],
                joins: vec![],
                filters: vec![],
                limit: Some(10),
                offset: None,
            })
            .unwrap(),
        vec![json!({"records.value": "coherent"})]
    );
    restored.flush_index();
    let hits = restored
        .hybrid_search(HybridSearchInput {
            query_vector: vec![1.0, 0.0, 0.0, 0.0],
            k: 1,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: Some("notes".to_string()),
            ef_search: None,
            oversample: None,
        })
        .unwrap();
    assert_eq!(hits[0].node.id, "note-a");
}

#[test]
fn restore_rejects_tampered_bundle_without_creating_target() {
    let source_root = fresh("u9_tamper_source");
    let bundle_path = format!("{}/u9-tamper.genesis", fresh("u9_tamper_bundle"));
    let restore_root = fresh("u9_tamper_restore");
    let source = open(&source_root);
    source
        .add_node(node("note-a", [1.0, 0.0, 0.0, 0.0], None))
        .unwrap();
    source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone().into(),
        })
        .unwrap();

    let mut bytes = fs::read(&bundle_path).unwrap();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0x01;
    fs::write(&bundle_path, bytes).unwrap();

    assert!(Storage::restore_backup(BackupRestoreRequest {
        bundle_path: bundle_path.into(),
        target_root: restore_root.clone().into(),
    })
    .is_err());
    assert!(!Path::new(&restore_root).exists());
}

#[test]
fn restore_rejects_incompatible_manifest_without_creating_target() {
    let source_root = fresh("u9_compat_source");
    let bundle_path = format!("{}/u9-compat.genesis", fresh("u9_compat_bundle"));
    let restore_root = fresh("u9_compat_restore");
    let source = open(&source_root);
    source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone().into(),
        })
        .unwrap();

    let mut bytes = fs::read(&bundle_path).unwrap();
    let range = manifest_json_range(&bytes);
    let manifest = String::from_utf8(bytes[range.clone()].to_vec()).unwrap();
    let incompatible = manifest.replace("\"format_version\":1", "\"format_version\":2");
    assert_ne!(manifest, incompatible);
    assert_eq!(incompatible.len(), range.len());
    bytes[range].copy_from_slice(incompatible.as_bytes());
    fs::write(&bundle_path, bytes).unwrap();

    let error = Storage::restore_backup(BackupRestoreRequest {
        bundle_path: bundle_path.into(),
        target_root: restore_root.clone().into(),
    })
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("unsupported Genesis backup format"));
    assert!(!Path::new(&restore_root).exists());
}

#[test]
fn restore_rejects_duplicate_traversal_and_trailing_entries() {
    for (name, replacement, append_trailing) in [
        ("u9-duplicate", "edges.bin", false),
        ("u9-traversal", "../xx.bin", false),
        ("u9-trailing", "nodes.bin", true),
    ] {
        let source_root = fresh(&format!("{name}-source"));
        let bundle_path = format!("{}/backup.genesis", fresh(&format!("{name}-bundle")));
        let restore_root = fresh(&format!("{name}-restore"));
        let source = open(&source_root);
        source
            .add_node(node("note-a", [1.0, 0.0, 0.0, 0.0], None))
            .unwrap();
        source
            .add_node(node("note-b", [0.0, 1.0, 0.0, 0.0], None))
            .unwrap();
        source
            .add_edge(EdgeInput {
                id: Some("note-a-links-note-b".to_string()),
                from: "note-a".to_string(),
                to: "note-b".to_string(),
                rel: "LINKS".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            })
            .unwrap();
        source
            .export_backup(BackupExportRequest {
                destination: bundle_path.clone().into(),
            })
            .unwrap();

        let mut bytes = fs::read(&bundle_path).unwrap();
        if append_trailing {
            bytes.extend_from_slice(b"unexpected-entry");
        } else {
            replace_manifest_value(&mut bytes, "nodes.bin", replacement);
            replace_archive_path(&mut bytes, "nodes.bin", replacement);
        }
        fs::write(&bundle_path, bytes).unwrap();

        assert!(
            Storage::restore_backup(BackupRestoreRequest {
                bundle_path: bundle_path.into(),
                target_root: restore_root.clone().into(),
            })
            .is_err(),
            "{name} bundle should be rejected"
        );
        assert!(!Path::new(&restore_root).exists());
    }
}

#[test]
fn restore_rejects_incompatible_engine_and_schema_without_creating_target() {
    // Derive the "newer schema" rewrite from the live constant: a hardcoded
    // v2→v3 literal silently became a no-op when Slice 0 bumped
    // SCHEMA_VERSION to 3 — the rewrite no longer matched, the manifest
    // stayed valid, and the "rejected" assertion failed against a restore
    // that correctly succeeded.
    let schema_from = format!("\"schema_version\":{}", SCHEMA_VERSION);
    let schema_to = format!("\"schema_version\":{}", SCHEMA_VERSION + 1);
    for (name, from, to) in [
        ("u9-engine", "genesis-block", "other-genesis"),
        ("u9-schema", schema_from.as_str(), schema_to.as_str()),
    ] {
        let source_root = fresh(&format!("{name}-source"));
        let bundle_path = format!("{}/backup.genesis", fresh(&format!("{name}-bundle")));
        let restore_root = fresh(&format!("{name}-restore"));
        let source = open(&source_root);
        source
            .export_backup(BackupExportRequest {
                destination: bundle_path.clone().into(),
            })
            .unwrap();

        let original = fs::read(&bundle_path).unwrap();
        let mut bytes = original.clone();
        replace_manifest_value(&mut bytes, from, to);
        assert_ne!(
            bytes, original,
            "{name}: the manifest rewrite must actually match — a no-op here \
             means the fixture drifted from the real manifest shape"
        );
        fs::write(&bundle_path, bytes).unwrap();

        assert!(
            Storage::restore_backup(BackupRestoreRequest {
                bundle_path: bundle_path.into(),
                target_root: restore_root.clone().into(),
            })
            .is_err(),
            "{name} bundle should be rejected"
        );
        assert!(!Path::new(&restore_root).exists());
    }
}

/// The inverse of the rejection test above, and the bug it used to hide:
/// restore demanded EXACT `engine_version` equality, so every backup became
/// unrestorable after any release at all. Compatibility is the SCHEMA
/// version's job (gated above); a bundle from an older engine build with the
/// same schema must restore, and the graph inside it must be intact.
#[test]
fn restore_accepts_older_engine_version_with_same_schema() {
    let source_root = fresh("u9-oldver-source");
    let bundle_path = format!("{}/backup.genesis", fresh("u9-oldver-bundle"));
    let restore_root = fresh("u9-oldver-restore");
    let source = open(&source_root);
    source
        .add_node(node("survivor", [1.0, 0.0, 0.0, 0.0], None))
        .unwrap();
    source.flush_index();
    source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone().into(),
        })
        .unwrap();
    drop(source);

    // Rewrite the manifest to claim an older engine build. The replacement is
    // length-matched so the archive offsets after the manifest stay valid.
    let original = fs::read(&bundle_path).unwrap();
    let mut bytes = original.clone();
    let from = format!("\"engine_version\":\"{ENGINE_VERSION}\"");
    let older = format!(
        "0.0.{}",
        "9".repeat(ENGINE_VERSION.len().saturating_sub(4).max(1))
    );
    let to = format!("\"engine_version\":\"{older}\"");
    assert_eq!(
        from.len(),
        to.len(),
        "fixture must be length-matched: {from} vs {to}"
    );
    replace_manifest_value(&mut bytes, &from, &to);
    assert_ne!(
        bytes, original,
        "engine_version rewrite must match the manifest"
    );
    fs::write(&bundle_path, bytes).unwrap();

    Storage::restore_backup(BackupRestoreRequest {
        bundle_path: bundle_path.into(),
        target_root: restore_root.clone().into(),
    })
    .expect("a same-schema bundle from an older engine version must restore");

    let restored = open(&restore_root);
    assert!(
        restored.node_view("survivor").is_some(),
        "restored graph must carry the exported node"
    );
}

#[test]
fn export_and_restore_reject_live_or_existing_destinations() {
    let source_root = fresh("u9_destination_source");
    let source = open(&source_root);
    source
        .add_node(node("note-a", [1.0, 0.0, 0.0, 0.0], None))
        .unwrap();

    assert!(source
        .export_backup(BackupExportRequest {
            destination: Path::new(&source_root).join("inside-live-root.genesis"),
        })
        .is_err());

    let bundle_dir = fresh("u9_destination_bundle");
    fs::create_dir_all(&bundle_dir).unwrap();
    let bundle_path = format!("{bundle_dir}/u9-destination.genesis");
    let existing_bundle = Path::new(&bundle_path).to_path_buf();
    fs::write(&existing_bundle, b"existing").unwrap();
    assert!(source
        .export_backup(BackupExportRequest {
            destination: existing_bundle,
        })
        .is_err());
    fs::remove_file(&bundle_path).unwrap();
    source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone().into(),
        })
        .unwrap();
    let existing_target = fresh("u9_existing_restore_target");
    fs::create_dir_all(&existing_target).unwrap();
    assert!(Storage::restore_backup(BackupRestoreRequest {
        bundle_path: bundle_path.into(),
        target_root: existing_target.into(),
    })
    .is_err());
}
