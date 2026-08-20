# `benchmark/` — Independent Benchmark Suite

Reproducible, schema-verified benchmarks anyone can run on their own hardware.
**Start with [`../BENCHMARKING.md`](../BENCHMARKING.md).**

## Contents

| File | Purpose |
|------|---------|
| `run_smoke.{sh,ps1}` | Short soak smoke test (~2 min) — proves the pipeline |
| `run_soak_12h.{sh,ps1}` | 1h / 12h heavy soak (duration-bounded) |
| `run_graph_bench.{sh,ps1}` | Graph traversal latency + throughput |
| `run_vector_bench.{sh,ps1}` | Vector k-NN latency + **measured** recall@k |
| `run_moat_bench.{sh,ps1}` | G3 moat bench — fused vector+graph+AS-OF vs the DIY single-file assembly (optionally vs libSQL/DiskANN, optionally on a real embedding corpus) |
| `gen_corpus_bge_m3.py` | Build a **real** embedding corpus for the moat bench from this repo's own prose via a local Ollama `bge-m3` (resumable; writes a provenance manifest) |
| `collect_env.py` | Capture OS/CPU/RAM/disk/rustc into `env.json` (stdlib only) |
| `assemble_result.py` | Merge engine metrics + env + git into schema `result.json`; render `summary.md` |
| `verify_report.py` | Validate a `result.json` (the trust gate) |
| `result_schema.json` | JSON Schema (draft-07) for `result.json` |
| `report_template.md` | Template for the rendered `summary.md` |
| `test_verify_report.py` | `unittest` suite for the verifier + schema |
| `fixtures/` | Synthetic sample reports for the tests (clearly marked, not real results) |
| `results/` | Run outputs land here (git-ignored) |
| `_lib.sh`, `_lib.ps1` | Shared helpers for the runner scripts |

## One-liners

```bash
# smoke (Linux/macOS)
bash benchmark/run_smoke.sh
# verifier self-test
python -m unittest benchmark.test_verify_report -v
# verify a produced report
python benchmark/verify_report.py benchmark/results/soak_smoke/<dir>/result.json
```

## Moat bench: the two optional modes

Both are **off by default** so the clone-and-run path stays self-contained
(no downloads, no extra compile). They exist because the WP-3.2 moat verdict
shipped with two named caveats, and
[`DECISION--WP33-GNSE-BACKLOG.md`](../docs/DECISION--WP33-GNSE-BACKLOG.md)
scheduled closing both before the moat claim is used publicly.

```bash
# 1. libSQL/DiskANN competitor rows (adds ~2 min of compile for the
#    `libsql-baseline` feature). Attacks the vector-only control.
GB_MOAT_LIBSQL=1 bash benchmark/run_moat_bench.sh

# 2. Real embeddings instead of synthetic unit vectors. Build the corpus once
#    (needs a local Ollama serving bge-m3), then point the bench at it.
python benchmark/gen_corpus_bge_m3.py --out benchmark/fixtures/corpus_bge_m3
GB_MOAT_VECTORS=benchmark/fixtures/corpus_bge_m3.f32 \
  GB_MOAT_DIM=1024 GB_MOAT_N=11000 bash benchmark/run_moat_bench.sh
```

The corpus generator is resumable: embeddings are appended row-by-row, so an
interrupted run continues where it stopped instead of discarding the work. Its
manifest records model, dim, count, sha256 and the source commit, and the bench
copies that manifest into the run's `result.json` — a real-corpus number is
never reported without its provenance.

Run real-vs-synthetic **at the same N**: that is a controlled A/B on the one
variable the caveat names (vector distribution), rather than a comparison that
also moves corpus size.

## Design notes

- **The engine never self-reports its environment.** The Rust harnesses emit only
  what they can *observe* (latency, recall, disk, reopen timing) to a partial
  `metrics.json`. `assemble_result.py` adds the commit, dirty-tree status, version,
  host environment, and externally-measured peak RAM. This keeps the hardware
  claims independent of the benchmark code.
- **No fabricated numbers.** Recall is measured against an exact brute-force ground
  truth; latency/disk/RAM are observed. Fixtures are clearly marked synthetic and
  exist only to test the verifier.
- **Stdlib-only Python.** No `pip install` — an external reproducer needs only a
  stock Python 3.8+.
