---
id: AUDIT--ALGO-PARAM-COUPLING-PROTO
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M8d — PROTO--ALGO-PARAM-COUPLING acceptance audit
tags:
  - msp
  - m8
  - m8d
  - audit
  - proto
  - algo
  - param
  - governance
crosslinks: {"references":["PROTO--ALGO-PARAM-COUPLING","CONCEPT--PROTO-ALGO-PARAM-COUPLING","ADR--GRAPH-IS-GKS-DOMAIN","FRAMEWORK--CROSSLINKS-VOCABULARY","CONCEPT--PROTO-PATTERN"]}
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/algo-param-coupling.ts"}
  - {"file":"packages/msp/test/validator/proto/algo-param-coupling.test.ts"}
  - {"file":"gks/proto/PROTO--ALGO-PARAM-COUPLING.md"}
phase_override:
  skip_blueprint: true
  reason: "Incremental PROTO rule plugged into the already-blueprinted loader scaffold (BLUEPRINT--PROTO-LOADER). Doc-to-code chain: CONCEPT--PROTO-ALGO-PARAM-COUPLING -> PROTO--ALGO-PARAM-COUPLING -> predicate -> this audit; per-rule predicates do not each warrant a separate phase-3 blueprint."
created_at: 2026-05-05T18:11:00.000+07:00
---

# AUDIT — PROTO--ALGO-PARAM-COUPLING (M8d)

## Scope

Closes M8d Tier 2 governance PROTO. Implements the bi-directional
`tunes` ↔ `tunable_by` reciprocal validator declared in
`CONCEPT--PROTO-ALGO-PARAM-COUPLING`. Built atop the M8a PROTO loader
foundation; no shared infrastructure modified.

## Acceptance criteria from CONCEPT

| # | Criterion | Result |
|---|---|---|
| 1 | `tunable_by` values are `PARAM--*` ids (type-pairing) | done — predicate flags non-PARAM refs as severity:error |
| 2 | `tunes` values are `ALGO--*` ids (type-pairing) | done — predicate flags non-ALGO refs as severity:error |
| 3 | Reciprocal back-link present in partner atom | done — emits an error when partner is in index but lacks the reciprocal |
| 4 | Existence checks delegated to GKS (per `ADR--GRAPH-IS-GKS-DOMAIN`) | done — predicate skips reciprocal check when partner missing from index |
| 5 | Vacuous-pass on repos with zero ALGO/PARAM atoms | done — covered by test |
| 6 | Multi-PARAM ALGO has every reciprocal independently checked | done — covered by test |

## What shipped

| File | Purpose |
|---|---|
| `src/validator/proto/algo-param-coupling.ts` | Predicate impl; default-exports `Predicate` |
| `gks/proto/PROTO--ALGO-PARAM-COUPLING.md` | PROTO atom; `status: draft`, `severity: error`, enforces `FRAMEWORK--CROSSLINKS-VOCABULARY` |
| `test/validator/proto/algo-param-coupling.test.ts` | 7 tests (vacuous, ok, wrong-reciprocal, type-mismatch ×2, missing-partner, multi-PARAM) |

## Test summary

```
✓ test/validator/proto/algo-param-coupling.test.ts (7 tests) 5ms
```

All passing. Tests cover:

1. Vacuous pass when no ALGO/PARAM atoms exist
2. Happy path — ALGO ↔ PARAM mutually reciprocate
3. Wrong reciprocal (PARAM tunes points to a different ALGO) → error
4. Type mismatch — `tunable_by` references non-PARAM → error
5. Type mismatch — `tunes` references non-ALGO → error
6. Missing partner in index → no error from this PROTO (GKS owns existence)
7. Multi-PARAM ALGO — every reciprocal independently checked, only the
   broken one flags

## Status decision: shipped as `draft`

The repo currently has **zero `ALGO--*` and zero `PARAM--*` atoms**, so
this predicate is vacuously passing. Per the Tier 2 rollout plan, ship
as `draft` to:

- exercise the M8a loader against a non-trivial predicate
- give early warning when the first ALGO/PARAM atom lands without a back-link
- allow promotion to `stable` once observed in action

`shouldFailExit()` skips draft PROTOs, so this cannot regress CI.

## Boundary check (per ADR--GRAPH-IS-GKS-DOMAIN)

Existence (does `PARAM--X` actually exist?) is **NOT** checked here —
deliberately. `gks validate --links` runs against the canonical store
and catches missing crosslink targets. This PROTO only enforces the
type/reciprocal invariant when both ends are visible in the index.

Test #6 (`missing partner in index → no error from this PROTO`)
explicitly locks in this boundary so future "helpful" expansions can't
accidentally double-enforce.

## Non-goals (per CONCEPT)

- Existence enforcement (GKS)
- Migration of pre-existing un-paired atoms (none exist)
- Decision on whether `tunes` allows multi-ALGO (predicate is structurally
  multi-ALGO compatible by iterating, but the policy is not formalised)

## Source

- `CONCEPT--PROTO-ALGO-PARAM-COUPLING` (rule + scope)
- `ADR--GRAPH-IS-GKS-DOMAIN` (boundary)
- `FRAMEWORK--CROSSLINKS-VOCABULARY` (the FRAME this PROTO mechanises)
- M8a PROTO loader (`src/validator/proto/loader.ts`) — unchanged
