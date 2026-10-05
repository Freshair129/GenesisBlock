# RCA: HQL1 Property Projection Was Partially Replayed

## Status / Date

Root cause confirmed; correction verified / 2026-10-05.

## Symptom

Both tests in `hql2_hql1_property_projection_tests` fail after replaying the
D7 extension onto mainline. A valid `MATCH (n) RETURN n.prop.score` returns the
node ID as `Utf8` instead of the JSON property value, while a labeled property
projection that must fail closed returns rows successfully.

## Evidence

- `lower_hql_v1` recognizes `HqlField::Prop` and sets `property_projection` at
  `src/query/hql2/legacy.rs:149-154`.
- The zero-hop branch still unconditionally lowers to
  `RETURN __hql1_node.id AS id` at line 179 and applies the legacy label and
  ID-filter paths without rejecting them for property projections.
- Focused replay verification:
  `cargo test --locked --offline --no-default-features --test
  hql2_hql1_property_projection_tests` — 0 passed, 2 failed; one observed
  `Utf8("node:a")`, and the fail-closed case unexpectedly returned `Ok(Rows)`.

## Root Cause

Conflict resolution kept mainline's original zero-hop ID projection while
retaining the incoming property-projection discriminator. It did not integrate
the corresponding JSON `prop(...)` lowering or the property-only shape guards,
so the new discriminator was unused by the zero-hop output path.

## Why the Issue Escaped Detection

The D7 property tests were introduced on the feature branch and had not run
against the conflict-resolved mainline source. The merge replay therefore
compiled before exercising the property output type and fail-closed boundary.

## Proposed Prevention

For the single-property form, require an unlabeled zero-hop scan with no
predicate, ordering, limit, temporal selector or additional projection; emit
`prop(__hql1_node, <JSON-escaped key>)`. Keep the existing ID path unchanged,
then rerun the focused property and adapter targets.

## Outcome

RED reproduced as described above. After adding the property-specific shape
guards and JSON-escaped `prop(...)` lowering, the focused property target passes
2/2; the adapter target passes 13/13 and the explicit root HQL2 sweep passes
404/0/1 across 39 targets. Existing ID projection and one-hop ordering targets
also pass 2/2 each.
