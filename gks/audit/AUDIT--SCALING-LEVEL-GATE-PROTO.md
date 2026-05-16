---
id: AUDIT--SCALING-LEVEL-GATE-PROTO
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M8c — PROTO--SCALING-LEVEL-GATE structural FEAT-chain check
tags:
  - msp
  - proto
  - scaling-levels
  - governance
  - audit
  - m8c
crosslinks: {"references":["PROTO--SCALING-LEVEL-GATE","CONCEPT--PROTO-SCALING-LEVEL-GATE","FRAMEWORK--SCALING-LEVELS","FEAT--PROTO-LOADER"]}
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/scaling-level-gate.ts"}
  - {"file":"gks/proto/PROTO--SCALING-LEVEL-GATE.md"}
  - {"file":"packages/msp/test/validator/proto/scaling-level-gate.test.ts"}
phase_override:
  skip_blueprint: true
  reason: "Incremental PROTO rule plugged into the already-blueprinted loader scaffold (BLUEPRINT--PROTO-LOADER). Doc-to-code chain: CONCEPT--PROTO-SCALING-LEVEL-GATE -> PROTO--SCALING-LEVEL-GATE -> predicate -> this audit; per-rule predicates do not each warrant a separate phase-3 blueprint."
created_at: 2026-05-05T18:00:00.000+07:00
---

# M8c — [[PROTO--SCALING-LEVEL-GATE]]

## Scope

Implements `[[PROTO--SCALING-LEVEL-GATE]]` per `[[CONCEPT--PROTO-SCALING-LEVEL-GATE]]`,
shipping as `status: draft`. This is one of four parallel governance PROTOs
following M8a's loader foundation; it does NOT modify any shared infrastructure.

## What shipped

| File | Purpose |
|---|---|
| `src/validator/proto/scaling-level-gate.ts` | Predicate impl — `default export` follows the M8a loader contract |
| `gks/proto/[[PROTO--SCALING-LEVEL-GATE]].md` | Atom (`status: draft`, `severity: warning`, `enforces: [[FRAMEWORK--SCALING-LEVELS]]`) |
| `test/validator/proto/scaling-level-gate.test.ts` | 7 vitest cases covering full chain, missing CONCEPT/ADR, missing BLUEPRINT, `level_override` escape hatch, BLUEPRINT backlink, no-FEAT, empty index |

## Predicate domain

PROTO predicates run via `npm run msp:validate --all`. They receive
`atomicIndex: AtomicIndexEntry[]` + `repoRoot` and have no access to git
diff state. The full PR-time L1/L2/L3 classifier described in
`[[CONCEPT--PROTO-SCALING-LEVEL-GATE]]` is therefore deferred to a future CI
workflow (potentially M8c-2).

This PROTO instead performs the **structural** half of the rule: for each
FEAT atom in the index, walk its `crosslinks.references` ∪
`crosslinks.implements` and confirm:

- L2 (default): at least one CONCEPT and one ADR linked.
- L3 (signalled by an explicit `level: 'L3'` in the FEAT, or by a BLUEPRINT
  participating in the chain): same as L2 plus a BLUEPRINT linked (either
  directly from the FEAT, or backlinked from a BLUEPRINT atom).
- `level_override: 'L1' | 'L2' | 'L3'` is the escape hatch and takes
  precedence.

## Verification

```
npx vitest run test/validator/proto/scaling-level-gate.test.ts
  → 7 passed

npm run typecheck
  → clean

npm run msp:index
  → 145 atoms indexed (PROTO--SCALING-LEVEL-GATE included)

npx tsx src/validator/cli.ts --all
  → 145 atoms passed
  → PROTO--SAMPLE-RULE [draft, warning] passed
  → PROTO--SCALING-LEVEL-GATE [draft, warning] surfaces 1 warning on existing
    FEAT--MSP-MCP-TOOL-EXPANSION (missing ADR linkage) — non-blocking due to
    draft status
  → exit 0 (draft PROTOs never fail-exit per M8a loader policy)

npm run msp:check-links
  → status: OK

npm test
  → 505 tests passed (67 files)
```

The single warning surfaced against the existing corpus is real signal —
that FEAT genuinely lacks an ADR crosslink — not a predicate bug. Cleaning
that up is out of scope for this PROTO; it can be addressed when the
PROTO promotes from `draft` to `stable`.

## Out of scope

- PR-diff line/file count classification (CI workflow concern).
- Heuristic threshold tuning ([[PARAM--SCALING-LEVEL-THRESHOLDS]] — future).
- AUDIT chain coverage (separate PROTO).
- Promoting to `stable` (gradual rollout — happens after the corpus is
  audited and any real violations are fixed).

## Hard constraints honoured

- ❌ Did NOT modify `src/validator/proto/{loader.ts,types.ts,sample.ts}`,
  `src/validator/cli.ts`, or any other shared infrastructure.
- ❌ Did NOT modify any other PROTO atoms or impls.
- ❌ Did NOT add runtime dependencies.
- ✅ Shipped as `status: draft`.
- ✅ No git-diff classification; predicate is pure-from-index.

## Source

`[[CONCEPT--PROTO-SCALING-LEVEL-GATE]]`, `[[CONCEPT--PROTO-PATTERN]]`,
`[[FRAMEWORK--SCALING-LEVELS]]`, `[[FEAT--PROTO-LOADER]]`.

## Connections
- [[BLUEPRINT--PROTO-LOADER]]

