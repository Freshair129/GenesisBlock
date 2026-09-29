use genesis_block_native::{
    AccessContext, BatchInput, EdgeInput, GenerationInfo, GenesisTransaction, HybridSearchInput,
    NeighborInput, NodeInput, OpenOptions, ReadLease, ReadView, RelationalColumn,
    RelationalColumnType, RelationalFilter, RelationalMutationGroup, RelationalMutationKind,
    RelationalQuery, RelationalRowMutation, RelationalSchemaPackage, RelationalTable, Storage,
    TemporalRead, VectorUpsertInput,
};
use serde_json::{json, Value};
use std::{fs, path::Path, time::Duration};

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn node(id: &str, props: Value, embedding: Option<Vec<f64>>) -> NodeInput {
    NodeInput {
        id: Some(id.into()),
        labels: vec!["P6".into()],
        props: Some(props),
        embedding,
        lang: Some("en".into()),
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

fn fixture_schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "default".into(),
        schema_version: 1,
        previous_version: None,
        package_id: "00000000-0000-4000-8000-000000000006".into(),
        schema_hash: String::new(),
        named_queries: vec![],
        tables: vec![RelationalTable {
            name: "items".into(),
            columns: vec![
                RelationalColumn::required("id", RelationalColumnType::Text),
                RelationalColumn::required("name", RelationalColumnType::Text),
            ],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
    }
}

fn commit_cross_domain_fixture(storage: &Storage) -> u64 {
    storage
        .register_relational_schema(fixture_schema())
        .unwrap();
    let receipt = storage
        .commit_transaction(GenesisTransaction {
            transaction_id: "p6-generation-cross-domain".into(),
            expected_frontier: None,
            relational: vec![RelationalMutationGroup {
                namespace: "default".into(),
                mutations: vec![RelationalRowMutation {
                    table: "items".into(),
                    kind: RelationalMutationKind::Insert,
                    values: json!({"id": "note-1", "name": "generation coverage"}),
                    key: None,
                }],
            }],
            graph: BatchInput {
                nodes: vec![
                    node("note-1", json!({"name": "generation coverage"}), None),
                    node("project-1", json!({"name": "P6"}), None),
                ],
                edges: vec![EdgeInput {
                    id: Some("edge-note-project".into()),
                    from: "note-1".into(),
                    to: "project-1".into(),
                    rel: "BELONGS_TO".into(),
                    props: None,
                    valid_from: None,
                    supersede: None,
                    impact: Some(0.5),
                    caused_by: None,
                }],
            },
            vectors: vec![VectorUpsertInput {
                node_id: "note-1".into(),
                collection: "default".into(),
                embedding: vec![1.0, 0.0, 0.0, 0.0],
            }],
        })
        .unwrap();
    assert!(receipt.stable);
    receipt.commit_sequence
}

fn read_context() -> AccessContext {
    AccessContext {
        principal: "p6-generation-test".into(),
        namespace: "default".into(),
    }
}

fn temporal_read() -> TemporalRead {
    TemporalRead {
        as_of: None,
        tx_as_of: None,
    }
}

fn hybrid_query() -> HybridSearchInput {
    HybridSearchInput {
        query_vector: vec![1.0, 0.0, 0.0, 0.0],
        k: 1,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: Some("default".into()),
        ef_search: None,
        oversample: None,
    }
}

fn neighbor_query() -> NeighborInput {
    NeighborInput {
        depth: Some(1),
        rel: Some("BELONGS_TO".into()),
        rels: None,
        direction: Some("out".into()),
        as_of: None,
        include_invalid: Some(false),
        limit: Some(10),
    }
}

fn relational_query() -> RelationalQuery {
    RelationalQuery {
        namespace: "default".into(),
        table: "items".into(),
        columns: vec!["items.name".into()],
        joins: vec![],
        filters: vec![RelationalFilter::equal("items.id", json!("note-1"))],
        limit: Some(1),
        offset: None,
    }
}

fn assert_generation_equal(left: &GenerationInfo, right: &GenerationInfo) {
    assert_eq!(left.generation_id, right.generation_id);
    assert_eq!(left.wal_frontier, right.wal_frontier);
    assert_eq!(left.publication_seq, right.publication_seq);
    assert_eq!(left.txn_frontier, right.txn_frontier);
    assert_eq!(left.history_horizon, right.history_horizon);
    assert_eq!(left.acl_revision, right.acl_revision);
    assert_eq!(
        left.component_manifest_sha256,
        right.component_manifest_sha256
    );
}

fn assert_publication_sequence(generation: &GenerationInfo) {
    assert_eq!(
        generation.publication_seq,
        generation
            .wal_frontier
            .checked_add(1)
            .expect("publication sequence must follow the covered WAL frontier")
    );
    assert_eq!(generation.component_manifest_sha256.len(), 64);
    assert!(generation
        .component_manifest_sha256
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit()));
}

fn assert_cross_domain_fixture(storage: &Storage) {
    assert!(storage.node_view("note-1").is_some());
    let neighbors = storage
        .neighbors("note-1".into(), neighbor_query(), false)
        .unwrap();
    assert_eq!(
        neighbors
            .iter()
            .map(|neighbor| neighbor.node.id.as_str())
            .collect::<Vec<_>>(),
        vec!["project-1"]
    );
    assert_eq!(
        storage.query_relational(relational_query()).unwrap(),
        vec![json!({"items.name": "generation coverage"})]
    );
    storage.flush_index();
    let hits = storage.hybrid_search(hybrid_query()).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node.id, "note-1");
}

fn find_manifest_component_path(value: &Value, file_name: &str) -> Option<String> {
    match value {
        Value::Object(object) => {
            let path = object.get("path").and_then(Value::as_str);
            let has_bytes = object.get("bytes").and_then(Value::as_u64).is_some();
            let has_hash = object.get("sha256").and_then(Value::as_str).is_some();
            if let (Some(path), true, true) = (path, has_bytes, has_hash) {
                if Path::new(path).file_name().and_then(|name| name.to_str()) == Some(file_name) {
                    return Some(path.to_owned());
                }
            }
            object
                .values()
                .find_map(|child| find_manifest_component_path(child, file_name))
        }
        Value::Array(values) => values
            .iter()
            .find_map(|child| find_manifest_component_path(child, file_name)),
        _ => None,
    }
}

#[test]
fn publication_covers_one_cross_domain_frontier_and_reuses_unchanged_generation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = open(dir.path());
    let transaction_frontier = commit_cross_domain_fixture(&storage);

    let generation = storage.publish_generation().unwrap();
    assert_eq!(generation.wal_frontier, transaction_frontier);
    assert_eq!(generation.txn_frontier, transaction_frontier);
    assert_publication_sequence(&generation);

    let repeated = storage.publish_generation().unwrap();
    assert_generation_equal(&generation, &repeated);

    let lease = storage
        .pin_generation(read_context(), temporal_read(), Duration::from_secs(30))
        .unwrap();
    let observed = storage
        .with_read_lease(&lease, |view: &ReadView| {
            let node = view.node_view("note-1")?.map(|node| node.id);
            let neighbor_ids = view
                .neighbors("note-1".into(), neighbor_query(), false)?
                .into_iter()
                .map(|neighbor| neighbor.node.id)
                .collect::<Vec<_>>();
            let hits = view
                .hybrid_search(hybrid_query())?
                .into_iter()
                .map(|hit| hit.node.id)
                .collect::<Vec<_>>();
            let rows = view.query_relational(relational_query())?;
            Ok((node, neighbor_ids, hits, rows))
        })
        .unwrap();

    assert_eq!(observed.0.as_deref(), Some("note-1"));
    assert_eq!(observed.1, vec!["project-1"]);
    assert_eq!(observed.2, vec!["note-1"]);
    assert_eq!(
        observed.3,
        vec![json!({"items.name": "generation coverage"})]
    );
}

#[test]
fn pin_generation_lazily_covers_a_committed_write() {
    let dir = tempfile::tempdir().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(node("before-generation", json!({"v": 1}), None))
        .unwrap();
    let initial = storage.publish_generation().unwrap();

    storage
        .add_node(node(
            "after-generation",
            json!({"v": 2}),
            Some(vec![0.0, 1.0, 0.0, 0.0]),
        ))
        .unwrap();

    // ReadLease is opaque by contract; visibility through the newly pinned ReadView
    // is the observable guarantee for pin_generation's lazy publication path.
    let lease: ReadLease = storage
        .pin_generation(read_context(), temporal_read(), Duration::from_secs(30))
        .unwrap();
    let observed = storage
        .with_read_lease(&lease, |view| {
            let node = view.node_view("after-generation")?;
            let hits = view.hybrid_search(HybridSearchInput {
                query_vector: vec![0.0, 1.0, 0.0, 0.0],
                k: 1,
                alpha: Some(0.0),
                lang: None,
                as_of: None,
                collection: Some("default".into()),
                ef_search: None,
                oversample: None,
            })?;
            Ok((
                node.is_some(),
                hits.into_iter().map(|hit| hit.node.id).collect::<Vec<_>>(),
            ))
        })
        .unwrap();

    assert!(
        observed.0,
        "the pinned generation must include the committed node"
    );
    assert_eq!(observed.1, vec!["after-generation"]);
    assert!(initial.publication_seq > initial.wal_frontier);
}

#[test]
fn invalid_component_manifest_falls_back_to_the_complete_wal_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let storage = open(dir.path());
    commit_cross_domain_fixture(&storage);
    storage.publish_generation().unwrap();
    drop(storage);

    let state: Value =
        serde_json::from_slice(&fs::read(dir.path().join("state.json")).unwrap()).unwrap();
    let nodes_component = find_manifest_component_path(&state, "nodes.bin")
        .expect("the P6 manifest must include the graph node component");
    let component_path = dir.path().join(nodes_component);
    assert!(component_path.is_file());
    fs::remove_file(component_path).unwrap();

    let recovered = open(dir.path());
    assert_cross_domain_fixture(&recovered);
}

#[test]
fn generation_record_survives_compact_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let storage = open(dir.path());
    commit_cross_domain_fixture(&storage);
    let before_compact = storage.publish_generation().unwrap();
    let stale_snapshot = fs::read(dir.path().join("state.json")).unwrap();
    storage.compact().unwrap();
    drop(storage);
    fs::write(dir.path().join("state.json"), stale_snapshot).unwrap();

    let reopened = open(dir.path());
    let after_reopen = reopened.publish_generation().unwrap();

    assert_generation_equal(&before_compact, &after_reopen);
    assert_cross_domain_fixture(&reopened);
}

#[test]
fn snapshot_generation_must_match_its_signed_wal_materialization() {
    let dir = tempfile::tempdir().unwrap();
    let storage = open(dir.path());
    commit_cross_domain_fixture(&storage);
    let published = storage.publish_generation().unwrap();
    storage.compact().unwrap();
    drop(storage);

    let state_path = dir.path().join("state.json");
    let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    let generation_id = state["p6"]["generation"]["generation_id"]
        .as_u64()
        .expect("snapshot must contain the signed generation materialization");
    state["p6"]["generation"]["generation_id"] = json!(generation_id + 1);
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

    let reopened = open(dir.path());
    let recovered = reopened.publish_generation().unwrap();
    assert_generation_equal(&published, &recovered);
    assert_cross_domain_fixture(&reopened);
}

#[test]
fn read_view_rejects_unsupported_transaction_time_selectors() {
    let dir = tempfile::tempdir().unwrap();
    let storage = open(dir.path());
    let transaction_frontier = commit_cross_domain_fixture(&storage);
    let generation = storage.publish_generation().unwrap();
    let lease = storage
        .pin_generation(
            read_context(),
            TemporalRead {
                as_of: None,
                tx_as_of: Some(transaction_frontier.max(generation.history_horizon)),
            },
            Duration::from_secs(30),
        )
        .unwrap();

    storage
        .with_read_lease(&lease, |view| {
            let neighbors = view.neighbors("note-1".into(), neighbor_query(), false);
            let hybrid = view.hybrid_search(hybrid_query());
            let versions = view.node_versions("note-1", None);
            let is_unsupported =
                |message: String| message.starts_with("TEMPORAL_SELECTOR_UNSUPPORTED");
            let supported = [
                neighbors
                    .map(|_| ())
                    .map_err(|error| error.to_string())
                    .is_err_and(is_unsupported),
                hybrid
                    .map(|_| ())
                    .map_err(|error| error.to_string())
                    .is_err_and(is_unsupported),
                versions
                    .map(|_| ())
                    .map_err(|error| error.to_string())
                    .is_err_and(is_unsupported),
            ];
            assert_eq!(
                supported,
                [true, true, true],
                "neighbors, hybrid_search, and node_versions must reject lease tx_as_of"
            );
            Ok(())
        })
        .unwrap();

    let valid_time_lease = storage
        .pin_generation(
            read_context(),
            TemporalRead {
                as_of: Some("2026-09-22T00:00:00Z".into()),
                tx_as_of: None,
            },
            Duration::from_secs(30),
        )
        .unwrap();
    storage
        .with_read_lease(&valid_time_lease, |view| {
            let versions = view.node_versions("note-1", None);
            assert!(versions.is_err_and(|error| error
                .to_string()
                .starts_with("TEMPORAL_SELECTOR_UNSUPPORTED")));
            Ok(())
        })
        .unwrap();
}
