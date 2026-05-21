---
id: PROTO--SCALING-LEVEL-GATE
phase: 2
type: proto
status: draft
severity: warning
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--SCALING-LEVEL-GATE — enforce L1/L2/L3 chain consistency on FEAT
  atoms (M8c)
tags: &a1
  - msp
  - proto
  - scaling-levels
  - governance
  - m8c
crosslinks: &a2
  enforces:
    - FRAMEWORK--SCALING-LEVELS
  references:
    - CONCEPT--PROTO-SCALING-LEVEL-GATE
    - FEAT--PROTO-LOADER
linked_symbols: &a3
  - file: packages/msp/src/validator/proto/scaling-level-gate.ts
created_at: 2026-05-05T18:00:00.000+07:00
aliases: &a4
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  id: PROTO--SCALING-LEVEL-GATE
  phase: 2
  type: proto
  status: draft
  severity: warning
  vault_id: default
  tier: safety
  source_type: axiomatic
  title: PROTO--SCALING-LEVEL-GATE — enforce L1/L2/L3 chain consistency on FEAT
    atoms (M8c)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-05T18:00:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Machine-enforced invariant
  attributes:
    id: PROTO--SCALING-LEVEL-GATE
    phase: 2
    type: proto
    status: draft
    severity: warning
    vault_id: default
    tier: safety
    source_type: axiomatic
    title: PROTO--SCALING-LEVEL-GATE — enforce L1/L2/L3 chain consistency on FEAT
      atoms (M8c)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-05T18:00:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Machine-enforced invariant
    attributes:
      id: PROTO--SCALING-LEVEL-GATE
      phase: 2
      type: proto
      status: draft
      severity: warning
      vault_id: default
      tier: safety
      source_type: axiomatic
      title: PROTO--SCALING-LEVEL-GATE — enforce L1/L2/L3 chain consistency on FEAT
        atoms (M8c)
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-05T18:00:00.000+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Machine-enforced invariant
      attributes:
        domain: proto
      domain: proto
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: proto
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: proto
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# PROTO — SCALING-LEVEL-GATE

## Rule

Each `FEAT--*` atom must declare an atomic chain that matches its implied
Scaling Level (per `[[FRAMEWORK--SCALING-LEVELS]]`):

- **L2** (default for FEATs): at least one `CONCEPT--*` AND one `ADR--*`
  appears in the FEAT's `crosslinks.references` or `crosslinks.implements`.
- **L3** (FEATs that touch core / multi-module surfaces, signalled by an
  explicit `level: 'L3'` in the FEAT frontmatter OR by a `BLUEPRINT--*` atom
  participating in the chain): the L2 chain plus a `BLUEPRINT--*` linkage —
  either listed in the FEAT's crosslinks, or declared by a BLUEPRINT atom
  that references the FEAT.
- **L1**: no chain expectation; reachable only via explicit
  `level_override: 'L1'` (escape hatch).

## Schema

Reads `atomicIndex: AtomicIndexEntry[]` from the validator's
`PredicateContext`. No git diff, no file system I/O — this PROTO performs a
*structural* check at the gks/ level. The full PR-time classifier (which
inspects the actual diff to auto-detect L1/L2/L3 from line/file counts) is
out of scope for the predicate domain and would land later as a CI workflow.

Optional FEAT frontmatter fields the predicate respects:

- `level: 'L1' | 'L2' | 'L3'` — explicit tag, forces the expectation.
- `level_override: 'L1' | 'L2' | 'L3'` — escape hatch (takes precedence).

## Predicate

```ts
for (const feat of atomicIndex.filter(a => a.type === 'feat')) {
  const hits = classifyChain(atomicIndex, feat) // { hasConcept, hasAdr, hasBlueprint }
  const expected = decideExpectedLevel(feat, hits) // L1 | L2 | L3
  if (expected === 'L1') continue
  // missing = required atoms minus hits → emit warning
}
```

Implementation: `src/validator/proto/scaling-level-gate.ts`.

## Trigger

`msp:validate --all` (runs after the regular validator rules, alongside
other PROTO predicates).

## Severity

`warning` — non-blocking. Stays draft while we audit the existing FEAT
inventory; promote to `stable` once the corpus is clean.

## Status

`draft` — gradual rollout. Violations surface in validator output but do
not fail-exit per the PROTO loader's draft policy.

## Out of scope

- PR-diff-based L1/L2/L3 auto-classification (would be a CI workflow,
  potentially M8c-2).
- Heuristic threshold tuning (touch-count, line-count) — see
  `[[CONCEPT--PROTO-SCALING-LEVEL-GATE]]` for thresholds and the
  `[[PARAM--SCALING-LEVEL-THRESHOLDS]]` future hook.
- AUDIT atom presence (a separate PROTO will gate AUDIT chain coverage).

## Source

`[[FRAMEWORK--SCALING-LEVELS]]`, `[[CONCEPT--PROTO-SCALING-LEVEL-GATE]]`,
`[[CONCEPT--MSP-ROADMAP]]` §2 M8c.

## Connections

- [[FEAT--PROTO-LOADER]]
