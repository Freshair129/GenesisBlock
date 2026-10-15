# RCA: HQL2 Vector HistoryScan Is Not Exposed

## Status / Date

Root cause confirmed; approved vector HistoryScan slice implemented and locally verified / 2026-10-03.

## Symptom

H2-D11 persists vector records with durable revisions, but HQL2 `HISTORY VECTOR`
and typed-IR `HistoryScan` with kind `vector` cannot read those revisions.

## Evidence

- `src/query/hql2/hql2.pest` reuses `kind` for history, annotation, and retraction;
  the current kind alternatives omit vector.
- `src/query/hql2/wire.rs` `EntityKind` omits Vector, and both HQL lowering and
  typed-IR binding therefore cannot produce a vector HistoryScan.
- `src/lib.rs` `hql2_history_scan` routes Node/Edge to the graph floor, Row to the
  row floor, and Annotation to the annotation floor, but explicitly rejects Vector.
- The existing `hql2_revision_subject_readable` already maps a vector revision to
  its owner Node ACL; the gap is source exposure and floor selection, not a missing
  vector persistence or ACL primitive.
- H2-D11 defines vector revision IDs as the compact JSON `(owner_id,
  collection_id)` tuple, and the P6/P8 addendum now records that key and the vector
  source floor as the approved HistoryScan contract.
- The grammar-change resource gate required a hash re-pin. The new
  `history_kind` replaces the same single rule wrapper only on the HistoryScan
  path; it adds no recursion and leaves the existing lexical-unit, AST-layout,
  and Pest dependency bounds unchanged. An isolated allocator probe for the
  escaped vector identity query is added alongside the pinned adversarial cases.

## Root Cause

HistoryScan was implemented for Node, Edge, Row, and Annotation before vector
history was included in its source allowlist. The omission spans the HQL grammar,
typed wire/lowering/binding, and storage floor dispatch; durable vector writes and
ChangeScan support were added independently, leaving no end-to-end vector read
path.

## Why the Issue Escaped Detection

Existing HistoryScan tests covered retained node revisions, annotation behavior,
source floors, and HQL/IR parity, while vector-specific tests covered durable
vector revision writes and ChangeScan events. No test joined those surfaces by
requesting a vector revision through HistoryScan.

## Prevention Implemented

Keep the history-only kind grammar separate from mutation `kind`, pin and probe
its bounded parser path, and retain HQL/typed-IR parity using the canonical
H2-D11 tuple identity. The existing P6 vector floor and owner-node ACL path
remain enforced. The subsequent transaction-time slice also checks that the
selected frontier cannot precede the vector floor.

## Outcome (Measured)

- RED: the new HQL vector-history case failed at `HQL_PARSE_ERROR`; the other
  10 tests in the focused target passed before implementation.
- GREEN at the vector-slice checkpoint: the focused HistoryScan/ChangeScan
  target passed 12/12, the explicit 31-target HQL2 regression sweep passed
  367/0/1, and the separate 11-target P6/schema-v6/compatibility sweep passed
  194/0/0.
- Latest related regression after transaction-time integration: focused
  HistoryScan/ChangeScan passes 14/14, five transaction-time focused targets
  pass 56/56, the explicit 31-target HQL2 sweep passes 373/0/1, and the
  separate 11-target P6/schema-v6/compatibility sweep passes 194/0/0.
- The isolated parser allocation probe measured the escaped vector-history
  query at 7,418 bytes peak with default diagnostics and 8,050 bytes with
  detailed diagnostics, against a 4,588,672-byte reservation. All existing
  wide-input, depth, Unicode and pinned-fixture probes passed after grammar
  re-pinning.
- No database migration or transport/release change was made. HQL2/IR
  `tx_as_of` is now locally verified through one no-fallback frontier; hosted
  CI for this revision, broad P8/P13 acceptance and independent review remain
  open. The protected probe was not selected or modified.
