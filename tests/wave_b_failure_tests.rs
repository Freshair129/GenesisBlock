use genesis_block_native::{EdgeInput, OpenOptions, Storage};
use serde_json::{json, Value};

#[test]
fn edge_history_apply_fault_requires_recovery_and_replays_acked_rebind() {
    let dir = tempfile::tempdir().unwrap();
    let opts = OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    };
    let s = Storage::open(opts.clone()).unwrap();
    for id in ["A", "B", "C", "D"] {
        s.add_node(
            serde_json::from_value(
                json!({"id":id,"labels":[],"valid_from":"2020-01-01T00:00:00Z"}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    let edge = |from: &str, to: &str| -> EdgeInput {
        serde_json::from_value(
            json!({"id":"e","from":from,"to":to,"rel":"R","valid_from":"2020-01-01T00:00:00Z"}),
        )
        .unwrap()
    };
    s.add_edge(edge("A", "B")).unwrap();
    let first = s.stable_frontier();
    let conn = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_edge_history BEFORE INSERT ON edge_versions BEGIN SELECT RAISE(ABORT, 'injected edge history failure'); END;").unwrap();
    let error = s.add_edge(edge("C", "D")).unwrap_err().to_string();
    assert!(error.contains("DURABLE_COMMIT_APPLY_FAILED"), "{error}");
    assert!(s
        .query_sql("SELECT * FROM edges", vec![])
        .unwrap_err()
        .to_string()
        .contains("RECOVERY_REQUIRED"));
    assert!(s.save_state().is_err());
    conn.execute_batch("DROP TRIGGER fail_edge_history")
        .unwrap();
    drop(conn);
    drop(s);
    let s = Storage::open(opts).unwrap();
    let query = |seed: &str, tx: Option<u64>| -> Value {
        let mut q = json!({"contract_version":"query-ir.v1","request_id":"fault","operation":{"kind":"traverse","seed_id":seed,"depth":1,"direction":"out","relations":["R"]}});
        if let Some(tx) = tx {
            q["temporal"] = json!({"tx_as_of":tx});
        }
        s.execute_query_ir_json(q).unwrap()
    };
    assert!(query("A", None)["data"].as_array().unwrap().is_empty());
    assert_eq!(query("C", None)["data"][0]["node"]["id"], "D");
    assert_eq!(query("A", Some(first))["data"][0]["node"]["id"], "B");
}
