# Nested graph Batch projection / recovery omission

Owner approved H1-R on 2026-10-29, expanding H1 to at most twelve files. HIGH risk / C-3. RCA recorded before production correction.

## Symptom / evidence

At main a14b6ad498d738e7a2343c901fff2b958035eba9 a correctly signed, roster-authorized schema-5 ordinary `Batch([Node(before), Batch([Node(after)])])` is accepted by reconcile and both nodes become resident. Reopen fails with `projection sync failed (journal replay did not recover props for every resident node); fresh WAL rebuild failed: journal replay did not recover props for every resident node`.

H1 first runtime and a controlled reproduction both fail on that nested ordinary positive control. In the controlled run the same flat batch reopens successfully. Six P6 generation/policy rejection cases with real before/after siblings pass, including bounded file/frontier/live-lease guards and reopen. No source changes occurred between those executions.

Raw logs, commands, preserved failing temporary store, SHA256 manifests, paused patch and first compile-only fixture correction are retained under `D:/GenesisBlock_Dev/local-cleanup-backups/20261019T030925Z-genesisblock/branch-review/p6-h1-boundary-qualification-20261029`. The initial E0608 fixture compile error is separate from this runtime defect. Independent source inspection confirmed the traversal disagreement; independent runtime verification was pending at that initial stop.

## Root cause

`reconcile_state_unlocked` preserves one original signed outer Batch in the WAL and recursively applies it to memory. `apply_event_memory` recursively traverses all Batch depths. Live `projection_apply_event` instead projects only immediate Node/Edge children. `projection_replay_wal` also walks only one Batch level. Nested nodes therefore become resident without their projected properties; lean-node hydration is incomplete and the replay completeness check detects missing properties on reopen.

## Why detection failed

The earlier nested-P6 fixture had no ordinary siblings. Ordinary batch compatibility fixtures mainly covered a single nesting level. Presence-only node assertions do not establish complete projected properties. H1 introduced an ordinary-only nested positive control and required reopen.

## Correction / prevention

Traverse existing graph Node/Edge leaves through nested Batch containers in encounter order while reusing the existing live SQLite transaction and preserving frame sequence/signature. Replay must reach nested graph leaves without changing admission, signature domain or existing direct-event behavior. Avoid recursively reacquiring the projection connection lock.

Regressions require exact live and reopened properties, flat and multiple nesting depths, graph Edge projection, original signed-envelope/frame preservation and P6 atomic rejection controls. Run P6, sync/consensus/journal compatibility, independent verification/review and full hosted CI before merge. Other nested relational/transaction/retraction composition mismatches are separately inventoried; this repair does not establish new accepted leaf kinds or complete broad P6 acceptance.

## Scope alignment and fixture investigation

Final self-check removes an unnecessary adjacent top-level Edge replay arm from the provisional implementation. Direct-event dispatch remains at baseline; this repair is limited to Batch graph traversal. The earlier broad author run (1,305 unique passes, zero failures, six existing ignored entries across 158 targets) is retained with its earlier source manifest and labelled pre-final, not claimed as execution of later source bytes. The final candidate requires fresh relevant HQL/P6 execution, independent verification, strict checks, review and complete hosted CI.

Fixture-only compile/setup errors are retained in external FIXTURE-RCA.md: optional error detail indexing, diagnostic TempDir ownership, exact-size signature JSON construction, initial genesis-base dependency overhead, default collection definition sequencing, and compacted CollectionMaterialization counting. Each correction follows its typed/source evidence and preserves the actual error/byte/cursor assertions; no runtime repair was made for fixture errors. Final controls also require dependency-before-data order, one exact original Batch frame, literal compacted node properties and oversized real base0 failure without a cursor. These remain within the approved H1 boundary cases.


## Final-source local verification

On source/test/Cargo manifest `2889964b54b5b2a8e19d630dfca87222688f28eef9c5764d9593295830fba84b`, author library/HQL2/peer verification passed 526 unique tests across 46 targets (zero failures, one existing parser-child ignore invoked by its passing parent). A separate executor passed 193 unique tests across 26 P6/projection/recovery/journal/sync/consensus targets with zero failed/ignored/filtered entries and unchanged source hashes. The executor used the same Windows host/checkout and shared Cargo registry/compiled target; OS TEMP was initially absent, while compile-time target scratch paths are shared and reinitialized by existing tests. This is not new-machine, fresh-build, sandbox, new-oracle or whole-P6 acceptance. Raw logs and root artifact verification remain in the external evidence folder. Strict checks, whole-branch review and exact-head hosted outcomes are separately required and recorded there.
