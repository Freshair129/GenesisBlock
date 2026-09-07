//! Fixed-seed Wave B replay audit. The argument must name a new fixture directory.
//! Run with --no-default-features --features bins --bin wave-b-replay.
use genesis_block_native::*;
use serde_json::{json, Value};
use std::{fs, path::Path, path::PathBuf, time::Instant};

fn measure(storage: &Storage, tx: Option<u64>) -> (Value, Vec<Vec<String>>) {
    let mut samples = Vec::new();
    let mut answers = Vec::new();
    for i in 0..100 {
        let mut request = json!({
            "contract_version": "query-ir.v1", "request_id": "bench",
            "operation": {"kind": "traverse", "seed_id": format!("n{}", i % 50),
                "depth": 2, "direction": "out", "relations": ["R"]}
        });
        if let Some(tx) = tx {
            request["temporal"] = json!({"tx_as_of": tx});
        }
        let start = Instant::now();
        let result = storage.execute_query_ir_json(request).unwrap();
        samples.push(start.elapsed().as_secs_f64() * 1e6);
        let mut ids: Vec<String> = result["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["node"]["id"].as_str().unwrap().to_owned())
            .collect();
        ids.sort();
        answers.push(ids);
    }
    samples.sort_by(f64::total_cmp);
    let count: usize = answers.iter().map(Vec::len).sum();
    (
        json!({"p50_us": samples[50], "p95_us": samples[95],
            "p99_us": samples[99], "result_count": count}),
        answers,
    )
}

fn journal_bytes(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    for directory in ["journal", "wal"] {
        for entry in fs::read_dir(root.join(directory)).unwrap() {
            let path = entry.unwrap().path();
            assert!(path.is_file(), "unexpected journal entry");
            files.push((path.clone(), fs::read(path).unwrap()));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn main() {
    let path = PathBuf::from(std::env::args().nth(1).expect("new fixture path"));
    assert!(
        !path.exists(),
        "fixture must be new; existing data is never removed"
    );
    let options = OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    };
    let storage = Storage::open(options.clone()).unwrap();
    let start = Instant::now();
    let nodes = (0..500)
        .map(|i| NodeInput {
            id: Some(format!("n{i}")),
            labels: vec![],
            props: None,
            embedding: None,
            lang: None,
            valid_from: Some("2020-01-01T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .collect();
    storage
        .execute_batch(BatchInput {
            nodes,
            edges: vec![],
        })
        .unwrap();
    let mut seed = 20260908u64;
    let edges: Vec<_> = (0..1500)
        .map(|i| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            EdgeInput {
                id: Some(format!("e{i}")),
                from: format!("n{}", i % 500),
                to: format!("n{}", (seed >> 32) % 500),
                rel: "R".into(),
                props: None,
                valid_from: Some("2020-01-01T00:00:00Z".into()),
                supersede: None,
                impact: None,
                caused_by: None,
            }
        })
        .collect();
    storage
        .execute_batch(BatchInput {
            nodes: vec![],
            edges: edges.clone(),
        })
        .unwrap();
    let initial_ms = start.elapsed().as_secs_f64() * 1000.;
    let old = storage.stable_frontier();
    let start = Instant::now();
    for (i, mut edge) in edges.into_iter().take(100).enumerate() {
        edge.from = format!("n{}", (i + 250) % 500);
        edge.to = format!("n{}", (i + 251) % 500);
        storage.add_edge(edge).unwrap();
    }
    let rebind_ms = start.elapsed().as_secs_f64() * 1000.;
    let (current, current_answers) = measure(&storage, None);
    let (history, history_answers) = measure(&storage, Some(old));
    storage.save_state().unwrap();
    drop(storage);

    let projection_bytes = fs::metadata(path.join("projection.sqlite")).unwrap().len();
    let identity = fs::read(path.join("identity.bin")).unwrap();
    let journal = journal_bytes(&path);
    // Allowlist only the disposable files produced by this default-collection fixture.
    // Identity, WAL, journal, lock metadata and unknown files are preserved.
    for name in [
        "state.json",
        "nodes.bin",
        "edges.bin",
        "edges_retired.bin",
        "vec_default.bin",
        "meta_default.bin",
        "fvec_default.bin",
        "projection.sqlite",
        "projection.sqlite-wal",
        "projection.sqlite-shm",
    ] {
        let file = path.join(name);
        if file.exists() {
            fs::remove_file(file).unwrap();
        }
    }
    assert_eq!(fs::read(path.join("identity.bin")).unwrap(), identity);
    assert_eq!(journal_bytes(&path), journal);

    let start = Instant::now();
    let storage = Storage::open(options).unwrap();
    let replay_ms = start.elapsed().as_secs_f64() * 1000.;
    assert_eq!(fs::read(path.join("identity.bin")).unwrap(), identity);
    assert_eq!(storage.nodes.len(), 500);
    assert_eq!(storage.edges.len(), 1500);
    let (replayed_current, replayed_current_answers) = measure(&storage, None);
    let (replayed_history, replayed_history_answers) = measure(&storage, Some(old));
    assert_eq!(
        replayed_current_answers, current_answers,
        "current answers changed after replay"
    );
    assert_eq!(
        replayed_history_answers, history_answers,
        "historical answers changed after replay"
    );
    println!(
        "RESULT {}",
        json!({
            "seed": 20260908, "nodes": 500, "edges": 1500, "rebinds": 100, "samples": 100,
            "initial_ms": initial_ms, "rebind_ms": rebind_ms, "current": current,
            "history": history, "projection_bytes": projection_bytes, "replay_ms": replay_ms,
            "replayed_current": replayed_current, "replayed_history": replayed_history,
            "identity_preserved": true, "journal_preserved_during_cleanup": true,
            "query_answers_preserved": true
        })
    );
}
