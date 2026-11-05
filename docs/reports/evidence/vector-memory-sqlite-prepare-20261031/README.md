---
status: evidence
lifecycle: active
date: 2026-10-31
---
# SQLite prepare follow-up evidence — correctness qualified, timings in progress

This packet is not a final 100k qualification. `qualification-receipt.json` is `passed:true` for the final immutable correctness source only: 159 targets, 1,328 passed / 0 failed / 7 ignored, plus all eight static/native gates and Node 88 passed / 0 failed / 0 skipped. Its 287-path source manifest and gate-file hashes do not imply timing qualification.

- `qualification-summary.json`, `qualification-receipt.json`, `final-runtime-fresh-20261031T213000/`: source-bound fresh complete final-source results. Earlier pre-lint/interruptions remain separate.
- `red2-run/`: real pinned SQLite RED (first-read callback 1); `green-run/` and final library log: GREEN (publication callback 1, first-read callback 0).
- `qualification-attempt1/`: 159-target pass before the test-only C-string lint correction, plus the preserved Clippy failure.
- `resource-hold/`: historical final E69 attempt with 85 completed targets and an explicit interrupted HQL target. These partial results are not summed into the fresh whole-suite pass.
- `source-chain.txt`, `rca-investigation.txt`, production patch and traced-variant metadata: mechanism, proposal history, and unchanged query/budget regions.
- `reproducer/`, `matrix-real/`, `matrix-analysis.json`: pinned SQLite 3.46.0 harness and confirmed deferred-checkpoint mechanism without Genesis replay. `matrix-real-interference.json` excludes these diagnostic timings from isolated performance conclusions because external compilers arrived during the tail.
- `matrix-isolated/`, `matrix-isolated-analysis.json`, `matrix-isolated-guard-attempt2/`: fresh four-mode run passed with unchanged caps and continuously monitored competition. Backup first prepare 2.915156 s; fixed first prepare 27 us; explicit publication maintenance 3.220885 s. Fixed-order single observation, no OS cache reset or statistical performance claim.
- `100k-copy-verification-postcommit/`: original paired copies passed full hashes, 109 files / 12,689,795,140 B each, source unchanged.
- `100k-before-guard/`, `100k-two-open-attempt1/`: actual same-process two-open 100k run TIMEOUT at the original 1,200 s cap before backup-complete/query 0. Replay/edge-history timings and raw 100,001-property trace retained; this does not qualify query deadline. Relocation receipt maps the archived paths; database files remain excluded.
- `fresh-reopen-variant-metadata.json`, `release-variants-fresh-*/`: new paired diagnostics scope property tracing only to query 0, with the same observer in both variants, pinned Rust 1.96. Production engine unchanged.
- `run-fresh-workload.py`, `prepare-fresh-case-copies.py`, `analyze-fresh-case.py`: separate legitimate fixture preparation (no searches) and fresh-process RW reopen, with unchanged 1,200 s / 2 GiB / 5,000 ms query caps and all count/coverage/marker/result gates. They never qualify or rename the failed two-open lifecycle.
- `postcommit-resource-hold.json`, `fixture-preparation-wait-status.json`: bounded resource wait expired without opening the new fixture; fresh query deadline and current real-BGE pair remain NOT_RUN.
- `audit-commit-receipt.json`, `verify-source-postcommit-receipt.json`: authorized checkpoint `f98a324`, 1,742 audit paths committed, seven H2a staged paths preserved, 287 source hashes unchanged. Existing test results are working-tree source qualification, not a fresh whole-suite CI run at that commit.
- `release-variants-resume-*/`: all three source-bound private release builds passed. Original monitor-error receipts, fresh owned-process stop proof and corrected StrictMode lifecycle fixtures are retained. Build success is not runtime qualification.
- `controller-validation.json`: current SHA-bound strict normal/timeout fixtures plus the real arriving-compiler branch. The latter stopped only owned copy-preparation Python, preserved its partial artifacts, and recorded competing identities separately; it is not a benchmark timeout.

Databases, signing keys, binaries and Cargo targets are excluded. Large logs are exact gzip copies with uncompressed SHA256 in `artifact-manifest.json`. Old audit/follow-up evidence folders are not relabeled or regenerated.

Compilation/callback correctness timings occurred on a busy host and must not be used as performance comparisons. Benchmark admission and query/time/RAM caps remain unchanged. No code was deployed or committed.
