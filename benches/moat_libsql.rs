// WP-3.3 follow-up 1: the libSQL/DiskANN competitor row for the G3 moat bench.
//
// WHY THIS IS A SEPARATE BINARY (load-bearing, do not "simplify" it back into
// moat_bench.rs): `libsql-ffi` and `rusqlite` both compile a bundled SQLite and
// export the same `sqlite3_*` symbols. Linking both into one binary fails on
// MSVC with LNK2005 duplicate symbols; where it does link, the linker silently
// resolves every call to ONE implementation — which would run the engine's own
// `projection.sqlite` and the competitor on the same accidental SQLite build
// and invalidate BOTH sides of the comparison. Process isolation is the only
// sound option, so this binary links libSQL and NOT the engine.
//
// Comparability is preserved by construction rather than by co-location:
//   * identical corpus — same seed, same `gen_vec`/`valid_from_for`/edge rules
//     as moat_bench.rs, so vector-for-vector the two processes see the same data
//     (or the same real-embedding file via GB_MOAT_VECTORS);
//   * identical protocol — same warmup/runs split, same percentile code, same
//     query pool derived from the same seed in the same order;
//   * same host, back to back.
// The config block is echoed into the metrics so a mismatched pairing is
// detectable rather than silently averaged.
//
// What it measures — the two shapes the follow-up is about:
//   q4_libsql: vector-only top-k. DiskANN vs the engine's HNSW. This is the
//              row the decision doc expects to narrow, since it is the axis
//              libSQL actually indexes.
//   q1_libsql: the fused vector+graph+AS-OF shape. Expected NOT to close,
//              because the graph and temporal axes are unchanged — but that is
//              a prediction, so it is measured rather than assumed.
//
// Run (writes <out>/moat_libsql_metrics.json):
//   GB_MOAT_OUT=<dir> GB_MOAT_N=100000 GB_MOAT_DIM=1024 GB_MOAT_RUNS=30 \
//   cargo run --release --no-default-features --features "bins,libsql-baseline" \
//     --bin moat-libsql

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::fs;
use std::time::Instant;

const AS_OF_T: &str = "2023-01-01T00:00:00Z";
const RRF_K: f64 = 60.0;
/// The ANN index is time-blind: `vector_top_k` cannot see `valid_from/valid_to`,
/// so the AS-OF shape must over-fetch and post-filter. This is the pattern the
/// design forces on its users, and the factor is reported.
const ASOF_OVERFETCH: usize = 4;

fn env_usize(k: &str, d: usize) -> usize {
    std::env::var(k)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(d)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

fn stats(mut us: Vec<f64>) -> serde_json::Value {
    us.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = us.len() as f64;
    let mean = us.iter().sum::<f64>() / n;
    let var = us.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1.0).max(1.0);
    let ci95 = 1.96 * (var.sqrt() / n.sqrt());
    serde_json::json!({
        "runs": us.len(),
        "p50_us": percentile(&us, 50.0),
        "p95_us": percentile(&us, 95.0),
        "p99_us": percentile(&us, 99.0),
        "mean_us": mean,
        "ci95_us": ci95,
    })
}

/// Byte-for-byte the same generator as moat_bench.rs — the corpora must match.
fn gen_vec(rng: &mut StdRng, dim: usize) -> Vec<f32> {
    let mut v: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0f32..1.0f32)).collect();
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
    for x in v.iter_mut() {
        *x /= norm;
    }
    v
}

fn valid_from_for(i: usize) -> String {
    format!("202{}-0{}-01T00:00:00Z", i % 4, 1 + (i % 9))
}

/// libSQL's `F32_BLOB(n)` is raw little-endian f32 and both INSERT and
/// `vector_top_k` accept the blob directly. Binding it beats the
/// `vector32('[...]')` TEXT form, which at 1024 dim would make libSQL parse
/// ~8 KB of SQL text per row — a cost no real user of this design would pay.
fn vec_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// Reciprocal-rank fusion — identical constant and shape to moat_bench.rs so
/// the fused row differs only in the store underneath it.
fn rrf(lists: &[Vec<String>], k: usize) -> Vec<String> {
    let mut score: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for list in lists {
        for (rank, id) in list.iter().enumerate() {
            *score.entry(id.clone()).or_default() += 1.0 / (RRF_K + rank as f64 + 1.0);
        }
    }
    let mut v: Vec<(String, f64)> = score.into_iter().collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    v.truncate(k);
    v.into_iter().map(|(id, _)| id).collect()
}

struct Lib {
    rt: tokio::runtime::Runtime,
    conn: libsql::Connection,
    _db: libsql::Database,
    calls: std::cell::Cell<u32>,
}

impl Lib {
    fn create(path: &str, dim: usize) -> Self {
        for suffix in ["", "-wal", "-shm"] {
            let _ = fs::remove_file(format!("{path}{suffix}"));
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let db = rt
            .block_on(libsql::Builder::new_local(path).build())
            .unwrap();
        let conn = db.connect().unwrap();
        rt.block_on(async {
            conn.execute_batch(&format!(
                "PRAGMA journal_mode=WAL;
                 PRAGMA synchronous=NORMAL;
                 CREATE TABLE nodes(
                     id TEXT PRIMARY KEY, v INTEGER, emb F32_BLOB({dim}),
                     valid_from TEXT NOT NULL, valid_to TEXT);
                 CREATE TABLE edges(
                     id TEXT PRIMARY KEY, src TEXT NOT NULL, dst TEXT NOT NULL,
                     rel TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT);
                 CREATE INDEX idx_edges_src ON edges(src);
                 CREATE INDEX idx_edges_dst ON edges(dst);"
            ))
            .await
            .unwrap();
        });
        Lib {
            rt,
            conn,
            _db: db,
            calls: std::cell::Cell::new(0),
        }
    }

    fn bump(&self) {
        self.calls.set(self.calls.get() + 1);
    }

    /// Rows first, index after — the bulk-build order, and the DiskANN build
    /// time is counted in ingest exactly as the engine's HNSW build is.
    fn ingest(&self, vectors: &[Vec<f32>], edges: &[(usize, usize, Option<String>)]) {
        self.rt.block_on(async {
            let tx = self.conn.transaction().await.unwrap();
            for (i, v) in vectors.iter().enumerate() {
                tx.execute(
                    "INSERT INTO nodes(id, v, emb, valid_from, valid_to)
                     VALUES (?1, ?2, ?3, ?4, NULL)",
                    libsql::params![format!("n{i}"), i as i64, vec_blob(v), valid_from_for(i)],
                )
                .await
                .unwrap();
            }
            for (j, (from, to, closed)) in edges.iter().enumerate() {
                tx.execute(
                    "INSERT INTO edges(id, src, dst, rel, valid_from, valid_to)
                     VALUES (?1, ?2, ?3, 'references', ?4, ?5)",
                    libsql::params![
                        format!("e{from}_{to}_{j}"),
                        format!("n{from}"),
                        format!("n{to}"),
                        valid_from_for(from + to),
                        closed.clone(),
                    ],
                )
                .await
                .unwrap();
            }
            tx.commit().await.unwrap();
            self.conn
                .execute_batch("CREATE INDEX nodes_ann ON nodes (libsql_vector_idx(emb))")
                .await
                .unwrap();
        });
    }

    /// Native DiskANN top-k: one statement, already ordered — no app-side rank
    /// pass, which is libSQL's structural advantage over the brute-scan B1.
    fn vector_topk(&self, q: &[f32], k: usize) -> Vec<String> {
        self.bump();
        self.rt.block_on(async {
            let mut rows = self
                .conn
                .query(
                    "SELECT n.id FROM vector_top_k('nodes_ann', ?1, ?2) AS t
                     JOIN nodes n ON n.rowid = t.id",
                    libsql::params![vec_blob(q), k as i64],
                )
                .await
                .unwrap();
            let mut out = Vec::with_capacity(k);
            while let Some(r) = rows.next().await.unwrap() {
                out.push(r.get::<String>(0).unwrap());
            }
            out
        })
    }

    fn vector_topk_asof(&self, q: &[f32], k: usize, as_of: &str) -> Vec<String> {
        self.bump();
        self.rt.block_on(async {
            let mut rows = self
                .conn
                .query(
                    "SELECT n.id FROM vector_top_k('nodes_ann', ?1, ?2) AS t
                     JOIN nodes n ON n.rowid = t.id
                     WHERE n.valid_from <= ?3 AND (n.valid_to IS NULL OR n.valid_to > ?3)
                     LIMIT ?4",
                    libsql::params![vec_blob(q), (k * ASOF_OVERFETCH) as i64, as_of, k as i64],
                )
                .await
                .unwrap();
            let mut out = Vec::with_capacity(k);
            while let Some(r) = rows.next().await.unwrap() {
                out.push(r.get::<String>(0).unwrap());
            }
            out
        })
    }

    fn hops(&self, seed: &str, depth: u32, as_of: &str) -> Vec<String> {
        self.bump();
        self.rt.block_on(async {
            let mut rows = self
                .conn
                .query(
                    "WITH RECURSIVE reach(id, d) AS (
                         SELECT dst, 1 FROM edges
                           WHERE src = ?1 AND valid_from <= ?2
                             AND (valid_to IS NULL OR valid_to > ?2)
                         UNION
                         SELECT e.dst, r.d + 1 FROM edges e JOIN reach r ON e.src = r.id
                           WHERE r.d < ?3 AND e.valid_from <= ?2
                             AND (e.valid_to IS NULL OR e.valid_to > ?2)
                     )
                     SELECT DISTINCT reach.id FROM reach JOIN nodes n ON n.id = reach.id
                       WHERE n.valid_from <= ?2 AND (n.valid_to IS NULL OR n.valid_to > ?2)",
                    libsql::params![seed, as_of, depth as i64],
                )
                .await
                .unwrap();
            let mut out = Vec::new();
            while let Some(r) = rows.next().await.unwrap() {
                out.push(r.get::<String>(0).unwrap());
            }
            out
        })
    }
}

fn main() {
    let out = std::env::var("GB_MOAT_OUT").unwrap_or_else(|_| ".".to_string());
    let n = env_usize("GB_MOAT_N", 100_000);
    let dim = env_usize("GB_MOAT_DIM", 1024);
    let runs = env_usize("GB_MOAT_RUNS", 30);
    let warmup = env_usize("GB_MOAT_WARMUP", 3);
    let k = env_usize("GB_MOAT_K", 10);
    let seed = env_usize("GB_MOAT_SEED", 42) as u64;
    let edges_per = env_usize("GB_MOAT_EDGES_PER_NODE", 5);

    println!("moat-libsql: N={n} dim={dim} runs={runs} k={k} seed={seed}");
    let ts_start = chrono::Utc::now();

    // Corpus generation mirrors moat_bench.rs EXACTLY (same seed, same call
    // order) so both processes measure the same data.
    let mut rng = StdRng::seed_from_u64(seed);
    let corpus_path = std::env::var("GB_MOAT_VECTORS").ok();
    let (vectors, n, corpus_kind) = match corpus_path.as_deref() {
        Some(p) => {
            let raw = fs::read(p).unwrap_or_else(|e| panic!("GB_MOAT_VECTORS {p}: {e}"));
            let per = dim * 4;
            assert_eq!(raw.len() % per, 0, "corpus size not a multiple of dim*4");
            let take = n.min(raw.len() / per);
            let v: Vec<Vec<f32>> = (0..take)
                .map(|i| {
                    raw[i * per..(i + 1) * per]
                        .chunks_exact(4)
                        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect()
                })
                .collect();
            (v, take, format!("real:{p}"))
        }
        None => {
            let v: Vec<Vec<f32>> = (0..n).map(|_| gen_vec(&mut rng, dim)).collect();
            (v, n, "synthetic seeded unit vectors".to_string())
        }
    };
    let mut edge_list: Vec<(usize, usize, Option<String>)> = Vec::with_capacity(n * edges_per);
    for i in 0..n {
        for _ in 0..edges_per {
            let r: f64 = rng.gen();
            let target = ((r * r) * n as f64) as usize % n;
            if target == i {
                continue;
            }
            let closed = if rng.gen::<f64>() < 0.10 {
                Some("2022-06-01T00:00:00Z".to_string())
            } else {
                None
            };
            edge_list.push((i, target, closed));
        }
    }
    let queries: Vec<Vec<f32>> = (0..runs + warmup)
        .map(|_| {
            let base = &vectors[rng.gen_range(0..n)];
            let mut q: Vec<f32> = base
                .iter()
                .map(|x| x + rng.gen_range(-0.05f32..0.05f32))
                .collect();
            let norm = q.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
            for x in q.iter_mut() {
                *x /= norm;
            }
            q
        })
        .collect();

    let db_path = format!("{out}/moat_libsql.db");
    let lib = Lib::create(&db_path, dim);
    let t0 = Instant::now();
    lib.ingest(&vectors, &edge_list);
    let ingest_s = t0.elapsed().as_secs_f64();
    println!("  libsql ingest (incl. DiskANN build): {ingest_s:.1}s");

    let mut report = serde_json::Map::new();
    macro_rules! bench {
        ($name:expr, $f:expr) => {{
            let mut us: Vec<f64> = Vec::with_capacity(runs);
            let mut calls = 0u32;
            for r in 0..(runs + warmup) {
                let t = Instant::now();
                let c = $f(r);
                let e = t.elapsed().as_secs_f64() * 1e6;
                if r >= warmup {
                    us.push(e);
                    calls = c;
                }
            }
            let s = stats(us);
            println!(
                "  {:<12} libsql p50 {:>10.0}us  calls {}",
                $name,
                s["p50_us"].as_f64().unwrap(),
                calls
            );
            report.insert(
                $name.to_string(),
                serde_json::json!({ "libsql": s, "libsql_calls": calls }),
            );
        }};
    }

    bench!("q4_libsql", |r: usize| {
        lib.calls.set(0);
        let _ = lib.vector_topk(&queries[r], k);
        lib.calls.get()
    });
    bench!("q1_libsql", |r: usize| {
        lib.calls.set(0);
        let vec_list = lib.vector_topk_asof(&queries[r], 20, AS_OF_T);
        let mut hop_list: Vec<String> = Vec::new();
        for s in vec_list.iter().take(5) {
            hop_list.extend(lib.hops(s, 2, AS_OF_T));
        }
        let _ = rrf(&[vec_list, hop_list], k);
        lib.bump(); // fusion pass
        lib.calls.get()
    });

    let ts_end = chrono::Utc::now();
    let metrics = serde_json::json!({
        "benchmark_id": "moat_libsql",
        "timestamp_start": ts_start.to_rfc3339(),
        "timestamp_end": ts_end.to_rfc3339(),
        "duration_sec": (ts_end - ts_start).num_seconds(),
        // Echoed so a report can ASSERT this run pairs with the engine run it
        // is being compared against, instead of trusting the file name.
        "config": {
            "n": n, "dim": dim, "runs": runs, "warmup": warmup, "k": k,
            "seed": seed, "edges_per_node": edges_per, "as_of": AS_OF_T,
            "corpus": corpus_kind,
        },
        "results": {
            "pass": true,
            "competitor": "libSQL 0.9 (core, embedded local) + native DiskANN vector index",
            "process_model": "SEPARATE process from the engine run: libsql-ffi and rusqlite \
                              export the same sqlite3_* symbols and cannot be linked into one \
                              binary soundly. Same host, same seeded corpus, same protocol.",
            "asof_overfetch": ASOF_OVERFETCH,
            "asof_note": "vector_top_k cannot push a temporal predicate into the ANN index, so \
                          AS-OF shapes over-fetch k*overfetch and post-filter in SQL.",
            "ingest": { "libsql_s": ingest_s },
            "queries": report,
        },
    });
    let out_path = format!("{out}/moat_libsql_metrics.json");
    fs::write(&out_path, serde_json::to_string_pretty(&metrics).unwrap()).unwrap();
    println!("  metrics JSON written: {out_path}");

    drop(lib);
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(format!("{db_path}{suffix}"));
    }
}
