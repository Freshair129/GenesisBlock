# 100k same-process SQLite reopen reproducer

`reopen100k-vector_memory.rs` is an external copy of the repeat benchmark with one opt-in workload mode. It adds a non-vector marker through `Storage.add_node`, drops `Storage` normally so its ordinary snapshot path runs, reopens the same database in the same process, and makes query 0 the first Storage-level operation after that reopen. It then runs the existing 24-query sample for both k=10 and k=100, followed by index coverage validation and the vector metadata summary. After query work, it checks the marker through `Storage.node_view` and reports whether actual node/metadata counts are 100,001/100,000 and the `bench` coverage row is `READY` with source/index counts 100,000. These checks do not run before query 0.

The mode refuses a non-100,000 x 1,024 fixture, a non-RW open, a repeat count other than 2, a sample count other than 24, `GB_EF` other than 200, missing query diagnostics, or a Rayon thread count other than 4. It records result-count minimum/maximum; the lifecycle gate requires q0 to return exactly 10 rows and the 24-query sweeps to return at least 10/100 rows for k=10/100. Recall is reported without adding a threshold; compare it with existing quality contracts and the real-embedding pair. Lifecycle, completeness, and coverage gates are separate from retrieval quality. This same-process run does not claim an OS-cold cache state. The marker uses a generated unique ID and `embedding: None`; expected totals are 100,001 nodes and 100,000 vector nodes. It does not issue SQL against the projection, call `save_state` explicitly, add a warmup query, or change the engine's 5,000 ms query budget.

## Inputs and disposable target

Use the existing fixture at:

`C:\Users\suanr\genesis-vector-memory-20261031\inputs\synthetic-100000-1024`

Run only against a disposable writable copy of the prepared database; do not point the mode at a canonical/source database. The observed source database used to prepare this run was:

`C:\Users\suanr\genesis-vector-memory-followup-20261031\query-reopen-repeat-20261031T134858\new\gdb`

This source artifact is not copied or modified by this reproducer preparation. The run remains subject to the parent harness's 1,200-second deadline and 2 GiB RSS limit.

## Opt-in environment

Set the same benchmark binary and existing two-pass controls used by the repeat harness, plus:

```powershell
$env:GB_MEMORY_OPEN_ONLY = '<disposable-gdb-path>'
$env:GB_MEMORY_OPEN_OUT = '<output-json-path>'
$env:GB_MEMORY_OPEN_READ_WRITE = '1'
$env:GB_MEMORY_OPEN_REPEAT = '2'
$env:GB_MEMORY_OPEN_QUERY_SAMPLES = '24'
$env:GB_EF = '200'
$env:GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN = '1'
$env:GB_MEMORY_INPUT = 'C:\Users\suanr\genesis-vector-memory-20261031\inputs\synthetic-100000-1024'
$env:GB_VECTOR_QUERY_DIAGNOSTICS = '1'
$env:RAYON_NUM_THREADS = '4'
$env:GB_MEMORY_RSS_LIMIT_BYTES = '2147483648'
```

Then invoke the same benchmark executable that calls this `run()` entrypoint. Normal repeat behavior is unchanged when `GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN` is unset. This source preparation does not build or execute the benchmark.

## Explicit copy and run controllers

`prepare-100k-cases.py` is an explicit later invocation. It verifies the source `projection.sqlite-wal` is absent or zero bytes, requires at least 50 GiB free, refuses existing `before/gdb`, `after/gdb`, or copy-manifest destinations, and copies the source tree to both case paths. It uses filesystem copy/hash operations only; it does not open SQLite or launch a benchmark. The manifest records per-file component hashes, file counts, byte totals, tree hashes, and source/copy equality. It also rechecks that the source tree stayed unchanged during copying.

```powershell
python .\prepare-100k-cases.py
```

`run-100k-case.py` requires a config for exactly one case. Example for the candidate case; the `before` config uses the `before` path and expects both engine and q0 states to be `error`.

```json
{
  "binary_path": "C:\\path\\to\\frozen-vbench.exe",
  "binary_sha256": "<64-character frozen SHA-256>",
  "case_path": "C:\\Users\\suanr\\genesis-sqlite-prepare-20261031\\100k-cases\\after",
  "input_path": "C:\\Users\\suanr\\genesis-vector-memory-20261031\\inputs\\synthetic-100000-1024",
  "expected_engine_state": "ok",
  "expected_query0_state": "ok",
  "args": []
}
```

Invoke it explicitly after the frozen binary and case copies are reviewed:

```powershell
python .\run-100k-case.py --config .\100k-cases\after\run-config.json
```

The runner clears inherited `GB_*`, `RAYON_NUM_THREADS`, and `RETENTION` variables, then sets the complete profile documented above. It writes direct child stdout/stderr files and a receipt containing the engine PID/context, actual process exit code, parsed result state, and input tree SHA-256/file-count/byte-count before and after. It imposes no timeout or CPU admission policy; the parent guard owns the 1,200-second deadline, 2 GiB limit, and admission check. It does not reset the OS file cache. An exit code of zero means the configured expected states were observed and the input stayed unchanged; `before` therefore records its expected query failure. Every receipt says `qualification_claimed: false`; neither script launches itself or claims retrieval qualification.