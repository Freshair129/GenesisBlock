use genesis_block_native::{AccessContext, NodeInput, OpenOptions, Storage, TemporalRead};
use serde_json::json;
use std::{path::Path, thread, time::Duration};
use tempfile::tempdir;

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

fn access() -> AccessContext {
    AccessContext {
        principal: "local-owner".into(),
        namespace: "default".into(),
    }
}

fn current_temporal() -> TemporalRead {
    TemporalRead {
        as_of: None,
        tx_as_of: None,
    }
}

fn node(id: &str) -> NodeInput {
    NodeInput {
        id: Some(id.into()),
        labels: vec!["TEST".into()],
        props: Some(json!({"id": id})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

fn assert_error_code(error: impl std::fmt::Display, code: &str) {
    let message = error.to_string();
    assert!(
        message.starts_with(code),
        "expected error prefix {code}, got: {message}"
    );
}

#[test]
fn lease_validates_at_the_generation_it_pins() {
    let dir = tempdir().unwrap();
    let storage = open(dir.path());

    let lease = storage
        .pin_generation(access(), current_temporal(), Duration::from_secs(5))
        .unwrap();

    storage.validate_lease(&lease).unwrap();
}

#[test]
fn committed_write_makes_the_prior_generation_stale() {
    let dir = tempdir().unwrap();
    let storage = open(dir.path());
    let lease = storage
        .pin_generation(access(), current_temporal(), Duration::from_secs(5))
        .unwrap();

    storage.add_node(node("after-pin")).unwrap();

    let error = storage
        .validate_lease(&lease)
        .expect_err("a write beyond the published generation must stale its lease");
    assert_error_code(error, "GENERATION_STALE");
}

#[test]
fn monotonic_ttl_expiry_returns_lease_expired() {
    let dir = tempdir().unwrap();
    let storage = open(dir.path());
    let lease = storage
        .pin_generation(access(), current_temporal(), Duration::from_millis(10))
        .unwrap();

    thread::sleep(Duration::from_millis(30));

    let error = storage
        .validate_lease(&lease)
        .expect_err("the lease must expire after its monotonic deadline");
    assert_error_code(error, "LEASE_EXPIRED");
}

#[test]
fn revocation_advances_the_storage_fence_for_all_outstanding_leases() {
    let dir = tempdir().unwrap();
    let storage = open(dir.path());
    let first = storage
        .pin_generation(access(), current_temporal(), Duration::from_secs(5))
        .unwrap();
    let second = storage
        .pin_generation(access(), current_temporal(), Duration::from_secs(5))
        .unwrap();

    storage.revoke_lease(&first).unwrap();

    for lease in [&first, &second] {
        let error = storage
            .validate_lease(lease)
            .expect_err("advancing the storage fence must invalidate every older lease");
        assert_error_code(error, "LEASE_REVOKED");
    }

    let after_revoke = storage
        .pin_generation(access(), current_temporal(), Duration::from_secs(5))
        .unwrap();
    storage.validate_lease(&after_revoke).unwrap();
}

#[test]
fn lease_from_a_previous_storage_owner_fails_after_reopen() {
    let dir = tempdir().unwrap();
    let previous_owner = open(dir.path());
    let lease = previous_owner
        .pin_generation(access(), current_temporal(), Duration::from_secs(5))
        .unwrap();
    drop(previous_owner);

    let reopened = open(dir.path());

    let error = reopened
        .validate_lease(&lease)
        .expect_err("a reopened Storage handle must reject leases from its former owner token");
    assert_error_code(error, "LEASE_OWNER_MISMATCH");
}

#[test]
fn tx_as_of_below_generation_history_horizon_is_rejected() {
    let dir = tempdir().unwrap();
    let storage = open(dir.path());
    storage.add_node(node("history-seed")).unwrap();
    storage.save_state().unwrap();
    let generation = storage.publish_generation().unwrap();
    assert!(generation.history_horizon > 0);

    let error = match storage.pin_generation(
        access(),
        TemporalRead {
            as_of: None,
            tx_as_of: Some(generation.history_horizon - 1),
        },
        Duration::from_secs(5),
    ) {
        Err(error) => error,
        Ok(_) => panic!("a tx_as_of before the pinned generation horizon must fail closed"),
    };
    assert_error_code(error, "TEMPORAL_BEYOND_HORIZON");
}
