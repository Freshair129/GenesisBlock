# RCA: HQL2 P7 Edge Oracle Missing Endpoint Revisions

## Status / Date

Root cause confirmed; fixture correction verified / 2026-10-06.

## Symptom

The focused command
`cargo test --locked --offline --no-default-features --jobs 1 --target-dir target/hql2-execution --test hql2_storage_source_tests`
reported 9 passed and 1 failed. In
`hql_and_ir_source_scans_have_identical_node_edge_and_row_results`, HQL returned
the expected `edge:depends` EntityRef while the P7 expected bag was empty.

## Evidence

- `tests/hql2_storage_source_tests.rs:107-125` builds the P7 catalog from rows
  selected only for the requested record kind. For an Edge scan, it included the
  edge revision but not its Node endpoint revisions.
- `tests/support/hql2_graph_reference.rs:686-696` resolves an identity only by
  searching revisions present in the catalog.
- `tests/support/hql2_graph_reference.rs:777-782` makes an edge active only when
  both endpoint identities resolve.
- The failing run printed one current P7 Edge revision (`edge:depends`, relation
  `DEPENDS`, transaction 5) and frontier 8, but the oracle returned no rows.
  HQL and typed IR both returned that same edge.

## Root Cause

The test adapter gave the P7 oracle an incomplete catalog for Edge scans. It
declared Node history capability but omitted the endpoint Node revisions needed
by the oracle's documented edge-visibility dependency check. P7 therefore
correctly filtered the edge as inactive; this was a fixture-construction defect,
not an HQL2 runtime defect.

## Why the Issue Escaped Detection

Earlier source-scan evidence covered Node identity against P7 and exercised Edge
filtering through HQL tests, but did not compare an Edge scan to P7 with its
endpoint revisions present. The incomplete adapter was introduced when extending
the source oracle to Edge and Row kinds.

## Fix (Decided)

For the P7 Edge oracle only, include current Node revisions alongside current
Edge revisions in the catalog. Keep the scan source restricted to Edge, so Node
records are dependencies and never become expected scan rows. Do not change
production code or the frozen query contract.

## Outcome (Measured)

After including Node revisions as Edge dependencies while retaining an Edge-only
scan source, the exact reproduction passed 1/1. The four modified HQL2 targets
then passed 32/32 combined. No production code or query contract changed.

## Proposed Prevention

When constructing an independent catalog for a source kind with structural
dependencies, include the dependency revisions required by the oracle while
keeping the source operator's selected kind explicit. Retain focused differential
coverage for both membership and full EntityRef identity.
