use genesis_block_native::{BatchInput, NodeInput, OpenOptions, Storage};
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
    })
    .unwrap()
}

fn projection_path(path: &str) -> PathBuf {
    Path::new(path).join("projection.sqlite")
}

#[test]
fn sqlite_projection_bootstraps_schema_and_props() {
    let path = fresh("sqlite_projection_bootstrap");
    let storage = open(&path);

    storage
        .add_node(NodeInput {
            id: Some("sqlite-node".to_string()),
            labels: vec!["Doc".to_string(), "Spec".to_string()],
            props: Some(json!({"name": "sqlite", "v": 1})),
            embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
            lang: Some("en".to_string()),
            valid_from: Some("2026-07-19T00:00:00Z".to_string()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    let sqlite_path = projection_path(&path);
    assert!(sqlite_path.exists(), "projection.sqlite should exist");

    let conn = Connection::open(sqlite_path).unwrap();
    let props_tables: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('props', 'node_labels', 'projection_state')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(props_tables, 3, "all projection tables should exist");

    let node_u32 = storage.get_u32("sqlite-node").unwrap();
    let payload: String = conn
        .query_row(
            "SELECT payload FROM props WHERE node_u32 = ?1",
            [node_u32],
            |row| row.get(0),
        )
        .unwrap();
    let labels: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM node_labels WHERE node_u32 = ?1",
            [node_u32],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&payload).unwrap()["name"],
        "sqlite"
    );
    assert_eq!(labels, 2);
}

#[test]
fn sqlite_s1_resident_nodes_are_lean_but_views_are_hydrated() {
    let path = fresh("sqlite_s1_lean_nodes");
    let storage = open(&path);

    storage
        .add_node(NodeInput {
            id: Some("lean-node".to_string()),
            labels: vec!["Lean".to_string()],
            props: Some(json!({"payload": "from-sqlite", "n": 7})),
            embedding: Some(vec![0.0, 0.0, 1.0, 0.0]),
            lang: Some("en".to_string()),
            valid_from: Some("2026-07-19T00:00:00Z".to_string()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    let node_u32 = storage.get_u32("lean-node").unwrap();
    let resident = storage.nodes.get(&node_u32).unwrap();
    assert!(
        resident.props.is_null(),
        "resident node props should be lean"
    );

    let hydrated = storage.node_view("lean-node").unwrap();
    assert_eq!(hydrated.props["payload"], "from-sqlite");
    assert_eq!(hydrated.props["n"], 7);

    let projected = storage.projection_props(node_u32).unwrap().unwrap();
    assert_eq!(projected["payload"], "from-sqlite");
}

#[test]
fn sqlite_projection_rebuilds_when_sidecar_missing() {
    let path = fresh("sqlite_projection_rebuild_missing");
    {
        let storage = open(&path);
        storage
            .add_node(NodeInput {
                id: Some("rebuild-node".to_string()),
                labels: vec!["Rebuild".to_string()],
                props: Some(json!({"ok": true})),
                embedding: Some(vec![0.0, 1.0, 0.0, 0.0]),
                lang: Some("en".to_string()),
                valid_from: Some("2026-07-19T00:00:00Z".to_string()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
        storage.save_state().unwrap();
    }

    fs::remove_file(projection_path(&path)).unwrap();

    let reopened = open(&path);
    let node_u32 = reopened.get_u32("rebuild-node").unwrap();
    let props = reopened.projection_props(node_u32).unwrap().unwrap();
    assert_eq!(props["ok"], true);
}

#[test]
fn sqlite_projection_tracks_batch_nodes_and_snapshots() {
    let path = fresh("sqlite_projection_batch_snapshot");
    {
        let storage = open(&path);
        storage
            .execute_batch(BatchInput {
                nodes: vec![
                    NodeInput {
                        id: Some("batch-a".to_string()),
                        labels: vec!["Batch".to_string()],
                        props: Some(json!({"n": 1})),
                        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
                        lang: Some("en".to_string()),
                        valid_from: Some("2026-07-19T00:00:00Z".to_string()),
                        caused_by: None,
                        ttl: None,
                        collection: None,
                    },
                    NodeInput {
                        id: Some("batch-b".to_string()),
                        labels: vec!["Batch".to_string(), "Second".to_string()],
                        props: Some(json!({"n": 2})),
                        embedding: Some(vec![0.0, 1.0, 0.0, 0.0]),
                        lang: Some("en".to_string()),
                        valid_from: Some("2026-07-19T00:00:00Z".to_string()),
                        caused_by: None,
                        ttl: None,
                        collection: None,
                    },
                ],
                edges: vec![],
            })
            .unwrap();
        storage.save_state().unwrap();
    }

    let conn = Connection::open(projection_path(&path)).unwrap();
    let prop_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM props", [], |row| row.get(0))
        .unwrap();
    let label_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM node_labels", [], |row| row.get(0))
        .unwrap();
    let watermark: String = conn
        .query_row(
            "SELECT value FROM projection_state WHERE key = 'node_clock'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(prop_rows, 2);
    assert_eq!(label_rows, 3);
    assert_ne!(watermark, "0");
}

#[test]
fn sqlite_projection_heals_even_when_lamport_watermark_is_ahead() {
    let path = fresh("sqlite_projection_ahead_watermark");
    {
        let storage = open(&path);
        storage
            .add_node(NodeInput {
                id: Some("lagged-node".to_string()),
                labels: vec!["Recovery".to_string()],
                props: Some(json!({"healed": true})),
                embedding: None,
                lang: Some("en".to_string()),
                valid_from: Some("2026-07-20T00:00:00Z".to_string()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
    }

    let conn = Connection::open(projection_path(&path)).unwrap();
    conn.execute("DELETE FROM props", []).unwrap();
    conn.execute(
        "UPDATE projection_state SET value = '4294967295' WHERE key = 'node_clock'",
        [],
    )
    .unwrap();
    drop(conn);

    let reopened = open(&path);
    let node_u32 = reopened.get_u32("lagged-node").unwrap();
    let props = reopened.projection_props(node_u32).unwrap().unwrap();
    assert_eq!(props["healed"], true);
}

#[test]
fn sqlite_projection_uses_full_lww_clock_for_equal_lamport_times() {
    let path = fresh("sqlite_projection_full_lww_clock");
    {
        let storage = open(&path);
        storage
            .add_node(NodeInput {
                id: Some("clock-node".to_string()),
                labels: vec!["Clock".to_string()],
                props: Some(json!({"winner": "wal"})),
                embedding: None,
                lang: Some("en".to_string()),
                valid_from: Some("2026-07-20T00:00:00Z".to_string()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
    }

    let conn = Connection::open(projection_path(&path)).unwrap();
    conn.execute(
        "UPDATE props SET payload = ?1, clock_peer = 'zzzzzzzzzzzzzzzz'",
        [json!({"winner": "higher-peer"}).to_string()],
    )
    .unwrap();
    drop(conn);

    let reopened = open(&path);
    let node_u32 = reopened.get_u32("clock-node").unwrap();
    let props = reopened.projection_props(node_u32).unwrap().unwrap();
    assert_eq!(props["winner"], "higher-peer");
}

#[test]
fn sqlite_projection_recovers_from_invalid_sidecar() {
    let path = fresh("sqlite_projection_corrupt_sidecar");
    {
        let storage = open(&path);
        storage
            .add_node(NodeInput {
                id: Some("corrupt-node".to_string()),
                labels: vec!["Recovery".to_string()],
                props: Some(json!({"recovered": true})),
                embedding: None,
                lang: Some("en".to_string()),
                valid_from: Some("2026-07-20T00:00:00Z".to_string()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
    }

    fs::write(projection_path(&path), b"not a sqlite database").unwrap();

    let reopened = open(&path);
    let node_u32 = reopened.get_u32("corrupt-node").unwrap();
    let props = reopened.projection_props(node_u32).unwrap().unwrap();
    assert_eq!(props["recovered"], true);
}
