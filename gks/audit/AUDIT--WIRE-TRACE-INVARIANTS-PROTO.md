---
id: AUDIT--WIRE-TRACE-INVARIANTS-PROTO
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: AUDIT — wire PROTO--SYMBOLS-TRACE-INVARIANTS to its predicate + fix
  ProtoStatus 'active'
tags:
  - msp
  - proto
  - validator
  - symbol-graph
  - audit
crosslinks:
  references:
    - PROTO--SYMBOLS-TRACE-INVARIANTS
    - ADR--SYMBOLS-PROCESS-TRACING
    - BLUEPRINT--PROTO-LOADER
    - AUDIT--PROTO-LINKED-SYMBOLS-PATH-DRIFT
phase_override:
  skip_blueprint: true
  reason: Wiring fix + a one-line type correction on the already-blueprinted PROTO
    loader scaffold (BLUEPRINT--PROTO-LOADER); no new code surface.
created_at: 2026-05-14T19:40:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  domain: audit
---

# AUDIT — wire [[PROTO--SYMBOLS-TRACE-INVARIANTS]]

## Scope

Follow-up from `[[AUDIT--PROTO-LINKED-SYMBOLS-PATH-DRIFT]]`, which left two
trace PROTOs not loading. This closes the tractable one.

`[[PROTO--SYMBOLS-TRACE-INVARIANTS]]` had a predicate file
(`packages/msp/src/validator/proto/trace-invariants.ts` — whose own
header comment names this atom as its owner) but the atom was never
wired to it: no `crosslinks.enforces`, no `linked_symbols`. The loader
requires both, so it was silently dropped.

## What shipped

| File | Change |
|---|---|
| `gks/proto/[[PROTO--SYMBOLS-TRACE-INVARIANTS]].md` | Added `crosslinks.enforces: [[[ADR--SYMBOLS-PROCESS-TRACING]]]` and `linked_symbols` → `packages/msp/src/validator/proto/trace-invariants.ts`. |
| `packages/msp/src/validator/proto/types.ts` | `ProtoStatus` now includes `'active'`. `normaliseStatus()` already accepted `'active'` and cast it, but the type omitted it — a latent inconsistency. |
| `packages/msp/src/validator/proto/loader.ts` | `runProtos()` `byStatus` record gains an `active: 0` key. Without it, an `active` PROTO produced `byStatus.active = undefined + 1 = NaN`. This atom is `status: active`, so the fix is load-bearing. |
| `packages/msp/test/validator/proto/loader.test.ts` | Four `byStatus` test literals updated for the new key. |

Loader count: **13 → 14** PROTOs discovered and run.

## Notes

- `trace-invariants.ts` is currently a structural-check **stub**
  (`return { ok: true, violations: [] }`). Wiring it does not add
  enforcement today — it makes the atom→code link real so that when the
  referential-integrity checks are implemented, the PROTO is already
  discovered. The predicate runs as `severity: warning` (no `severity:`
  field on the atom → loader default).

## Out of scope — [[PROTO--TRACE-INVARIANTS]]

`[[PROTO--TRACE-INVARIANTS]]` (note: no `SYMBOLS-` prefix) is a *separate*,
descriptive Thai-prose atom with the same title and overlapping scope
(acyclic / termination / referential-integrity rules). It has no
predicate and no `linked_symbols` — it is governance prose, not an
executable PROTO, so it is correctly not loaded.

It does, however, look like a **likely contradiction** with
`[[PROTO--SYMBOLS-TRACE-INVARIANTS]]`: same `type: proto`, same title,
overlapping rules, both non-superseded (`[[PROTO--TRACE-INVARIANTS]]` is
`stable`, `[[PROTO--SYMBOLS-TRACE-INVARIANTS]]` is `active`). Resolving that
— supersede one, or merge — is a contradiction-policy decision left to
a follow-up, not a mechanical wiring fix.

## Verification

- `msp:validate --all` — `[[PROTO--SYMBOLS-TRACE-INVARIANTS]]` now loads and
  runs; 14 PROTOs total.
- `npm run typecheck --workspace=packages/msp` and `--workspace=packages/gks` — clean.
- `test/validator/proto/` — 102 tests pass (loader suite 17/17).

## Connections
- [[BLUEPRINT--PROTO-LOADER]]

