// Head-to-head vector benchmark harness (GenesisDB side).
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

    // --- Insert (durable: each add_node fsyncs the WAL) ---
    let t = Instant::now();
    for i in 0..n {
        let emb: Vec<f64> = corpus[i * dim..(i + 1) * dim].iter().map(|&x| x as f64).collect();
        storage
            .add_node(NodeInput {
                id: Some(i.to_string()),
                labels: vec!["doc".to_string()],
                props: None,
                embedding: Some(emb),
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
            })
            .unwrap();
    }
    let insert_sec = t.elapsed().as_secs_f64();

    // --- k-NN query (alpha=0 => pure vector search via HNSW) ---
    let mut lats_us: Vec<f64> = Vec::with_capacity(q);
    let mut topk: Vec<Vec<i64>> = Vec::with_capacity(q);
    for qi in 0..q {
        let qv: Vec<f64> = queries[qi * dim..(qi + 1) * dim].iter().map(|&x| x as f64).collect();
        let t0 = Instant::now();
        let res = storage
            .hybrid_search(HybridSearchInput { query_vector: qv, k: k as u32, alpha: Some(0.0), lang: None, as_of: None })
            .unwrap();
        lats_us.push(t0.elapsed().as_nanos() as f64 / 1000.0);
        topk.push(res.iter().take(k).map(|nb| nb.node.id.parse::<i64>().unwrap_or(-1)).collect());
    }
    let mut sorted = lats_us.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let out = serde_json::json!({
        "engine": "GenesisDB (hnsw_rs)",
        "model": model,
        "n": n, "q": q, "dim": dim, "k": k,
        "insert_sec": insert_sec,
        "insert_per_sec": n as f64 / insert_sec,
        "q_p50_us": percentile(&sorted, 50.0),
        "q_p95_us": percentile(&sorted, 95.0),
        "q_mean_us": lats_us.iter().sum::<f64>() / lats_us.len() as f64,
        "topk": topk,
        "durability": "per-op WAL fsync"
    });
    let mut f = fs::File::create(format!("{bench}/genesis_results.json")).unwrap();
    f.write_all(serde_json::to_string_pretty(&out).unwrap().as_bytes()).unwrap();
    println!(
        "GenesisDB: insert {:.0} vec/s ({:.2}s), query p50 {:.1}µs p95 {:.1}µs",
        n as f64 / insert_sec, insert_sec, percentile(&sorted, 50.0), percentile(&sorted, 95.0)
    );
}
