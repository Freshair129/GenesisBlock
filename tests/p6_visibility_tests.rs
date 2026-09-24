use genesis_block_native::*;
use serde_json::json;
use std::{fmt::Display, path::Path, time::Duration};
use tempfile::TempDir;

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn add_node(storage: &Storage, id: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: vec!["P6".into()],
            props: Some(json!({"id": id})),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn owner() -> PolicyAdminActor {
    PolicyAdminActor {
        access: AccessContext {
            principal: "local-owner".into(),
            namespace: "default".into(),
        },
    }
}

fn grant(principal: &str, action: AccessAction, resource: AccessResource) -> AccessGrant {
    AccessGrant {
        principal: principal.into(),
        action,
        resource,
    }
}

fn owner_policy_grant() -> AccessGrant {
    grant(
        "local-owner",
        AccessAction::ManagePolicy,
        AccessResource::Namespace("default".into()),
    )
}

fn node_read_grant(node_id: &str) -> AccessGrant {
    grant(
        "reader",
        AccessAction::Read,
        AccessResource::Node(node_id.into()),
    )
}

fn namespace_read_grant() -> AccessGrant {
    grant(
        "reader",
        AccessAction::Read,
        AccessResource::Namespace("default".into()),
    )
}

fn enforced_policy(revision: u64, grants: Vec<AccessGrant>) -> AccessPolicy {
    AccessPolicy {
        revision,
        mode: AccessPolicyMode::Enforced,
        grants,
    }
}

fn enable_policy(storage: &Storage, grants: Vec<AccessGrant>) {
    storage
        .replace_access_policy(owner(), 0, enforced_policy(1, grants))
        .unwrap();
}

fn reader_lease(storage: &Storage) -> ReadLease {
    storage
        .pin_generation(
            AccessContext {
                principal: "reader".into(),
                namespace: "default".into(),
            },
            TemporalRead {
                as_of: None,
                tx_as_of: None,
            },
            Duration::from_secs(30),
        )
        .unwrap()
}

fn traverse_request() -> QueryIrRequest {
    serde_json::from_value(json!({
        "contract_version": "query-ir.v1",
        "request_id": "p6-visibility-traverse",
        "operation": {
            "kind": "traverse",
            "seed_id": "seed",
            "depth": 1,
            "direction": "out",
            "relations": ["LINKS_TO"]
        }
    }))
    .unwrap()
}

fn assert_error_prefix<T, E: Display>(result: std::result::Result<T, E>, prefix: &str) {
    let error = match result {
        Ok(_) => panic!("expected error prefix {prefix}"),
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(
        message.starts_with(prefix),
        "expected error prefix {prefix}, got: {message}"
    );
}

fn tamper_access_policy_signature(active: &[u8]) -> Vec<u8> {
    const HEADER_LEN: usize = 14;
    const FRAME_HEADER_LEN: usize = 16;

    let mut output = active[..HEADER_LEN].to_vec();
    let mut offset = HEADER_LEN;
    let mut changed = false;
    while offset + FRAME_HEADER_LEN <= active.len() {
        let payload_len =
            u32::from_le_bytes(active[offset..offset + 4].try_into().unwrap()) as usize;
        let seq_bytes: [u8; 8] = active[offset + 4..offset + 12].try_into().unwrap();
        let end = offset + FRAME_HEADER_LEN + payload_len;
        assert!(end <= active.len(), "active journal frame must be complete");
        let mut value: serde_json::Value =
            serde_json::from_slice(&active[offset + FRAME_HEADER_LEN..end]).unwrap();
        if value["event"]
            .as_object()
            .is_some_and(|event| event.contains_key("AccessPolicyChanged"))
        {
            let signature = value["signature"].as_array_mut().unwrap();
            let first = signature[0].as_u64().unwrap() as u8;
            signature[0] = serde_json::json!(first ^ 1);
            changed = true;
        }
        let payload = serde_json::to_vec(&value).unwrap();
        let mut crc_input = seq_bytes.to_vec();
        crc_input.extend_from_slice(&payload);
        output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        output.extend_from_slice(&seq_bytes);
        output.extend_from_slice(&crc32c::crc32c(&crc_input).to_le_bytes());
        output.extend_from_slice(&payload);
        offset = end;
    }
    assert_eq!(
        offset,
        active.len(),
        "all active journal bytes must be framed"
    );
    assert!(
        changed,
        "expected a signed AccessPolicyChanged journal event"
    );
    output
}

fn legacy_jsonl_with_tampered_policy_signature(active: &[u8]) -> Vec<u8> {
    const HEADER_LEN: usize = 14;
    const FRAME_HEADER_LEN: usize = 16;

    let corrupted = tamper_access_policy_signature(active);
    let mut offset = HEADER_LEN;
    let mut output = Vec::new();
    while offset + FRAME_HEADER_LEN <= corrupted.len() {
        let payload_len =
            u32::from_le_bytes(corrupted[offset..offset + 4].try_into().unwrap()) as usize;
        let end = offset + FRAME_HEADER_LEN + payload_len;
        let payload = &corrupted[offset + FRAME_HEADER_LEN..end];
        let value: serde_json::Value = serde_json::from_slice(payload).unwrap();
        if value["event"]
            .as_object()
            .is_some_and(|event| event.contains_key("AccessPolicyChanged"))
        {
            output.extend_from_slice(payload);
            output.push(b'\n');
            break;
        }
        offset = end;
    }
    assert!(
        !output.is_empty(),
        "expected a tampered AccessPolicyChanged JSONL event"
    );
    output
}

#[test]
fn disabled_policy_preserves_legacy_direct_reads() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "seed");

    assert_eq!(storage.node_view("seed").unwrap().id, "seed");
    let response = storage.execute_query_ir(traverse_request()).unwrap();
    assert_eq!(response["request_id"], "p6-visibility-traverse");
}

#[test]
fn enforced_policy_denies_unscoped_direct_reads() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "seed");
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("seed")],
    );

    assert_error_prefix(
        storage.execute_query_ir(traverse_request()),
        "ACCESS_CONTEXT_REQUIRED",
    );
}

#[test]
fn exact_node_grant_is_point_scoped_and_does_not_authorize_query_ir() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "allowed-node");
    add_node(&storage, "other-node");
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("allowed-node")],
    );
    let lease = reader_lease(&storage);

    storage
        .with_read_lease(&lease, |view| {
            let allowed = view.node_view("allowed-node")?;
            assert_eq!(serde_json::to_value(allowed).unwrap()["id"], "allowed-node");
            assert_error_prefix(view.node_view("other-node"), "ACCESS_DENIED");
            assert_error_prefix(view.execute_query_ir(traverse_request()), "ACCESS_DENIED");
            Ok(())
        })
        .unwrap();
}

#[test]
fn broad_query_ir_read_requires_default_namespace_grant() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "seed");
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("seed")],
    );

    let exact_node_lease = reader_lease(&storage);
    storage
        .with_read_lease(&exact_node_lease, |view| {
            assert_error_prefix(view.execute_query_ir(traverse_request()), "ACCESS_DENIED");
            Ok(())
        })
        .unwrap();

    storage
        .replace_access_policy(
            owner(),
            1,
            enforced_policy(2, vec![owner_policy_grant(), namespace_read_grant()]),
        )
        .unwrap();
    let namespace_lease = reader_lease(&storage);
    let response = storage
        .with_read_lease(&namespace_lease, |view| {
            view.execute_query_ir(traverse_request())
        })
        .unwrap();
    assert_eq!(response["request_id"], "p6-visibility-traverse");
}

#[test]
fn policy_compare_and_swap_rejects_stale_expected_revision() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("seed")],
    );

    assert_error_prefix(
        storage.replace_access_policy(
            owner(),
            0,
            enforced_policy(1, vec![owner_policy_grant(), node_read_grant("seed")]),
        ),
        "ACCESS_POLICY_REVISION_CONFLICT",
    );
}
#[test]
fn policy_change_revokes_leases_pinned_to_the_previous_acl_revision() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "seed");
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("seed")],
    );
    let old_lease = reader_lease(&storage);

    storage
        .replace_access_policy(
            owner(),
            1,
            enforced_policy(2, vec![owner_policy_grant(), namespace_read_grant()]),
        )
        .unwrap();

    assert_error_prefix(storage.validate_lease(&old_lease), "LEASE_REVOKED");
}

#[test]
fn enforced_policy_survives_compact_and_reopen() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "allowed-node");
    add_node(&storage, "other-node");
    storage.save_state().unwrap();
    let stale_snapshot = std::fs::read(dir.path().join("state.json")).unwrap();
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("allowed-node")],
    );
    storage.compact().unwrap();
    drop(storage);
    std::fs::write(dir.path().join("state.json"), stale_snapshot).unwrap();

    let reopened = open(dir.path());
    assert_error_prefix(
        reopened.execute_query_ir(traverse_request()),
        "ACCESS_CONTEXT_REQUIRED",
    );
    let lease = reader_lease(&reopened);
    reopened
        .with_read_lease(&lease, |view| {
            let allowed = view.node_view("allowed-node")?;
            assert_eq!(serde_json::to_value(allowed).unwrap()["id"], "allowed-node");
            assert_error_prefix(view.node_view("other-node"), "ACCESS_DENIED");
            Ok(())
        })
        .unwrap();
}

#[test]
fn enforced_policy_denies_every_unscoped_result_read_entrypoint() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "seed");
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("seed")],
    );

    let results = vec![
        storage.projection_props(1).map(|_| ()),
        storage.get_relational_schema("default").map(|_| ()),
        storage.list_relational_schemas().map(|_| ()),
        storage.query_sql("SELECT 1", vec![]).map(|_| ()),
        storage
            .execute_named_query(NamedQueryRequest {
                namespace: "default".into(),
                schema_version: 1,
                query_name: "hidden-query".into(),
                parameters: serde_json::json!({}),
                limit: None,
            })
            .map(|_| ()),
        storage
            .studio_graph_scene(StudioGraphSceneRequest::default())
            .map(|_| ()),
        storage.studio_inspect_entity("missing").map(|_| ()),
        storage
            .query(QueryInput {
                from: None,
                to: None,
                rel: None,
                as_of: None,
                include_invalid: None,
                limit: None,
            })
            .map(|_| ()),
        storage.calculate_structural_gaps().map(|_| ()),
        storage
            .execute_hql_read_only_with_budget("not valid HQL", None)
            .map(|_| ()),
    ];
    let denied = results
        .iter()
        .map(|result| {
            result
                .as_ref()
                .is_err_and(|error| error.to_string().starts_with("ACCESS_CONTEXT_REQUIRED"))
        })
        .collect::<Vec<_>>();

    storage.meta_history.insert(
        7,
        vec![SuperNode {
            cluster_id: 7,
            theme: "protected".into(),
            member_count: 1,
            impact: 1.0,
            centroid: vec![0.0],
            timestamp: "2026-09-22T00:00:00Z".into(),
            drift: None,
        }],
    );
    let metadata_hidden = storage.get_meta_history(7).is_empty();
    assert!(
        metadata_hidden,
        "legacy Vec-returning metadata reads must fail closed without exposing history"
    );
    assert_eq!(
        denied,
        vec![true; 10],
        "every Result-returning unscoped read must return ACCESS_CONTEXT_REQUIRED"
    );
}

#[test]
fn tampered_current_snapshot_policy_is_recovered_from_signed_wal() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "seed");
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("seed")],
    );
    storage.compact().unwrap();
    drop(storage);

    let state_path = dir.path().join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    state["p6"]["access_policy"] = serde_json::json!({
        "revision": 0,
        "mode": "Disabled",
        "grants": []
    });
    std::fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

    let reopened = open(dir.path());
    assert_error_prefix(
        reopened.execute_query_ir(traverse_request()),
        "ACCESS_CONTEXT_REQUIRED",
    );
}

#[test]
fn non_default_revision_zero_snapshot_policy_without_signed_wal_fails_closed() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "seed");
    storage.compact().unwrap();
    drop(storage);

    let state_path = dir.path().join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    state["p6"]["access_policy"] = serde_json::json!({
        "revision": 0,
        "mode": "Enforced",
        "grants": []
    });
    std::fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

    assert_error_prefix(
        Storage::open(OpenOptions {
            path: dir.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(2),
            retention: Some("full".into()),
        }),
        "RECOVERY_REQUIRED",
    );
}

#[test]
fn enforced_policy_fails_closed_legacy_unscoped_accessors() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "seed");
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("seed")],
    );

    assert!(storage.get_u32("seed").is_none());
    assert!(storage.find_fuzzy_id("seed").is_none());
    assert!(storage.list_collections().is_empty());
}

#[test]
fn invalid_local_policy_signature_in_journal_fails_closed_on_replay() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("seed")],
    );
    let active_path = dir.path().join("wal").join("active.gwal");
    let corrupted_active = tamper_access_policy_signature(&std::fs::read(&active_path).unwrap());
    drop(storage);

    std::fs::remove_file(dir.path().join("state.json")).unwrap();
    let journal_dir = dir.path().join("journal");
    if journal_dir.exists() {
        std::fs::remove_dir_all(&journal_dir).unwrap();
    }
    std::fs::write(active_path, corrupted_active).unwrap();

    let opened = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    });
    let error = match opened {
        Err(error) => error.to_string(),
        Ok(storage) => {
            drop(storage);
            panic!("invalid P6 event signature must fail closed during open")
        }
    };
    assert!(
        error.starts_with("RECOVERY_REQUIRED"),
        "expected RECOVERY_REQUIRED, got: {error}"
    );
}

#[test]
fn invalid_local_policy_signature_in_legacy_jsonl_fails_closed_on_replay() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("seed")],
    );
    let active_path = dir.path().join("wal").join("active.gwal");
    let legacy_line =
        legacy_jsonl_with_tampered_policy_signature(&std::fs::read(&active_path).unwrap());
    drop(storage);

    std::fs::remove_file(dir.path().join("state.json")).unwrap();
    std::fs::remove_dir_all(dir.path().join("journal")).unwrap();
    std::fs::remove_dir_all(dir.path().join("wal")).unwrap();
    std::fs::write(dir.path().join("genesis-graph.wal"), legacy_line).unwrap();

    let opened = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    });
    let error = match opened {
        Err(error) => error.to_string(),
        Ok(storage) => {
            drop(storage);
            panic!("invalid legacy P6 event signature must fail closed during open")
        }
    };
    assert!(
        error.starts_with("RECOVERY_REQUIRED"),
        "expected RECOVERY_REQUIRED, got: {error}"
    );
}

#[test]
fn read_view_scope_is_not_inherited_by_a_captured_storage_call() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "allowed-node");
    enable_policy(
        &storage,
        vec![owner_policy_grant(), node_read_grant("allowed-node")],
    );
    let lease = reader_lease(&storage);

    storage
        .with_read_lease(&lease, |view| {
            assert!(view.node_view("allowed-node").is_ok());
            assert!(storage.get_u32("allowed-node").is_none());
            assert_error_prefix(
                storage.execute_query_ir(traverse_request()),
                "ACCESS_CONTEXT_REQUIRED",
            );
            Ok(())
        })
        .unwrap();
}
