use super::*;
use tempfile::TempDir;

fn open_full_store(directory: &TempDir) -> Storage {
    try_open_full_store(directory).unwrap()
}

fn try_open_full_store(directory: &TempDir) -> Result<Storage> {
    Storage::open(OpenOptions {
        path: directory.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".to_string()),
    })
}

fn refresh_test_p6_manifest(directory: &TempDir) {
    let state_path = directory.path().join("state.json");
    let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    let manifest = &mut state["p6"]["manifest"];
    let frontier = manifest["snapshot_frontier"].as_u64().unwrap();
    let components = manifest["components"].as_array_mut().unwrap();
    for component in components.iter_mut() {
        let name = component["path"].as_str().unwrap().to_string();
        let (bytes, digest) = Storage::sha256_file(&directory.path().join(&name)).unwrap();
        component["bytes"] = serde_json::json!(bytes);
        component["sha256"] = serde_json::json!(digest);
    }
    components.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    let canonical = serde_json::to_vec(&serde_json::json!({
        "version": 1,
        "snapshot_frontier": frontier,
        "components": components,
    }))
    .unwrap();
    manifest["manifest_sha256"] = serde_json::json!(hex::encode(Sha256::digest(canonical)));
    fs::write(state_path, state.to_string()).unwrap();
}

fn legacy_node_event_json(impact: Option<&str>) -> String {
    let impact = impact
        .map(|value| format!(",\"impact\":{value}"))
        .unwrap_or_default();
    format!(
        "{{\"Node\":{{\"id\":\"legacy-node\",\"labels\":[\"Legacy\"],\"props\":{{\"source\":\"signed\"}}{impact},\"lang\":\"en\",\"valid_from\":\"2026-11-01T00:00:00Z\",\"valid_to\":null,\"caused_by\":null,\"expires_at\":null,\"clock\":{{\"time\":7,\"peer_id\":\"legacy-peer\"}},\"collection\":null}}}}"
    )
}

#[test]
fn signed_legacy_wire_roundtrips_absent_null_and_numeric_presence() {
    let signing_key = SigningKey::from_bytes(&rand::random::<[u8; 32]>());
    for (wire_value, expected) in [
        (None, LegacyImpactWire::Absent),
        (Some("null"), LegacyImpactWire::Null),
        (Some("0.75"), LegacyImpactWire::Number(0.75)),
    ] {
        let raw = legacy_node_event_json(wire_value);
        let event: Event = serde_json::from_str(&raw).unwrap();
        let Event::Node(node) = &event else {
            panic!("expected a node event");
        };
        assert_eq!(node.legacy_wire_impact, expected);
        let canonical = canonical_event_bytes(&event).unwrap();
        assert_eq!(canonical, raw.as_bytes());
        let signature = signing_key.sign(&canonical);
        signing_key
            .verifying_key()
            .verify(&canonical, &signature)
            .unwrap();
        let tampered = canonical_event_bytes(&Event::Node(NodeOutput {
            legacy_wire_impact: LegacyImpactWire::Number(0.5),
            ..node.clone()
        }))
        .unwrap();
        if expected != LegacyImpactWire::Number(0.5) {
            assert!(signing_key
                .verifying_key()
                .verify(&tampered, &signature)
                .is_err());
        }
        assert!(serde_json::to_value(node).unwrap().get("impact").is_none());

        let new_event = Event::Node(NodeOutput {
            legacy_wire_impact: LegacyImpactWire::Absent,
            ..node.clone()
        });
        assert!(serde_json::to_value(new_event).unwrap()["Node"]
            .get("impact")
            .is_none());
    }
}

fn legacy_graph_transaction(storage: &Storage) -> Event {
    let node = NodeOutput {
        id: "legacy-revision-node".to_string(),
        labels: vec!["Legacy".to_string()],
        props: serde_json::json!({"source":"signed"}),
        legacy_wire_impact: LegacyImpactWire::Number(0.75),
        embedding: None,
        lang: Some("en".to_string()),
        valid_from: "2026-11-01T00:00:00Z".to_string(),
        valid_to: None,
        caused_by: None,
        expires_at: None,
        clock: LogicalClock {
            time: 17,
            peer_id: storage.local_peer_id.clone(),
        },
        collection: None,
    };
    let edge = EdgeOutput {
        id: "legacy-revision-edge".to_string(),
        from: node.id.clone(),
        to: "legacy-target".to_string(),
        rel: "REFERENCES".to_string(),
        props: serde_json::json!({"source":"signed"}),
        valid_from: "2026-11-01T00:00:00Z".to_string(),
        valid_to: None,
        recorded_at: "2026-11-01T00:00:00Z".to_string(),
        superseded_by: None,
        legacy_wire_impact: LegacyImpactWire::Number(0.25),
        caused_by: None,
        clock: LogicalClock {
            time: 18,
            peer_id: storage.local_peer_id.clone(),
        },
    };
    let mut revisions = storage
        .build_record_revision_transaction(
            std::slice::from_ref(&node),
            std::slice::from_ref(&edge),
            &[],
            &[],
            &[],
        )
        .unwrap()
        .unwrap();
    for mutation in &mut revisions.mutations {
        match mutation.kind {
            uee_v2::RecordKindV2::Node if mutation.id == node.id => {
                mutation.payload =
                    serde_json::to_value(LegacyNodeEventWireRef::from(&node)).unwrap();
            }
            uee_v2::RecordKindV2::Edge if mutation.id == edge.id => {
                mutation.payload =
                    serde_json::to_value(LegacyEdgeEventWireRef::from(&edge)).unwrap();
            }
            _ => {}
        }
    }
    Event::Transaction(GenesisTransactionEvent {
        transaction_id: "legacy-signed-revision-chain".to_string(),
        origin_commit_seq: 0,
        local_frame_seq: None,
        payload_hash: "a".repeat(64),
        relational: Vec::new(),
        nodes: vec![node],
        edges: vec![edge],
        vectors: Vec::new(),
        node_retractions: Vec::new(),
        advances_txn_frontier: true,
        record_revision_transaction: Some(revisions),
    })
}

fn assert_migrated_projection(storage: &Storage) {
    let node_u32 = storage
        .get_u32_unlocked("legacy-revision-node")
        .expect("replayed node is interned");
    let node_props = storage
        .projection_props_unlocked(node_u32)
        .unwrap()
        .expect("node properties projected");
    assert_eq!(node_props[LEGACY_IMPACT_PROP], 0.75);
    assert!(node_props.get(LEGACY_ORIGINAL_PROPS_PROP).is_none());

    let edge_key = Storage::edge_key("legacy-revision-edge");
    let edge = storage.edges.get(&edge_key).expect("replayed edge is live");
    assert_eq!(edge.props[LEGACY_IMPACT_PROP], 0.25);
    assert!(serde_json::to_value(edge.value())
        .unwrap()
        .get("impact")
        .is_none());

    let conn = storage.projection_db.lock();
    let edge_payload: String = conn
        .query_row(
            "SELECT payload FROM edge_versions WHERE id=?1 ORDER BY tx_from DESC LIMIT 1",
            ["legacy-revision-edge"],
            |row| row.get(0),
        )
        .unwrap();
    let edge_payload: Value = serde_json::from_str(&edge_payload).unwrap();
    assert!(edge_payload.get("impact").is_none());
    assert_eq!(edge_payload["props"][LEGACY_IMPACT_PROP], 0.25);

    let edge_column_impact: Option<f64> = conn
        .query_row(
            "SELECT impact FROM edges WHERE id=?1",
            ["legacy-revision-edge"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(edge_column_impact, None);

    for (kind, id, expected) in [
        ("node", "legacy-revision-node", 0.75),
        ("edge", "legacy-revision-edge", 0.25),
    ] {
        let payload: String = conn
            .query_row(
                "SELECT payload_json FROM hql2_record_revisions WHERE kind=?1 AND record_id=?2 AND tx_to IS NULL",
                params![kind, id],
                |row| row.get(0),
            )
            .unwrap();
        let payload: Value = serde_json::from_str(&payload).unwrap();
        assert!(payload.get("impact").is_none());
        assert_eq!(payload["props"][LEGACY_IMPACT_PROP], expected);
    }
}

#[test]
fn legacy_impact_values_remain_bound_in_proofs_and_revision_payloads() {
    let directory = TempDir::new().unwrap();
    let storage = open_full_store(&directory);
    let event = legacy_graph_transaction(&storage);
    let Event::Transaction(transaction) = &event else {
        unreachable!();
    };
    storage
        .validate_graph_revision_binding(transaction)
        .expect("the exact signed legacy payload binds to its graph event");

    let node = &transaction.nodes[0];
    let node_wire = serde_json::to_value(LegacyNodeEventWireRef::from(node)).unwrap();
    let mut normalized_node = node.clone();
    migrate_legacy_node_impact(&mut normalized_node).unwrap();
    let node_normalized = serde_json::to_value(&normalized_node).unwrap();
    let node_public_without_migration = serde_json::to_value(node).unwrap();
    assert!(graph_revision_payload_matches_node(node, &node_wire));
    assert!(graph_revision_payload_matches_node(node, &node_normalized));
    assert!(!graph_revision_payload_matches_node(
        node,
        &node_public_without_migration
    ));

    let mut altered_node = node.clone();
    altered_node.legacy_wire_impact = LegacyImpactWire::Number(0.5);
    assert!(!node_projection_matches_proof(&altered_node, node));
    assert!(node_projection_matches_proof(node, node));

    let edge = &transaction.edges[0];
    let edge_wire = serde_json::to_value(LegacyEdgeEventWireRef::from(edge)).unwrap();
    let mut normalized_edge = edge.clone();
    migrate_legacy_edge_impact(&mut normalized_edge).unwrap();
    let edge_normalized = serde_json::to_value(&normalized_edge).unwrap();
    let edge_public_without_migration = serde_json::to_value(edge).unwrap();
    assert!(graph_revision_payload_matches_edge(edge, &edge_wire));
    assert!(graph_revision_payload_matches_edge(edge, &edge_normalized));
    assert!(!graph_revision_payload_matches_edge(
        edge,
        &edge_public_without_migration
    ));
    assert!(storage.compacted_base_proof_matches_event(&Event::Edge(edge.clone()), &event));

    let mut altered_edge = edge.clone();
    altered_edge.legacy_wire_impact = LegacyImpactWire::Number(0.5);
    assert!(!storage.compacted_base_proof_matches_event(&Event::Edge(altered_edge), &event));

    let mut missing_node_revision = transaction.clone();
    let node_revision = missing_node_revision
        .record_revision_transaction
        .as_mut()
        .unwrap()
        .mutations
        .iter_mut()
        .find(|mutation| {
            mutation.kind == uee_v2::RecordKindV2::Node && mutation.id == "legacy-revision-node"
        })
        .unwrap();
    node_revision
        .payload
        .as_object_mut()
        .unwrap()
        .remove("impact");
    assert!(storage
        .validate_graph_revision_binding(&missing_node_revision)
        .is_err());

    let mut missing_edge_revision = transaction.clone();
    let edge_revision = missing_edge_revision
        .record_revision_transaction
        .as_mut()
        .unwrap()
        .mutations
        .iter_mut()
        .find(|mutation| {
            mutation.kind == uee_v2::RecordKindV2::Edge && mutation.id == "legacy-revision-edge"
        })
        .unwrap();
    edge_revision
        .payload
        .as_object_mut()
        .unwrap()
        .remove("impact");
    assert!(storage
        .validate_graph_revision_binding(&missing_edge_revision)
        .is_err());

    let mut normalized_node_revision = transaction.clone();
    normalized_node_revision
        .record_revision_transaction
        .as_mut()
        .unwrap()
        .mutations
        .iter_mut()
        .find(|mutation| {
            mutation.kind == uee_v2::RecordKindV2::Node && mutation.id == "legacy-revision-node"
        })
        .unwrap()
        .payload = node_normalized;
    normalized_node_revision
        .record_revision_transaction
        .as_mut()
        .unwrap()
        .mutations
        .iter_mut()
        .find(|mutation| {
            mutation.kind == uee_v2::RecordKindV2::Edge && mutation.id == "legacy-revision-edge"
        })
        .unwrap()
        .payload = edge_normalized;
    storage
        .validate_graph_revision_binding(&normalized_node_revision)
        .expect("normalized graph revisions retain the generic legacy value");
}

#[test]
fn legacy_revision_read_normalizes_without_changing_the_persisted_payload() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE hql2_record_revisions (
                database_id TEXT, namespace TEXT, kind TEXT, record_id TEXT,
                revision_id TEXT, operation TEXT, valid_from TEXT, valid_to TEXT,
                tx_from INTEGER, tx_to INTEGER, payload_json TEXT
            );",
        )
        .unwrap();
    let Event::Node(node) =
        serde_json::from_str::<Event>(&legacy_node_event_json(Some("0.75"))).unwrap()
    else {
        unreachable!();
    };
    let original_payload = serde_json::to_string(&LegacyNodeEventWireRef::from(&node)).unwrap();
    connection
        .execute(
            "INSERT INTO hql2_record_revisions(
                database_id, namespace, kind, record_id, revision_id, operation,
                valid_from, valid_to, tx_from, tx_to, payload_json
            ) VALUES('db', 'default', 'node', ?1, 'rev-1', 'upsert',
                '2026-11-01T00:00:00Z', NULL, 1, NULL, ?2)",
            params![node.id, original_payload],
        )
        .unwrap();

    let mut budget = query::hql2::exec::ExecutionBudgetV2::new(&None).unwrap();
    let normalized = hql2_read_revision_payload(
        &connection,
        "db",
        "default",
        "node",
        &node.id,
        "rev-1",
        1,
        "2026-11-01T00:00:00Z",
        &mut budget,
    )
    .unwrap();
    assert!(normalized.get("impact").is_none());
    assert_eq!(normalized["props"][LEGACY_IMPACT_PROP], 0.75);

    let persisted: String = connection
        .query_row(
            "SELECT payload_json FROM hql2_record_revisions WHERE record_id=?1",
            [&node.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(persisted, original_payload);
}

#[test]
fn legacy_revision_migration_is_schema_gated_and_v6_errors_stay_fail_closed() {
    let legacy_directory = TempDir::new().unwrap();
    fs::write(
        legacy_directory.path().join("state.json"),
        r#"{"schema_version":5}"#,
    )
    .unwrap();
    let legacy_storage = open_full_store(&legacy_directory);
    assert_eq!(legacy_storage.storage_schema_version, LEGACY_SCHEMA_VERSION);
    let legacy_connection = legacy_storage.projection_db.lock();
    let legacy_revision_table_exists: bool = legacy_connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='hql2_record_revisions')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!legacy_revision_table_exists);
    assert_eq!(
        legacy_storage
            .legacy_node_version_props_from_revision(
                &legacy_connection,
                &legacy_storage.database_id(),
                "legacy-node",
                1,
                &[],
                &Value::Null,
                "2026-11-01T00:00:00Z",
                &None,
                &None,
                1,
                "legacy-peer",
            )
            .unwrap(),
        None,
        "schema 5 has no revision history to normalize",
    );
    drop(legacy_connection);

    let current_directory = TempDir::new().unwrap();
    let current_storage = open_full_store(&current_directory);
    assert_eq!(current_storage.storage_schema_version, SCHEMA_VERSION);
    let current_connection = current_storage.projection_db.lock();
    current_connection
        .execute_batch("DROP TABLE hql2_record_revisions")
        .unwrap();
    let error = current_storage
        .legacy_node_version_props_from_revision(
            &current_connection,
            &current_storage.database_id(),
            "legacy-node",
            1,
            &[],
            &Value::Null,
            "2026-11-01T00:00:00Z",
            &None,
            &None,
            1,
            "legacy-peer",
        )
        .expect_err("schema 6 must not hide a missing revision table");
    assert!(error.to_string().contains("no such table"), "{error}");
}

#[test]
fn verified_old_snapshot_and_matching_projection_marker_keep_legacy_metadata() {
    let directory = TempDir::new().unwrap();
    let storage = open_full_store(&directory);
    let event = legacy_graph_transaction(&storage);
    let Event::Transaction(transaction) = &event else {
        unreachable!();
    };
    storage
        .validate_graph_revision_binding(transaction)
        .unwrap();
    let signed_bytes = canonical_event_bytes(&event).unwrap();
    let signed = SignedEvent {
        event: serde_json::from_slice(&signed_bytes).unwrap(),
        signature: storage.signing_key.sign(&signed_bytes).to_bytes().to_vec(),
        signer_peer_id: storage.local_peer_id.clone(),
    };
    assert!(storage.verify_raw_event_signature(&signed));
    let node = transaction.nodes[0].clone();
    let mut edge = transaction.edges[0].clone();

    let (ack_tx, ack_rx) = unbounded();
    storage
        .wal_sender
        .send(WalMsg::Append(Box::new(signed), ack_tx))
        .unwrap();
    let frame_seq = ack_rx.recv().unwrap().unwrap();
    storage
        .commit_sequence
        .fetch_max(frame_seq, Ordering::SeqCst);
    storage.replay_journal(None, true);
    storage
        .projection_apply_event(&event, frame_seq, false, None)
        .unwrap();

    let target = storage
        .add_node(NodeInput {
            id: Some("legacy-target".into()),
            labels: vec!["Target".into()],
            props: Some(serde_json::json!({"source":"target"})),
            embedding: None,
            lang: None,
            valid_from: Some("2026-11-01T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    assert_eq!(target.id, "legacy-target");

    let marker = storage.projection_journal_fingerprint().unwrap();
    {
        let projection = storage.projection_db.lock();
        Storage::projection_store_journal_fingerprint(&projection, &marker).unwrap();
    }
    storage.save_state().unwrap();
    assert!(storage.projection_journal_fingerprint_matches(&marker));

    // Recreate the prior NodeOutput/EdgeOutput snapshot wire shape after the
    // normal writer has produced a structurally valid P6 snapshot.
    let mut legacy_node = node.clone();
    // Pre-retirement snapshots saved `null` here because insert_node_lean
    // kept application props only in the SQLite projection.
    legacy_node.props = Value::Null;
    legacy_node.legacy_wire_impact = LegacyImpactWire::Number(0.75);
    let node_entries = storage
        .nodes
        .iter()
        .map(|entry| {
            let value = if entry.value().id == legacy_node.id {
                legacy_node.clone()
            } else {
                entry.value().clone()
            };
            (*entry.key(), value)
        })
        .collect::<Vec<_>>();
    let node_wire_entries = node_entries
        .iter()
        .map(|(key, value)| (*key, LegacyNodeEventWireRef::from(value)))
        .collect::<Vec<_>>();
    fs::write(
        directory.path().join("nodes.bin"),
        serde_json::to_vec(&node_wire_entries).unwrap(),
    )
    .unwrap();
    edge.legacy_wire_impact = LegacyImpactWire::Number(0.25);
    let edge_entry = (Storage::edge_key(&edge.id), edge.clone());
    let edge_entries = vec![(edge_entry.0, LegacyEdgeEventWireRef::from(&edge_entry.1))];
    fs::write(
        directory.path().join("edges.bin"),
        serde_json::to_vec(&edge_entries).unwrap(),
    )
    .unwrap();

    let node_u32 = storage.get_u32_unlocked(&node.id).unwrap();
    let old_node_props = node.props.to_string();
    let old_edge_props = edge.props.to_string();
    let old_edge_payload = serde_json::to_string(&LegacyEdgeEventWireRef::from(&edge)).unwrap();
    let old_node_revision = serde_json::to_string(&LegacyNodeEventWireRef::from(&node)).unwrap();
    let old_edge_revision = old_edge_payload.clone();
    drop(storage);

    let projection = Connection::open(directory.path().join(PROJECTION_DB_FILE)).unwrap();
    projection
        .execute(
            "UPDATE props SET payload=?1 WHERE node_u32=?2",
            params![old_node_props, node_u32],
        )
        .unwrap();
    projection
        .execute(
            "UPDATE node_versions SET payload=?1 WHERE id=?2 AND retracted=0",
            params![node.props.to_string(), node.id],
        )
        .unwrap();
    projection
        .execute(
            "UPDATE edges SET props=?1, impact=0.25 WHERE id=?2",
            params![old_edge_props, edge.id],
        )
        .unwrap();
    projection
        .execute(
            "UPDATE edge_versions SET payload=?1 WHERE id=?2",
            params![old_edge_payload, edge.id],
        )
        .unwrap();
    projection
        .execute(
            "UPDATE hql2_record_revisions SET payload_json=?1 WHERE kind='node' AND record_id=?2",
            params![old_node_revision, node.id],
        )
        .unwrap();
    projection
        .execute(
            "UPDATE hql2_record_revisions SET payload_json=?1 WHERE kind='edge' AND record_id=?2",
            params![old_edge_revision, edge.id],
        )
        .unwrap();
    let raw_edge_before: String = projection
        .query_row(
            "SELECT payload FROM edge_versions WHERE id=?1",
            [&edge.id],
            |row| row.get(0),
        )
        .unwrap();
    let raw_revision_before: String = projection
        .query_row(
            "SELECT payload_json FROM hql2_record_revisions WHERE kind='edge' AND record_id=?1",
            [&edge.id],
            |row| row.get(0),
        )
        .unwrap();
    drop(projection);
    refresh_test_p6_manifest(&directory);

    let reopened = open_full_store(&directory);
    assert!(!reopened.recovery_required.load(Ordering::SeqCst));
    let current_marker = reopened.projection_journal_fingerprint().unwrap();
    assert_eq!(current_marker, marker);
    assert!(reopened.projection_journal_fingerprint_matches(&current_marker));

    let node_view = reopened.node_view(&node.id).unwrap();
    assert_eq!(node_view.props["source"], "signed");
    assert_eq!(node_view.props[LEGACY_IMPACT_PROP], 0.75);
    assert!(serde_json::to_value(&node_view)
        .unwrap()
        .get("impact")
        .is_none());
    assert_eq!(
        reopened
            .projection_props_unlocked(node_u32)
            .unwrap()
            .unwrap()[LEGACY_IMPACT_PROP],
        0.75
    );
    let node_history = reopened.node_versions_unlocked(&node.id, None).unwrap();
    assert_eq!(node_history["versions"][0]["props"]["source"], "signed");
    assert_eq!(
        node_history["versions"][0]["props"][LEGACY_IMPACT_PROP],
        0.75
    );
    let historical_node = reopened
        .resolve_node_as_of(&node_view, "2026-11-02T00:00:00Z")
        .unwrap();
    assert_eq!(historical_node.props["source"], "signed");
    assert_eq!(historical_node.props[LEGACY_IMPACT_PROP], 0.75);
    assert!(serde_json::to_value(&historical_node)
        .unwrap()
        .get("impact")
        .is_none());
    let edge_key = Storage::edge_key(&edge.id);
    let live_edge = reopened.edges.get(&edge_key).unwrap();
    assert_eq!(live_edge.props[LEGACY_IMPACT_PROP], 0.25);
    assert!(serde_json::to_value(live_edge.value())
        .unwrap()
        .get("impact")
        .is_none());
    drop(live_edge);

    let mut query_budget = QueryBudgetState::new(QueryBudget::default().resolve().unwrap());
    let neighbors = reopened
        .neighbors_tx_view(
            node.id.clone(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".into()),
                as_of: None,
                include_invalid: None,
                limit: Some(5),
            },
            reopened.stable_frontier(),
            &mut query_budget,
        )
        .unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].path[0].props[LEGACY_IMPACT_PROP], 0.25);
    assert!(serde_json::to_value(&neighbors[0].path[0])
        .unwrap()
        .get("impact")
        .is_none());

    let mut revision_budget = query::hql2::exec::ExecutionBudgetV2::new(&None).unwrap();
    let database_id = reopened.database_id();
    let normalized_revision = {
        let conn = reopened.projection_db.lock();
        let revision_id: String = conn
            .query_row(
                "SELECT revision_id FROM hql2_record_revisions WHERE kind='edge' AND record_id=?1",
                [&edge.id],
                |row| row.get::<_, String>(0),
            )
            .unwrap();
        hql2_read_revision_payload(
            &conn,
            &database_id,
            "default",
            "edge",
            &edge.id,
            &revision_id,
            reopened.stable_frontier() as i64,
            "2026-11-02T00:00:00Z",
            &mut revision_budget,
        )
        .unwrap()
    };
    assert!(normalized_revision.get("impact").is_none());
    assert_eq!(normalized_revision["props"][LEGACY_IMPACT_PROP], 0.25);

    let conn = reopened.projection_db.lock();
    let raw_edge_after: String = conn
        .query_row(
            "SELECT payload FROM edge_versions WHERE id=?1",
            [&edge.id],
            |row| row.get(0),
        )
        .unwrap();
    let raw_revision_after: String = conn
        .query_row(
            "SELECT payload_json FROM hql2_record_revisions WHERE kind='edge' AND record_id=?1",
            [&edge.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(raw_edge_after, raw_edge_before);
    assert_eq!(raw_revision_after, raw_revision_before);
    drop(conn);

    reopened.save_state().unwrap();
    drop(reopened);
    let restored = open_full_store(&directory);
    assert!(!restored.recovery_required.load(Ordering::SeqCst));
    let restored_node = restored.node_view(&node.id).unwrap();
    assert_eq!(restored_node.props["source"], "signed");
    assert_eq!(restored_node.props[LEGACY_IMPACT_PROP], 0.75);
    assert!(serde_json::to_value(&restored_node)
        .unwrap()
        .get("impact")
        .is_none());
    assert!(restored.projection_journal_fingerprint_matches(
        &restored.projection_journal_fingerprint().unwrap()
    ));
}

#[test]
fn matching_marker_with_conflicting_or_missing_legacy_projection_fails_closed() {
    let directory = TempDir::new().unwrap();
    let storage = open_full_store(&directory);
    let node = storage
        .add_node(NodeInput {
            id: Some("conflicting-legacy-node".into()),
            labels: vec!["Legacy".into()],
            props: Some(serde_json::json!({"source":"signed"})),
            embedding: None,
            lang: None,
            valid_from: Some("2026-11-01T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    let marker = storage.projection_journal_fingerprint().unwrap();
    {
        let projection = storage.projection_db.lock();
        Storage::projection_store_journal_fingerprint(&projection, &marker).unwrap();
    }
    storage.save_state().unwrap();
    assert!(storage.projection_journal_fingerprint_matches(&marker));
    drop(storage);

    let mut nodes: Vec<(u32, NodeOutput)> =
        serde_json::from_slice(&fs::read(directory.path().join("nodes.bin")).unwrap()).unwrap();
    let (node_u32, snapshot_node) = nodes
        .iter_mut()
        .find(|(_, candidate)| candidate.id == node.id)
        .unwrap();
    snapshot_node.props = Value::Null;
    snapshot_node.legacy_wire_impact = LegacyImpactWire::Number(0.5);
    let node_u32 = *node_u32;
    let legacy_wire_nodes = nodes
        .iter()
        .map(|(node_u32, node)| (*node_u32, LegacyNodeEventWireRef::from(node)))
        .collect::<Vec<_>>();
    fs::write(
        directory.path().join("nodes.bin"),
        serde_json::to_vec(&legacy_wire_nodes).unwrap(),
    )
    .unwrap();

    let projection = Connection::open(directory.path().join(PROJECTION_DB_FILE)).unwrap();
    projection
        .execute(
            "UPDATE props SET payload=?1 WHERE node_u32=?2",
            params![
                serde_json::json!({"source":"signed",(LEGACY_IMPACT_PROP):0.4}).to_string(),
                node_u32
            ],
        )
        .unwrap();
    drop(projection);
    refresh_test_p6_manifest(&directory);

    let error = match try_open_full_store(&directory) {
        Ok(opened) => {
            drop(opened);
            panic!("conflicting legacy projection was accepted");
        }
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(message.contains("RECOVERY_REQUIRED"), "{message}");
    assert!(message.contains("KIMPACT_MIGRATION_CONFLICT"), "{message}");

    let projection = Connection::open(directory.path().join(PROJECTION_DB_FILE)).unwrap();
    projection
        .execute("DELETE FROM props WHERE node_u32=?1", [node_u32])
        .unwrap();
    drop(projection);
    refresh_test_p6_manifest(&directory);
    let error = match try_open_full_store(&directory) {
        Ok(opened) => {
            drop(opened);
            panic!("legacy snapshot with missing projection was accepted");
        }
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(message.contains("RECOVERY_REQUIRED"), "{message}");
    assert!(message.contains("no properties projection"), "{message}");
}

#[test]
fn signed_legacy_transaction_replays_into_normalized_projection_and_snapshot() {
    let directory = TempDir::new().unwrap();
    let storage = open_full_store(&directory);
    let event = legacy_graph_transaction(&storage);
    let Event::Transaction(transaction) = &event else {
        unreachable!();
    };
    storage
        .validate_graph_revision_binding(transaction)
        .unwrap();

    let original_wire = canonical_event_bytes(&event).unwrap();
    let decoded_event: Event = serde_json::from_slice(&original_wire).unwrap();
    assert_eq!(
        canonical_event_bytes(&decoded_event).unwrap(),
        original_wire
    );
    let signed = SignedEvent {
        event: decoded_event,
        signature: storage.signing_key.sign(&original_wire).to_bytes().to_vec(),
        signer_peer_id: storage.local_peer_id.clone(),
    };
    assert!(storage.verify_raw_event_signature(&signed));
    let signed_bytes = canonical_event_bytes(&signed.event).unwrap();
    assert!(String::from_utf8(signed_bytes)
        .unwrap()
        .contains("\"impact\":0.75"));

    let mut tampered = event.clone();
    let Event::Transaction(tampered_transaction) = &mut tampered else {
        unreachable!();
    };
    tampered_transaction
        .record_revision_transaction
        .as_mut()
        .unwrap()
        .mutations
        .iter_mut()
        .find(|mutation| {
            mutation.kind == uee_v2::RecordKindV2::Node && mutation.id == "legacy-revision-node"
        })
        .unwrap()
        .payload["props"]["source"] = serde_json::json!("tampered");
    assert!(storage
        .validate_graph_revision_binding(tampered_transaction)
        .is_err());
    assert!(!storage.verify_raw_event_signature(&SignedEvent {
        event: tampered,
        signature: signed.signature.clone(),
        signer_peer_id: signed.signer_peer_id.clone(),
    }));

    let (ack_tx, ack_rx) = unbounded();
    storage
        .wal_sender
        .send(WalMsg::Append(Box::new(signed.clone()), ack_tx))
        .unwrap();
    let sequence = ack_rx.recv().unwrap().unwrap();
    storage
        .commit_sequence
        .fetch_max(sequence, Ordering::SeqCst);
    assert!(storage
        .events_since_seq(sequence.saturating_sub(1))
        .iter()
        .any(|signed| matches!(&signed.event, Event::Transaction(_))));
    storage.replay_journal(None, true);
    storage
        .projection_apply_event(&event, sequence, false, None)
        .unwrap();
    assert_migrated_projection(&storage);
    drop(storage);

    let reopened = open_full_store(&directory);
    assert!(!reopened.recovery_required.load(Ordering::SeqCst));
    assert_migrated_projection(&reopened);

    // The materialized edge history and revision projection are deliberately
    // checked above: historical/as-of reads use these SQLite payloads rather
    // than the live DashMap. Save then reopen once more to exercise snapshot
    // restoration after authentic legacy replay.
    reopened.save_state().unwrap();
    drop(reopened);

    let restored = open_full_store(&directory);
    assert!(!restored.recovery_required.load(Ordering::SeqCst));
    assert_migrated_projection(&restored);
}

#[test]
fn legacy_props_migration_wraps_shapes_and_rejects_conflicts() {
    let mut scalar_props = serde_json::json!("source-value");
    let mut impact = LegacyImpactWire::Number(0.5);
    migrate_legacy_impact(&mut scalar_props, &mut impact).unwrap();
    assert_eq!(scalar_props[LEGACY_IMPACT_PROP], 0.5);
    assert_eq!(scalar_props[LEGACY_ORIGINAL_PROPS_PROP], "source-value");
    assert_eq!(impact, LegacyImpactWire::Absent);

    let mut matching_props = serde_json::json!({(LEGACY_IMPACT_PROP): 0.0});
    let mut matching_impact = LegacyImpactWire::Number(0.0);
    migrate_legacy_impact(&mut matching_props, &mut matching_impact).unwrap();

    let mut conflicting_props = serde_json::json!({(LEGACY_IMPACT_PROP): 0.4});
    let mut conflicting_impact = LegacyImpactWire::Number(0.5);
    assert!(
        migrate_legacy_impact(&mut conflicting_props, &mut conflicting_impact)
            .unwrap_err()
            .to_string()
            .contains("KIMPACT_MIGRATION_CONFLICT")
    );
    assert_eq!(conflicting_props[LEGACY_IMPACT_PROP], 0.4);
}
