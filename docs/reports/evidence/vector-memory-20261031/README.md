# Vector memory evidence

Date: 2026-10-31. Local scratch databases and immutable executables are retained at
`C:\Users\suanr\genesis-vector-memory-20261031`; they are not included here.

- `provenance.json`: exact base, build profile, executable/source/input hashes,
  per-measurement source version and vector/sidecar snapshot hash comparisons.
- `baseline/`, `candidate/`: completed release-profile, full-retention matrix rows.
  The candidate 100k row uses final proof-sharing code; other matrix candidates
  use the vector-optimization stage-one source. Full retention skips proof folding.
- `baseline-repeat/`, `candidate-repeat/`: F32 pair launched 24 ms apart on the
  same loaded host; matched contention, not isolated performance qualification.
- `candidate-vector-stage1.patch`, `candidate-proof-sharing.patch`: engine stages
  relative to base HEAD and then stage one; public wire/API proof types unchanged.
- `proof-lifecycle.json`: matched debug integration executables through process
  exit/Drop, with 4 GiB/180 s guards. Censored baselines are lower-bound evidence;
  these tests do not assert checkpoint success.
- `proof-explicit.json`: debug NAPI companion directly awaits save_state, records
  its result, then checks fresh read-only reopen count/indexed and search sanity.
  At 128 rows both checkpoints succeed and vector snapshot hashes match. At 1000
  the candidate preserves the existing proof-size rejection; WAL reopen succeeds.
- `bounded-and-focused-tests.log`: combined named raw test logs. Individual log
  names in JSON correspond to sections here and files in the retained scratch root.
- `final-test-summary.json`, `final-bounded-tests.json`: final-source test outcomes;
  limits/timeouts are incomplete, never passes. Earlier phase-one evidence is kept.
- `hnsw-dependency.json`: archive checksum matches Cargo.lock, registry source
  matches that archive. `hnsw_vector_bytes` is payload-only, not total graph RAM.
- `resource-admission.json`: run safety budgets; these are not RAM-sizing claims.

Decimal MB are used in the report; safety budgets use GiB. RSS is Windows working
set. Allocation counters are not RSS. No file here certifies live-provider quality,
Linux/cgroup sizing, production deployment or release acceptance.

`diagnostic-scripts/` retains the exact host-specific adapters used for the
separate default-retention diagnostics and final Rust run. They use Windows
process counters and recorded scratch paths/executable names; review and change
the scratch root/build outputs for another host. They are evidence, not a generic
production maintenance command. Explicit-checkpoint cases refuse existing output
folders/databases. Preserve the original result directories before any rerun.
The supported vector matrix runner and streamed harness live under `benches/`.
