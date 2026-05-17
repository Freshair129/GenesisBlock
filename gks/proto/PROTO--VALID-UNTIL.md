---
id: PROTO--VALID-UNTIL
phase: 2
type: proto
status: draft
severity: warning
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--VALID-UNTIL — decision atrophy guard (scan valid_until fields)
tags:
  - msp
  - proto
  - lifecycle
  - valid-until
  - atrophy
  - m9a
crosslinks:
  enforces:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
  references:
    - CONCEPT--DECISION-ATROPHY-GUARDS
    - FEAT--PROTO-LOADER
linked_symbols:
  - file: packages/msp/src/validator/proto/valid-until.ts
created_at: 2026-05-05T18:10:00.000+07:00
aliases:
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  domain: proto
---

# PROTO — VALID-UNTIL

## Rule

For every atom in the index whose frontmatter declares a `valid_until:` ISO 8601 date:

- If `now > valid_until` → emit a `warning` violation (`atom X expired Y days ago`).
- If `valid_until - 30 days < now <= valid_until` → emit an `info` violation (`atom X expires in N days`).
- Atoms without a `valid_until` field are skipped.
- Atoms with `status: superseded` are skipped (already known to be obsolete).

## Schema

Reads the atomic index from `PredicateContext`, then opens each atom file under `gks/<path>` to parse its YAML frontmatter (the index does not always carry `valid_until`). No mutation, pure scan.

## Predicate

```ts
for (const atom of ctx.atomicIndex) {
  if (atom.status === 'superseded') continue
  const fm = parseFrontmatter(await readFile(resolve(ctx.repoRoot, 'gks', atom.path)))
  const validUntil = fm?.valid_until
  if (typeof validUntil !== 'string') continue
  const expiry = new Date(validUntil)
  const daysUntilExpiry = Math.floor((expiry.getTime() - now.getTime()) / 86_400_000)
  if (daysUntilExpiry < 0) violations.push({ severity: 'warning', message: `expired ${-daysUntilExpiry} days ago` })
  else if (daysUntilExpiry < 30) violations.push({ severity: 'info', message: `expires in ${daysUntilExpiry} days` })
}
```

Implementation: `src/validator/proto/valid-until.ts`.

## Time injection

Tests inject "now" via the `MSP_NOW` environment variable (any ISO 8601 string parseable by `new Date()`). The predicate signature does not accept options, so an env var override is the least-intrusive seam. Default is real wall clock.

## Trigger

`msp:validate --all` (alongside other PROTO predicates). Optionally a weekly CI cron — see `[[CONCEPT--DECISION-ATROPHY-GUARDS]]` Guard 1.

## Severity

`warning` (PROTO-level) — atrophy is a process concern, not a CI fail. Individual violations may be `warning` (expired) or `info` (near expiry); neither fails-exit, even when this PROTO is later promoted to `stable`.

## Status

`draft` — real atoms in this repo (e.g. `[[CONCEPT--MSP-ROADMAP]]` with `valid_until: 2026-08-01`) may already be expired or near-expiring. Shipping as draft so the loader reports them without breaking CI. Promote to `stable` after the team has reviewed and either updated or superseded the affected atoms.

## Source

`[[CONCEPT--DECISION-ATROPHY-GUARDS]]`, `[[FEAT--PROTO-LOADER]]`.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

