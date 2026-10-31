---
status: evidence
lifecycle: frozen
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
- `prepare-100k-cases.py` / `run-100k-case.py`: pending 100k workload, with first query before coverage/metadata; no warm-up. File copies exist but full hash verification was resource-held before qualification; no database was opened by preparation.
- `release-variants-resume-*/`: all three source-bound private release builds passed. Original monitor-error receipts, fresh owned-process stop proof and corrected StrictMode lifecycle fixtures are retained. Build success is not runtime qualification.
- `controller-validation.json`: current SHA-bound strict normal/timeout fixtures plus the real arriving-compiler branch. The latter stopped only owned copy-preparation Python, preserved its partial artifacts, and recorded competing identities separately; it is not a benchmark timeout.

Databases, signing keys, binaries and Cargo targets are excluded. Large logs are exact gzip copies with uncompressed SHA256 in `artifact-manifest.json`. Old audit/follow-up evidence folders are not relabeled or regenerated.

Compilation/callback correctness timings occurred on a busy host and must not be used as performance comparisons. Benchmark admission and query/time/RAM caps remain unchanged. No code was deployed or committed.
