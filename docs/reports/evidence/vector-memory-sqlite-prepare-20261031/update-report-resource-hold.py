from pathlib import Path
import json
r=Path(__file__).parent
p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md')
s=p.read_text();a=s.index('<!-- SQLITE_PREPARE_FOLLOWUP_START -->');b=s.index('<!-- SQLITE_PREPARE_FOLLOWUP_END -->',a)+len('<!-- SQLITE_PREPARE_FOLLOWUP_END -->')
block='''<!-- SQLITE_PREPARE_FOLLOWUP_START -->
## SQLite prepare follow-up: confirmed deferral, final gates on resource hold

Production core SHA256: `5b8bb9bed5dde5fa6e7d4d255da3acce6c1bfe892092456133adb65a78998fdd`.
Final projection-test SHA256: `e69dc7dd5cbc5043aa5ed9c717e449e50246a8fbd8ad56ac95ed135718c99be2`.
This section supersedes the current-status interpretation of earlier stages;
all earlier timings retain their original source and workload provenance.

**The deferral mechanism is fixed and regression-tested. The 1 GB timed causal
reproducer and the 100k reopen deadline are still NOT_RUN/unqualified.** They must
not be inferred from the library tests, historical timings, or an exit code alone.
The source-bound qualification receipt deliberately remains `passed:false`.

The pinned versions are rusqlite **0.32.1**, libsqlite3-sys **0.30.1**, and bundled
SQLite **3.46.0**. The amalgamation SHA256 is
`c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d`.
Backup commits the live destination WAL through SQLite's lower-level B-tree API,
sets a pending WAL callback and resets the schema. It bypasses the SQL executor's
callback dispatch. The next prepare loads schema through a nested SELECT; when
that statement reaches DONE, it dispatches the default 1,000-frame passive
checkpoint. Thus publication I/O can be charged to the first query's prepare.
This is an engine lifecycle mismatch with the pinned API, not evidence of an
upstream SQLite defect. The exact attribution of the historical 2.631-second
real-embedding tail still needs the separate timed causal matrix.

| Real pinned SQLite callback fixture | Before | After |
|---|---:|---:|
| Hook calls during publication | 0 | 1 |
| Hook calls during first property read | 1 | 0 |
| Frames checkpointed during publication | Deferred | 2,352 / 2,352 |
| First-read callback after releasing an old reader | 1 | 0 |

The production change scopes/drops `Backup`, keeps the destination mutex held,
and fully executes `PRAGMA wal_checkpoint(PASSIVE)` before publication returns.
This completes real maintenance at its ownership boundary; it does not execute a
dummy search or prime a query cache. It moves page I/O into open/publication and
**does not remove that I/O or claim a RAM saving**. PASSIVE accepts partial/busy
progress, preserves old read-only snapshots, and does not truncate the WAL. Hard
maintenance errors explicitly say that projection publication already committed.
Open's existing recovery-required guard then prevents a failed recovery from
checkpointing its state on Drop. A direct FFI checkpoint alone was rejected as a
fix because it can leave the pending SQL-layer callback undispatched.

Three regressions cover deferred callback ownership, a reader held across
publication and released before the first new read, and committed data observed
through an independent connection after a denied maintenance PRAGMA. The hook
mirrors SQLite's default passive 1,000-frame behavior; it has no wall-clock gate.
Read-only review found no actionable production or test issues after cleanup.

| Verification stage | Result and scope |
|---|---|
| RED, original core `812afd...`, new regressions | Existing 2 passed; new 3 failed as expected; first read dispatched the deferred callback |
| GREEN library, production core `5b8bb9...` | 57 passed / 0 failed / 1 existing manual-profiler ignore |
| Whole 159-target core suite, pre-lint test source `40c4a1...` | 1,328 passed / 0 failed / 7 ignored; all six formerly timed-out targets completed under the original limits |
| Strict Clippy, pre-lint test source | Failed on the new test's manual NUL-terminated string literal; raw failure retained |
| Final test source `e69dc7...` | Replaced that literal with `c"main"`; production core unchanged; locked/offline compile completed |
| Final library tests | 57 passed / 0 failed / 1 ignored on `e69dc7...` |
| Final whole-suite attempt | 85 targets completed, zero failures; `hql_cypher_tests` interrupted by resource hold at 216.8 s, before its 300 s cap; not a complete suite or a timeout result |
| Final strict Clippy, Node/NAPI and host-mobile checks | Pending after the test-only literal fix; earlier source-stage results are not relabeled as final |
| Timed pinned-SQLite matrix / 100k / new real-embedding pair | NOT_RUN; resource admission blocked |

The final rebuild changed all 159 executable hashes, so earlier binary results
were not reused as if they qualified the final set. All binaries were frozen and
hashed; the interrupted set requires a fresh complete run after resource release.
The seven pre-existing staged files remain protected and unchanged. No commit,
deployment, release, production database write, or structural vector change was made.

At 20:26 local time the host had 84% CPU, seven unrelated compiler processes and
10.57 GiB free RAM; earlier samples reached 98% CPU with thirteen compilers.
Only the owned test controller and its owned current test were stopped. Their
birth/path identities and zero remaining owned tests are recorded in the hold
receipt. Unrelated processes were preserved. The unchanged limits are:

- Core tests: 300 seconds / 4 GiB per executable.
- Timed benchmark jobs: 1,200 seconds / 2 GiB tracked process-tree RSS.
- Engine query deadline: 5,000 ms.
- Benchmark admission: 10-second CPU average below 20%, at least 6 GiB free RAM,
  and no competing compiler/benchmark job.

Compilation and callback/content tests were explicitly separated from idle-host
latency qualification. No rejected admission is treated as a measurement. A
PID-only early monitor mistakenly waited for an older VCTIP helper; the helper was
not stopped. Raw corrections and a fresh successful Cargo receipt are retained.
The replacement controller validates PID, birth, path/name and the original
process handle, bounds cleanup, and fails closed on uncertain ownership. Release
build preparation uses an exclusive lock, source-bound gates and a private target
directory; it cannot overwrite D's target executable or restore sources while an
owned compiler may still be active. This controller preparation is not itself a
benchmark pass and still needs its execution checks in a resource window.

The prepared causal matrix opens the source read-only and uses disposable copies:
direct open; backup without Genesis replay; the same backup with automatic
checkpointing disabled only as a diagnostic control; and backup followed by the
explicit publication maintenance. It records prepare/row/finalization timing and
WAL/backfill state. It never treats disabling checkpointing as a production fix.

The prepared 100k workload makes one legitimate non-vector `Storage.add_node`,
uses normal Drop/snapshot, reopens read/write in the same process and runs query 0
before searches, coverage, metadata or marker probes. It retains 100,000 vectors /
1,024 dimensions and checks 100,001 nodes, complete k10/k100 result counts, READY
coverage of all 100,000 vectors, and marker recovery. Both 24-query sweeps report
recall; lifecycle `state:ok` alone cannot qualify retrieval quality. The run parser
must prove the second open's marker miss and actual backup publication. Reading
the 100-byte SHM header is disclosed diagnostic observation; no filesystem-cold
claim is made. The original full 100k ingest lifecycle remains a separate
historical NOT_RUN under its unchanged cap.

Remaining gates: fresh final 159-target suite; strict static/native checks;
controller completion/timeout checks; pinned timed causal matrix; the matched
100k before/after reopen cases; and real BGE before/after latency/recall/RSS.
No new MB/percentage RAM saving or statistical performance non-regression is
claimed. Retain the Arena/HNSW two-copy design and existing quantization/sidecars.
Evidence: [SQLite follow-up artifacts](evidence/vector-memory-sqlite-prepare-20261031/README.md).
<!-- SQLITE_PREPARE_FOLLOWUP_END -->'''
p.write_text(s[:a]+block+s[b:],newline='\n')
