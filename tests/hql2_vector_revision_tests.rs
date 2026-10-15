use genesis_block_native::{
    Event, LogicalClock, NodeInput, OpenOptions, SignedEvent, Storage, VectorEvent,
};
use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;
use tempfile::TempDir;
use uuid::Uuid;

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

fn vector_revisions(storage: &Storage) -> Vec<Value> {
    storage
        .events_since_seq(0)
        .into_iter()
        .filter_map(|signed| serde_json::to_value(signed.event).ok())
        .filter_map(|event| event.get("Transaction").cloned())
        .filter_map(|transaction| transaction.get("record_revision_transaction").cloned())
        .filter_map(|revisions| revisions.get("mutations").cloned())
        .filter_map(|mutations| mutations.as_array().cloned())
        .flatten()
        .filter(|mutation| mutation.get("kind").and_then(Value::as_str) == Some("vector"))
        .collect()
}

#[test]
fn schema6_primary_and_secondary_vectors_have_durable_revision_identity() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(NodeInput {
            id: Some("vector-owner".into()),
            labels: vec!["Note".into()],
            props: None,
            embedding: Some(vec![0.125, 0.25]),
            lang: Some("en".into()),
            valid_from: Some("2026-09-29T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: Some("default".into()),
        })
        .unwrap();
    storage
        .add_vector("vector-owner".into(), "default".into(), vec![0.5, 0.75])
        .unwrap();

    let revisions = vector_revisions(&storage);
    assert_eq!(
        revisions.len(),
        2,
        "both embedding writes need vector revisions"
    );
    let collection = storage
        .list_collections()
        .into_iter()
        .find(|collection| collection.name == "default")
        .unwrap();
    let fingerprint_tuple = (
        collection.name.as_str(),
        collection.model.as_str(),
        u16::try_from(collection.dim).unwrap(),
        collection.metric.as_str(),
        collection.quant.as_str(),
    );
    let mut fingerprint_hasher = Sha256::new();
    fingerprint_hasher.update(b"genesis.hql2.vector-space.v1:");
    fingerprint_hasher.update(serde_json::to_vec(&fingerprint_tuple).unwrap());
    let expected_fingerprint = hex::encode(fingerprint_hasher.finalize());
    let mut revision_ids = Vec::new();
    for mutation in &revisions {
        let revision_id = mutation["revision_id"].as_str().unwrap();
        assert_eq!(Uuid::parse_str(revision_id).unwrap().get_version_num(), 4);
        revision_ids.push(revision_id.to_string());
        let payload = &mutation["payload"];
        assert_eq!(payload["owner_id"], "vector-owner");
        assert_eq!(payload["collection"], "default");
        assert_eq!(payload["embedding"].as_array().unwrap().len(), 2);
        assert_eq!(payload["space_fingerprint"], expected_fingerprint);
        assert_eq!(payload["owner_revision_id"].as_str().unwrap().len(), 36);
    }
    assert_ne!(revision_ids[0], revision_ids[1]);

    storage.compact().unwrap();
    drop(storage);
    let reopened = open(dir.path());
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let rows: Vec<(String, Option<i64>)> = projection
        .prepare(
            "SELECT revision_id, tx_to FROM hql2_record_revisions
             WHERE kind='vector' ORDER BY tx_from",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].1.is_some());
    assert!(rows[1].1.is_none());
    assert_eq!(vector_revisions(&reopened).len(), 2);
}

#[test]
fn schema6_rejects_unversioned_vector_events_before_wal_append() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(NodeInput {
            id: Some("vector-owner".into()),
            labels: vec!["Note".into()],
            props: None,
            embedding: None,
            lang: Some("en".into()),
            valid_from: Some("2026-09-29T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    let frontier = storage.stable_frontier();
    let vector = VectorEvent {
        node_id: "vector-owner".into(),
        collection: Some("default".into()),
        embedding: vec![0.125, 0.25],
        lang: Some("en".into()),
        clock: LogicalClock::default(),
    };
    let local_error = storage
        .persist(&Event::Vector(vector.clone()))
        .unwrap_err()
        .to_string();
    assert!(local_error.contains("UPGRADE_REQUIRED"), "{local_error}");
    assert_eq!(storage.stable_frontier(), frontier);

    for event in [
        Event::Vector(vector.clone()),
        Event::VectorMaterialized(vector),
    ] {
        let peer_error = storage
            .reconcile_state(vec![SignedEvent {
                event,
                signature: vec![],
                signer_peer_id: "untrusted-peer".into(),
            }])
            .unwrap_err()
            .to_string();
        assert!(peer_error.contains("UPGRADE_REQUIRED"), "{peer_error}");
        assert_eq!(storage.stable_frontier(), frontier);
    }
}
