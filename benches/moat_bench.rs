// G3 moat bench (WP-3.2, BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS §3, GNSE
// plan Phase 3): the engine's fused vector+graph+AS-OF jobs vs the DIY
// baseline the ROUND2 interview named primary — one SQLite file with
// brute-force f32 vector scan (the sqlite-vec-stable model), recursive-CTE
// hops, app-layer RRF fusion, and the published single-axis audit-history
// temporal pattern. Both sides run IN-PROCESS in the same Rust binary: no
// cross-runtime timing bias, and the baseline gets a faster glue layer
// (compiled Rust) than the TS/Python it would really have — every reported
// win is therefore a lower bound.
//
// Two gates, per ROUND2 G3-e:
//   (a) the bitemporal correctness scenarios (WP-3.1 matrix) run against
//       BOTH sides — pass/fail per scenario with the reason;
//   (b) latency: p50/p99 + 95% CI over ≥30 measured runs per query shape,
//       plus in-process call counts, with the §3.6 / ROUND2 STOP numbers
//       applied mechanically to produce the verdict line.
//
// Deterministic and clone-and-run: seeded corpus, no model downloads.
// Nothing here mutates engine behaviour — public Storage API only.
//
// Run (writes <out>/moat_bench_metrics.json):
//   GB_MOAT_OUT=<dir> GB_MOAT_N=100000 GB_MOAT_DIM=1024 GB_MOAT_RUNS=30 \
//   GB_MOAT_SEED=42 \
//   cargo run --release --no-default-features --features bins --bin moat-bench

use genesis_block_native::{
    BatchInput, EdgeInput, HybridSearchInput, NeighborInput, NodeInput, OpenOptions, Storage,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rusqlite::{params, Connection};
use std::fs;
use std::time::Instant;

const AS_OF_T: &str = "2023-01-01T00:00:00Z";
const RRF_K: f64 = 60.0;

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

fn gen_vec(rng: &mut StdRng, dim: usize) -> Vec<f32> {
    let mut v: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0f32..1.0f32)).collect();
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
    for x in v.iter_mut() {
        *x /= norm;
    }
    v
}

/// Deterministic valid_from: nodes/edges spread across 2020–2023 so the
/// AS-OF selector (2023-01-01) bisects the corpus.
fn valid_from_for(i: usize) -> String {
    format!("202{}-0{}-01T00:00:00Z", i % 4, 1 + (i % 9))
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

// ---------------------------------------------------------------------------
// The DIY baseline (B1): one SQLite file, best published patterns.
// ---------------------------------------------------------------------------

struct Baseline {
    conn: Connection,
    dim: usize,
    /// Statements + app-layer fusion passes actually issued per query shape,
    /// recorded honestly as they run.
    calls: std::cell::Cell<u32>,
}

impl Baseline {
    fn create(path: &str, dim: usize) -> Self {
        let _ = fs::remove_file(path);
        let conn = Connection::open(path).unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.pragma_update(None, "synchronous", "NORMAL").unwrap();
        conn.execute_batch(
            "CREATE TABLE nodes(
                 id TEXT PRIMARY KEY, v INTEGER, vec BLOB,
                 valid_from TEXT NOT NULL, valid_to TEXT);
             -- The published single-axis pattern: an audit-history side table
             -- appended on every change (bytefish.de model). One time axis.
             CREATE TABLE nodes_history(
                 id TEXT NOT NULL, v INTEGER,
                 valid_from TEXT NOT NULL, valid_to TEXT,
                 changed_at TEXT NOT NULL);
             CREATE TABLE edges(
                 id TEXT PRIMARY KEY, src TEXT NOT NULL, dst TEXT NOT NULL,
                 rel TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT);
             CREATE INDEX idx_edges_src ON edges(src);
             CREATE INDEX idx_edges_dst ON edges(dst);
             -- q6 (vector time-travel, SPEC--EPOCH-HNSW E2): the DIY emulation
             -- of the engine's epoch stamps — commit-seq columns beside the
             -- vector so a brute scan can answer 'top-k as believed at T'.
             CREATE TABLE tx_nodes(
                 id TEXT PRIMARY KEY, vec BLOB NOT NULL,
                 created_seq INTEGER NOT NULL, retired_seq INTEGER);",
        )
        .unwrap();
        Baseline {
            conn,
            dim,
            calls: std::cell::Cell::new(0),
        }
    }

    fn bump(&self) {
        self.calls.set(self.calls.get() + 1);
    }

    /// Brute-force f32 scan — the sqlite-vec-stable model (its author's own
    /// published numbers are for exactly this scan) — with the temporal
    /// window pushed into SQL. One statement + one app-side ranking pass.
    fn vector_topk(&self, q: &[f32], k: usize, as_of: Option<&str>) -> Vec<(String, f32)> {
        self.bump();
        let sql = match as_of {
            Some(_) => {
                "SELECT id, vec FROM nodes
                 WHERE vec IS NOT NULL
                   AND valid_from <= ?1 AND (valid_to IS NULL OR valid_to > ?1)"
            }
            None => "SELECT id, vec FROM nodes WHERE vec IS NOT NULL AND valid_to IS NULL",
        };
        let mut stmt = self.conn.prepare_cached(sql).unwrap();
        let mut scored: Vec<(String, f32)> = Vec::new();
        let mut walk = |row: &rusqlite::Row| -> rusqlite::Result<()> {
            let id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            let vec: Vec<f32> = blob
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            debug_assert_eq!(vec.len(), self.dim);
            scored.push((id, dot(q, &vec)));
            Ok(())
        };
        match as_of {
            Some(t) => {
                let mut rows = stmt.query(params![t]).unwrap();
                while let Some(row) = rows.next().unwrap() {
                    walk(row).unwrap();
                }
            }
            None => {
                let mut rows = stmt.query([]).unwrap();
                while let Some(row) = rows.next().unwrap() {
                    walk(row).unwrap();
                }
            }
        }
        self.bump(); // app-side rank pass
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(k);
        scored
    }

    /// q6: vector top-k as believed at commit T — brute f32 scan under the tx
    /// predicate `created_seq <= T AND (retired_seq IS NULL OR retired_seq > T)`,
    /// the natural SQLite emulation of the engine's epoch stamps.
    fn vector_topk_at_tx(&self, q: &[f32], k: usize, t: i64) -> Vec<(String, f32)> {
        self.bump();
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, vec FROM tx_nodes
                 WHERE created_seq <= ?1 AND (retired_seq IS NULL OR retired_seq > ?1)",
            )
            .unwrap();
        let mut scored: Vec<(String, f32)> = Vec::new();
        let mut rows = stmt.query(params![t]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let id: String = row.get(0).unwrap();
            let blob: Vec<u8> = row.get(1).unwrap();
            let vec: Vec<f32> = blob
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            scored.push((id, dot(q, &vec)));
        }
        self.bump(); // app-side rank pass
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(k);
        scored
    }

    /// Recursive-CTE n-hop reach with the temporal window on every edge.
    fn hops(&self, seed: &str, depth: u32, as_of: Option<&str>) -> Vec<String> {
        self.bump();
        let t = as_of.unwrap_or("9999-01-01T00:00:00Z");
        let mut stmt = self
            .conn
            .prepare_cached(
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
            )
            .unwrap();
        stmt.query_map(params![seed, t, depth], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// RRF fusion (identical app-layer code path for both sides — the glue is
// deliberately shared so only the store latency differs).
// ---------------------------------------------------------------------------

fn rrf(lists: &[Vec<String>], k: usize) -> Vec<String> {
    let mut score: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for list in lists {
        for (rank, id) in list.iter().enumerate() {
            *score.entry(id.clone()).or_default() += 1.0 / (RRF_K + rank as f64 + 1.0);
        }
    }
    let mut ranked: Vec<(String, f64)> = score.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
    ranked.into_iter().take(k).map(|(id, _)| id).collect()
}

// ---------------------------------------------------------------------------
// Correctness gate — the WP-3.1 scenario set run against a store interface.
// ---------------------------------------------------------------------------

struct ScenarioResult {
    name: &'static str,
    engine: bool,
    sqlite: bool,
    note: &'static str,
}

/// Runs the bitemporal scenarios against BOTH stores using small dedicated
/// entities (ids prefixed `cx_`) so bench data stays untouched.
fn correctness_gate(s: &Storage, b: &Baseline) -> Vec<ScenarioResult> {
    let mut out = Vec::new();

    // Setup, engine side: cx_doc v1 (2020) superseded to v2; hub -> doc.
    for (id, v) in [("cx_hub", 0i64), ("cx_doc", 1)] {
        s.add_node(NodeInput {
            id: Some(id.to_string()),
            labels: vec!["CX".to_string()],
            props: Some(serde_json::json!({ "v": v })),
            embedding: None,
            lang: None,
            valid_from: Some("2020-01-01T00:00:00Z".to_string()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    }
    s.add_edge(EdgeInput {
        id: Some("cx_e1".to_string()),
        from: "cx_hub".to_string(),
        to: "cx_doc".to_string(),
        rel: "references".to_string(),
        props: None,
        valid_from: Some("2020-01-01T00:00:00Z".to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
    let s1 = s.stable_frontier();
    s.supersede_node(
        "cx_doc".to_string(),
        Some(serde_json::json!({ "v": 2 })),
        None,
    )
    .unwrap();

    // Setup, baseline side: the audit-history pattern records the change.
    b.conn
        .execute(
            "INSERT INTO nodes(id, v, vec, valid_from, valid_to) VALUES
             ('cx_hub', 0, NULL, '2020-01-01T00:00:00Z', NULL),
             ('cx_doc', 2, NULL, '2020-01-01T00:00:00Z', NULL)",
            [],
        )
        .unwrap();
    b.conn
        .execute(
            "INSERT INTO nodes_history(id, v, valid_from, valid_to, changed_at)
             VALUES ('cx_doc', 1, '2020-01-01T00:00:00Z', ?1, ?1)",
            params![AS_OF_T],
        )
        .unwrap();
    b.conn
        .execute(
            "INSERT INTO edges(id, src, dst, rel, valid_from, valid_to) VALUES
             ('cx_e1', 'cx_hub', 'cx_doc', 'references', '2020-01-01T00:00:00Z', NULL)",
            [],
        )
        .unwrap();

    // S1: valid-time point query on the superseded node → must serve v1.
    {
        let eng = s
            .execute_query_ir_json(serde_json::json!({
                "contract_version": "query-ir.v1", "request_id": "cx1",
                "temporal": { "valid_at": "2021-06-01T00:00:00Z" },
                "operation": { "kind": "traverse", "seed_id": "cx_hub",
                               "depth": 1, "relations": ["references"], "direction": "out" }
            }))
            .unwrap();
        let engine_ok = eng["data"][0]["node"]["props"]["v"] == 1;
        // Baseline CAN answer this from the history table (single axis).
        let v: i64 = b
            .conn
            .query_row(
                "SELECT v FROM nodes_history
                 WHERE id = 'cx_doc' AND valid_from <= ?1
                   AND (valid_to IS NULL OR valid_to > ?1)",
                params!["2021-06-01T00:00:00Z"],
                |r| r.get(0),
            )
            .unwrap_or(-1);
        out.push(ScenarioResult {
            name: "valid_time_point_on_superseded",
            engine: engine_ok,
            sqlite: v == 1,
            note: "single-axis history table can answer this one",
        });
    }

    // S2: two-axis — "what did we believe at commit S1 about mid-2021?"
    {
        let eng = s
            .execute_query_ir_json(serde_json::json!({
                "contract_version": "query-ir.v1", "request_id": "cx2",
                "temporal": { "valid_at": "2021-06-01T00:00:00Z", "tx_as_of": s1 },
                "operation": { "kind": "traverse", "seed_id": "cx_hub",
                               "depth": 1, "relations": ["references"], "direction": "out" }
            }))
            .unwrap();
        let row = &eng["data"][0]["node"];
        let engine_ok = row["props"]["v"] == 1 && row["valid_to"].is_null();
        // The audit-history pattern has ONE time column pair; there is no
        // transaction-time selector to bind — the query is inexpressible
        // against this schema (bytefish.de documents the underlying defect:
        // no stable transaction time across triggers).
        out.push(ScenarioResult {
            name: "two_axis_belief_at_commit",
            engine: engine_ok,
            sqlite: false,
            note: "no tx axis in the published audit-history pattern",
        });
    }

    // S3: retroactive correction — edge turns out to have ended 2021-06-01;
    // the SAME valid-time question (2022) must flip its answer.
    {
        s.retract_edge(
            "cx_e1".to_string(),
            Some("2021-06-01T00:00:00Z".to_string()),
        )
        .unwrap();
        let eng = s
            .execute_query_ir_json(serde_json::json!({
                "contract_version": "query-ir.v1", "request_id": "cx3",
                "temporal": { "valid_at": "2022-01-01T00:00:00Z" },
                "operation": { "kind": "traverse", "seed_id": "cx_hub",
                               "depth": 1, "relations": ["references"], "direction": "out" }
            }))
            .unwrap();
        let engine_ok = eng["data"].as_array().unwrap().is_empty();
        // Baseline: an UPDATE rewrites the row in place — the correction
        // succeeds, but the pre-correction belief is destroyed (no way to
        // ask "what did we answer last week"), which is scenario S2 again.
        b.conn
            .execute(
                "UPDATE edges SET valid_to = '2021-06-01T00:00:00Z' WHERE id = 'cx_e1'",
                [],
            )
            .unwrap();
        let n: i64 = b
            .conn
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE src = 'cx_hub'
                   AND valid_from <= '2022-01-01T00:00:00Z'
                   AND (valid_to IS NULL OR valid_to > '2022-01-01T00:00:00Z')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        out.push(ScenarioResult {
            name: "retroactive_correction_flips_answer",
            engine: engine_ok,
            sqlite: n == 0,
            note: "both flip; baseline destroys the pre-correction belief doing so",
        });
    }

    // S4: interval boundaries — start inclusive, end exclusive.
    {
        let vis = |t: &str| -> bool {
            let eng = s
                .execute_query_ir_json(serde_json::json!({
                    "contract_version": "query-ir.v1", "request_id": "cx4",
                    "temporal": { "valid_at": t },
                    "operation": { "kind": "traverse", "seed_id": "cx_hub",
                                   "depth": 1, "relations": ["references"], "direction": "out" }
                }))
                .unwrap();
            !eng["data"].as_array().unwrap().is_empty()
        };
        let engine_ok = vis("2020-01-01T00:00:00Z")
            && vis("2021-05-31T23:59:59Z")
            && !vis("2021-06-01T00:00:00Z")
            && !vis("2019-12-31T23:59:59Z");
        let q = |t: &str| -> bool {
            let n: i64 = b
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM edges WHERE src = 'cx_hub'
                       AND valid_from <= ?1 AND (valid_to IS NULL OR valid_to > ?1)",
                    params![t],
                    |r| r.get(0),
                )
                .unwrap();
            n > 0
        };
        let sqlite_ok = q("2020-01-01T00:00:00Z")
            && q("2021-05-31T23:59:59Z")
            && !q("2021-06-01T00:00:00Z")
            && !q("2019-12-31T23:59:59Z");
        out.push(ScenarioResult {
            name: "interval_boundaries",
            engine: engine_ok,
            sqlite: sqlite_ok,
            note: "plain WHERE handles boundaries once the window survives (see S3)",
        });
    }

    // S5: audit reconstruction — full version trail with provenance links.
    {
        let chain = s.node_versions("cx_doc", None).unwrap();
        let versions = chain["versions"].as_array().unwrap().clone();
        let engine_ok = versions.len() >= 3
            && versions
                .iter()
                .any(|r| r["caused_by"].as_str().is_some_and(|c| c.contains('@')));
        // Baseline history rows exist but carry no provenance identity: the
        // "what caused this version" link (WP-2.3 caused_by chain) has no
        // schema slot in the published pattern.
        let hist: i64 = b
            .conn
            .query_row(
                "SELECT COUNT(*) FROM nodes_history WHERE id = 'cx_doc'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let _ = hist;
        out.push(ScenarioResult {
            name: "audit_chain_with_provenance",
            engine: engine_ok,
            sqlite: false,
            note: "history rows exist but no caused_by/provenance identity",
        });
    }

    out
}

// ---------------------------------------------------------------------------

fn main() {
    let out = std::env::var("GB_MOAT_OUT").unwrap_or_else(|_| ".".to_string());
    let n = env_usize("GB_MOAT_N", 100_000);
    let dim = env_usize("GB_MOAT_DIM", 1024);
    let runs = env_usize("GB_MOAT_RUNS", 30);
    let warmup = env_usize("GB_MOAT_WARMUP", 3);
    let k = env_usize("GB_MOAT_K", 10);
    let seed = env_usize("GB_MOAT_SEED", 42) as u64;
    let edges_per = env_usize("GB_MOAT_EDGES_PER_NODE", 5);

    println!("moat-bench: N={n} dim={dim} runs={runs} k={k} seed={seed} edges/node={edges_per}");
    let ts_start = chrono::Utc::now();

    // --- corpus ---
    // Default: deterministic synthetic unit vectors. With GB_MOAT_VECTORS set
    // (WP-3.3 follow-up 2) the vectors are REAL embeddings loaded from a flat
    // little-endian f32 file — see benchmark/gen_corpus_bge_m3.py, which also
    // writes a manifest recording the model, dim, count and sha256. Everything
    // else (queries, seeds, edges, protocol) is identical, so synthetic-vs-real
    // at matched N is a controlled A/B on the one variable the moat verdict
    // caveated: vector DISTRIBUTION (random unit vectors are isotropic; real
    // embeddings are anisotropic and clustered, which changes ANN graph quality
    // but not a full scan's cost).
    let mut rng = StdRng::seed_from_u64(seed);
    let corpus_path = std::env::var("GB_MOAT_VECTORS").ok();
    let (vectors, n, dim, corpus_kind) = match corpus_path.as_deref() {
        Some(p) => {
            let raw = fs::read(p).unwrap_or_else(|e| panic!("GB_MOAT_VECTORS {p}: {e}"));
            let per = dim * 4;
            assert!(
                raw.len() >= per,
                "GB_MOAT_VECTORS {p}: {} bytes is less than one {dim}-dim vector",
                raw.len()
            );
            assert_eq!(
                raw.len() % per,
                0,
                "GB_MOAT_VECTORS {p}: {} bytes is not a multiple of dim {dim} * 4 — \
                 pass GB_MOAT_DIM matching the corpus manifest",
                raw.len()
            );
            let available = raw.len() / per;
            let take = n.min(available);
            let v: Vec<Vec<f32>> = (0..take)
                .map(|i| {
                    raw[i * per..(i + 1) * per]
                        .chunks_exact(4)
                        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect()
                })
                .collect();
            println!("  corpus: REAL embeddings from {p} ({take} of {available} available)");
            (v, take, dim, format!("real:{p}"))
        }
        None => {
            let v: Vec<Vec<f32>> = (0..n).map(|_| gen_vec(&mut rng, dim)).collect();
            (
                v,
                n,
                dim,
                "synthetic seeded unit vectors (NOT bge-m3; latency-comparable, recall-inert)"
                    .to_string(),
            )
        }
    };
    // Edges: mildly preferential targets (rng^2 skew), ~10% closed windows.
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
    // Query pool: perturbed corpus rows (a true neighbour exists in-set).
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
    let seeds: Vec<String> = (0..runs + warmup)
        .map(|_| format!("n{}", rng.gen_range(0..n)))
        .collect();

    // --- ingest: engine ---
    let db_dir = format!("{out}/moat_engine_db");
    let _ = fs::remove_dir_all(&db_dir);
    let storage = Storage::open(OpenOptions {
        path: db_dir.clone(),
        page_cache_mb: Some(256),
        read_only: Some(false),
        vector_dim: Some(dim as u32),
        retention: Some("full".to_string()),
    })
    .unwrap();
    let t0 = Instant::now();
    for chunk_start in (0..n).step_by(5000) {
        let end = (chunk_start + 5000).min(n);
        let nodes: Vec<NodeInput> = (chunk_start..end)
            .map(|i| NodeInput {
                id: Some(format!("n{i}")),
                labels: vec!["THING".to_string()],
                props: Some(serde_json::json!({ "v": i })),
                embedding: Some(vectors[i].iter().map(|x| *x as f64).collect()),
                lang: Some("en".to_string()),
                valid_from: Some(valid_from_for(i)),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .collect();
        storage
            .execute_batch(BatchInput {
                nodes,
                edges: Vec::new(),
            })
            .unwrap();
    }
    for chunk in edge_list.chunks(10000) {
        let edges: Vec<EdgeInput> = chunk
            .iter()
            .enumerate()
            .map(|(j, (from, to, _closed))| EdgeInput {
                id: Some(format!("e{from}_{to}_{j}")),
                from: format!("n{from}"),
                to: format!("n{to}"),
                rel: "references".to_string(),
                props: None,
                valid_from: Some(valid_from_for(from + to)),
                supersede: None,
                impact: None,
                caused_by: None,
            })
            .collect();
        storage
            .execute_batch(BatchInput {
                nodes: Vec::new(),
                edges,
            })
            .unwrap();
    }
    // Closed windows via retroactive retract (bitemporal soft-delete).
    let mut retracted = 0usize;
    for (j, (from, to, closed)) in edge_list.iter().enumerate() {
        if let Some(t) = closed {
            storage
                .retract_edge(format!("e{from}_{to}_{j}"), Some(t.clone()))
                .unwrap();
            retracted += 1;
        }
    }
    storage.flush_index();
    let engine_ingest_s = t0.elapsed().as_secs_f64();
    println!(
        "  engine ingest: {engine_ingest_s:.1}s ({n} nodes, {} edges, {retracted} retro-closed)",
        edge_list.len()
    );

    // --- ingest: baseline ---
    let sqlite_path = format!("{out}/moat_baseline.sqlite");
    let baseline = Baseline::create(&sqlite_path, dim);
    let t0 = Instant::now();
    {
        let tx = baseline.conn.unchecked_transaction().unwrap();
        {
            let mut ins = tx
                .prepare("INSERT INTO nodes(id, v, vec, valid_from, valid_to) VALUES (?1, ?2, ?3, ?4, NULL)")
                .unwrap();
            for (i, v) in vectors.iter().enumerate() {
                let blob: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
                ins.execute(params![format!("n{i}"), i as i64, blob, valid_from_for(i)])
                    .unwrap();
            }
            let mut inse = tx
                .prepare("INSERT INTO edges(id, src, dst, rel, valid_from, valid_to) VALUES (?1, ?2, ?3, 'references', ?4, ?5)")
                .unwrap();
            for (j, (from, to, closed)) in edge_list.iter().enumerate() {
                inse.execute(params![
                    format!("e{from}_{to}_{j}"),
                    format!("n{from}"),
                    format!("n{to}"),
                    valid_from_for(from + to),
                    closed.as_deref(),
                ])
                .unwrap();
            }
        }
        tx.commit().unwrap();
    }
    let sqlite_ingest_s = t0.elapsed().as_secs_f64();
    println!("  sqlite ingest: {sqlite_ingest_s:.1}s");

    // --- q6 tx cohort (vector time-travel, SPEC--EPOCH-HNSW E2) ---
    // Its own engine collection + baseline table so the main corpus (q1–q5)
    // is untouched: 1000 nodes exist at tx_mark, half are retracted after it.
    // Both sides then answer "vector top-k as believed at tx_mark".
    let txn = 1000.min(n);
    storage
        .create_collection(
            "txbench".to_string(),
            "synthetic".to_string(),
            dim as u32,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    for chunk_start in (0..txn).step_by(500) {
        let end = (chunk_start + 500).min(txn);
        let nodes: Vec<NodeInput> = (chunk_start..end)
            .map(|i| NodeInput {
                id: Some(format!("tx{i}")),
                labels: vec!["THING".to_string()],
                props: Some(serde_json::json!({ "v": i })),
                embedding: Some(vectors[i].iter().map(|x| *x as f64).collect()),
                lang: Some("en".to_string()),
                valid_from: Some(valid_from_for(i)),
                caused_by: None,
                ttl: None,
                collection: Some("txbench".to_string()),
            })
            .collect();
        storage
            .execute_batch(BatchInput {
                nodes,
                edges: Vec::new(),
            })
            .unwrap();
    }
    let tx_mark = storage.stable_frontier();
    for i in (1..txn).step_by(2) {
        storage.retract_node(&format!("tx{i}")).unwrap();
    }
    storage.flush_index();
    {
        let tx = baseline.conn.unchecked_transaction().unwrap();
        {
            let mut ins = tx
                .prepare(
                    "INSERT INTO tx_nodes(id, vec, created_seq, retired_seq)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .unwrap();
            for (i, v) in vectors.iter().take(txn).enumerate() {
                let blob: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
                let retired: Option<i64> = (i % 2 == 1).then_some(tx_mark as i64 + 1 + i as i64);
                ins.execute(params![format!("tx{i}"), blob, (i + 1) as i64, retired])
                    .unwrap();
            }
        }
        tx.commit().unwrap();
    }
    println!(
        "  q6 tx cohort: {txn} nodes, {} retracted after tx_mark={tx_mark}",
        txn / 2
    );

    // --- correctness gate ---
    let scenarios = correctness_gate(&storage, &baseline);
    for sc in &scenarios {
        println!(
            "  correctness {:<38} engine={} sqlite={} ({})",
            sc.name, sc.engine, sc.sqlite, sc.note
        );
    }

    // --- query shapes ---
    // Per-shape closures return (result_len, in_process_calls) so calls are
    // recorded from what actually ran, not asserted.
    let engine_q1 = |q: &[f32]| -> (usize, u32) {
        let mut calls = 0u32;
        let hits = storage
            .hybrid_search(HybridSearchInput {
                query_vector: q.iter().map(|x| *x as f64).collect(),
                k: 20,
                alpha: Some(0.0),
                lang: None,
                as_of: Some(AS_OF_T.to_string()),
                collection: None,
                ef_search: None,
                oversample: None,
            })
            .unwrap();
        calls += 1;
        let vec_list: Vec<String> = hits.iter().map(|h| h.node.id.clone()).collect();
        let mut hop_list: Vec<String> = Vec::new();
        for seed in vec_list.iter().take(5) {
            let nb = storage
                .neighbors(
                    seed.clone(),
                    NeighborInput {
                        depth: Some(2),
                        rel: Some("references".to_string()),
                        rels: None,
                        direction: Some("out".to_string()),
                        as_of: Some(AS_OF_T.to_string()),
                        include_invalid: None,
                        limit: Some(200),
                    },
                    false,
                )
                .unwrap();
            calls += 1;
            hop_list.extend(nb.into_iter().map(|x| x.node.id));
        }
        let fused = rrf(&[vec_list, hop_list], k);
        calls += 1; // fusion pass
        (fused.len(), calls)
    };
    let sqlite_q1 = |q: &[f32]| -> (usize, u32) {
        baseline.calls.set(0);
        let top = baseline.vector_topk(q, 20, Some(AS_OF_T));
        let vec_list: Vec<String> = top.iter().map(|(id, _)| id.clone()).collect();
        let mut hop_list: Vec<String> = Vec::new();
        for seed in vec_list.iter().take(5) {
            hop_list.extend(baseline.hops(seed, 2, Some(AS_OF_T)));
        }
        let fused = rrf(&[vec_list, hop_list], k);
        baseline.bump(); // fusion pass
        (fused.len(), baseline.calls.get())
    };

    let engine_q3 = |q: &[f32], seed: &str| -> (usize, u32) {
        let nb = storage
            .neighbors(
                seed.to_string(),
                NeighborInput {
                    depth: Some(3),
                    rel: Some("references".to_string()),
                    rels: None,
                    direction: Some("out".to_string()),
                    as_of: Some(AS_OF_T.to_string()),
                    include_invalid: None,
                    limit: Some(500),
                },
                false,
            )
            .unwrap();
        let hits = storage
            .hybrid_search(HybridSearchInput {
                query_vector: q.iter().map(|x| *x as f64).collect(),
                k: 10,
                alpha: Some(0.0),
                lang: None,
                as_of: Some(AS_OF_T.to_string()),
                collection: None,
                ef_search: None,
                oversample: None,
            })
            .unwrap();
        let fused = rrf(
            &[
                nb.into_iter().map(|x| x.node.id).collect(),
                hits.into_iter().map(|h| h.node.id).collect(),
            ],
            k,
        );
        (fused.len(), 3)
    };
    let sqlite_q3 = |q: &[f32], seed: &str| -> (usize, u32) {
        baseline.calls.set(0);
        let hop_list = baseline.hops(seed, 3, Some(AS_OF_T));
        let top = baseline.vector_topk(q, 10, Some(AS_OF_T));
        let fused = rrf(&[hop_list, top.into_iter().map(|(id, _)| id).collect()], k);
        baseline.bump();
        (fused.len(), baseline.calls.get())
    };

    let engine_q4 = |q: &[f32]| -> (usize, u32) {
        let hits = storage
            .hybrid_search(HybridSearchInput {
                query_vector: q.iter().map(|x| *x as f64).collect(),
                k: k as u32,
                alpha: Some(0.0),
                lang: None,
                as_of: None,
                collection: None,
                ef_search: None,
                oversample: None,
            })
            .unwrap();
        (hits.len(), 1)
    };
    let sqlite_q4 = |q: &[f32]| -> (usize, u32) {
        baseline.calls.set(0);
        let top = baseline.vector_topk(q, k, None);
        (top.len(), baseline.calls.get())
    };

    // q6 (E2): vector top-k as believed at commit tx_mark — the engine's
    // epoch-stamped tx path vs the baseline's stamped brute scan. A capability
    // row (excluded from min_cross): the baseline emulates the stamps but the
    // correctness gate already shows it lacks the two-axis semantics around them.
    let engine_q6 = |q: &[f32]| -> (usize, u32) {
        let resp = storage
            .execute_query_ir_json(serde_json::json!({
                "contract_version": "query-ir.v1",
                "request_id": "moat-q6",
                "operation": {
                    "kind": "search", "mode": "vector",
                    "query_vector": q.iter().map(|x| *x as f64).collect::<Vec<f64>>(),
                    "collection": "txbench", "k": k
                },
                "temporal": { "tx_as_of": tx_mark }
            }))
            .unwrap();
        (resp["data"].as_array().map(|a| a.len()).unwrap_or(0), 1)
    };
    let sqlite_q6 = |q: &[f32]| -> (usize, u32) {
        baseline.calls.set(0);
        let top = baseline.vector_topk_at_tx(q, k, tx_mark as i64);
        (top.len(), baseline.calls.get())
    };

    let engine_q5 = |seed: &str| -> (usize, u32) {
        let nb = storage
            .neighbors(
                seed.to_string(),
                NeighborInput {
                    depth: Some(3),
                    rel: Some("references".to_string()),
                    rels: None,
                    direction: Some("out".to_string()),
                    as_of: None,
                    include_invalid: None,
                    limit: Some(1000),
                },
                false,
            )
            .unwrap();
        (nb.len(), 1)
    };
    let sqlite_q5 = |seed: &str| -> (usize, u32) {
        baseline.calls.set(0);
        let r = baseline.hops(seed, 3, None);
        (r.len(), baseline.calls.get())
    };

    // --- measurement loop ---
    let mut report = serde_json::Map::new();
    let mut ratios: Vec<(String, f64)> = Vec::new();
    macro_rules! bench_pair {
        ($name:expr, $eng:expr, $sql:expr) => {{
            let mut eng_us: Vec<f64> = Vec::with_capacity(runs);
            let mut sql_us: Vec<f64> = Vec::with_capacity(runs);
            let mut eng_calls = 0u32;
            let mut sql_calls = 0u32;
            for r in 0..(runs + warmup) {
                let t = Instant::now();
                let (_, c) = $eng(r);
                let e = t.elapsed().as_secs_f64() * 1e6;
                let t = Instant::now();
                let (_, c2) = $sql(r);
                let s = t.elapsed().as_secs_f64() * 1e6;
                if r >= warmup {
                    eng_us.push(e);
                    sql_us.push(s);
                    eng_calls = c;
                    sql_calls = c2;
                }
            }
            let e_stats = stats(eng_us);
            let s_stats = stats(sql_us);
            let ratio = s_stats["p50_us"].as_f64().unwrap()
                / e_stats["p50_us"].as_f64().unwrap().max(1e-9);
            println!(
                "  {:<4} engine p50 {:>10.0}us  sqlite p50 {:>10.0}us  ratio {:>6.2}x  calls {}/{}",
                $name,
                e_stats["p50_us"].as_f64().unwrap(),
                s_stats["p50_us"].as_f64().unwrap(),
                ratio,
                eng_calls,
                sql_calls
            );
            ratios.push(($name.to_string(), ratio));
            report.insert(
                $name.to_string(),
                serde_json::json!({
                    "engine": e_stats, "sqlite": s_stats,
                    "engine_calls": eng_calls, "sqlite_calls": sql_calls,
                    "sqlite_over_engine_p50": ratio,
                }),
            );
        }};
    }

    // Engine vs libSQL/DiskANN. Recorded under its own keys so the primary
    // verdict (engine vs the ROUND2-named single-SQLite-file baseline) is not
    // silently redefined by a second competitor.
    bench_pair!("q1", |r: usize| engine_q1(&queries[r]), |r: usize| {
        sqlite_q1(&queries[r])
    });
    bench_pair!(
        "q3",
        |r: usize| engine_q3(&queries[r], &seeds[r]),
        |r: usize| sqlite_q3(&queries[r], &seeds[r])
    );
    bench_pair!("q4", |r: usize| engine_q4(&queries[r]), |r: usize| {
        sqlite_q4(&queries[r])
    });
    bench_pair!("q5", |r: usize| engine_q5(&seeds[r]), |r: usize| sqlite_q5(
        &seeds[r]
    ));
    bench_pair!("q6", |r: usize| engine_q6(&queries[r]), |r: usize| {
        sqlite_q6(&queries[r])
    });

    // --- verdict (mechanical application of the STOP numbers) ---
    // §3.6 uses round-trips for the service-composed baseline; embedded vs
    // embedded (ROUND2 concession) the honest axes are the correctness gate
    // + the fused-query latency ratio (ROUND2 G3-e bar: >= 5x at 100k).
    let cross: Vec<f64> = ratios
        .iter()
        .filter(|(n, _)| n == "q1" || n == "q3")
        .map(|(_, r)| *r)
        .collect();
    let min_cross = cross.iter().cloned().fold(f64::INFINITY, f64::min);
    let sqlite_fail_count = scenarios.iter().filter(|s| !s.sqlite).count();
    let engine_all_pass = scenarios.iter().all(|s| s.engine);
    let verdict = if !engine_all_pass {
        "INVALID: engine failed its own correctness gate"
    } else if min_cross >= 5.0 {
        "PROCEED: >=5x on every cross-dimension query (ROUND2 G3-e) + correctness gap"
    } else if min_cross >= 1.3 {
        "MARGINAL: latency between 1.3x and 5x - moat rests on the correctness gap; re-run at 1M"
    } else {
        "DEAD-ON-LATENCY: <1.3x - only the correctness gap remains as differentiator"
    };
    println!("  verdict: {verdict} (min cross-dim ratio {min_cross:.2}x, sqlite fails {sqlite_fail_count}/{} scenarios)", scenarios.len());

    let ingest_json = serde_json::json!({
        "engine_s": engine_ingest_s, "sqlite_s": sqlite_ingest_s,
    });
    // WP-3.3 follow-up 1 does NOT run in this process. libsql-ffi and
    // rusqlite both define the `sqlite3_*` symbols, so a binary linking both
    // either fails to link (LNK2005) or — worse — silently resolves every
    // call to ONE of the two implementations, which would put the engine's own
    // projection database and the competitor on the same accidental SQLite and
    // invalidate both sides. The libSQL/DiskANN rows are therefore produced by
    // the separate `moat-libsql` binary from the SAME seeded corpus and the
    // same measurement protocol; `benchmark/run_moat_bench.sh` runs both and
    // the report pairs them.
    let libsql_json = serde_json::json!({
        "status": "measured out-of-process by the `moat-libsql` binary",
        "reason": "libsql-ffi and rusqlite export the same sqlite3_* symbols; \
                   linking both into one binary is unsound (see moat_libsql.rs)",
        "metrics_file": "moat_libsql_metrics.json",
    });

    let ts_end = chrono::Utc::now();
    let metrics = serde_json::json!({
        "benchmark_id": "moat",
        "timestamp_start": ts_start.to_rfc3339(),
        "timestamp_end": ts_end.to_rfc3339(),
        "duration_sec": (ts_end - ts_start).num_seconds(),
        "config": {
            "n": n, "dim": dim, "runs": runs, "warmup": warmup, "k": k,
            "seed": seed, "edges_per_node": edges_per, "as_of": AS_OF_T,
            "corpus": corpus_kind,
            "corpus_manifest": corpus_path.as_ref().and_then(|p| {
                let m = std::path::Path::new(p).with_extension("manifest.json");
                fs::read_to_string(m).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            }),
            "baseline": "single SQLite file: brute f32 scan (sqlite-vec-stable model) + recursive CTE + shared Rust RRF glue + audit-history temporal pattern",
            "process_model": "both stores in-process in one Rust binary (baseline glue faster than its real TS/Python - reported wins are lower bounds)",
            "q2_status": "skipped: engine lexical/FTS axis (S3) not shipped - hybrid vec+lex shape not comparable yet",
            "q6_status": "vector time-travel row (SPEC--EPOCH-HNSW E2): 1000-node tx cohort in its own collection/table, half retracted after tx_mark; capability row, excluded from min_cross",
        },
        "results": {
            // The run is trustworthy iff the engine passed its own
            // correctness gate; the verdict itself is a judgement, not a
            // pass/fail of the harness.
            "pass": engine_all_pass,
            // Envelope summary fields (verify_report.py common checks): the
            // flagship fused query (q1), engine side, in ms.
            "total_nodes": n + 2,
            "query_latency_p50_ms": report["q1"]["engine"]["p50_us"].as_f64().unwrap_or(0.0) / 1000.0,
            "query_latency_p95_ms": report["q1"]["engine"]["p95_us"].as_f64().unwrap_or(0.0) / 1000.0,
            "query_latency_p99_ms": report["q1"]["engine"]["p99_us"].as_f64().unwrap_or(0.0) / 1000.0,
            "ingest": ingest_json,
            "libsql_baseline": libsql_json,
            "queries": report,
            "correctness": scenarios.iter().map(|s| serde_json::json!({
                "scenario": s.name, "engine": s.engine, "sqlite": s.sqlite, "note": s.note,
            })).collect::<Vec<_>>(),
            "verdict": {
                "line": verdict,
                "min_cross_dim_ratio_p50": min_cross,
                "sqlite_correctness_failures": sqlite_fail_count,
                "scenario_count": scenarios.len(),
                "stop_numbers": {
                    "spec_3_6": "kill if <20% p50 saving AND <2x round-trip cut; proceed if >=2x round-trips AND >=30% p50",
                    "round2_g3e": "embedded-vs-embedded: correctness suite must-pass + >=5x fused p50 at 100k",
                },
            },
        },
    });
    let out_path = format!("{out}/moat_bench_metrics.json");
    fs::write(&out_path, serde_json::to_string_pretty(&metrics).unwrap()).unwrap();
    println!("  metrics JSON written: {out_path}");

    // Bench artifacts are large; leave the DBs for inspection only if asked.
    let _ = fs::remove_dir_all(&db_dir);
    let _ = fs::remove_file(&sqlite_path);
}
