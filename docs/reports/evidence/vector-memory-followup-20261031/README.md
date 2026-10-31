---
status: evidence
lifecycle: frozen
date: 2026-10-31
---

# Vector-memory timeout follow-up evidence

This directory records the follow-up candidate, separately from the original
`vector-memory-20261031` measurements. Neither folder retroactively relabels its
historical binaries with later source. All experiments use disposable databases.
Source changes, raw failures, bounded runs and successful runs remain distinct.

- `provenance.json`: current source and frozen stage/binary hashes.
- `followup-engine.patch` / `followup-cargo.patch`: delta from the delivered initial
  vector-memory candidate, rather than from HEAD before that audit.
- `final-test-summary.json`, `final-runtime.json`, `final-test-artifacts.json` and
  `rust-test-logs.txt`: full core results, per-target guards and frozen executable
  hashes. All 159 executables completed; 1,325 passed, zero failed, seven ignored.
  One ignore is the newly added manual profiler, which was explicitly invoked.
- `six-runtime.json` / `six2-runtime.json`: earlier source-only stages, including
  their 300 s timeouts. They are not final-candidate passes.
- `*-red.log` / `*-green.log`: work-count regression evidence. The three
  `reopen-*-red.log` controls actually passed and show no repeated raw verification;
  their filenames do not mean a reproduced defect.
- `optimized-dependency-profiles.json` / Cargo JSON build receipts: checked engine
  opt-level 0 and three checked dev dependencies at opt-level 3. Release settings,
  pinned versions, test/query budgets, signature checks and proof caps are unchanged.
- `node-full.log`, `napi-smoke.json`, feature/Clippy/dts receipts: separate native
  and static qualification. No hosted CI, deployment or release claim.
- `native-controls/runs/native-perf-20261031T063931Z/comparison-summary.json`:
  successful isolated ABBAAB checkpoint comparison, six writers plus six fresh
  read-only verifiers. The before source includes inactive vector-query tracing;
  no vector query executes in this workload and diagnostic stderr is empty.
  The source hashes in that receipt are authoritative; its shorthand relationship
  string predates this clarification.
- `native-controls/runs/native-perf-20261031T063817Z`: excluded pilot whose verifier
  used the wrong output-field name. The corrected verifier was rerun on six fresh
  databases. Raw grep-based `serialization_error_observations` can match successful
  empty arrays in structured output; the actual six arrays and stderr are empty.
  The corrected comparison counts six jobs; an older raw summary serialized `[6]`.

The native checkpoint comparison reduces median save time by 11.17% while increasing
median process peak RSS by 11.264 MB (9.36%). It does not demonstrate RAM savings in
that workload, and native wrapper finalization does not measure Storage Drop.

`query-reopen-repeat-20261031T134858/two-open-comparison-summary.json` records the
completed 100k old/new two-open diagnostics. All four first queries passed the
unchanged 5 s budget. Traced q0 includes output overhead; the additional warm sweep
is 24 unique queries repeated five times. The original full-ingest deadline was
not reproduced. `main-lifecycle-not-run.json` keeps the original 1,200 s cap and
explains why the historical approximately 40-minute full 100k sequence was not
launched or split across budgets.

`isolated-real-comparison.json` and `real-release-controls/runs/real-bge-release-pair-20261031T073308Z/`
record the completed serial real BGE-M3 F32 pair. All search/persistence/coverage
checks passed and the vector-file hashes match. Recall@10/@100 is unchanged;
search@100 p95 increased 7.47% in this single pair. Root OS peak RSS increased
0.795 MB (0.22%). These are observations, not statistical non-regression gates.
The large reopened-query p99 (about 2.7 s) prompted an additional bounded trace;
its results are separate from the uninstrumented performance pair.

`query-reopen-bge-candidate-trace-20261031T145352/analysis-summary.json` isolates
that real-workload tail to one projection-props hydration. The subsequent
`projection-props-split-20261031T151315/split-stage-result.json` localizes it to a
successful SQLite prepare (2.631 s); mutex/row read/finalize/JSON are negligible.
Both diagnostics completed within the original guards. They do not prove the
underlying SQLite/OS cause or fix the historical 100k deadline. No warm-up or
query-budget change is delivered. The split run's raw launch source-metadata field
was stale; `run-config-provenance-correction.json` distinguishes it from the
authoritative frozen build source/binary hashes. Failed prelaunch controllers and
the corrected diagnostic-only E0505 compile failure are preserved separately.

`final-static-validation.json` and `verify-source-receipt.json` record the final
format/docs/diff/provenance checks. The code comparison covers 253 compiled
code/config/declaration files; seven original staged paths are protected separately.
Eight noncompiled documentation paths are classified explicitly. Attempt 1 wrongly
included the evolving audit report/RCA in the compiled-source comparison; its
receipt and verifier are preserved as `*-attempt-1.*`. No compiled source mismatch
or original staged-file drift was found.

Query-diagnostic and isolated release receipts are retained under their named run
directories. First-query deadlines remain 5,000 ms; passing diagnostics do not prove
the cause of a historical deadline. Scratch databases, build trees, native binaries
and private identity files are intentionally excluded from this portable export.
External originals remain under `C:\Users\suanr\genesis-vector-memory-followup-20261031`.
