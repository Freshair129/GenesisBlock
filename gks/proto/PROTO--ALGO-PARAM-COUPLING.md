---
id: PROTO--ALGO-PARAM-COUPLING
phase: 2
type: proto
status: stable
severity: error
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--ALGO-PARAM-COUPLING — bi-directional tunes ↔ tunable_by reciprocal validator
tags:
  - msp
  - proto
  - algo
  - param
  - coupling
  - governance
  - m8d
crosslinks: {"enforces":["FRAMEWORK--CROSSLINKS-VOCABULARY"],"references":["CONCEPT--PROTO-ALGO-PARAM-COUPLING","CONCEPT--PROTO-PATTERN","ADR--GRAPH-IS-GKS-DOMAIN"]}
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/algo-param-coupling.ts"}
created_at: 2026-05-05T18:11:00.000+07:00
---

# PROTO — ALGO-PARAM-COUPLING

## Rule

For every atom with `crosslinks.tunable_by: [...]`:

1. Each value MUST be a `PARAM--*` id.
2. If the referenced PARAM is present in the atomic index, it MUST declare
   `crosslinks.tunes: [..., <this atom's id>, ...]`.

Mirror, for every atom with `crosslinks.tunes: [...]`:

1. Each value MUST be an `ALGO--*` id.
2. If the referenced ALGO is present, it MUST declare
   `crosslinks.tunable_by: [..., <this atom's id>, ...]`.

**Existence checks are out of scope** (per `[[ADR--GRAPH-IS-GKS-DOMAIN]]`):
if the partner atom is missing from the index, GKS's `validate --links`
catches that. This PROTO only enforces type-pairing + reciprocal coupling
when both ends are visible in the index.

## Schema

Reads `atomic_index.jsonl` from the validator's `PredicateContext`.
Inspects `crosslinks.tunable_by` and `crosslinks.tunes` on every atom.
No additional file I/O.

## Predicate

```ts
for (const atom of ctx.atomicIndex) {
  for (const target of atom.crosslinks?.tunable_by ?? []) {
    if (!target.startsWith('PARAM--')) violations.push({...})
    const partner = ctx.atomicIndex.find(a => a.id === target)
    if (partner && !(partner.crosslinks?.tunes ?? []).includes(atom.id)) {
      violations.push({ message: 'reciprocal tunes missing', severity: 'error' })
    }
  }
  // mirror for crosslinks.tunes → ALGO--
}
```

Implementation: `src/validator/proto/algo-param-coupling.ts`.

## Trigger

`msp:validate --all` (runs after the regular validator rules through the
PROTO loader from M8a).

## Severity

`error` — broken coupling means a documented tunable doesn't have a
reciprocal back-link, signalling either a missed update or a typo in
the linked id.

## Status

`draft` — ship as draft per the Tier 2 rollout plan. The repo currently
has zero `ALGO--*` and zero `PARAM--*` atoms, so this predicate is
**vacuously passing**. Promote to `stable` once the first ALGO/PARAM
atom pair lands and the predicate is observed catching real drift.

## Source

`[[CONCEPT--PROTO-ALGO-PARAM-COUPLING]]`, `[[ADR--GRAPH-IS-GKS-DOMAIN]]`,
`[[FRAMEWORK--CROSSLINKS-VOCABULARY]]`.

## Connections
- [[CONCEPT--PROTO-PATTERN]]

