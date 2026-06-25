---
name: run-bench-audit
description: Run the GenesisBlockDB benchmark/audit harnesses (Criterion bench + the [[bin]] load/audit harnesses) before claiming "no perf regression" on storage/index/HQL changes. Use when the user says "run the benchmarks", "check for perf regression", "run the audit", "rerun ldbc/industrial/scientific/snb/shadow-sync/hql-stress", or after any change to src/lib.rs storage/index/HQL paths.
---

# Run Bench / Audit Harnesses

GenesisBlockDB ships a Criterion bench plus several `[[bin]]` load/audit harnesses.
Per `CLAUDE.md`, these MUST be run before claiming no perf regression on
**storage / index / HQL** changes. This skill makes that run prescriptive and
captures the Windows/PowerShell gotchas that have bitten past sessions.

Run the steps in order. Skip a harness only if it is clearly unrelated to the
change, and say which you skipped and why.

## 0. Preconditions (read before running)

- **The `bins` feature is mandatory.** Every harness is gated behind the
  off-by-default `bins` feature. A `cargo run --release --bin <name>` **without**
  `--features bins` fails with exit 101: *"target requires the features: bins"*.
  Always pass `--features bins` (PowerShell: `--features="bins"`).
- **Build first, measure second.** Do a `cargo build --release --features bins`
  once so compile time is not counted inside the first harness run.
- **Do NOT suppress stderr while hunting a crash.** Never use `2>$null` / `2>&1`
  on these binaries in PowerShell — it hides the real error (a past HNSW OOM,
  `memory allocation of … bytes failed`, was masked for several attempts this way).
  If you must split streams, use `-RedirectStandardOutput` and let stderr through.

## 1. Criterion micro-bench (not gated)

```bash
cargo bench --bench ldbc_lite
```

This is the only bench **not** behind `bins`. Criterion writes a baseline under
`target/criterion/`; on a second run it reports `change: [-x% +y%]`. Treat a
regression > the noise band (typically a few %) as a real signal, not noise.

## 2. Load / audit harnesses (gated behind `bins`)

Run the ones relevant to the change. Names come from `Cargo.toml [[bin]]`:

```bash
cargo run --release --features bins --bin industrial-audit     # broad storage/index load
cargo run --release --features bins --bin scientific-audit     # recall / correctness audit
cargo run --release --features bins --bin snb-ingestion        # single-threaded ingest
cargo run --release --features bins --bin snb-bulk-ingestion   # batch ingest path
cargo run --release --features bins --bin shadow-sync-stress   # CRDT sync under load
cargo run --release --features bins --bin hql-query-stress     # HQL execution under load
```

Other available bins: `vbench-genesis`, `graph-bench`, `gov-kimpact-bench`,
`edge-interning-audit`, and the server `genesis-db-server`. Pick by what the
change touches (vectors → vbench/scientific; graph → graph-bench; governance →
gov-kimpact; edges → edge-interning-audit).

## 3. Clean stale scratch DBs between reruns (Windows)

Harness DBs live under `.brain/` (gitignored). On Windows, `shutil.rmtree` /
`remove_dir_all` **silently fails if a prior run left a file lock**, and the next
run dies with *"already exists in catalog"* and produces no numbers. Before a
rerun, remove the relevant scratch dir, e.g.:

```powershell
Remove-Item -Recurse -Force ".brain\industrial_audit_db"  # adjust per harness
```

(Single-file/in-memory engines don't have this issue — only the multi-file ones do.)

## 4. Record results, compare against baseline

- Capture the key measured numbers (RSS, ingest throughput, recall@k, query
  latency) into the run log. **Do not commit the log** — `*.log` is gitignored.
- Compare against the last recorded baseline in `.brain/session/SESSION--*.md`
  or the relevant `AUDIT--*` / `RCA--*` doc. State the delta explicitly.
- A claim of "no regression" is only valid if you ran the harness that exercises
  the changed path and the delta is within noise. Otherwise report the number.

## 5. Validate the run (deterministic check)

Run `scripts/check-bins-feature.sh "<the command you ran>"` to confirm any
`cargo run --bin` invocation included `--features bins` before you trust its
output. See `scripts/`.

## Notes

- Be honest in the record: a harness that crashed, was skipped, or produced
  suspect numbers goes in as-is — the point is an accurate perf trail, not a
  green checkmark.
- For the read-your-write subtlety inside tests (HNSW indexing is async, call
  `flush_index()`), see `CLAUDE.md` → "HNSW indexing is async".
