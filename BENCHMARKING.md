# Benchmarking GenesisBlockDB — Reproducibility Guide

This guide lets **anyone** clone the repo, run a fixed set of commands, and
produce machine-readable results plus raw logs that others can independently
verify. It is deliberately written for an external reproducer, not just the
maintainer.

> **Internal audit vs reproducible benchmark.** The numbers in `docs/AUDIT--*.md`
> are *internal* audits run by the maintainer on one machine. The suite described
> here is how you (or anyone) produce a **reproducible** result on your own
> hardware. They are different credibility levels — see
> [`docs/benchmarks/INDEPENDENT-BENCHMARKS.md`](docs/benchmarks/INDEPENDENT-BENCHMARKS.md).

---

## 1. What you can run

| # | Benchmark | Command | Self-contained? | Typical time |
|---|-----------|---------|-----------------|--------------|
| 1 | Short soak **smoke** | `run_smoke` | yes | ~2 min |
| 2 | **1-hour** soak | `run_soak_12h` (`SOAK_DURATION_SEC=3600`) | yes | ~1 h |
| 3 | **12-hour** soak | `run_soak_12h` | yes | ~12 h |
| 4 | Graph traversal | `run_graph_bench` | yes | 1–10 min |
| 5 | Vector search (+ real recall@k) | `run_vector_bench` | yes | 1–10 min |

The soak run also exercises **(6) reopen/load verification**, **(7) disk growth /
WAL-compaction evidence**, and **(8) memory usage** — all captured into the same
`result.json`. **(9) environment & hardware capture** runs automatically at the
start of every benchmark (`benchmark/collect_env.py`).

Every benchmark is self-contained: vectors and graphs are generated
deterministically from a fixed seed, so **no model download or external dataset
is required**. (The older `vbench-genesis` harness that replays real bge-m3
vectors still exists for head-to-head comparisons, but it is not part of this
reproducible suite because it needs a Python + model-download step.)

---

## 2. Requirements

| Need | Version / note |
|------|----------------|
| Rust toolchain | stable, **1.96+** (`rustup default stable`) |
| Python | **3.8+** (stdlib only — no `pip install` needed) |
| git | any recent version (used to stamp commit + dirty status) |
| Disk | smoke/graph/vector: a few GB free. **12h soak: 50+ GB free** on the DB drive |
| RAM | smoke/graph/vector: 2–4 GB. 12h soak: 8 GB+ recommended |

### Supported OS

- **Linux** — fully supported (primary CI target). Peak-RAM uses `/proc` VmHWM (true peak).
- **Windows 10/11** — fully supported via the `.ps1` scripts. Peak-RAM uses the process `PeakWorkingSet64`.
- **macOS** — supported via the `.sh` scripts. Peak-RAM is *sampled* (`ps rss`), so it is approximate; this is recorded in `env.json._notes`.

If a metric cannot be collected on your OS, it is written as `null` in
`result.json` and the reason is appended to `env.json._notes`. The suite never
fabricates a value.

---

## 3. Quick start (smoke test)

A new user should start here. This proves the whole pipeline works in ~2 minutes.

### Linux / macOS

```bash
git clone https://github.com/Freshair129/GenesisBlock.git
cd GenesisBlock
bash benchmark/run_smoke.sh
```

### Windows (PowerShell)

```powershell
git clone https://github.com/Freshair129/GenesisBlock.git
cd GenesisBlock
.\benchmark\run_smoke.ps1
```

You should see a run directory printed and, at the end:

```
PASS: .../result.json (soak_smoke) is a complete, clean, successful benchmark report.
```

> The verifier **rejects a dirty working tree** by default (so results map to a
> known commit). If you are iterating locally with uncommitted changes, add
> `GB_ALLOW_DIRTY=1` (bash) or `$env:GB_ALLOW_DIRTY=1` (PowerShell) — but a
> submittable result must come from a clean tree.

---

## 4. Running each benchmark

All commands assume you are in the repo root. Results are written under
`benchmark/results/<benchmark_id>/<timestamp>_<short_commit>/` (see §6).

### 4.1 Short smoke soak

```bash
# Linux/macOS
bash benchmark/run_smoke.sh
# override duration (seconds):
SOAK_DURATION_SEC=60 bash benchmark/run_smoke.sh
```
```powershell
# Windows
.\benchmark\run_smoke.ps1
$env:SOAK_DURATION_SEC=60; .\benchmark\run_smoke.ps1
```

### 4.2 1-hour soak

```bash
SOAK_DURATION_SEC=3600 bash benchmark/run_soak_12h.sh
```
```powershell
$env:SOAK_DURATION_SEC=3600; .\benchmark\run_soak_12h.ps1
```

### 4.3 12-hour soak

Run this on a machine you can leave alone for half a day. Route the database to a
fast SSD with plenty of space via `SOAK_TMPDIR`.

```bash
# Linux/macOS
SOAK_TMPDIR=/mnt/ssd/gsoak bash benchmark/run_soak_12h.sh
```
```powershell
# Windows
$env:SOAK_TMPDIR="D:\gsoak"; .\benchmark\run_soak_12h.ps1
```

Tunable knobs (all optional; defaults shown) — see `tests/soak_tests.rs`:

| Env var | Default | Meaning |
|---------|---------|---------|
| `SOAK_DURATION_SEC` | `43200` | wall-clock target |
| `SOAK_NODES_PER_CYCLE` | `500` | nodes ingested per cycle |
| `SOAK_COMPACT_EVERY` | `20` | compact (save_state) every N cycles |
| `SOAK_QUERY_K` | `10` | k for the per-cycle probe query |
| `SOAK_EF_SEARCH` | `200` | HNSW ef_search |
| `SOAK_DIM` | `16` | embedding dimension |
| `SOAK_RECALL_THRESH` | `0.10` | max recall-miss rate before `pass=false` |
| `SOAK_MAX_CYCLES` | `0` | hard cycle cap (0 = unlimited); marks `interrupted` |

### 4.4 Graph traversal benchmark

```bash
bash benchmark/run_graph_bench.sh
GB_GRAPH_N=1000000 GB_GRAPH_FANOUT=8 bash benchmark/run_graph_bench.sh
```
```powershell
.\benchmark\run_graph_bench.ps1
$env:GB_GRAPH_N=1000000; .\benchmark\run_graph_bench.ps1
```

### 4.5 Vector search benchmark (with real recall@k)

```bash
bash benchmark/run_vector_bench.sh
GB_VEC_N=200000 GB_VEC_DIM=256 GB_VEC_Q=2000 bash benchmark/run_vector_bench.sh
```
```powershell
.\benchmark\run_vector_bench.ps1
$env:GB_VEC_N=200000; $env:GB_VEC_DIM=256; .\benchmark\run_vector_bench.ps1
```

Recall is the **measured** overlap with an exact brute-force ground truth
computed in-process — it is not a hardcoded claim.

---

## 5. Where result files are written

Each run creates a directory:

```
benchmark/results/<benchmark_id>/<UTC-timestamp>_<short-commit>/
├── result.json     # machine-readable, conforms to benchmark/result_schema.json
├── raw.log         # full benchmark stdout
├── stderr.log      # benchmark stderr (if any)
├── env.json        # host environment captured at run start
├── summary.md      # human-readable summary rendered from result.json
└── metrics.json    # raw engine-emitted metrics (pre-assembly; for debugging)
```

`benchmark/results/` is git-ignored except for a `.gitkeep` — your raw runs stay
local until you choose to attach them to an issue (see §8).

---

## 6. Verifying a result

```bash
python benchmark/verify_report.py benchmark/results/soak_smoke/<dir>/result.json
```

`verify_report.py` exits `0` only when the report is **complete, from a clean
tree, and represents a successful run**. It rejects: missing commit, missing
environment metadata, dirty tree (unless `--allow-dirty`), `pass != true`,
interrupted runs, missing latency metrics, `total_nodes == 0`, missing reopen
verification (soak), and a 12h profile whose `duration_sec < 43200` that is not
explicitly marked interrupted. See
[`docs/benchmarks/INDEPENDENT-BENCHMARKS.md`](docs/benchmarks/INDEPENDENT-BENCHMARKS.md).

Run the verifier's own test suite:

```bash
python -m unittest benchmark.test_verify_report -v
```

---

## 7. CI / self-hosted runner

See [`docs/benchmarks/CI-RUNNERS.md`](docs/benchmarks/CI-RUNNERS.md) for the full
plan. In short:

- **GitHub-hosted runner**: smoke benchmark only (fast, fits the job time limit).
- **Self-hosted runner**: the 1h/12h soak — GitHub-hosted runners cap job time
  (and share noisy I/O), so a 12h soak must run on a self-hosted runner labeled
  `self-hosted, soak`, triggered manually via `workflow_dispatch`.

---

## 8. Submitting independent results

1. Run on a **clean tree** (no `--allow-dirty`) so the result maps to a commit.
2. Confirm `python benchmark/verify_report.py <dir>/result.json` prints `PASS`.
3. Open an issue using the **Independent Benchmark Result** template
   (`.github/ISSUE_TEMPLATE/independent_benchmark_result.md`).
4. Attach `result.json` and `raw.log` (and `env.json` if asked).
5. State whether you modified the repo, your PASS/FAIL, and any notes.

Reproduced results get added to the table in
[`docs/benchmarks/INDEPENDENT-BENCHMARKS.md`](docs/benchmarks/INDEPENDENT-BENCHMARKS.md).

---

## 9. Caveats (read before comparing numbers)

- **Embedded vs server.** This suite drives the in-process `Storage` core
  directly. Numbers are **not** comparable to the REST server surface
  (`/v1/*`), which adds HTTP + JSON serialization overhead. Don't compare an
  embedded p50 to another database's networked p50.
- **Post-process filters.** HQL `WHERE`/`ORDER BY`/`LIMIT` projections are applied
  *after* dispatch, not pushed into the index. A query's latency includes that
  post-filtering; account for it when comparing to engines that filter in-index.
- **OS noise.** Latency is wall-clock on a loaded OS. Background processes,
  thermal throttling, and power profiles all move the numbers. Run on an idle
  machine; expect run-to-run variance, especially at p99.
- **Disk differences.** Disk model, filesystem, and whether the DB sits on
  SSD/NVMe/HDD dominate ingest and compaction figures. The repo lives on `G:`
  (HDD) on the dev box, but soak DBs are routed to an SSD via `SOAK_TMPDIR` —
  always record where your DB lived (`env.json` does this).
- **Approximate index.** HNSW is approximate; recall depends on `ef_search` and
  dimension. The vector benchmark reports *measured* recall@k so you can see the
  recall/latency trade-off on your machine rather than trusting a single number.
- **A single run is not a statistical claim.** One `result.json` is one sample.
  Credibility comes from independent reproduction across machines — that is the
  whole point of this suite.
