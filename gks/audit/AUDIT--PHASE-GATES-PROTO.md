---
id: AUDIT--PHASE-GATES-PROTO
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M8b — PROTO--PHASE-GATES — enforce P0..P6 ordering at PR-time (ships as draft)
tags:
  - msp
  - proto
  - phase-gates
  - audit
  - m8b
crosslinks: {"references":["PROTO--PHASE-GATES","CONCEPT--PROTO-PHASE-GATES","FRAMEWORK--PHASE-GOVERNANCE","FEAT--PROTO-LOADER","BLUEPRINT--PROTO-LOADER","CONCEPT--PROTO-PATTERN"]}
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/phase-gates.ts"}
  - {"file":"gks/proto/PROTO--PHASE-GATES.md"}
  - {"file":"packages/msp/test/validator/proto/phase-gates.test.ts"}
  - {"file":"scripts/msp/propose.mjs"}
phase_override:
  skip_blueprint: true
  reason: "PROTO atom + its predicate are M8a self-similar; the M8a BLUEPRINT--PROTO-LOADER covers the loader contract this predicate plugs into"
created_at: 2026-05-05T18:00:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# M8b — [[PROTO--PHASE-GATES]] (draft)

## Scope

Implements `[[CONCEPT--PROTO-PHASE-GATES]]` as a PROTO predicate plugged into
the M8a loader. Mechanises the doc-to-code phase order from
`[[FRAMEWORK--PHASE-GOVERNANCE]]`.

## What shipped

| File | Purpose |
|---|---|
| `src/validator/proto/phase-gates.ts` | Predicate impl (default export) |
| `gks/proto/[[PROTO--PHASE-GATES]].md` | PROTO atom — `status: draft`, `severity: error`, enforces [[FRAMEWORK--PHASE-GOVERNANCE]] |
| `test/validator/proto/phase-gates.test.ts` | 9 tests covering the predicate |
| `gks/audit/[[AUDIT--PHASE-GATES-PROTO]].md` | This file |

## Rule (per [[CONCEPT--PROTO-PHASE-GATES]])

1. **Hard error** — every phase-5 / phase-6 atom (FEAT or AUDIT) that
   writes code via `linked_symbols` must be preceded by a phase-3
   BLUEPRINT atom whose `linked_symbols` covers at least one of the same
   files.
2. **Soft warning** — phase-2 ADR atoms with no phase-1 CONCEPT
   referenced via `crosslinks.references`.

## Escape hatch

Atoms can opt out of the BLUEPRINT requirement via frontmatter:

```yaml
phase_override:
  skip_blueprint: true
  reason: "out-of-scope code under examples/, hooks/, internal scripts"
```

The predicate reads each candidate's atom file from disk to inspect this
field (it's not present in `atomic_index.jsonl`).

## Ships as draft

`status: draft` in the PROTO atom means even `severity: error` violations
do **not** fail-exit (per the M8a loader's `shouldFailExit` rule). On the
real repo, the predicate currently flags:

- 3 phase-6 AUDIT atoms whose `linked_symbols` point to files outside
  the BLUEPRINT-covered set (msp_spec.md, scripts/msp/propose.mjs +
  test/scripts/propose.test.ts, .github/workflows/test.yml)
- 5 phase-2 ADRs that reference no CONCEPT-- via `crosslinks.references`
  (soft warnings)

These are surfaced for review but don't block CI. Promotion to
`status: stable` requires either adding the missing BLUEPRINTs or
opting out via `phase_override.skip_blueprint`.

## Decisions during impl

1. **Predicate reads disk for `phase_override`** — the AtomicIndexEntry
   shape doesn't carry `phase_override`. Reading the atom file is an
   acceptable cost (only the violators are read; healthy atoms never
   trigger disk I/O).

2. **Phase 5 OR 6 + type FEAT or AUDIT** — `[[CONCEPT--PROTO-PHASE-GATES]]`
   says "phase-5 atom (FEAT/AUDIT writing code)"; the prompt explicitly
   includes phase-6 AUDITs. In this repo FEATs sit at phase 2 and AUDITs
   at 5/6 — the predicate handles both.

3. **Coverage = "≥1 file in common"** — a BLUEPRINT can cover an AUDIT
   even if the AUDIT lists more `linked_symbols` (e.g. tests added
   later). The AUDIT just needs *some* overlap with a BLUEPRINT. This
   matches the spirit of [[CONCEPT--PROTO-PHASE-GATES]] ("a phase-3
   BLUEPRINT atom whose linked_symbols includes X").

4. **Soft-warning ADR check** — only inspects `crosslinks.references`,
   not `crosslinks.implements` etc. ADRs typically decide on a CONCEPT
   via `references`.

5. **No new runtime deps** — uses `yaml` (already a dep) and
   `node:fs/promises`.

## Acceptance criteria from prompt

- [x] Pass when every code-writing phase-5 atom has a backing phase-3 BLUEPRINT (test #3)
- [x] Fail when a phase-5 FEAT's linked_symbols isn't covered (test #4)
- [x] Honour `skip_blueprint` override (test #5)
- [x] Soft warning when ADR has no CONCEPT (test #6)
- [x] No phase-5 atoms → vacuously pass (test #2)
- [x] Empty atomicIndex → vacuously pass (test #1)
- [x] Plus 3 extra: ADR with CONCEPT passes, no linked_symbols ignored, multiple violators flagged independently

## Verification

- `npm test` → **507 passed** (was 498; +9 from new tests)
- `npm run typecheck` → clean
- `npx tsx src/validator/cli.ts --all` → 145/145 atoms pass; PROTO summary shows 1 passed (sample) + 1 failed (phase-gates draft, doesn't fail-exit); exits 0
- `npm run msp:check-links` → OK

## Counts

- Atoms 144 → 145 (+1: [[PROTO--PHASE-GATES]]) → 146 (+1: this AUDIT)
- Tests 498 → 507 (+9)

## Hard constraints honoured

- ❌ `src/validator/proto/loader.ts` — untouched
- ❌ `src/validator/proto/types.ts` — untouched
- ❌ `src/validator/proto/sample.ts` — untouched
- ❌ `src/validator/cli.ts` — untouched
- ❌ Other `gks/proto/` files — untouched
- ❌ `atomic_contract.yaml` — untouched
- ❌ No new runtime deps

## Source

`[[CONCEPT--PROTO-PHASE-GATES]]`, `[[FRAMEWORK--PHASE-GOVERNANCE]]`, `[[FEAT--PROTO-LOADER]]`, `[[BLUEPRINT--PROTO-LOADER]]`, `[[CONCEPT--PROTO-PATTERN]]`.
