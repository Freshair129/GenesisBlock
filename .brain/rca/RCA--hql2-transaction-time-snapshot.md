# RCA: HQL2 Transaction-Time Snapshot Frontier Drift

## Status / Date

Root cause confirmed; approved P8 runtime path implemented and locally verified / 2026-10-03.

## Symptom

HQL2 and typed-IR requests with an explicit `tx_as_of` (or HQL `AT TX`)
cannot return the historical transaction snapshot specified by P8.

## Evidence

- `src/query/hql2/request.rs::normalize` parses the selector, rejects values
  below `history_horizon`, rejects values beyond the observed frontier, then
  unconditionally returns `CAPABILITY_UNSUPPORTED` with reason
  `transaction_time_execution`.
- `src/lib.rs` passes `options.tx` into the P6 lease, but encodes
  `QueryResultV2.snapshot.tx` from `lease.generation.wal_frontier` rather than
  the selected transaction sequence.
- Revision-backed HQL2 operations (`hql2_history_scan`, `hql2_change_scan`,
  `hql2_scan`, graph snapshot, annotation lookup/hydration and vector reads)
  read `lease.generation.wal_frontier` directly. Removing only the request
  rejection would therefore return current-state rows under a historical
  request.
- P8 specifies `Snapshot.tx` as the selected local frame sequence and states
  historical selection uses validated `tx_as_of`; P6's previous generic HQL
  rejection wording did not distinguish legacy `execute_hql` from P8's
  `Storage::query_v2` HQL2/IR path.
- Existing tests cover P6 horizon/future behavior and fail-closed HQL2
  transaction-time rejection, but do not prove HQL/IR reads at a prior
  transaction frontier or assert the returned `Snapshot.tx`.
- The tests-first regression reproduced the gap before implementation, then
  passed after request normalization, source operators, hydration and result
  metadata were bound to one selected frontier. Five focused targets pass
  56/56; HistoryScan/ChangeScan passes 14/14; the explicit 31-target HQL2
  sweep passes 373/0/1 and the separate 11-target P6/schema-v6/compatibility
  sweep passes 194/0/0. These are local regression results only.

## Root Cause

P6 lease construction accepts temporal selectors, but HQL2 execution stopped at
validation: its normalizer intentionally rejected `tx_as_of`, while downstream
source scans and result encoding were never refactored to consume one selected
frontier. The contract, request gate, source visibility and snapshot metadata
were therefore not connected end to end.

## Why the Issue Escaped Detection

Prior HQL2 regression suites used the default current frontier, and explicit
transaction-time tests asserted only rejection. Existing legacy ReadView
operations have independent temporal support/rejection rules, so their passing
P6 tests did not exercise P8's HQL2 binder, source scans, hydration or result
snapshot mapping.

## Prevention Implemented and Remaining Gates

Bind one selected frontier `S = tx_as_of.unwrap_or(pinned_generation_frontier)`
for the entire HQL/IR execution. Thread it through every revision-backed source,
property hydration, graph/vector/annotation path and HistoryScan/ChangeScan;
keep lease, catalog and current policy pinned to the validated generation;
encode `Snapshot.tx = S`; enforce global and per-source floors and never fall
back to current rows. HQL/IR parity tests now cover old and current
frontiers, source/revision visibility, graph traversal, annotation lookup,
vector and row floors, horizon/future bounds, exact `Snapshot.tx`, and no
fallback. Keep the focused five-target set and named 31-/11-target sweeps as
regression gates. The full P8/P13, transport, hosted-CI and independent-review
acceptance gates remain open; local tests do not substitute for them.
