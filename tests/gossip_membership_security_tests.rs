use genesis_block_native::{
    AuthorizedPeerConfig, BatchInput, ConsensusProposal, EdgeInput, Event, GenesisTransaction,
    GenesisTransactionEvent, GossipMessage, HybridSearchInput, LogicalClock, NodeInput, NodeOutput,
    OpenOptions, PeerMembershipOptions, Storage, VectorEvent,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn open(name: &str) -> (tempfile::TempDir, Storage) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap_or_else(|error| panic!("{name}: {error}"));
    (dir, storage)
}

fn open_current_schema(name: &str) -> (tempfile::TempDir, Storage) {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap_or_else(|error| panic!("{name}: {error}"));
    (dir, storage)
}

fn open_with_identity(name: &str, identity_path: &Path) -> (tempfile::TempDir, Storage) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    fs::copy(identity_path, dir.path().join("identity.bin")).unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap_or_else(|error| panic!("{name}: {error}"));
    (dir, storage)
}

fn reopen(name: &str, dir: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: dir.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn reopen_read_only(name: &str, dir: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: dir.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(true),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn peer(storage: &Storage, port: u16, master_authority: bool) -> AuthorizedPeerConfig {
    let identity = storage.local_identity();
    AuthorizedPeerConfig {
        peer_id: identity.peer_id,
        public_key_hex: identity.public_key_hex,
        gossip_addr: format!("127.0.0.1:{port}"),
        master_authority,
    }
}

fn configure(storage: &Storage, peers: Vec<AuthorizedPeerConfig>, port: u16, local_master: bool) {
    storage
        .configure_peer_membership(peers, &format!("127.0.0.1:{port}"), local_master)
        .unwrap();
}

fn node(id: &str, labels: &[&str]) -> NodeOutput {
    NodeOutput {
        id: id.to_string(),
        labels: labels.iter().map(|label| label.to_string()).collect(),
        props: serde_json::json!({"id": id}),
        impact: None,
        embedding: None,
        lang: Some("en".into()),
        valid_from: "2026-01-01T00:00:00Z".into(),
        valid_to: None,
        caused_by: None,
        expires_at: None,
        clock: LogicalClock {
            time: 1,
            peer_id: "test".into(),
        },
        collection: None,
    }
}

fn node_input(id: &str, labels: &[&str]) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: labels.iter().map(|label| label.to_string()).collect(),
        props: Some(serde_json::json!({"id": id, "source": "security-test"})),
        embedding: None,
        lang: Some("en".into()),
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

fn write_legacy_journal_event(dir: &Path, signed_event: &genesis_block_native::SignedEvent) {
    let mut jsonl = serde_json::to_vec(signed_event).unwrap();
    jsonl.push(b'\n');
    fs::write(dir.join("genesis-graph.wal"), jsonl).unwrap();
}

fn prepare_legacy_database(dir: &Path) {
    let storage = Storage::open(OpenOptions {
        path: dir.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap();
    drop(storage);

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if matches!(
            entry.file_name().to_str(),
            Some("identity.bin" | "genesis.lock")
        ) {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(path).unwrap();
        } else {
            fs::remove_file(path).unwrap();
        }
    }
    assert!(dir.join("identity.bin").exists());
    fs::write(dir.join("state.json"), r#"{"schema_version":5}"#).unwrap();
}

#[test]
fn roster_rejects_a_peer_id_that_does_not_match_its_key() {
    let (_dir, storage) = open("identity-binding");
    let (_remote_dir, remote) = open("identity-binding-remote");
    let mut config = peer(&remote, 31002, false);
    config.peer_id = "0000000000000000".into();

    let error = storage
        .configure_peer_membership(vec![config], "127.0.0.1:31001", true)
        .unwrap_err();
    assert!(error.to_string().contains("PEER_MEMBERSHIP_INVALID"));
    let too_many = vec![peer(&remote, 31002, false); 257];
    let error = storage
        .configure_peer_membership(too_many, "127.0.0.1:31001", true)
        .unwrap_err();
    assert!(error.to_string().contains("roster exceeds"));
    assert!(storage
        .configure_peer_membership(Vec::new(), "127.0.0.1:31001", true)
        .is_ok());
}

#[test]
fn unknown_valid_signer_cannot_enter_sync_or_change_the_wal() {
    let (_known_dir, known) = open("known-signer");
    let (_unknown_dir, unknown) = open("unknown-signer");
    let (_destination_dir, destination) = open("unknown-destination");
    configure(&destination, vec![peer(&known, 31002, false)], 31003, true);

    let event = unknown.sign_event(&Event::Node(node("untrusted", &["USER"])));
    let before = destination.stable_frontier();
    let error = destination.reconcile_state(vec![event]).unwrap_err();
    assert!(error.to_string().contains("SYNC_AUTH_REQUIRED"));
    assert_eq!(destination.stable_frontier(), before);
    assert!(destination.node_view("untrusted").is_none());
}

#[test]
fn roster_member_can_sync_ordinary_state_but_only_master_member_can_sync_master_state() {
    let (_source_dir, source) = open("ordinary-source");
    let (_ordinary_dir, ordinary_destination) = open("ordinary-destination");
    configure(
        &ordinary_destination,
        vec![peer(&source, 31101, false)],
        31102,
        true,
    );

    ordinary_destination
        .reconcile_state(vec![
            source.sign_event(&Event::Node(node("ordinary", &["USER"])))
        ])
        .unwrap();
    assert!(ordinary_destination.node_view("ordinary").is_some());

    let (_master_dir, master_destination) = open("master-destination");
    configure(
        &master_destination,
        vec![peer(&source, 31103, false)],
        31104,
        true,
    );
    let before = master_destination.stable_frontier();
    let error = master_destination
        .reconcile_state(vec![
            source.sign_event(&Event::Node(node("master-blocked", &["MASTER"])))
        ])
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    assert_eq!(master_destination.stable_frontier(), before);
    assert!(master_destination.node_view("master-blocked").is_none());
}

#[test]
fn master_authority_covers_nested_batches_transactions_and_retractions() {
    let (_source_dir, source) = open("master-source");
    let (_destination_dir, destination) = open("master-nested-destination");
    configure(&destination, vec![peer(&source, 31201, false)], 31202, true);
    let transaction = Event::Transaction(GenesisTransactionEvent {
        transaction_id: "nested-master-transaction".into(),
        origin_commit_seq: 0,
        local_frame_seq: None,
        payload_hash: String::new(),
        relational: Vec::new(),
        nodes: vec![node("nested-master", &["MASTER"])],
        edges: Vec::new(),
        vectors: Vec::new(),
        node_retractions: Vec::new(),
        advances_txn_frontier: true,
        record_revision_transaction: None,
    });
    let nested = Event::Batch(vec![Event::Batch(vec![transaction])]);
    let error = destination
        .reconcile_state(vec![source.sign_event(&nested)])
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    assert!(destination.node_view("nested-master").is_none());

    destination
        .reconcile_state(vec![
            destination.sign_event(&Event::Node(node("protected-master", &["MASTER"])))
        ])
        .unwrap();
    let unauthorized_retraction = Event::NodeRetract {
        id: "protected-master".into(),
        clock: LogicalClock {
            time: 2,
            peer_id: source.local_peer_id.clone(),
        },
        retracted_at: "2026-10-16T00:00:00Z".into(),
    };
    let error = destination
        .reconcile_state(vec![source.sign_event(&unauthorized_retraction)])
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    assert!(destination.node_view("protected-master").is_some());
}

#[test]
fn local_peer_id_does_not_skip_signature_verification_and_batch_preflight_is_atomic() {
    let (_dir, storage) = open("local-signature-bypass");
    let forged_local = genesis_block_native::SignedEvent {
        event: Event::Node(node("forged-local", &["USER"])),
        signature: vec![0; 64],
        signer_peer_id: storage.local_peer_id.clone(),
    };
    let before = storage.stable_frontier();
    assert!(storage.reconcile_state(vec![forged_local]).is_err());
    assert_eq!(storage.stable_frontier(), before);
    assert!(storage.node_view("forged-local").is_none());

    let valid = storage.sign_event(&Event::Node(node("valid-prefix", &["USER"])));
    let invalid = genesis_block_native::SignedEvent {
        event: Event::Node(node("invalid-suffix", &["USER"])),
        signature: vec![1; 64],
        signer_peer_id: storage.local_peer_id.clone(),
    };
    let before = storage.stable_frontier();
    assert!(storage.reconcile_state(vec![valid, invalid]).is_err());
    assert_eq!(storage.stable_frontier(), before);
    assert!(storage.node_view("valid-prefix").is_none());
}

#[test]
fn reconcile_batch_cannot_downgrade_master_state_with_a_later_nonmaster_event() {
    let (_master_dir, master) = open("batch-downgrade-master");
    let (_ordinary_dir, ordinary) = open("batch-downgrade-ordinary");
    let (_destination_dir, destination) = open("batch-downgrade-destination");
    configure(
        &destination,
        vec![peer(&master, 31112, true), peer(&ordinary, 31113, false)],
        31111,
        true,
    );

    let privileged = master.sign_event(&Event::Node(node("batch-protected", &["MASTER"])));
    let mut downgrade = node("batch-protected", &["USER"]);
    downgrade.clock.time = 2;
    let unprivileged = ordinary.sign_event(&Event::Node(downgrade));
    let before = destination.stable_frontier();

    let error = destination
        .reconcile_state(vec![privileged, unprivileged])
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    assert_eq!(destination.stable_frontier(), before);
    assert!(destination.node_view("batch-protected").is_none());
}

#[test]
fn local_direct_writes_respect_master_authority_for_live_and_retracted_ids() {
    let (_master_dir, master) = open("local-master-write-master");
    let (_destination_dir, storage) = open("local-master-write-destination");
    configure(&storage, vec![peer(&master, 31990, true)], 31991, false);
    let master_peer_id = master.local_identity().peer_id;
    storage
        .reconcile_state(vec![
            master.sign_event(&Event::Node(node("local-protected", &["MASTER"]))),
            master.sign_event(&Event::Node(node("local-retracted", &["MASTER"]))),
            master.sign_event(&Event::NodeRetract {
                id: "local-retracted".into(),
                clock: LogicalClock {
                    time: 2,
                    peer_id: master_peer_id,
                },
                retracted_at: "2026-10-16T00:00:00Z".into(),
            }),
        ])
        .unwrap();

    let before = storage.stable_frontier();
    let error = storage
        .add_node(node_input("local-protected", &["USER"]))
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    let error = storage
        .add_node(node_input("local-retracted", &["USER"]))
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    let error = storage
        .supersede_node(
            "local-protected".into(),
            Some(serde_json::json!({"source": "unauthorized"})),
            None,
        )
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    let error = storage
        .execute_batch(BatchInput {
            nodes: vec![node_input("local-protected", &["USER"])],
            edges: Vec::new(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    let error = storage
        .commit_transaction(GenesisTransaction {
            transaction_id: "local-unauthorized-master-update".into(),
            expected_frontier: None,
            relational: Vec::new(),
            graph: BatchInput {
                nodes: vec![node_input("local-protected", &["USER"])],
                edges: Vec::new(),
            },
            vectors: Vec::new(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    let error = storage
        .add_vector("local-protected".into(), "default".into(), vec![0.0, 1.0])
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    let error = storage
        .persist(&Event::Node(node("local-protected", &["USER"])))
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));

    assert_eq!(storage.stable_frontier(), before);
    assert!(storage.node_view("local-protected").is_some());
    assert!(storage.node_view("local-retracted").is_none());
}

fn nonmaster_cannot_replace_a_master_nodes_vector(schema6: bool) {
    let open = |name| {
        if schema6 {
            open_current_schema(name)
        } else {
            open(name)
        }
    };
    let (_master_dir, master) = open("vector-master-source");
    let (_attacker_dir, attacker) = open("vector-nonmaster-attacker");
    let (_destination_dir, destination) = open("vector-master-destination");
    configure(
        &destination,
        vec![peer(&master, 31981, true), peer(&attacker, 31982, false)],
        31980,
        false,
    );
    configure(&attacker, Vec::new(), 31983, false);

    // Ordinary vector replication remains available to non-master members in
    // both storage schemas.
    attacker
        .add_node(node_input("ordinary-vector-owner", &["USER"]))
        .unwrap();
    attacker
        .add_vector(
            "ordinary-vector-owner".into(),
            "default".into(),
            vec![0.5, 0.5],
        )
        .unwrap();
    destination
        .reconcile_state(attacker.events_since_seq(0))
        .unwrap();
    assert!(destination.node_view("ordinary-vector-owner").is_some());

    master
        .add_node(node_input("protected-vector-owner", &["MASTER"]))
        .unwrap();
    master
        .add_vector(
            "protected-vector-owner".into(),
            "default".into(),
            vec![1.0, 0.0],
        )
        .unwrap();
    destination
        .reconcile_state(master.events_since_seq(0))
        .unwrap();

    let source_frontier = master.stable_frontier();
    master
        .add_vector(
            "protected-vector-owner".into(),
            "default".into(),
            vec![0.0, 1.0],
        )
        .unwrap();
    let master_vector_update = master
        .events_since_seq(source_frontier)
        .into_iter()
        .last()
        .expect("master vector update should be journaled");
    let forged_authority = attacker.sign_event(&master_vector_update.event);
    let before = destination.stable_frontier();

    let error = destination
        .reconcile_state(vec![forged_authority])
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    assert_eq!(destination.stable_frontier(), before);

    let error = destination
        .add_vector(
            "protected-vector-owner".into(),
            "default".into(),
            vec![0.0, 1.0],
        )
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    assert_eq!(destination.stable_frontier(), before);

    // The same valid update is accepted when its signer has configured MASTER
    // authority, preserving normal replication for protected vectors.
    destination
        .reconcile_state(vec![master_vector_update])
        .unwrap();
    assert!(destination.stable_frontier() > before);
}

#[test]
fn master_vector_mutations_require_authority_for_schema5_and_schema6() {
    nonmaster_cannot_replace_a_master_nodes_vector(false);
    nonmaster_cannot_replace_a_master_nodes_vector(true);
}

#[test]
fn vector_before_master_creation_in_same_batch_requires_master_authority() {
    let (_master_dir, master) = open("batch-vector-master");
    let (_attacker_dir, attacker) = open("batch-vector-attacker");
    let (_destination_dir, destination) = open("batch-vector-destination");
    configure(
        &destination,
        vec![peer(&master, 31991, true), peer(&attacker, 31992, false)],
        31990,
        false,
    );

    let node_id = "batch-vector-protected";
    let vector = attacker.sign_event(&Event::Vector(VectorEvent {
        node_id: node_id.into(),
        collection: Some("default".into()),
        embedding: vec![1.0, 0.0],
        lang: Some("en".into()),
        clock: LogicalClock {
            time: 1,
            peer_id: attacker.local_peer_id.clone(),
        },
    }));
    let master_creation = master.sign_event(&Event::Node(node(node_id, &["MASTER"])));
    let before = destination.stable_frontier();

    let error = destination
        .reconcile_state(vec![vector, master_creation])
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    assert_eq!(destination.stable_frontier(), before);
    assert!(destination.node_view(node_id).is_none());
}

fn compacted_master_vectors_survive_nonmaster_relay(schema6: bool) {
    let open = |name| {
        if schema6 {
            open_current_schema(name)
        } else {
            open(name)
        }
    };
    let (_master_dir, master) = open("compacted-vector-master");
    let (_relay_dir, relay) = open("compacted-vector-relay");
    let (_receiver_dir, receiver) = open("compacted-vector-receiver");
    configure(
        &master,
        vec![peer(&relay, 31995, false), peer(&receiver, 31996, false)],
        31994,
        true,
    );
    configure(
        &relay,
        vec![peer(&master, 31994, true), peer(&receiver, 31996, false)],
        31995,
        false,
    );
    configure(
        &receiver,
        vec![peer(&master, 31994, true), peer(&relay, 31995, false)],
        31996,
        false,
    );

    master
        .create_collection(
            "code".into(),
            "security-test".into(),
            2,
            Some("L2".into()),
            None,
            None,
            None,
        )
        .unwrap();
    let mut protected = node_input("compacted-master-vector", &["MASTER"]);
    protected.embedding = Some(vec![1.0, 0.0]);
    master.add_node(protected).unwrap();
    master
        .add_vector(
            "compacted-master-vector".into(),
            "code".into(),
            vec![0.25, 0.75],
        )
        .unwrap();
    relay.reconcile_state(master.events_since_seq(0)).unwrap();

    relay
        .add_node(node_input("compacted-relay-user-vector", &["USER"]))
        .unwrap();
    relay
        .add_vector(
            "compacted-relay-user-vector".into(),
            "code".into(),
            vec![0.0, 1.0],
        )
        .unwrap();
    relay.compact().unwrap();
    relay.compact().unwrap();

    receiver.reconcile_state(relay.events_since_seq(0)).unwrap();
    receiver.flush_index();
    let search = |storage: &Storage, collection: Option<&str>| {
        storage
            .hybrid_search(HybridSearchInput {
                query_vector: vec![0.25, 0.75],
                k: 10,
                alpha: Some(0.0),
                lang: None,
                as_of: None,
                collection: collection.map(str::to_string),
                ef_search: None,
                oversample: None,
            })
            .unwrap()
            .into_iter()
            .map(|hit| hit.node.id)
            .collect::<Vec<_>>()
    };
    assert!(search(&receiver, None).contains(&"compacted-master-vector".into()));
    let custom_collection_hits = search(&receiver, Some("code"));
    assert!(custom_collection_hits.contains(&"compacted-master-vector".into()));
    assert!(custom_collection_hits.contains(&"compacted-relay-user-vector".into()));
}

#[test]
fn compacted_master_vectors_keep_authority_and_schema5_schema6_relay() {
    compacted_master_vectors_survive_nonmaster_relay(false);
    compacted_master_vectors_survive_nonmaster_relay(true);
}

fn open_legacy_master_journal_with_membership(
    dir: &Path,
    signer: &Storage,
    peer_config: AuthorizedPeerConfig,
    node_id: &str,
) -> Result<Storage, String> {
    prepare_legacy_database(dir);
    write_legacy_journal_event(
        dir,
        &signer.sign_event(&Event::Node(node(node_id, &["MASTER"]))),
    );
    Storage::open_with_membership(
        OpenOptions {
            path: dir.to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(2),
            retention: Some("full".into()),
        },
        PeerMembershipOptions {
            local_gossip_addr: "127.0.0.1:31970".into(),
            local_master_authority: Some(false),
            peers: vec![peer_config],
        },
    )
    .map_err(|error| error.to_string())
}

#[test]
fn membership_open_rejects_legacy_master_event_from_unrostered_signer() {
    let (_attacker_dir, attacker) = open_current_schema("legacy-unrostered-attacker");
    let (_trusted_dir, trusted_member) = open_current_schema("legacy-trusted-member");
    let target = tempdir().unwrap();

    let error = open_legacy_master_journal_with_membership(
        target.path(),
        &attacker,
        peer(&trusted_member, 31971, true),
        "legacy-unrostered-master",
    )
    .err()
    .expect("unrostered signer should fail startup authorization");

    assert!(error.contains("RECOVERY_AUTH_REQUIRED"));
}

#[test]
fn membership_open_rejects_legacy_master_event_from_nonmaster_member() {
    let (_member_dir, nonmaster_member) = open_current_schema("legacy-nonmaster-member");
    let target = tempdir().unwrap();

    let error = open_legacy_master_journal_with_membership(
        target.path(),
        &nonmaster_member,
        peer(&nonmaster_member, 31972, false),
        "legacy-nonmaster-master",
    )
    .err()
    .expect("non-master signer should fail startup authorization");

    assert!(error.contains("RECOVERY_AUTH_REQUIRED"));
}

#[test]
fn membership_open_replays_legacy_master_event_from_authorized_member() {
    let (_master_dir, master_member) = open_current_schema("legacy-authorized-master");
    let target = tempdir().unwrap();

    let reopened = open_legacy_master_journal_with_membership(
        target.path(),
        &master_member,
        peer(&master_member, 31973, true),
        "legacy-authorized-master-node",
    )
    .expect("authorized historical master event should recover");

    assert!(reopened
        .node_view("legacy-authorized-master-node")
        .is_some());
}

#[test]
fn late_membership_configuration_preflights_replayed_journal() {
    let (_attacker_dir, attacker) = open_current_schema("late-membership-attacker");
    let (_unrelated_dir, unrelated_member) = open_current_schema("late-membership-unrelated");
    let target = tempdir().unwrap();
    prepare_legacy_database(target.path());
    write_legacy_journal_event(
        target.path(),
        &attacker.sign_event(&Event::Node(node(
            "late-membership-untrusted-master",
            &["MASTER"],
        ))),
    );

    let reopened = Storage::open(OpenOptions {
        path: target.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap();
    assert!(reopened
        .node_view("late-membership-untrusted-master")
        .is_some());

    let error = reopened
        .configure_peer_membership(
            vec![peer(&unrelated_member, 31974, false)],
            "127.0.0.1:31975",
            false,
        )
        .unwrap_err();
    assert!(error.to_string().contains("RECOVERY_AUTH_REQUIRED"));

    // Failed validation leaves configuration available for a corrected roster.
    reopened
        .configure_peer_membership(vec![peer(&attacker, 31976, true)], "127.0.0.1:31977", false)
        .expect("authorized historical signer should permit late configuration");
}

#[test]
fn late_membership_configuration_preflights_compacted_master_history() {
    let (_attacker_dir, attacker) = open_current_schema("late-compacted-master");
    let (_unrelated_dir, unrelated_member) = open_current_schema("late-compacted-unrelated");
    let target = tempdir().unwrap();
    prepare_legacy_database(target.path());
    let options = || OpenOptions {
        path: target.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    };
    let storage = Storage::open_with_membership(
        options(),
        PeerMembershipOptions {
            local_gossip_addr: "127.0.0.1:31978".into(),
            local_master_authority: Some(false),
            peers: vec![peer(&attacker, 31979, true)],
        },
    )
    .unwrap();
    storage
        .reconcile_state(vec![attacker.sign_event(&Event::Node(node(
            "late-compacted-master-node",
            &["MASTER"],
        )))])
        .unwrap();
    storage.compact().unwrap();
    drop(storage);

    let reopened = Storage::open(options()).unwrap();
    assert!(reopened.node_view("late-compacted-master-node").is_some());
    let error = reopened
        .configure_peer_membership(
            vec![peer(&unrelated_member, 31980, true)],
            "127.0.0.1:31981",
            false,
        )
        .unwrap_err();
    assert!(error.to_string().contains("RECOVERY_AUTH_REQUIRED"));

    reopened
        .configure_peer_membership(vec![peer(&attacker, 31982, true)], "127.0.0.1:31983", false)
        .expect("compacted proof should authorize late configuration with the correct roster");
}

#[test]
fn retracted_identity_cannot_be_resurrected_by_a_nonmaster_peer() {
    let (_master_dir, master) = open("tombstone-master");
    let (_ordinary_dir, ordinary) = open("tombstone-ordinary");
    let (_destination_dir, destination) = open("tombstone-destination");
    configure(
        &destination,
        vec![peer(&master, 31122, true), peer(&ordinary, 31123, false)],
        31121,
        true,
    );
    destination
        .reconcile_state(vec![
            master.sign_event(&Event::Node(node("retracted-master", &["MASTER"])))
        ])
        .unwrap();
    destination
        .reconcile_state(vec![master.sign_event(&Event::NodeRetract {
            id: "retracted-master".into(),
            clock: LogicalClock {
                time: 2,
                peer_id: master.local_peer_id.clone(),
            },
            retracted_at: "2026-10-16T00:00:00Z".into(),
        })])
        .unwrap();

    let mut resurrection = node("retracted-master", &["USER"]);
    resurrection.clock.time = 3;
    let before = destination.stable_frontier();
    let error = destination
        .reconcile_state(vec![ordinary.sign_event(&Event::Node(resurrection))])
        .unwrap_err();
    assert!(error.to_string().contains("GOVERNANCE_AUTH_REQUIRED"));
    assert_eq!(destination.stable_frontier(), before);
    assert!(destination.node_view("retracted-master").is_none());
}

#[test]
fn compaction_preserves_master_signatures_for_nodes_and_retractions() {
    let (_master_dir, master) = open("compaction-master-origin");
    let (_relay_dir, relay) = open("compaction-nonmaster-relay");
    let (_receiver_dir, receiver) = open("compaction-receiver");

    configure(
        &master,
        vec![peer(&relay, 31212, false), peer(&receiver, 31213, false)],
        31211,
        true,
    );
    configure(
        &relay,
        vec![peer(&master, 31211, true), peer(&receiver, 31213, false)],
        31212,
        false,
    );
    configure(
        &receiver,
        vec![peer(&master, 31211, true), peer(&relay, 31212, false)],
        31213,
        false,
    );

    master
        .add_node(node_input("master-live-after-fold", &["MASTER"]))
        .unwrap();
    master
        .add_node(node_input("master-retracted-after-fold", &["MASTER"]))
        .unwrap();
    master
        .reconcile_state(vec![master.sign_event(&Event::Batch(vec![Event::Node(
            node("master-batched-after-fold", &["MASTER"]),
        )]))])
        .unwrap();
    master
        .reconcile_state(vec![master.sign_event(&Event::Transaction(
            GenesisTransactionEvent {
                transaction_id: "master-transaction-after-fold".into(),
                origin_commit_seq: 0,
                local_frame_seq: None,
                payload_hash: String::new(),
                relational: Vec::new(),
                nodes: vec![node("master-transactional-after-fold", &["MASTER"])],
                edges: Vec::new(),
                vectors: Vec::new(),
                node_retractions: Vec::new(),
                advances_txn_frontier: false,
                record_revision_transaction: None,
            },
        ))])
        .unwrap();
    master.retract_node("master-retracted-after-fold").unwrap();
    relay.reconcile_state(master.events_since_seq(0)).unwrap();

    relay.compact().unwrap();
    relay.compact().unwrap();
    let relayed = relay.events_since_seq(0);
    let live_master_event = relayed.iter().find(
        |signed| matches!(&signed.event, Event::Node(node) if node.id == "master-live-after-fold"),
    );
    assert!(live_master_event.is_some());
    assert_eq!(
        live_master_event.unwrap().signer_peer_id,
        relay.local_peer_id
    );
    assert!(relayed.iter().any(|signed| {
        signed.signer_peer_id == relay.local_peer_id
            && matches!(&signed.event, Event::Node(node) if node.id == "master-batched-after-fold")
    }));
    assert!(relayed.iter().any(|signed| {
        signed.signer_peer_id == relay.local_peer_id
            && matches!(&signed.event, Event::Node(node) if node.id == "master-transactional-after-fold")
    }));
    let master_retraction_event = relayed.iter().find(|signed| {
        matches!(&signed.event, Event::NodeRetract { id, .. } if id == "master-retracted-after-fold")
    });
    assert!(master_retraction_event.is_some());
    assert_eq!(
        master_retraction_event.unwrap().signer_peer_id,
        relay.local_peer_id
    );
    assert!(!relayed.iter().any(|signed| match &signed.event {
        Event::Batch(events) => events.iter().any(|event| {
            matches!(event, Event::Node(node) if node.id == "master-batched-after-fold")
        }),
        Event::Transaction(transaction) => transaction
            .nodes
            .iter()
            .any(|node| node.id == "master-transactional-after-fold"),
        _ => false,
    }));

    receiver.reconcile_state(relayed).unwrap();
    assert!(receiver.node_view("master-live-after-fold").is_some());
    assert!(receiver.node_view("master-batched-after-fold").is_some());
    assert!(receiver
        .node_view("master-transactional-after-fold")
        .is_some());
    assert!(receiver.node_view("master-retracted-after-fold").is_none());
}

#[test]
fn compacted_master_proof_does_not_replay_a_stale_nested_retraction() {
    let (_master_dir, master) = open("compaction-nested-master-origin");
    let (_relay_dir, relay) = open("compaction-nested-relay");
    let (_receiver_dir, receiver) = open("compaction-nested-receiver");
    configure(
        &master,
        vec![peer(&relay, 31222, false), peer(&receiver, 31223, false)],
        31221,
        true,
    );
    configure(
        &relay,
        vec![peer(&master, 31221, true), peer(&receiver, 31223, false)],
        31222,
        false,
    );
    configure(
        &receiver,
        vec![peer(&master, 31221, true), peer(&relay, 31222, false)],
        31223,
        false,
    );

    let nested_retraction = Event::Batch(vec![
        Event::NodeRetract {
            id: "resurrected-user-node".into(),
            clock: LogicalClock {
                time: 2,
                peer_id: master.local_peer_id.clone(),
            },
            retracted_at: "2026-10-16T00:00:00Z".into(),
        },
        Event::Node(node("still-live-master-node", &["MASTER"])),
    ]);
    let signed_nested_retraction = master.sign_event(&nested_retraction);
    let mut user_resurrection = node("resurrected-user-node", &["USER"]);
    user_resurrection.clock = LogicalClock {
        time: 3,
        peer_id: master.local_peer_id.clone(),
    };
    let signed_user_resurrection = master.sign_event(&Event::Node(user_resurrection));

    // This peer already has the stale tombstone when it receives the relay's
    // compacted base. The base must carry the later authorized USER transition
    // without replaying the old Batch's retraction a second time.
    receiver
        .reconcile_state(vec![signed_nested_retraction.clone()])
        .unwrap();
    relay
        .reconcile_state(vec![signed_nested_retraction, signed_user_resurrection])
        .unwrap();
    relay.compact().unwrap();
    for (index, signed) in relay.events_since_seq(0).into_iter().enumerate() {
        let kind = match &signed.event {
            Event::Node(node) => format!("Node {}", node.id),
            Event::Edge(edge) => format!("Edge {}", edge.id),
            Event::NodeRetract { id, .. } => format!("NodeRetract {id}"),
            Event::VectorMaterialized(vector) => format!("VectorMaterialized {}", vector.node_id),
            _ => "Other".to_string(),
        };
        receiver
            .reconcile_state(vec![signed])
            .unwrap_or_else(|error| panic!("compacted event {index} ({kind}) rejected: {error}"));
    }

    let resurrected = receiver
        .node_view("resurrected-user-node")
        .expect("compacted base should clear the stale tombstone");
    assert!(!resurrected.labels.iter().any(|label| label == "MASTER"));
    assert!(receiver.node_view("still-live-master-node").is_some());
}

#[test]
fn signed_gossip_requires_roster_identity_digest_and_pinned_endpoint() {
    let (_a_dir, a) = open("packet-a");
    let (_b_dir, b) = open("packet-b");
    configure(&a, vec![peer(&b, 31302, false)], 31301, true);
    configure(&b, vec![peer(&a, 31301, true)], 31302, false);

    let mut packet = a
        .sign_gossip_packet(GossipMessage::BootstrapRequired {
            reason: "test".into(),
        })
        .unwrap();
    assert!(b.verify_gossip_packet(&packet, "127.0.0.1:31301".parse().unwrap()));
    assert!(!b.verify_gossip_packet(&packet, "127.0.0.1:31399".parse().unwrap()));

    packet.message = GossipMessage::BootstrapRequired {
        reason: "tampered".into(),
    };
    assert!(!b.verify_gossip_packet(&packet, "127.0.0.1:31301".parse().unwrap()));

    let (_mismatch_dir, mismatch) = open("packet-roster-mismatch");
    configure(&mismatch, vec![peer(&a, 31301, false)], 31303, false);
    let fresh_packet = a
        .sign_gossip_packet(GossipMessage::BootstrapRequired {
            reason: "same-roster-membership-required".into(),
        })
        .unwrap();
    assert!(!mismatch.verify_gossip_packet(&fresh_packet, "127.0.0.1:31301".parse().unwrap()));
}

#[test]
fn consensus_vote_cannot_be_replayed_for_a_different_event_under_the_same_id() {
    let (_voter_dir, voter) = open("vote-replay-voter");
    let (_receiver_dir, receiver) = open("vote-replay-receiver");
    configure(&receiver, vec![peer(&voter, 31412, true)], 31411, true);

    let proposal_id = "reused-proposal-id".to_string();
    let approved_event = Event::Node(node("approved-event", &["MASTER"]));
    voter.proposals.insert(
        proposal_id.clone(),
        ConsensusProposal {
            proposal_id: proposal_id.clone(),
            signed_event: voter.sign_event(&approved_event),
            votes: Default::default(),
            quorum_signatures: Default::default(),
            committed: false,
        },
    );
    let vote = voter.sign_vote(proposal_id.clone(), true).unwrap();

    let substituted_event = Event::Node(node("substituted-event", &["MASTER"]));
    receiver.proposals.insert(
        proposal_id.clone(),
        ConsensusProposal {
            proposal_id: proposal_id.clone(),
            signed_event: receiver.sign_event(&substituted_event),
            votes: Default::default(),
            quorum_signatures: Default::default(),
            committed: false,
        },
    );

    let before = receiver.stable_frontier();
    let error = receiver
        .submit_vote(proposal_id.clone(), voter.local_peer_id.clone(), true, vote)
        .unwrap_err();
    assert!(error.to_string().contains("invalid vote signature"));
    assert_eq!(receiver.stable_frontier(), before);
    assert!(receiver.node_view("substituted-event").is_none());
    let stored = receiver.proposals.get(&proposal_id).unwrap();
    assert!(stored.votes.is_empty());
    assert!(stored.quorum_signatures.is_empty());
}

#[test]
fn consensus_vote_is_one_shot_and_identical_replays_are_idempotent() {
    let (_master_dir, master) = open("one-shot-master");
    let (voter_dir, voter_approval) = open("one-shot-voter");
    let (_voter_replay_dir, voter_rejection) = open_with_identity(
        "one-shot-voter-replay",
        &voter_dir.path().join("identity.bin"),
    );
    configure(
        &master,
        vec![peer(&voter_approval, 31422, false)],
        31421,
        true,
    );
    configure(
        &voter_approval,
        vec![peer(&master, 31421, true)],
        31422,
        false,
    );
    configure(
        &voter_rejection,
        vec![peer(&master, 31421, true)],
        31423,
        false,
    );

    let proposal_id = master
        .propose_consensus(Event::Node(node("one-shot-vote", &["USER"])), Vec::new())
        .unwrap();
    voter_approval
        .import_consensus_proposal(master.export_consensus_proposal(&proposal_id).unwrap())
        .unwrap();
    voter_rejection
        .import_consensus_proposal(master.export_consensus_proposal(&proposal_id).unwrap())
        .unwrap();

    let approval = voter_approval.sign_vote(proposal_id.clone(), true).unwrap();
    let local_flip = voter_approval
        .sign_vote(proposal_id.clone(), false)
        .unwrap_err();
    assert!(local_flip.to_string().contains("CONFLICTING_VOTE"));
    let rejection = voter_rejection
        .sign_vote(proposal_id.clone(), false)
        .unwrap();
    assert!(!master
        .submit_vote(
            proposal_id.clone(),
            voter_rejection.local_peer_id.clone(),
            false,
            rejection.clone(),
        )
        .unwrap());

    let error = master
        .submit_vote(
            proposal_id.clone(),
            voter_approval.local_peer_id.clone(),
            true,
            approval,
        )
        .unwrap_err();
    assert!(error.to_string().contains("CONFLICTING_VOTE"));

    assert!(!master
        .submit_vote(
            proposal_id.clone(),
            voter_rejection.local_peer_id.clone(),
            false,
            rejection.clone(),
        )
        .unwrap());
    let stored = master.proposals.get(&proposal_id).unwrap();
    assert_eq!(
        stored.votes.get(&voter_rejection.local_peer_id),
        Some(&false)
    );
    assert_eq!(
        stored
            .quorum_signatures
            .get(&voter_rejection.local_peer_id)
            .unwrap(),
        &rejection
    );
}

#[test]
fn externally_injected_public_vote_entries_cannot_count_toward_quorum() {
    let (_master_dir, master) = open("injected-vote-master");
    let (_voter_dir, voter) = open("injected-vote-voter");
    configure(&master, vec![peer(&voter, 31472, false)], 31471, true);
    configure(&voter, vec![peer(&master, 31471, true)], 31472, false);

    let proposal_id = master
        .propose_consensus(
            Event::Node(node("vote-map-injection", &["USER"])),
            Vec::new(),
        )
        .unwrap();
    voter
        .import_consensus_proposal(master.export_consensus_proposal(&proposal_id).unwrap())
        .unwrap();

    master
        .proposals
        .get_mut(&proposal_id)
        .unwrap()
        .votes
        .insert(voter.local_peer_id.clone(), true);
    let local_vote = master.sign_vote(proposal_id.clone(), true).unwrap();
    assert!(!master
        .submit_vote(
            proposal_id.clone(),
            master.local_peer_id.clone(),
            true,
            local_vote,
        )
        .unwrap());
    assert!(master.node_view("vote-map-injection").is_none());
    assert_eq!(
        master.proposals.get(&proposal_id).unwrap().votes.len(),
        1,
        "unverified public map entries are discarded"
    );

    let real_vote = voter.sign_vote(proposal_id.clone(), true).unwrap();
    assert!(master
        .submit_vote(proposal_id, voter.local_peer_id.clone(), true, real_vote)
        .unwrap());
    assert!(master.node_view("vote-map-injection").is_some());
}

#[test]
fn durable_vote_locks_survive_restart_and_reject_stale_conflicting_signatures() {
    let (master_dir, master) = open("durable-vote-master");
    let (voter_dir, voter) = open("durable-vote-voter");
    let (_equivocating_voter_dir, equivocating_voter) = open_with_identity(
        "durable-vote-equivocating-voter",
        &voter_dir.path().join("identity.bin"),
    );
    configure(&master, vec![peer(&voter, 31442, false)], 31441, true);
    configure(&voter, vec![peer(&master, 31441, true)], 31442, false);
    configure(
        &equivocating_voter,
        vec![peer(&master, 31441, true)],
        31443,
        false,
    );

    let proposal_id = master
        .propose_consensus(Event::Node(node("durable-vote", &["USER"])), Vec::new())
        .unwrap();
    let exported = master.export_consensus_proposal(&proposal_id).unwrap();
    voter.import_consensus_proposal(exported.clone()).unwrap();
    equivocating_voter
        .import_consensus_proposal(exported.clone())
        .unwrap();
    let stale_approval = voter.sign_vote(proposal_id.clone(), true).unwrap();
    let stale_approval_retry = stale_approval.clone();
    let first_rejection = equivocating_voter
        .sign_vote(proposal_id.clone(), false)
        .unwrap();

    assert!(!master
        .submit_vote(
            proposal_id.clone(),
            voter.local_peer_id.clone(),
            false,
            first_rejection,
        )
        .unwrap());

    drop(master);
    let master = reopen("durable-vote-master-reopen", master_dir.path());
    configure(&master, vec![peer(&voter, 31442, false)], 31441, true);
    master.import_consensus_proposal(exported.clone()).unwrap();
    let error = master
        .submit_vote(
            proposal_id.clone(),
            voter.local_peer_id.clone(),
            true,
            stale_approval,
        )
        .unwrap_err();
    assert!(error.to_string().contains("CONFLICTING_VOTE"));
    assert_eq!(
        master
            .proposals
            .get(&proposal_id)
            .unwrap()
            .votes
            .get(&voter.local_peer_id),
        Some(&false)
    );

    drop(voter);
    let voter = reopen("durable-vote-voter-reopen", voter_dir.path());
    configure(&voter, vec![peer(&master, 31441, true)], 31442, false);
    voter.import_consensus_proposal(exported).unwrap();
    let error = voter.sign_vote(proposal_id.clone(), false).unwrap_err();
    assert!(error.to_string().contains("CONFLICTING_VOTE"));
    assert_eq!(
        voter.sign_vote(proposal_id, true).unwrap(),
        stale_approval_retry
    );
}

#[test]
fn committed_proposals_are_idempotent_after_restart_and_reimport() {
    let (master_dir, master) = open("committed-replay-master");
    let (_voter_dir, voter) = open("committed-replay-voter");
    let (_other_dir, other) = open("committed-replay-other");
    configure(
        &master,
        vec![peer(&voter, 31452, false), peer(&other, 31453, false)],
        31451,
        true,
    );
    configure(
        &voter,
        vec![peer(&master, 31451, true), peer(&other, 31453, false)],
        31452,
        false,
    );

    let proposal_id = master
        .propose_consensus(Event::Node(node("committed-once", &["USER"])), Vec::new())
        .unwrap();
    let exported = master.export_consensus_proposal(&proposal_id).unwrap();
    voter.import_consensus_proposal(exported.clone()).unwrap();
    let master_vote = master.sign_vote(proposal_id.clone(), true).unwrap();
    let voter_vote = voter.sign_vote(proposal_id.clone(), true).unwrap();
    assert!(!master
        .submit_vote(
            proposal_id.clone(),
            master.local_peer_id.clone(),
            true,
            master_vote,
        )
        .unwrap());
    assert!(master
        .submit_vote(
            proposal_id.clone(),
            voter.local_peer_id.clone(),
            true,
            voter_vote.clone(),
        )
        .unwrap());
    assert!(master.node_view("committed-once").is_some());
    let before_restart_frontier = master.stable_frontier();
    // Model a crash after the WAL append but before the completion marker is
    // durable. The prepare record must recover the marker from the WAL frame.
    let commit_ledger = master_dir.path().join("consensus-commits.jsonl");
    let records = fs::read_to_string(&commit_ledger).unwrap();
    let mut lines = records.lines();
    let prepare_record = lines.next().unwrap();
    assert!(lines.next().unwrap().contains("\"committed\":true"));
    assert!(lines.next().is_none());
    fs::write(&commit_ledger, format!("{prepare_record}\n")).unwrap();
    drop(master);

    // A read-only process may inspect the database but must not complete the
    // pending durable receipt as a side effect of importing a proposal.
    let pending_ledger_bytes = fs::read(&commit_ledger).unwrap();
    let read_only_master = reopen_read_only("committed-replay-read-only", master_dir.path());
    let error = read_only_master
        .import_consensus_proposal(exported.clone())
        .unwrap_err();
    assert!(error.to_string().contains("read-only"));
    assert_eq!(fs::read(&commit_ledger).unwrap(), pending_ledger_bytes);
    drop(read_only_master);

    let master = reopen("committed-replay-master-reopen", master_dir.path());
    configure(
        &master,
        vec![peer(&voter, 31452, false), peer(&other, 31453, false)],
        31451,
        true,
    );
    // Recovery completes the pending receipt while the original signed event
    // is still in the journal. Compact before proposal re-import to prove the
    // commit remains recognizable after compaction rewrites that event.
    master.compact().unwrap();
    let after_recovery_compact_frontier = master.stable_frontier();
    assert_eq!(after_recovery_compact_frontier, before_restart_frontier);
    master.import_consensus_proposal(exported.clone()).unwrap();
    assert!(master.proposals.get(&proposal_id).unwrap().committed);
    let after_import_frontier = master.stable_frontier();
    assert_eq!(after_import_frontier, after_recovery_compact_frontier);
    assert!(master
        .submit_vote(proposal_id, voter.local_peer_id.clone(), true, voter_vote,)
        .unwrap());
    assert_eq!(master.stable_frontier(), after_import_frontier);
    assert!(master.node_view("committed-once").is_some());

    master.compact().unwrap();
    let after_compact_frontier = master.stable_frontier();
    drop(master);
    let master = reopen("committed-replay-master-folded-reopen", master_dir.path());
    configure(
        &master,
        vec![peer(&voter, 31452, false), peer(&other, 31453, false)],
        31451,
        true,
    );
    master.import_consensus_proposal(exported).unwrap();
    assert_eq!(master.stable_frontier(), after_compact_frontier);
}

#[test]
fn exported_proposal_can_be_imported_and_voted_on_by_an_authorized_member() {
    let (_master_dir, master) = open("proposal-export-master");
    let (_voter_dir, voter) = open("proposal-export-voter");
    configure(&master, vec![peer(&voter, 31432, false)], 31431, true);
    configure(&voter, vec![peer(&master, 31431, true)], 31432, false);

    let proposal_id = master
        .propose_consensus(Event::Node(node("relayed-proposal", &["USER"])), Vec::new())
        .unwrap();
    let mut exported = master.export_consensus_proposal(&proposal_id).unwrap();
    exported.votes.insert(voter.local_peer_id.clone(), false);
    exported
        .quorum_signatures
        .insert(voter.local_peer_id.clone(), vec![7; 64]);
    exported.committed = true;
    voter.import_consensus_proposal(exported).unwrap();

    let imported = voter.proposals.get(&proposal_id).unwrap();
    assert!(imported.votes.is_empty());
    assert!(imported.quorum_signatures.is_empty());
    assert!(!imported.committed);
    drop(imported);

    let local_vote = master.sign_vote(proposal_id.clone(), true).unwrap();
    assert!(!master
        .submit_vote(
            proposal_id.clone(),
            master.local_peer_id.clone(),
            true,
            local_vote,
        )
        .unwrap());
    let remote_vote = voter.sign_vote(proposal_id.clone(), true).unwrap();
    assert!(master
        .submit_vote(proposal_id, voter.local_peer_id.clone(), true, remote_vote,)
        .unwrap());
    assert!(master.node_view("relayed-proposal").is_some());
}

#[test]
fn schema6_consensus_rejects_raw_graph_events_before_voting() {
    let (_master_dir, master) = open_current_schema("raw-schema6-consensus-master");
    let (voter_dir, voter) = open_current_schema("raw-schema6-consensus-voter");
    configure(&master, vec![peer(&voter, 31482, false)], 31481, true);
    configure(&voter, vec![peer(&master, 31481, true)], 31482, false);

    let proposal_id = "raw-schema6-node-proposal".to_string();
    let proposal = ConsensusProposal {
        proposal_id: proposal_id.clone(),
        signed_event: master.sign_event(&Event::Node(node(
            "raw-schema6-consensus-node",
            &["MASTER"],
        ))),
        votes: Default::default(),
        quorum_signatures: Default::default(),
        committed: false,
    };
    let before = voter.stable_frontier();
    let error = voter
        .import_consensus_proposal(proposal.clone())
        .unwrap_err();
    assert!(error.to_string().contains("revision-bearing graph events"));
    assert_eq!(voter.stable_frontier(), before);
    assert!(!voter.proposals.contains_key(&proposal_id));
    assert!(voter.node_view("raw-schema6-consensus-node").is_none());

    // Defense in depth: even if an invalid proposal is injected into the
    // in-memory map by a stale or corrupted caller, the voting path rejects it
    // before creating a durable vote lock.
    voter.proposals.insert(proposal_id.clone(), proposal);
    let error = voter.sign_vote(proposal_id.clone(), true).unwrap_err();
    assert!(error.to_string().contains("revision-bearing graph events"));
    assert!(!voter_dir.path().join("consensus-votes.jsonl").exists());
    assert_eq!(voter.stable_frontier(), before);
}

#[test]
fn schema6_consensus_rejects_malformed_revision_envelope_before_admission() {
    let (_master_dir, master) = open_current_schema("malformed-consensus-master");
    let (voter_dir, voter) = open_current_schema("malformed-consensus-voter");
    configure(&master, vec![peer(&voter, 31484, false)], 31483, true);
    configure(&voter, vec![peer(&master, 31483, true)], 31484, false);

    let proposal_id = master
        .propose_consensus(
            Event::Node(node("malformed-consensus-node", &["USER"])),
            Vec::new(),
        )
        .unwrap();
    let mut proposal = master.export_consensus_proposal(&proposal_id).unwrap();
    let Event::Transaction(transaction) = &mut proposal.signed_event.event else {
        panic!("schema-6 graph proposal should use a revision transaction");
    };
    transaction
        .record_revision_transaction
        .as_mut()
        .expect("graph proposal must carry a revision envelope")
        .mutations[0]
        .revision_id = "not-a-uuid".to_string();
    proposal.signed_event = master.sign_event(&proposal.signed_event.event);

    let before = voter.stable_frontier();
    voter.import_consensus_proposal(proposal).unwrap_err();
    assert_eq!(voter.stable_frontier(), before);
    assert!(!voter.proposals.contains_key(&proposal_id));
    assert!(voter.node_view("malformed-consensus-node").is_none());
    assert!(!voter_dir.path().join("consensus-votes.jsonl").exists());
}

#[test]
fn consensus_quorum_uses_the_fixed_roster_not_discovered_peers() {
    let (_local_dir, local) = open("quorum-local");
    let (_peer_a_dir, peer_a) = open("quorum-peer-a");
    let (_peer_b_dir, peer_b) = open("quorum-peer-b");
    configure(
        &local,
        vec![peer(&peer_a, 31402, false), peer(&peer_b, 31403, false)],
        31401,
        true,
    );
    configure(
        &peer_a,
        vec![peer(&local, 31401, true), peer(&peer_b, 31403, false)],
        31402,
        false,
    );

    // A discovered cache entry is not membership and cannot supply a voter key.
    let (_unlisted_dir, unlisted) = open("quorum-unlisted");
    local.peers.insert(
        unlisted.local_peer_id.clone(),
        genesis_block_native::SyncPeer {
            id: unlisted.local_peer_id.clone(),
            addr: "127.0.0.1:31404".into(),
            last_seen: 0,
            verifying_key: unlisted.verifying_key.to_bytes().to_vec(),
        },
    );

    let proposal = local
        .propose_consensus(Event::Node(node("quorum-master", &["USER"])), Vec::new())
        .unwrap();
    let stored_proposal = local
        .proposals
        .get(&proposal)
        .map(|proposal| proposal.value().clone())
        .unwrap();
    unlisted
        .proposals
        .insert(proposal.clone(), stored_proposal.clone());
    peer_a.proposals.insert(proposal.clone(), stored_proposal);

    let local_vote = local.sign_vote(proposal.clone(), true).unwrap();
    assert!(!local
        .submit_vote(
            proposal.clone(),
            local.local_peer_id.clone(),
            true,
            local_vote
        )
        .unwrap());

    assert!(local
        .submit_vote(
            proposal.clone(),
            unlisted.local_peer_id.clone(),
            true,
            Vec::new(),
        )
        .is_err());

    let authorized_vote = peer_a.sign_vote(proposal.clone(), true).unwrap();
    assert!(local
        .submit_vote(
            proposal,
            peer_a.local_peer_id.clone(),
            true,
            authorized_vote
        )
        .unwrap());
    assert!(local.node_view("quorum-master").is_some());
}

#[test]
fn current_schema_accepts_authenticated_compacted_graph_materializations() {
    let (_master_dir, master) = open_current_schema("schema6-compaction-master");
    let (_relay_dir, relay) = open_current_schema("schema6-compaction-relay");
    let (receiver_dir, receiver) = open_current_schema("schema6-compaction-receiver");
    configure(
        &master,
        vec![peer(&relay, 31512, false), peer(&receiver, 31513, false)],
        31511,
        true,
    );
    configure(
        &relay,
        vec![peer(&master, 31511, true), peer(&receiver, 31513, false)],
        31512,
        false,
    );
    configure(
        &receiver,
        vec![peer(&master, 31511, true), peer(&relay, 31512, false)],
        31513,
        false,
    );

    master
        .add_node(node_input("schema6-master-live", &["MASTER"]))
        .unwrap();
    let mut vector_node = node_input("schema6-user-vector", &["USER"]);
    vector_node.embedding = Some(vec![0.1, 0.2]);
    master.add_node(vector_node).unwrap();
    let mut large_proof_node = node_input("schema6-large-proof", &["USER"]);
    large_proof_node.props = Some(serde_json::json!({
        "payload": "x".repeat(15_000),
    }));
    master.add_node(large_proof_node).unwrap();
    master
        .add_node(node_input("schema6-master-retracted", &["MASTER"]))
        .unwrap();
    master.retract_node("schema6-master-retracted").unwrap();
    master
        .add_edge(EdgeInput {
            id: Some("schema6-compacted-edge".into()),
            from: "schema6-master-live".into(),
            to: "schema6-user-vector".into(),
            rel: "LINKS".into(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: Some(1.0),
            caused_by: None,
        })
        .unwrap();

    relay.reconcile_state(master.events_since_seq(0)).unwrap();
    relay
        .add_node(node_input("schema6-relay-user", &["USER"]))
        .unwrap();
    relay
        .add_edge(EdgeInput {
            id: Some("schema6-compacted-relay-edge".into()),
            from: "schema6-master-live".into(),
            to: "schema6-relay-user".into(),
            rel: "LINKS".into(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: Some(1.0),
            caused_by: None,
        })
        .unwrap();
    relay.compact().unwrap();

    let compacted = relay.events_since_seq(0);
    let revision_checkpoint = compacted
        .iter()
        .find(|signed| {
            matches!(
                &signed.event,
                Event::Transaction(transaction)
                    if transaction.record_revision_transaction.is_some()
                        && transaction.local_frame_seq.is_some()
            )
        })
        .expect("compaction should emit a revision checkpoint");
    let before_rejected_checkpoints = receiver.stable_frontier();

    let mut tampered_checkpoint = revision_checkpoint.clone();
    let Event::Transaction(transaction) = &mut tampered_checkpoint.event else {
        unreachable!();
    };
    transaction.local_frame_seq = transaction.local_frame_seq.map(|seq| seq + 1);
    assert!(receiver
        .reconcile_state(vec![tampered_checkpoint])
        .unwrap_err()
        .to_string()
        .contains("SYNC_AUTH_REQUIRED"));

    // A valid relay signature over a checkpoint shape is not enough: schema 6
    // requires a source event whose revision envelope binds the checkpoint.
    let raw_checkpoint = relay.sign_event(&revision_checkpoint.event);
    assert!(receiver.reconcile_state(vec![raw_checkpoint]).is_err());
    assert_eq!(receiver.stable_frontier(), before_rejected_checkpoints);

    let large_materialization = compacted
        .iter()
        .find(
            |signed| matches!(&signed.event, Event::Node(node) if node.id == "schema6-large-proof"),
        )
        .expect("large USER node should have a compacted materialization");
    let large_materialization_json = serde_json::to_value(large_materialization).unwrap();
    assert!(large_materialization_json["signature"]
        .as_str()
        .is_some_and(|signature| signature.starts_with("b64:")));
    assert!(serde_json::to_vec(large_materialization).unwrap().len() <= 54_000);
    for (index, signed) in compacted.into_iter().enumerate() {
        let kind = match &signed.event {
            Event::Node(_) => "Node",
            Event::Edge(_) => "Edge",
            Event::NodeRetract { .. } => "NodeRetract",
            Event::VectorMaterialized(_) => "VectorMaterialized",
            Event::Transaction(_) => "Transaction",
            Event::Batch(_) => "Batch",
            _ => "Other",
        };
        let detail = match &signed.event {
            Event::Transaction(transaction) => format!(
                "transaction={} local_frame_seq={:?} relational_groups={} nodes={} edges={} vectors={} retractions={} revision_mutations={:?}",
                transaction.transaction_id,
                transaction.local_frame_seq,
                transaction.relational.len(),
                transaction.nodes.len(),
                transaction.edges.len(),
                transaction.vectors.len(),
                transaction.node_retractions.len(),
                transaction.record_revision_transaction.as_ref().map(|revisions| {
                    revisions
                        .mutations
                        .iter()
                        .map(|mutation| (&mutation.kind, &mutation.id))
                        .collect::<Vec<_>>()
                })
            ),
            _ => String::new(),
        };
        receiver
            .reconcile_state(vec![signed])
            .unwrap_or_else(|error| {
                panic!("compacted event {index} ({kind}) rejected: {detail}: {error}")
            });
    }
    assert!(receiver.node_view("schema6-master-live").is_some());
    assert!(receiver.node_view("schema6-user-vector").is_some());
    assert!(receiver.node_view("schema6-relay-user").is_some());
    assert!(receiver.node_view("schema6-master-retracted").is_none());

    // Force projection recovery from the compacted WAL to exercise the same
    // source-proof check used during sync, then confirm revisions survived.
    drop(receiver);
    fs::remove_file(receiver_dir.path().join("projection.sqlite")).unwrap();
    let reopened = reopen("schema6-compaction-receiver-replay", receiver_dir.path());
    assert!(reopened.node_view("schema6-master-live").is_some());
    let projection =
        rusqlite::Connection::open(receiver_dir.path().join("projection.sqlite")).unwrap();
    let revision_count: i64 = projection
        .query_row("SELECT COUNT(*) FROM hql2_record_revisions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(
        revision_count > 0,
        "revision history should replay from checkpoint proofs"
    );
}

#[test]
fn compaction_keeps_later_user_update_after_master_declassification() {
    // Legacy peers accept revision-free batches; schema 6 rejects those at
    // ingress, so keep this replay-order regression on the schema-5 path.
    let (_master_dir, master) = open("schema5-declass-master");
    let (_relay_dir, relay) = open("schema5-declass-relay");
    let (_receiver_dir, receiver) = open("schema5-declass-receiver");
    configure(
        &master,
        vec![peer(&relay, 31612, false), peer(&receiver, 31613, false)],
        31611,
        true,
    );
    configure(
        &relay,
        vec![peer(&master, 31611, true), peer(&receiver, 31613, false)],
        31612,
        false,
    );
    configure(
        &receiver,
        vec![peer(&master, 31611, true), peer(&relay, 31612, false)],
        31613,
        false,
    );

    master
        .add_node(node_input("declassify-later-update", &["MASTER"]))
        .unwrap();
    let initial_master_event = master.events_since_seq(0);
    relay.reconcile_state(initial_master_event.clone()).unwrap();
    receiver.reconcile_state(initial_master_event).unwrap();

    let mut declassified = node_input("declassify-later-update", &["USER"]);
    declassified.props = Some(serde_json::json!({"state": "declassified"}));
    master.add_node(declassified).unwrap();
    relay.reconcile_state(master.events_since_seq(1)).unwrap();

    let mut later_user_update = node_input("declassify-later-update", &["USER"]);
    later_user_update.props = Some(serde_json::json!({"state": "updated"}));
    relay.add_node(later_user_update).unwrap();

    let current = relay
        .node_view("declassify-later-update")
        .expect("the later USER update should be resident before the stale batch");
    let mut stale = current.clone();
    stale.clock.time = stale.clock.time.saturating_sub(1);
    stale.props = serde_json::json!({"state": "stale-batch-leaf"});
    let stale_batch = master.sign_event(&Event::Batch(vec![
        Event::Node(current),
        Event::Node(stale),
    ]));
    relay.reconcile_state(vec![stale_batch]).unwrap();
    relay.compact().unwrap();

    for (index, signed) in relay.events_since_seq(0).into_iter().enumerate() {
        let kind = match &signed.event {
            Event::Node(node) => format!("Node {}", node.id),
            Event::Edge(edge) => format!("Edge {}", edge.id),
            Event::NodeRetract { id, .. } => format!("NodeRetract {id}"),
            Event::VectorMaterialized(vector) => format!("VectorMaterialized {}", vector.node_id),
            _ => "Other".to_string(),
        };
        receiver
            .reconcile_state(vec![signed])
            .unwrap_or_else(|error| panic!("compacted event {index} ({kind}) rejected: {error}"));
    }
    let node = receiver
        .node_view("declassify-later-update")
        .expect("compacted declassification and later USER state should sync");
    assert_eq!(node.labels, vec!["USER"]);
    assert_eq!(node.props["state"], "updated");
}
