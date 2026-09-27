# RCA: Intermittent Deadlock While Reporting Index Coverage

## Status / Date

Root cause confirmed; PR #168 fix in progress / 2026-09-27.

## Symptom

The full `cargo test --no-default-features` job on macOS stalled in `index_coverage_tests::coverage_requires_explicit_validation_and_tracks_frontiers` and was cancelled at the 15-minute job limit. The test had added its first vector but had not reached the marker immediately before its first `flush_index()`.

The same focused test completed on a separate macOS run, so the failure is timing-dependent. Linux and Windows completed the instrumented path in the diagnostic run.

## Evidence

- GitHub Actions run 36288015458, job 108532403282, was cancelled after the test had reported running for more than 60 seconds. The last test marker was `coverage added first vector`; the next marker, `coverage before first flush`, was absent.
- The test calls `default_report(&storage)` and checks its result between those two markers.
- `Storage::list_collections()` maps each collection through `VectorCollection::info()`. In `src/lib.rs`, `info()` reads `self.hnsw` to build the `indexed` field, then calls `coverage_report()`, which reads `self.hnsw` again.
- The first read is a temporary in the same `CollectionInfo` statement as the second read. Rust temporary scopes keep it alive through that statement.
- The async index worker may concurrently wait for the `hnsw` write lock in `ensure_hnsw()` after `add_node()` enqueues its vector.
- The pinned `parking_lot 0.12.5` RwLock uses task-fair locking and documents that recursively acquiring a read lock while a writer is waiting can deadlock.
- Diagnostic run 36289096629/job 108535558516 ran the same focused macOS test by itself. It passed in 0.06 seconds and reached every marker, confirming the race does not reproduce on every attempt.
- PR #168 Actions run 36279162678/job 108507577147 checked out merge commit `f423f405969886685975d4bc9c3c70167c05133b` (parents: current main `77e07f7b127f65a1f92cfc688ef15f322115d5f8` and PR head `82e46f958132f6119a5d9cbb72c302a1d17efc7f`). Its active Ubuntu output reports this same coverage test running for more than 60 seconds. The merge ref uses main's coverage-report implementation even though the older PR branch snapshot does not.

## Root Cause

`VectorCollection::info()` can recursively acquire the collection's `hnsw` RwLock within one statement. When the index worker is queued for the write lock, the task-fair lock blocks the second read. The first read guard is still alive until the statement ends, so the same thread cannot release it while blocked on its second read. This deadlocks the coverage report. The timing depends on whether the async index worker has queued its write when `list_collections()` runs.

## Why the Issue Escaped Detection

The race requires the metadata read to overlap index initialization. Most runs let the worker complete or avoid the writer/read interleaving, so the focused test and other OS runs can pass. Before this diagnostic change, the test had no phase markers and the Rust job had no explicit timeout; the old Linux runs therefore supplied no call-level evidence and could occupy a runner for hours.

## Proposed Prevention

Read the HNSW point count in its own statement, letting the read guard drop before calling `coverage_report()`. Keep the existing test's report read immediately after vector insertion, and retain a finite timeout on Rust CI jobs so any future hang produces a bounded, diagnosable failure.

## Follow-up Verification

- The fix reads the HNSW point count in a separate statement, releasing its read guard before `coverage_report()` reacquires the lock.
- GitHub Actions run `36289683946` at commit `7f7aa08e38eba037d8022c74ba7d87d388483e83` passed all jobs, including full `cargo test --no-default-features` on Ubuntu, Windows, and macOS. This confirms the fix across the full matrix, but is not yet a passing run on PR #168.
- The older Wave B timeout (run `35781128316`) remains a separate symptom; its exact blocked statement was not captured.
