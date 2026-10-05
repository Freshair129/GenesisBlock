use ed25519_dalek::{Signer, SigningKey};
use genesis_block_native::*;
use serde_json::{json, Value};
use std::{fmt::Display, fs, path::Path};
use tempfile::TempDir;

const ACTIVE_HEADER_LEN: usize = 14;
const FRAME_HEADER_LEN: usize = 16;

fn options(path: &Path) -> OpenOptions {
    OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    }
}

fn open(path: &Path) -> Storage {
    Storage::open(options(path)).unwrap()
}

fn owner() -> PolicyAdminActor {
    PolicyAdminActor {
        access: AccessContext {
            principal: "local-owner".into(),
            namespace: "default".into(),
        },
    }
}

fn policy(revision: u64) -> AccessPolicy {
    AccessPolicy {
        revision,
        mode: AccessPolicyMode::Enforced,
        grants: vec![
            AccessGrant {
                principal: "local-owner".into(),
                action: AccessAction::ManagePolicy,
                resource: AccessResource::Namespace("default".into()),
            },
            AccessGrant {
                principal: "reader".into(),
                action: AccessAction::Read,
                resource: AccessResource::Node("p6-acl-chain-node".into()),
            },
        ],
    }
}

fn transition(storage: &Storage, expected_revision: u64) {
    storage
        .replace_access_policy(owner(), expected_revision, policy(expected_revision + 1))
        .unwrap();
}

fn rewrite_policy_event(
    active: &[u8],
    signing_key: &SigningKey,
    event_number: usize,
    original_expected_revision: u64,
    original_policy_revision: u64,
    expected_revision: u64,
    policy_revision: u64,
    folded: bool,
    changed_reader_principal: Option<&str>,
) -> Vec<u8> {
    assert!(
        active.starts_with(b"GWA1"),
        "active WAL must have a GWA1 header"
    );
    assert!(
        active.len() >= ACTIVE_HEADER_LEN,
        "active WAL header is incomplete"
    );

    let mut output = active[..ACTIVE_HEADER_LEN].to_vec();
    let mut offset = ACTIVE_HEADER_LEN;
    let mut policy_event_count = 0;
    let mut changed_target = false;

    while offset < active.len() {
        assert!(
            offset + FRAME_HEADER_LEN <= active.len(),
            "active WAL frame header must be complete"
        );
        let payload_len =
            u32::from_le_bytes(active[offset..offset + 4].try_into().unwrap()) as usize;
        let seq_bytes: [u8; 8] = active[offset + 4..offset + 12].try_into().unwrap();
        let payload_start = offset + FRAME_HEADER_LEN;
        let end = payload_start
            .checked_add(payload_len)
            .expect("active WAL frame length must not overflow");
        assert!(
            end <= active.len(),
            "active WAL frame payload must be complete"
        );

        let mut signed: Value = serde_json::from_slice(&active[payload_start..end]).unwrap();
        if signed["event"].get("AccessPolicyChanged").is_some() {
            policy_event_count += 1;
            if policy_event_count == event_number {
                let acl_event = &mut signed["event"]["AccessPolicyChanged"];
                assert_eq!(acl_event["folded"], json!(false));
                assert_eq!(
                    acl_event["expected_revision"],
                    json!(original_expected_revision)
                );
                assert_eq!(
                    acl_event["policy"]["revision"],
                    json!(original_policy_revision)
                );
                acl_event["expected_revision"] = json!(expected_revision);
                acl_event["policy"]["revision"] = json!(policy_revision);
                acl_event["folded"] = json!(folded);
                if let Some(principal) = changed_reader_principal {
                    acl_event["policy"]["grants"][1]["principal"] = json!(principal);
                }

                let event: Event = serde_json::from_value(signed["event"].clone()).unwrap();
                let canonical_event = serde_json::to_vec(&event).unwrap();
                signed["event"] = serde_json::to_value(event).unwrap();
                signed["signature"] = json!(signing_key.sign(&canonical_event).to_bytes().to_vec());
                changed_target = true;
            }
        }

        let payload = serde_json::to_vec(&signed).unwrap();
        let mut crc_input = seq_bytes.to_vec();
        crc_input.extend_from_slice(&payload);
        output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        output.extend_from_slice(&seq_bytes);
        output.extend_from_slice(&crc32c::crc32c(&crc_input).to_le_bytes());
        output.extend_from_slice(&payload);
        offset = end;
    }

    assert!(
        policy_event_count >= event_number,
        "fixture must contain the target ACL event"
    );
    assert!(changed_target, "the selected ACL event must be changed");
    output
}

fn rewrite_second_policy_event_as_signed_gap(active: &[u8], signing_key: &SigningKey) -> Vec<u8> {
    rewrite_policy_event(active, signing_key, 2, 1, 2, 2, 3, false, None)
}

fn rewrite_first_policy_event_as_max_baseline(active: &[u8], signing_key: &SigningKey) -> Vec<u8> {
    rewrite_policy_event(
        active,
        signing_key,
        1,
        0,
        1,
        u64::MAX - 1,
        u64::MAX,
        true,
        None,
    )
}

fn write_snapshot_policy(dir: &Path, policy: &AccessPolicy) {
    let state_path = dir.join("state.json");
    let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["p6"]["access_policy"] = serde_json::to_value(policy).unwrap();
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
}

fn assert_recovery_required<E: Display>(result: std::result::Result<Storage, E>) {
    let error = match result {
        Ok(storage) => {
            drop(storage);
            panic!("a discontinuous signed ACL CAS chain must fail closed with RECOVERY_REQUIRED")
        }
        Err(error) => error.to_string(),
    };
    assert!(
        error.starts_with("RECOVERY_REQUIRED"),
        "expected RECOVERY_REQUIRED for the signed ACL CAS gap, got: {error}"
    );
}

#[test]
fn contiguous_signed_acl_transitions_survive_snapshot_reopen() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    transition(&storage, 0);
    transition(&storage, 1);
    storage.save_state().unwrap();
    drop(storage);

    let reopened = open(dir.path());
    transition(&reopened, 2);
}

#[test]
fn acl_policy_cannot_change_without_revision_successor_at_u64_max() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    transition(&storage, 0);
    storage.save_state().unwrap();

    let active_path = dir.path().join("wal").join("active.gwal");
    let rewritten_active = rewrite_first_policy_event_as_max_baseline(
        &fs::read(&active_path).unwrap(),
        &storage.signing_key,
    );
    drop(storage);

    fs::write(active_path, rewritten_active).unwrap();
    write_snapshot_policy(dir.path(), &policy(u64::MAX));
    let storage = open(dir.path());
    let mut changed_policy = policy(u64::MAX);
    changed_policy.grants[1].principal = "attacker".into();

    let result = storage.replace_access_policy(owner(), u64::MAX, changed_policy);
    assert!(
        matches!(result, Err(ref error) if error.to_string().starts_with("ACCESS_POLICY_REVISION_CONFLICT")),
        "same-revision policy replacement at u64::MAX must be rejected, got {result:?}"
    );
}

#[test]
fn signed_same_revision_acl_event_at_u64_max_fails_closed() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    transition(&storage, 0);
    transition(&storage, 1);
    storage.save_state().unwrap();

    let active_path = dir.path().join("wal").join("active.gwal");
    let max_baseline = rewrite_first_policy_event_as_max_baseline(
        &fs::read(&active_path).unwrap(),
        &storage.signing_key,
    );
    let mut changed_policy = policy(u64::MAX);
    changed_policy.grants[1].principal = "attacker".into();
    let same_revision_event = rewrite_policy_event(
        &max_baseline,
        &storage.signing_key,
        2,
        1,
        2,
        u64::MAX,
        u64::MAX,
        false,
        Some("attacker"),
    );
    drop(storage);

    fs::write(active_path, same_revision_event).unwrap();
    write_snapshot_policy(dir.path(), &changed_policy);
    assert_recovery_required(Storage::open(options(dir.path())));
}

#[test]
fn matching_snapshot_rejects_locally_valid_signed_acl_cas_gap() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    transition(&storage, 0);
    transition(&storage, 1);
    storage.save_state().unwrap();

    let active_path = dir.path().join("wal").join("active.gwal");
    let rewritten_active = rewrite_second_policy_event_as_signed_gap(
        &fs::read(&active_path).unwrap(),
        &storage.signing_key,
    );
    let state_path = dir.path().join("state.json");
    let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["p6"]["access_policy"] = serde_json::to_value(policy(3)).unwrap();
    drop(storage);

    fs::write(active_path, rewritten_active).unwrap();
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

    assert_recovery_required(Storage::open(options(dir.path())));
}

#[test]
fn folded_nonzero_acl_baseline_allows_next_contiguous_signed_transition() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    transition(&storage, 0);
    transition(&storage, 1);
    storage.compact().unwrap();

    let has_base_segment = fs::read_dir(dir.path().join("journal"))
        .unwrap()
        .filter_map(std::result::Result::ok)
        .map(|entry| fs::read(entry.path()).unwrap_or_default())
        .any(|bytes| bytes.starts_with(b"GSG1") && bytes.get(6) == Some(&2));
    assert!(
        has_base_segment,
        "compact must produce a signed base materialization"
    );

    transition(&storage, 2);
    storage.save_state().unwrap();
    drop(storage);

    let reopened = open(dir.path());
    transition(&reopened, 3);
}

#[test]
fn wal_only_replay_of_signed_acl_cas_gap_fails_closed() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    transition(&storage, 0);
    transition(&storage, 1);

    let active_path = dir.path().join("wal").join("active.gwal");
    let rewritten_active = rewrite_second_policy_event_as_signed_gap(
        &fs::read(&active_path).unwrap(),
        &storage.signing_key,
    );
    drop(storage);

    let state_path = dir.path().join("state.json");
    if state_path.exists() {
        fs::remove_file(state_path).unwrap();
    }
    let journal_dir = dir.path().join("journal");
    if journal_dir.exists() {
        fs::remove_dir_all(journal_dir).unwrap();
    }
    fs::write(active_path, rewritten_active).unwrap();

    assert_recovery_required(Storage::open(options(dir.path())));
}
