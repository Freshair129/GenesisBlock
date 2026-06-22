// Head-to-head vector benchmark harness (GenesisBlockDB side).
// Reads the SAME embedding vectors used by the Chroma harness so the comparison
// is apples-to-apples: identical corpus, identical queries, identical k.
// Vectors are produced by vbench.py (bge-m3, 1024-dim) and exchanged as raw
// little-endian f32. Results are written to genesis_results.json; recall vs the
// exact brute-force ground truth is computed afterwards by vbench.py.
//
// Run:  GB_VBENCH=C:\Users\freshair\gb_vbench cargo run --release --bin vbench-genesis

use genesis_block_native::{Storage, OpenOptions, NodeInput, HybridSearchInput};
use std::fs;
use std::io::Write;
use std::time::Instant;

fn read_f32(path: &str) -> Vec<f32> {
    let bytes = fs::read(path).expect("read f32 file");
    bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let idx = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn main() {
    let bench = std::env::var("GB_VBENCH").expect("set GB_VBENCH to the bench dir");
    let meta: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(format!("{bench}/meta.json")).unwrap()).unwrap();
    let n = meta["n"].as_u64().unwrap() as usize;
    let q = meta["q"].as_u64().unwrap() as usize;
    let dim = meta["dim"].as_u64().unwrap() as usize;
    let k = meta["k"].as_u64().unwrap() as usize;
    let model = meta["model"].as_str().unwrap_or("?").to_string();

    let corpus = read_f32(&format!("{bench}/corpus.f32"));
    let queries = read_f32(&format!("{bench}/queries.f32"));
    assert_eq!(corpus.len(), n * dim);
    assert_eq!(queries.len(), q * dim);

    let dbpath = format!("{bench}/gdb");
    let _ = fs::remove_dir_all(&dbpath);
    let storage = Storage::open(OpenOptions {
        path: dbpath,
        page_cache_mb: Some(256),
        read_only: Some(false),
        vector_dim: Some(dim as u32),
    })
    .expect("open storage");

    // HNSW build/search effort (override without rebuilding via GB_EF env).
    let efc: u32 = std::env::var("GB_EF").ok().and_then(|s| s.parse().ok()).unwrap_or(200);
    storage.set_index_params(efc, 100);

    // --- Insert via bulk path, streamed in chunks so we never materialize all
    //     N NodeInputs at once (at 1M x 1024 that f64 staging would be ~8 GB). ---
    let t = Instant::now();
    let chunk = 10_000usize;
    let mut i0 = 0usize;
    while i0 < n {
        let i1 = (i0 + chunk).min(n);
        let inputs: Vec<NodeInput> = (i0..i1)
            .map(|i| NodeInput {
                id: Some(i.to_string()),
                labels: vec!["doc".to_string()],
                props: None,
                embedding: Some(corpus[i * dim..(i + 1) * dim].iter().map(|&x| x as f64).collect()),
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None, collection: None,
            })
            .collect();
        storage.bulk_add_nodes(inputs).unwrap();
        i0 = i1;
    }
    let insert_sec = t.elapsed().as_secs_f64();
    let peak_rss_mb = {
        let mut s = sysinfo::System::new_all();
        s.refresh_all();
        sysinfo::get_current_pid().ok().and_then(|pid| s.process(pid).map(|p| p.memory())).unwrap_or(0) / 1024 / 1024
    };

    // k-NN query runner at the current ef_search (alpha=0 => pure vector search)
    let run_queries = |st: &Storage| -> (Vec<f64>, Vec<Vec<i64>>) {
        let mut lats = Vec::with_capacity(q);
        let mut tk = Vec::with_capacity(q);
        for qi in 0..q {
            let qv: Vec<f64> = queries[qi * dim..(qi + 1) * dim].iter().map(|&x| x as f64).collect();
            let t0 = Instant::now();
            let res = st
                .hybrid_search(HybridSearchInput { query_vector: qv, k: k as u32, alpha: Some(0.0), lang: None, as_of: None, collection: None, ef_search: None })
                .unwrap();
            lats.push(t0.elapsed().as_nanos() as f64 / 1000.0);
            tk.push(res.iter().take(k).map(|nb| nb.node.id.parse::<i64>().unwrap_or(-1)).collect());
        }
        (lats, tk)
    };

    // Recall–Latency frontier: build once, sweep ef_search at query time.
    if let Ok(sweep) = std::env::var("GB_EF_SWEEP") {
        let mut points = Vec::new();
        for tok in sweep.split(',') {
            let efs: u32 = tok.trim().parse().unwrap_or(100);
            storage.set_index_params(efc, efs);
            let (lats, tk) = run_queries(&storage);
            let mut s = lats.clone(); s.sort_by(|a, b| a.partial_cmp(b).unwrap());
            points.push(serde_json::json!({
                "ef_search": efs,
                "q_p50_us": percentile(&s, 50.0),
                "q_p95_us": percentile(&s, 95.0),
                "topk": tk
            }));
            println!("  ef_search={} -> p50 {:.1}µs", efs, percentile(&s, 50.0));
        }
        let out = serde_json::json!({
            "engine": "GenesisBlockDB (hnsw_rs)", "model": model, "ef_construction": efc,
            "n": n, "q": q, "dim": dim, "k": k, "insert_per_sec": n as f64 / insert_sec, "points": points
        });
        fs::write(format!("{bench}/genesis_frontier.json"), serde_json::to_string_pretty(&out).unwrap()).unwrap();
        println!("GenesisBlockDB frontier written ({} points)", points.len());
        return;
    }

    let (lats_us, topk) = run_queries(&storage);
    let mut sorted = lats_us.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let out = serde_json::json!({
        "engine": "GenesisBlockDB (hnsw_rs)",
        "model": model,
        "ef_construction": efc,
        "n": n, "q": q, "dim": dim, "k": k,
        "insert_sec": insert_sec,
        "build_sec": insert_sec,
        "insert_per_sec": n as f64 / insert_sec,
        "peak_rss_mb": peak_rss_mb,
        "q_p50_us": percentile(&sorted, 50.0),
        "q_p95_us": percentile(&sorted, 95.0),
        "q_mean_us": lats_us.iter().sum::<f64>() / lats_us.len() as f64,
        "topk": topk,
        "durability": "durable, batched WAL fsync (1024/chunk)"
    });
    let mut f = fs::File::create(format!("{bench}/genesis_results.json")).unwrap();
    f.write_all(serde_json::to_string_pretty(&out).unwrap().as_bytes()).unwrap();
    println!(
        "GenesisBlockDB: insert {:.0} vec/s (build {:.2}s), RSS {} MB, query p50 {:.1}µs p95 {:.1}µs",
        n as f64 / insert_sec, insert_sec, peak_rss_mb, percentile(&sorted, 50.0), percentile(&sorted, 95.0)
    );
}
