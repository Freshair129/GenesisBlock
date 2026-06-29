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
