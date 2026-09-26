//! Wave D per-index quality envelope.
//!
//! The harness intentionally reports one pass/fail row per index
//! configuration. It never turns a median across builds into a release gate.
//! Run a small local matrix with:
//!
//!   `$env:GB_WAVE_D_OUT='.brain/audit/wave-d'; cargo run --release --no-default-features --features bins --bin wave-d-quality-gate`
//!
//! The defaults are deliberately modest for a local audit. Increase
//! `GB_WAVE_D_N` and `GB_WAVE_D_Q` for a larger evidence run.

use genesis_block_native::{CollectionInfo, HybridSearchInput, NodeInput, OpenOptions, Storage};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::fs;
use std::time::Instant;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn unit_vector(rng: &mut StdRng, dim: usize) -> Vec<f32> {
    let mut vector: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    for value in &mut vector {
        *value /= norm.max(1e-9);
    }
    vector
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn exact_top_k(corpus: &[f32], live: &[bool], query: &[f32], dim: usize, k: usize) -> Vec<String> {
    let mut scored: Vec<(f32, usize)> = live
        .iter()
        .enumerate()
        .filter(|(_, is_live)| **is_live)
        .map(|(index, _)| (dot(query, &corpus[index * dim..(index + 1) * dim]), index))
        .collect();
    scored.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored
        .into_iter()
        .take(k)
        .map(|(_, index)| index.to_string())
        .collect()
}

fn info_for(storage: &Storage, name: &str) -> CollectionInfo {
    storage
        .list_collections()
        .into_iter()
        .find(|collection| collection.name == name)
        .expect("quality collection exists")
}

fn main() {
    let out = std::env::var("GB_WAVE_D_OUT").unwrap_or_else(|_| ".brain/audit/wave-d".into());
    let n = env_usize("GB_WAVE_D_N", 256);
    let q = env_usize("GB_WAVE_D_Q", 24);
    let dim = env_usize("GB_WAVE_D_DIM", 16);
    let k = env_usize("GB_WAVE_D_K", 5);
    let seed = env_usize("GB_WAVE_D_SEED", 4242) as u64;
    let quantizers: [(&str, bool, f64); 6] = [
        ("none", false, 0.95),
        ("f16", false, 0.90),
        ("sq8", false, 0.80),
        ("sq8", true, 0.90),
        ("bq", false, 0.50),
        ("bq", true, 0.80),
    ];
    let efs = [32_u32, 100_u32];
    let churns = [0_usize, 10, 50, 90];

    assert!(
        n > k && dim > 0 && q > 0,
        "N must exceed K and all sizes positive"
    );
    fs::create_dir_all(&out).expect("create output directory");
    let mut rng = StdRng::seed_from_u64(seed);
    let corpus: Vec<f32> = (0..n).flat_map(|_| unit_vector(&mut rng, dim)).collect();
    let queries: Vec<Vec<f32>> = (0..q)
        .map(|_| {
            let source = rng.gen_range(0..n);
            let mut query = corpus[source * dim..(source + 1) * dim].to_vec();
            for value in &mut query {
                *value += rng.gen_range(-0.02..0.02);
            }
            let norm = query.iter().map(|value| value * value).sum::<f32>().sqrt();
            for value in &mut query {
                *value /= norm.max(1e-9);
            }
            query
        })
        .collect();

    let mut rows = Vec::new();
    for (quant, rerank, recall_floor) in quantizers {
        for ef in efs {
            for churn in churns {
                let label = format!("{quant}-rerank{rerank}-ef{ef}-churn{churn}");
                let dbpath = format!("{out}/db-{label}");
                let _ = fs::remove_dir_all(&dbpath);
                let storage = Storage::open(OpenOptions {
                    path: dbpath.clone(),
                    page_cache_mb: Some(64),
                    read_only: Some(false),
                    vector_dim: Some(dim as u32),
                    retention: None,
                })
                .expect("open quality database");
                storage
                    .create_collection(
                        "quality".into(),
                        "wave-d-synthetic".into(),
                        dim as u32,
                        Some("cosine".into()),
                        Some(quant.into()),
                        Some(ef),
                        Some(rerank),
                    )
                    .expect("create quality collection");
                storage
                    .bulk_add_nodes(
                        (0..n)
                            .map(|index| NodeInput {
                                id: Some(index.to_string()),
                                labels: vec!["quality".into()],
                                props: None,
                                embedding: Some(
                                    corpus[index * dim..(index + 1) * dim]
                                        .iter()
                                        .map(|value| *value as f64)
                                        .collect(),
                                ),
                                lang: None,
                                valid_from: None,
                                caused_by: None,
                                ttl: None,
                                collection: Some("quality".into()),
                            })
                            .collect(),
                    )
                    .expect("ingest quality corpus");
                let mut live = vec![true; n];
                let retired = n * churn / 100;
                for index in 0..retired {
                    storage
                        .retract_node(&index.to_string())
                        .expect("retract churn row");
                    live[index] = false;
                }
                storage.flush_index();
                let flush_lag = storage.index_lag();

                let mut latencies = Vec::with_capacity(q);
                let mut hits = 0usize;
                let mut expected = 0usize;
                let mut shortfall = 0usize;
                let mut equal = 0usize;
                for query in &queries {
                    let wanted = exact_top_k(&corpus, &live, query, dim, k);
                    let started = Instant::now();
                    let result = storage
                        .hybrid_search(HybridSearchInput {
                            query_vector: query.iter().map(|value| *value as f64).collect(),
                            k: k as u32,
                            alpha: Some(0.0),
                            lang: None,
                            as_of: None,
                            collection: Some("quality".into()),
                            ef_search: Some(ef),
                            oversample: Some(4),
                        })
                        .expect("quality query");
                    latencies.push(started.elapsed().as_secs_f64() * 1000.0);
                    let got: Vec<String> = result.iter().map(|row| row.node.id.clone()).collect();
                    if got.len() < wanted.len() {
                        shortfall += wanted.len() - got.len();
                    }
                    let got_set: HashSet<&str> = got.iter().map(String::as_str).collect();
                    hits += wanted
                        .iter()
                        .filter(|id| got_set.contains(id.as_str()))
                        .count();
                    expected += wanted.len();
                    if got == wanted {
                        equal += 1;
                    }
                }
                latencies.sort_by(|left, right| left.partial_cmp(right).unwrap());
                let recall = if expected == 0 {
                    0.0
                } else {
                    hits as f64 / expected as f64
                };
                let checkpoint_start = Instant::now();
                storage.save_state().expect("checkpoint quality database");
                let checkpoint_ms = checkpoint_start.elapsed().as_secs_f64() * 1000.0;
                drop(storage);
                let reopen_start = Instant::now();
                let reopened = Storage::open(OpenOptions {
                    path: dbpath.clone(),
                    page_cache_mb: Some(64),
                    read_only: Some(false),
                    vector_dim: Some(dim as u32),
                    retention: None,
                })
                .expect("reopen quality database");
                let reopen_ms = reopen_start.elapsed().as_secs_f64() * 1000.0;
                let reopened_collection = info_for(&reopened, "quality");
                let pass = shortfall == 0 && recall >= recall_floor;
                println!(
                    "{label}: recall={recall:.4} shortfall={shortfall} p95={:.3}ms pass={pass}",
                    percentile(&latencies, 95.0)
                );
                rows.push(serde_json::json!({
                    "index_id": label,
                    "dataset": { "name": "synthetic_unit_vectors", "n": n, "query_count": q, "seed": seed },
                    "dimension": dim,
                    "metric": "cosine",
                    "quantizer": quant,
                    "rerank": rerank,
                    "ef_search": ef,
                    "filter": { "kind": "live_visibility", "churn_percent": churn, "selectivity": (n - retired) as f64 / n as f64 },
                    "k": k,
                    "exact_filtered_recall": recall,
                    "eligibility_shortfall": shortfall,
                    "result_equality_rate": equal as f64 / q as f64,
                    "latency_ms": { "p50": percentile(&latencies, 50.0), "p95": percentile(&latencies, 95.0), "p99": percentile(&latencies, 99.0) },
                    "flush_lag": flush_lag,
                    "checkpoint_ms": checkpoint_ms,
                    "reopen_ms": reopen_ms,
                    "memory": { "arena_resident_bytes": reopened_collection.arena_resident_bytes, "sidecar_resident_bytes": reopened_collection.sidecar_resident_bytes, "sidecar_disk_bytes": reopened_collection.sidecar_disk_bytes },
                    "recall_floor": recall_floor,
                    "pass": pass
                }));
                drop(reopened);
                let _ = fs::remove_dir_all(&dbpath);
            }
        }
    }

    let pass = rows.iter().all(|row| row["pass"] == true);
    let artifact = serde_json::json!({
        "artifact": "genesisdb.wave-d-quality.v1",
        "engine": "GenesisBlockDB",
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "matrix": { "quantizers": ["none", "f16", "sq8", "bq"], "ef_search": efs, "churn_percent": churns, "k": k },
        "pass": pass,
        "rows": rows
    });
    let output = format!("{out}/wave_d_quality_gate.json");
    fs::write(&output, serde_json::to_string_pretty(&artifact).unwrap())
        .expect("write quality artifact");
    println!(
        "wave-d-quality-gate: pass={pass} rows={} artifact={output}",
        rows.len()
    );
    if !pass {
        std::process::exit(1);
    }
}
