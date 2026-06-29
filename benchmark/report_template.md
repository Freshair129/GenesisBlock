# Benchmark Run — {{benchmark_id}}

> Rendered by `benchmark/assemble_result.py` from `result.json`. This is a
> human-readable summary; `result.json` is the machine-readable source of truth.

| Field | Value |
|-------|-------|
| Benchmark | `{{benchmark_id}}` |
| Project | {{project}} |
| Result | **{{pass}}** |
| Commit | `{{commit}}` |
| Repo dirty | {{repo_dirty}} |
| Engine version | {{version}} |
| Interrupted | {{interrupted}} |
| Start | {{timestamp_start}} |
| End | {{timestamp_end}} |
| Duration | {{duration_sec}} s |

## Environment

| | |
|-------|-------|
| OS | {{os}} |
| CPU | {{cpu}} |
| RAM | {{ram_gb}} GB |
| Disk | {{disk}} |
| rustc | {{rustc}} |
| cargo | {{cargo}} |

## Results

| Metric | Value |
|--------|-------|
| Total nodes | {{total_nodes}} |
| Cycles | {{cycles}} |
| Peak RAM (MB) | {{peak_ram_mb}} |
| Final disk (MB) | {{final_disk_mb}} |
| Recall miss rate | {{recall_miss_rate}} |
| Query latency p50 / p95 / p99 (ms) | {{query_p50}} / {{query_p95}} / {{query_p99}} |
| Ingest latency p50 / p95 (ms, per cycle) | {{ingest_p50}} / {{ingest_p95}} |
| Reopen OK | {{reopen_ok}} |
| Reopen load (s) | {{reopen_load_sec}} |

## Config

```json
{{config_json}}
```

## Verification

Validate this run with:

```bash
python benchmark/verify_report.py {{result_json}}
```

## Caveats

- This is an **independent / reproducible** benchmark artifact. A single run is
  not a statistical claim — see `docs/benchmarks/INDEPENDENT-BENCHMARKS.md` for
  how internal audit, reproducible, and community-reproduced results differ.
- Embedded (in-process `Storage`) numbers are **not** directly comparable to the
  REST-server surface (network + serialization overhead).
- Latency is wall-clock on a loaded OS; expect run-to-run noise. Disk and RAM
  figures depend heavily on the storage medium and machine.
- For soak runs, query latency is a single probe per cycle; ingest latency is
  the wall time to ingest one full cycle (`nodes_per_cycle` nodes), not per-node.
