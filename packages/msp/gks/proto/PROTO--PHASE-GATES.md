---
id: PROTO--PHASE-GATES
phase: 2
type: proto
status: draft
severity: error
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--PHASE-GATES — enforce P0..P6 phase ordering at PR-time (M8b)
tags:
  - msp
  - proto
  - phase-gates
  - governance
  - m8b
crosslinks: {"enforces":["FRAME--PHASE-GOVERNANCE"],"references":["CONCEPT--PROTO-PHASE-GATES","CONCEPT--PROTO-PATTERN","FEAT--PROTO-LOADER"]}
linked_symbols:
  - {"file":"src/validator/proto/phase-gates.ts"}
created_at: 2026-05-05T11:00:00.000Z
---

# PROTO — PHASE-GATES

## Rule

Per `FRAME--PHASE-GOVERNANCE`, the doc-to-code chain is

```
P0 FRAME → P1 CONCEPT → P2 ADR/FEAT → P3 BLUEPRINT → P5 CODE → P6 AUDIT
```

This PROTO mechanises two checks:

1. **Hard error** — every phase-5 / phase-6 atom (FEAT or AUDIT) that writes
   code via `linked_symbols` must be preceded by a phase-3 BLUEPRINT atom
   whose own `linked_symbols` covers at least one of the same files.
2. **Soft warning** — phase-2 ADR atoms that reference no phase-1 CONCEPT
   via `crosslinks.references` get a non-blocking warning.

## Schema

Reads `atomic_index.jsonl` (`PredicateContext.atomicIndex`) for phase + type
+ `linked_symbols`. Reads each candidate atom's source file from disk to
inspect optional `phase_override.skip_blueprint: true` escape hatch.

## Predicate

```ts
// pseudo-shape (real impl in linked_symbols)
for (const a of index) {
  if ((a.phase === 5 || a.phase === 6) && (a.type === 'feat' || a.type === 'audit') && hasFiles(a)) {
    if (!anyBlueprintCoversAny(a.linked_symbols)) {
      if (!await skipBlueprintOverride(a)) {
        emit error: '<id> writes code but has no phase-3 BLUEPRINT'
      }
    }
  }
  if (a.phase === 2 && a.type === 'adr' && !referencesAnyConcept(a)) {
    emit warning: '<id> ADR has no CONCEPT-- referenced'
  }
}
```

Implementation: `src/validator/proto/phase-gates.ts`.

## Trigger

`msp:validate --all` (after the regular validator rules and the M8a
loader's `PROTO--SAMPLE-RULE`).

## Severity

`error` — but `status: draft` means even errors don't fail-exit while we
gradually roll out. Promote to `stable` once existing repo atoms pass
clean (or are explicitly opted out via `phase_override`).

## Escape hatch

A phase-5/6 atom can opt out of the BLUEPRINT requirement by adding to
its frontmatter:

```yaml
phase_override:
  skip_blueprint: true
  reason: "out-of-scope code under examples/, hooks/, etc."
```

Intended for tiny code paths (hook fixtures, examples, internal scripts)
where a full BLUEPRINT would be overkill.

## Status

`draft` — predicate runs, prints violations, but does not fail CI. Real
governance once we audit existing atoms and either add the missing
BLUEPRINTs or opt them out via `phase_override`.

## Source

`CONCEPT--PROTO-PHASE-GATES`, `FRAME--PHASE-GOVERNANCE`, `CONCEPT--PROTO-PATTERN`.
