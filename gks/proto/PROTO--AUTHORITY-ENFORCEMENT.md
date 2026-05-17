---
id: PROTO--AUTHORITY-ENFORCEMENT
phase: 2
type: proto
status: draft
severity: error
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--AUTHORITY-ENFORCEMENT — sanity-check `.brain/msp/authority.yaml`
tags:
  - msp
  - proto
  - authority
  - tier
  - governance
  - m8e
crosslinks: {"enforces":["FRAMEWORK--AUTHORITY-MATRIX"],"references":["CONCEPT--PROTO-AUTHORITY-ENFORCEMENT","ADR--DELEGATION-POLICY","FEAT--PROTO-LOADER"]}
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/authority-enforcement.ts"}
created_at: 2026-05-05T18:10:00.000+07:00
aliases:
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: "Machine-enforced invariant"
---

# PROTO — AUTHORITY-ENFORCEMENT

## Rule

If a project opts into the tier model by checking in `.brain/msp/authority.yaml`,
the file must be well-formed and internally consistent:

- top-level mapping with `tiers` (T1/T2/T3 → string[]) and `allowed_paths`
  (T1/T2/T3 → string[])
- tier membership sets are disjoint (no user in two tiers)
- every entry under `tiers.<T>` and `allowed_paths.<T>` is a non-empty string
- each tier's `allowed_paths` includes at least one entry referencing the
  candidates queue (so T1 has somewhere legal to write — per
  `[[ADR--AGENT-WRITE-BOUNDARIES]]`)

If the file is absent the predicate passes vacuously — projects can opt in
when they're ready.

## Schema

Reads `<repoRoot>/.brain/msp/authority.yaml` directly. No atomic-index
inspection; the matrix is config data, not a knowledge atom. Predicate uses
`node:fs/promises.readFile` + `yaml.parse` (already a dep).

## Predicate

```ts
const predicate: Predicate = async (ctx) => {
  const path = resolve(ctx.repoRoot, '.brain/msp/authority.yaml')
  // ENOENT → vacuous pass
  // bad YAML → error
  // shape problems → list violations
}
```

Implementation: `src/validator/proto/authority-enforcement.ts`.

## Trigger

`msp:validate --all`. Runs after the regular validator rules, alongside
the other M8 PROTOs.

## Severity

`error` — once a project ships an `authority.yaml`, malformed shape should
block CI. Severity stays `draft` while the M8e rollout settles.

## Status

`draft` — gradual rollout. Promotes to `stable` after the companion CI
workflow (PR-time author/path matching) lands per `[[ADR--DELEGATION-POLICY]]`.

## Out of scope (for this PROTO)

PROTO predicates can't see the PR's git author, commit metadata, or diff.
The actual matrix enforcement (match author tier → check touched paths) is
a CI workflow concern (e.g. `.github/workflows/authority-check.yml`),
tracked in `[[AUDIT--AUTHORITY-ENFORCEMENT-PROTO]]` as future work.

## Source

`[[CONCEPT--PROTO-AUTHORITY-ENFORCEMENT]]`, `[[FRAMEWORK--AUTHORITY-MATRIX]]`,
`[[ADR--DELEGATION-POLICY]]`, `[[FEAT--PROTO-LOADER]]`.
