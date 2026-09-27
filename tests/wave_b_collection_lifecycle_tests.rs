use genesis_block_native::*;
use serde_json::json;
use std::{fs, path::Path};

fn options(p: &Path) -> OpenOptions {
    OpenOptions {
        path: p.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    }
}
fn insert(s: &Storage, id: &str, c: &str, v: Vec<f64>) {
    s.add_node(NodeInput {
        id: Some(id.into()),
        labels: vec![],
        props: Some(json!({"id":id})),
        embedding: Some(v),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: Some(c.into()),
    })
    .unwrap();
}
fn sidecar_rows(s: &Storage, p: &Path) -> Option<Vec<Vec<u8>>> {
    let bytes = fs::read(p.join("fvec_space.bin")).ok()?;
    let c = s.collections.get("space").unwrap();
    Some(
        ["A", "B"]
            .iter()
            .map(|id| {
                let aid = *c.node_to_arena.get(&s.get_u32(id).unwrap()).unwrap() as usize;
                bytes[aid * 8..(aid + 1) * 8].to_vec()
            })
            .collect(),
    )
}
#[test]
fn compacted_quantizers_and_empty_definitions_survive_journal_only() {
    for quant in ["none", "f16", "sq8", "sq8c", "bq"] {
        let dir = tempfile::tempdir().unwrap();
        let s = Storage::open(options(dir.path())).unwrap();
        s.create_collection("empty".into(), "m".into(), 2, None, None, None, None)
            .unwrap();
        s.create_collection(
            "space".into(),
            "m".into(),
            2,
            Some("cosine".into()),
            Some(quant.into()),
            Some(123),
            Some(true),
        )
        .unwrap();
        insert(&s, "A", "space", vec![10., 0.]);
        insert(&s, "B", "space", vec![1., 1.]);
        s.flush_index();
        s.perform_index_compaction().unwrap();
        let info = s
            .list_collections()
            .into_iter()
            .find(|c| c.name == "space")
            .unwrap();
        let original_sidecar = sidecar_rows(&s, dir.path());
        s.compact().unwrap();
        let backup_dir = tempfile::tempdir().unwrap();
        let bundle = backup_dir.path().join("backup.gbak");
        let restored_path = backup_dir.path().join("restored");
        s.export_backup(BackupExportRequest {
            destination: bundle.clone(),
        })
        .unwrap();
        Storage::restore_backup(BackupRestoreRequest {
            bundle_path: bundle,
            target_root: restored_path.clone(),
        })
        .unwrap();
        let restored = Storage::open(options(&restored_path)).unwrap();
        let restored_info = restored
            .list_collections()
            .into_iter()
            .find(|c| c.name == "space")
            .unwrap();
        assert_eq!(
            (
                restored_info.model,
                restored_info.metric,
                restored_info.quant,
                restored_info.rerank
            ),
            (
                info.model.clone(),
                info.metric.clone(),
                info.quant.clone(),
                info.rerank
            )
        );
        assert!(restored
            .list_collections()
            .iter()
            .any(|c| c.name == "empty"));
        assert_eq!(sidecar_rows(&restored, &restored_path), original_sidecar);
        drop(restored);
        drop(s);
        // Only fixture-owned disposable state; keep identity + authoritative journal.
        for entry in fs::read_dir(dir.path()).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "state.json"
                || name.ends_with(".bin") && name != "identity.bin"
                || name.starts_with("projection.sqlite")
            {
                fs::remove_file(entry.path()).unwrap();
            }
        }
        let s = Storage::open(options(dir.path())).unwrap();
        assert!(s.list_collections().iter().any(|c| c.name == "empty"));
        let after = s
            .list_collections()
            .into_iter()
            .find(|c| c.name == "space")
            .unwrap();
        assert_eq!(
            (
                after.model,
                after.metric,
                after.quant,
                after.ef_search,
                after.rerank
            ),
            (
                info.model,
                info.metric,
                info.quant,
                info.ef_search,
                info.rerank
            )
        );
        if let Some(rows) = original_sidecar {
            assert_eq!(
                sidecar_rows(&s, dir.path()).unwrap(),
                rows,
                "rerank values per node must survive {quant}"
            );
        }
        let rows = s
            .hybrid_search(HybridSearchInput {
                query_vector: vec![1., 0.],
                k: 2,
                alpha: Some(0.),
                lang: None,
                as_of: None,
                collection: Some("space".into()),
                ef_search: None,
                oversample: None,
            })
            .unwrap();
        assert_eq!(rows[0].node.id, "A", "cosine ranking survives {quant}");
        assert_eq!(after.count, 2);
    }
}
#[test]
fn unsupported_active_version_is_rejected_without_rewriting() {
    let dir = tempfile::tempdir().unwrap();
    drop(Storage::open(options(dir.path())).unwrap());
    let path = dir.path().join("wal/active.gwal");
    let mut bytes = fs::read(&path).unwrap();
    bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
    fs::write(&path, &bytes).unwrap();
    assert!(Storage::open(options(dir.path())).is_err());
    assert_eq!(fs::read(path).unwrap(), bytes);
}
#[test]
fn unknown_crc_valid_event_is_rejected_without_rewriting() {
    let dir = tempfile::tempdir().unwrap();
    drop(Storage::open(options(dir.path())).unwrap());
    let path = dir.path().join("wal/active.gwal");
    let mut bytes = fs::read(&path).unwrap();
    let payload = serde_json::to_vec(
        &json!({"event":{"FutureSchema":{}},"signature":[],"signer_peer_id":"unknown"}),
    )
    .unwrap();
    let seq = 100u64;
    let mut crc_input = seq.to_le_bytes().to_vec();
    crc_input.extend_from_slice(&payload);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&seq.to_le_bytes());
    bytes.extend_from_slice(&crc32c::crc32c(&crc_input).to_le_bytes());
    bytes.extend_from_slice(&payload);
    fs::write(&path, &bytes).unwrap();
    assert!(Storage::open(options(dir.path())).is_err());
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn legacy_manifest_upgrade_survives_later_journal_only_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let s = Storage::open(options(dir.path())).unwrap();
    s.create_collection(
        "space".into(),
        "legacy-model".into(),
        2,
        Some("cosine".into()),
        Some("f16".into()),
        Some(123),
        None,
    )
    .unwrap();
    s.create_collection(
        "empty".into(),
        "empty-model".into(),
        2,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    insert(&s, "A", "space", vec![10., 0.]);
    drop(s);
    // Reconstruct a v3-style journal: node frames only, collection configuration
    // exists solely in the snapshot manifest. Frame sequences are unsigned.
    let active = dir.path().join("wal/active.gwal");
    let bytes = fs::read(&active).unwrap();
    let mut converted = bytes[..14].to_vec();
    let mut offset = 14;
    let mut seq = 0u64;
    while offset + 16 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let payload = &bytes[offset + 16..offset + 16 + len];
        let event: SignedEvent = serde_json::from_slice(payload).unwrap();
        if !matches!(
            event.event,
            Event::CollectionDefinition(_) | Event::CollectionMaterialization(_)
        ) {
            seq += 1;
            let mut crc = seq.to_le_bytes().to_vec();
            crc.extend_from_slice(payload);
            converted.extend_from_slice(&(len as u32).to_le_bytes());
            converted.extend_from_slice(&seq.to_le_bytes());
            converted.extend_from_slice(&crc32c::crc32c(&crc).to_le_bytes());
            converted.extend_from_slice(payload);
        }
        offset += 16 + len;
    }
    fs::write(&active, converted).unwrap();
    for entry in fs::read_dir(dir.path().join("journal")).unwrap() {
        fs::remove_file(entry.unwrap().path()).unwrap();
    }
    let state_path = dir.path().join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["schema_version"] = json!(3);
    state["journal"]["frontier_seq"] = json!(seq);
    for c in state["collections"].as_array_mut().unwrap() {
        c.as_object_mut().unwrap().remove("definition");
    }
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let s = Storage::open(options(dir.path())).unwrap();
    assert_eq!(
        s.list_collections()
            .iter()
            .find(|c| c.name == "space")
            .unwrap()
            .model,
        "legacy-model"
    );
    drop(s);
    for entry in fs::read_dir(dir.path()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "state.json"
            || name.ends_with(".bin") && name != "identity.bin"
            || name.starts_with("projection.sqlite")
        {
            fs::remove_file(entry.path()).unwrap();
        }
    }
    let s = Storage::open(options(dir.path())).unwrap();
    assert!(s
        .list_collections()
        .iter()
        .any(|c| c.name == "empty" && c.model == "empty-model"));
    let c = s
        .list_collections()
        .into_iter()
        .find(|c| c.name == "space")
        .unwrap();
    assert_eq!(
        (c.model, c.metric, c.ef_search, c.count),
        ("legacy-model".into(), "Cosine".into(), Some(123), 1)
    );
}
