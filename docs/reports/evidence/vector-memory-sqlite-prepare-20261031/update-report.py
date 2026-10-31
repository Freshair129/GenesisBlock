from pathlib import Path
r=Path(__file__).parent
p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md')
s=p.read_text()
start='<!-- SQLITE_PREPARE_FOLLOWUP_START -->'
end='<!-- SQLITE_PREPARE_FOLLOWUP_END -->'
block='''<!-- SQLITE_PREPARE_FOLLOWUP_START -->
## SQLite prepare follow-up: publication maintenance

Current source: `5b8bb9bed5dde5fa6e7d4d255da3acce6c1bfe892092456133adb65a78998fdd`
(`src/lib.rs`). The preceding memory results and following timeout follow-up are
historical stages; they are not relabeled as measurements of this source.

The pinned dependency is **rusqlite 0.32.1 / libsqlite3-sys 0.30.1 / SQLite 3.46.0**.
A real SQLite regression confirms that backup commits the destination WAL below
SQLite's SQL executor, resets the destination schema, and leaves a pending WAL
callback. The first subsequent prepare reloads schema through a nested SELECT;
its completion dispatches the default 1,000-frame passive autocheckpoint. This
charges publication I/O to the first read. This confirms the deferral mechanism;
the separate 1 GB timed reproducer and 100k deadline gate below remain pending.

| Callback boundary, >1,000-frame fixture | Before | After |
|---|---:|---:|
| Hook calls during publication | 0 | 1 |
| Hook calls during first property read | 1 | 0 |
| Frames checkpointed during publication | Deferred | 2,352 / 2,352 |

`projection_publish_candidate` now drops `Backup`, retains the destination mutex,
and fully executes `PRAGMA wal_checkpoint(PASSIVE)` before returning. This is real
publication maintenance, with no dummy search or cache warm-up. It moves I/O to
open/publication; it does **not** eliminate that I/O. Pinned read-only snapshots
remain valid, partial/busy passive progress is accepted, and the WAL is not
truncated. Hard maintenance errors explicitly report that publication already
committed. Open's existing recovery-required guard prevents checkpoint-on-Drop
from publishing a failed recovery.

Three regressions cover deferred callback dispatch, an active read-only snapshot
released before the first new read, and committed data observed independently
after a denied checkpoint PRAGMA. RED: existing two projection tests passed and
all three new tests failed for the expected missing maintenance behavior. GREEN:
**57 library tests passed, zero failed, one existing manual profiler ignored**.
The callback test uses real SQLite and a hook that mirrors its default passive
1,000-frame behavior, without a wall-clock assertion. These correctness runs
occurred on a busy host and are not latency benchmarks. Read-only code review
found no actionable issues.

The fresh whole integration suite is in progress on all **159** compiled targets,
with the original 300-second / 4 GiB per-executable acceptance limits. Build
completed in 8m44s. A PID-only monitor incorrectly waited for an older VCTIP helper;
the controller was corrected to validate process birth/path/name and a fresh
cached build returned exit 0. No unrelated process was stopped. The waiting
pre-test driver was explicitly stopped and replaced; raw corrections are retained.

Timed reproducer and 100k performance status: **NOT_RUN yet**. Their unchanged
CPU admission (10-second average below 20%, free RAM at least 6 GiB) has rejected
launch on the currently busy host. Their 5,000 ms query deadline, 1,200-second
parent cap and 2 GiB process-tree RSS cap remain unchanged. Compilation and
callback/content correctness tests use the same time/RAM bounds but do not claim
idle-host timings. No rejected admission is labeled a benchmark result.

The prepared 100k workload uses a legitimate non-vector `Storage.add_node`, normal
Drop/snapshot, same-process writable reopen, then query 0 **before** any search,
coverage or metadata probe. It keeps 100,000 vectors / 1,024 dimensions and checks
100,001 nodes after the marker, READY index coverage, both 24-query k10/k100 sweeps,
and marker recovery. Editing the SQLite fingerprint row directly would invalidate
the P6 projection digest and force fresh recovery, so that shortcut is not used.
The original complete 100k ingest lifecycle remains a separate historical NOT_RUN
under the unchanged cap; it is not silently split or reclassified.

No RAM saving is claimed for this maintenance fix. New RSS, latency and recall
comparisons are pending; the original two-copy vector recommendation is unchanged.
<!-- SQLITE_PREPARE_FOLLOWUP_END -->

'''
if start in s:
 a=s.index(start);b=s.index(end,a)+len(end)
 s=s[:a]+block.rstrip()+s[b:]
else:s=s.replace('<!-- VECTOR_MEMORY_FOLLOWUP_START -->',block+'<!-- VECTOR_MEMORY_FOLLOWUP_START -->',1)
p.write_text(s,newline='\n')
