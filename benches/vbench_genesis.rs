// Head-to-head vector benchmark harness (GenesisBlockDB side).
// Reads the SAME embedding vectors used by the Chroma harness so the comparison
// is apples-to-apples: identical corpus, identical queries, identical k.
// Vectors are produced by vbench.py (bge-m3, 1024-dim) and exchanged as raw
// little-endian f32. Results are written to genesis_results.json; recall vs the
// exact brute-force ground truth is computed afterwards by vbench.py.
//
// Run:  GB_VBENCH=C:\Users\freshair\gb_vbench cargo run --release --bin vbench-genesis

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::io::{BufReader, Read, Write};
use std::time::Instant;

fn read_f32(path: &str) -> Vec<f32> {
    let bytes = fs::read(path).expect("read f32 file");
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
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

    // Corpus is STREAMED from disk during ingest (never fully resident). Loading
    // the whole f32 file would cost ~2× its size in RAM (u8 read + f32 copy) — 8 GB
    // at 1M×1024, which OOMs a 32 GB box once the engine also grows. Streaming also
    // makes the RSS read clean by construction: no harness corpus buffer to subtract.
    let corpus_path = format!("{bench}/corpus.f32");
    let corpus_bytes = fs::metadata(&corpus_path).expect("stat corpus").len() as usize;
    assert_eq!(corpus_bytes, n * dim * 4, "corpus.f32 size mismatch");
    let queries = read_f32(&format!("{bench}/queries.f32"));
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
    let efc: u32 = std::env::var("GB_EF")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    storage.set_index_params(efc, 100);

    // --- RSS / quant probe knobs (MARK XV P1) -------------------------------
    // GB_QUANT  = none | sq8 | bq   (default none = legacy default collection)
    // GB_RERANK = 1                  keep an exact f32 sidecar for rerank
    // GB_LIMIT  = <count>            ingest only the first N rows of the corpus,
    //                                so the RSS/latency scale curve can be swept
    //                                from ONE corpus. NOTE: recall scoring against
    //                                the full-corpus ground truth is only valid at
    //                                GB_LIMIT == n; use GB_LIMIT for RSS/latency.
    let quant = std::env::var("GB_QUANT")
        .unwrap_or_else(|_| "none".into())
        .to_lowercase();
    let rerank = std::env::var("GB_RERANK")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let limit = std::env::var("GB_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok());
    let n_eff = limit.map(|l| l.min(n)).unwrap_or(n);
    // Route ingest+search to a quantized collection when requested; else the
    // legacy default space (collection: None) so old invocations are unchanged.
    let coll: Option<String> = if quant != "none" {
        storage
            .create_collection(
                "bench".into(),
                model.clone(),
                dim as u32,
                None,
                Some(quant.clone()),
                None,
                Some(rerank),
            )
            .expect("create_collection");
        Some("bench".to_string())
    } else {
        None
    };

    // --- Insert via bulk path. The corpus is read from disk one chunk at a time
    //     (≈ chunk*dim*4 bytes resident), so neither the f32 corpus nor the full
    //     N NodeInputs are ever materialized at once. ---
    let mut corpus_rdr = BufReader::new(fs::File::open(&corpus_path).expect("open corpus"));
    let t = Instant::now();
    let chunk = 10_000usize;
    let mut i0 = 0usize;
    while i0 < n_eff {
        let i1 = (i0 + chunk).min(n_eff);
        let rows = i1 - i0;
        let mut buf = vec![0u8; rows * dim * 4];
        corpus_rdr.read_exact(&mut buf).expect("read corpus chunk");
        let fbuf: Vec<f32> = buf
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let inputs: Vec<NodeInput> = (0..rows)
            .map(|r| NodeInput {
                id: Some((i0 + r).to_string()),
                labels: vec!["doc".to_string()],
                props: None,
                embedding: Some(
                    fbuf[r * dim..(r + 1) * dim]
                        .iter()
                        .map(|&x| x as f64)
                        .collect(),
                ),
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: coll.clone(),
            })
            .collect();
        storage.bulk_add_nodes(inputs).unwrap();
        i0 = i1;
    }
    drop(corpus_rdr);
    // Drain the async HNSW backlog so the index is fully resident before RSS is
    // read (otherwise the probe undercounts the graph).
    storage.flush_index();
    let insert_sec = t.elapsed().as_secs_f64();
    let peak_rss_mb = {
        let mut s = sysinfo::System::new_all();
        s.refresh_all();
        sysinfo::get_current_pid()
            .ok()
            .and_then(|pid| s.process(pid).map(|p| p.memory()))
            .unwrap_or(0)
            / 1024
            / 1024
    };

    // k-NN query runner at the current ef_search (alpha=0 => pure vector search)
    let run_queries = |st: &Storage| -> (Vec<f64>, Vec<Vec<i64>>) {
        let mut lats = Vec::with_capacity(q);
        let mut tk = Vec::with_capacity(q);
        for qi in 0..q {
            let qv: Vec<f64> = queries[qi * dim..(qi + 1) * dim]
                .iter()
                .map(|&x| x as f64)
                .collect();
            let t0 = Instant::now();
            let res = st
                .hybrid_search(HybridSearchInput {
                    query_vector: qv,
                    k: k as u32,
                    alpha: Some(0.0),
                    lang: None,
                    as_of: None,
                    collection: coll.clone(),
                    ef_search: None,
                })
                .unwrap();
            lats.push(t0.elapsed().as_nanos() as f64 / 1000.0);
            tk.push(
                res.iter()
                    .take(k)
                    .map(|nb| nb.node.id.parse::<i64>().unwrap_or(-1))
                    .collect(),
            );
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
            let mut s = lats.clone();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap());
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
            "n": n_eff, "q": q, "dim": dim, "k": k, "quant": quant, "rerank": rerank,
            "peak_rss_mb": peak_rss_mb, "insert_per_sec": n_eff as f64 / insert_sec, "points": points
        });
        fs::write(
            format!("{bench}/genesis_frontier.json"),
            serde_json::to_string_pretty(&out).unwrap(),
        )
        .unwrap();
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
        "n": n_eff, "q": q, "dim": dim, "k": k, "quant": quant, "rerank": rerank,
        "insert_sec": insert_sec,
        "build_sec": insert_sec,
        "insert_per_sec": n_eff as f64 / insert_sec,
        "peak_rss_mb": peak_rss_mb,
        "q_p50_us": percentile(&sorted, 50.0),
        "q_p95_us": percentile(&sorted, 95.0),
        "q_mean_us": lats_us.iter().sum::<f64>() / lats_us.len() as f64,
        "topk": topk,
        "durability": "durable, batched WAL fsync (1024/chunk)"
    });
    let mut f = fs::File::create(format!("{bench}/genesis_results.json")).unwrap();
    f.write_all(serde_json::to_string_pretty(&out).unwrap().as_bytes())
        .unwrap();
    println!(
        "GenesisBlockDB: n={} quant={} rerank={} | insert {:.0} vec/s (build {:.2}s), RSS {} MB, query p50 {:.1}µs p95 {:.1}µs",
        n_eff, quant, rerank, n_eff as f64 / insert_sec, insert_sec, peak_rss_mb, percentile(&sorted, 50.0), percentile(&sorted, 95.0)
    );
}
