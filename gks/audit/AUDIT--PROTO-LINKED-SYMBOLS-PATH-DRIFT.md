---
id: AUDIT--PROTO-LINKED-SYMBOLS-PATH-DRIFT
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: AUDIT — PROTO linked_symbols path drift — 11 predicates rewired into
  msp:validate
tags: &a1
  - msp
  - proto
  - validator
  - monorepo
  - audit
crosslinks: &a2
  references:
    - BLUEPRINT--PROTO-LOADER
    - FEAT--PROTO-LOADER
    - CONCEPT--PROTO-PATTERN
    - ADR--MONOREPO-STRUCTURE
created_at: 2026-05-14T19:10:00.000+07:00
aliases: &a3
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--PROTO-LINKED-SYMBOLS-PATH-DRIFT
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: AUDIT — PROTO linked_symbols path drift — 11 predicates rewired into
    msp:validate
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T19:10:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--PROTO-LINKED-SYMBOLS-PATH-DRIFT
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: AUDIT — PROTO linked_symbols path drift — 11 predicates rewired into
      msp:validate
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T19:10:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      domain: audit
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — PROTO linked_symbols path drift

## Scope

The PROTO loader (`packages/msp/src/validator/proto/loader.ts`) resolves a
PROTO's predicate via `linked_symbols[0].file`, relative to the repo root.
A PROTO whose predicate file does not resolve is **silently dropped** — it
never runs, and `msp:validate --all` is none the wiser.

Discovered while wiring `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]`: only 2 of 16
PROTO atoms were actually loading. The other 14 carried stale
pre-monorepo `linked_symbols` paths (`src/validator/proto/…`) — correct
back when the validator ran from `packages/msp/`, wrong since the
2026-05-11 monorepo migration (`[[ADR--MONOREPO-STRUCTURE]]`) made the repo
root the working directory.

## What shipped

11 PROTO atoms had their `linked_symbols[0].file` prefixed with
`packages/msp/` so the loader resolves them from repo root:

`[[PROTO--ADR-MONOTONIC]]`, `[[PROTO--ALGO-PARAM-COUPLING]]`,
`[[PROTO--AUTHORITY-ENFORCEMENT]]`, `[[PROTO--EVIDENCE-FOR-DECISIONS]]`,
`[[PROTO--MASTER-BODY-SCHEMA]]`, `[[PROTO--MASTER-TOKEN-CAP]]`,
`[[PROTO--PHASE-GATES]]`, `[[PROTO--SAMPLE-RULE]]`,
`[[PROTO--SCALING-LEVEL-GATE]]`, `[[PROTO--SUMMARY-MIN]]`, `[[PROTO--VALID-UNTIL]]`.

Each predicate file already existed at the corrected path and exports a
valid `default` predicate; each atom already had a non-empty
`crosslinks.enforces`. The path was the only thing broken.

Loader count: **2 → 13** PROTOs now discovered and run (the 11 above plus
the 2 that were already correct: `[[PROTO--AUTO-GENERATED-MARKER]]`,
`[[PROTO--SCALE-LEVEL-GATE]]`, and `[[PROTO--SYMBOLS-FRAMEWORK-INVARIANTS]]`).

## CI impact

None. Every newly-loaded PROTO is `status: draft`, and the loader's
`shouldFailExit()` only fail-exits on a `stable` PROTO with a
`severity: error` violation. Draft PROTOs run, report, and never block —
per the gradual-rollout policy in `[[BLUEPRINT--PROTO-LOADER]]`.

## Findings surfaced

With `[[PROTO--PHASE-GATES]]` now loading, it reports 4 real violations that
were previously invisible — four phase-6 audit atoms write code via
`linked_symbols` but no phase-3 BLUEPRINT covers those files:

- `[[AUDIT--ALGO-PARAM-COUPLING-PROTO]]`
- `[[AUDIT--AUTHORITY-ENFORCEMENT-PROTO]]`
- `[[AUDIT--SCALING-LEVEL-GATE-PROTO]]`
- `[[AUDIT--VALID-UNTIL-PROTO]]`

These are genuine doc-to-code governance gaps, not regressions — the
PROTO is doing its job. Resolution (add a covering BLUEPRINT, or set
`phase_override.skip_blueprint: true` on each audit) is a follow-up; the
draft PROTO surfaces them without blocking.

## Out of scope

- `[[PROTO--TRACE-INVARIANTS]]` and `[[PROTO--SYMBOLS-TRACE-INVARIANTS]]` are
  still not loaded — they lack `crosslinks.enforces` and `linked_symbols`
  entirely (descriptive PROTOs, no wired predicate). Wiring them is a
  separate judgement call about predicate ownership of
  `trace-invariants.ts`, not mechanical path drift.
- The 4 phase-gates findings above.

## Verification

- `npx tsx packages/msp/src/validator/cli.ts --root=. --all` — 13 PROTOs
  load; 12 pass, 1 (`PHASE-GATES`) reports the 4 findings above; no
  fail-exit (all draft).
- All 11 edited atoms validate (`msp:validate`).
- No source code changed — only `linked_symbols` frontmatter.

## Connections
- [[FEAT--PROTO-LOADER]]
- [[CONCEPT--PROTO-PATTERN]]

