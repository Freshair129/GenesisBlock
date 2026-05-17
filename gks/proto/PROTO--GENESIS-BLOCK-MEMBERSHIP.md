---
id: PROTO--GENESIS-BLOCK-MEMBERSHIP
phase: 2
type: proto
status: draft
severity: error
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--GENESIS-BLOCK-MEMBERSHIP — machine-enforces the GENESIS-- Block Manifest membership + status-cascade contract
tags:
  - msp
  - proto
  - genesis
  - knowledge-block
  - manifest
  - taxonomy
crosslinks: {"enforces":["SPEC--GENESIS-BLOCK-MANIFEST"],"references":["SPEC--GENESIS-BLOCK-MANIFEST","CONCEPT--TAXONOMY-V2-3","CONCEPT--PROTO-PATTERN"]}
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/genesis-block-membership.ts"}
created_at: 2026-05-14T18:30:00.000+07:00
aliases:
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: "Machine-enforced invariant"
---

# PROTO — GENESIS-BLOCK-MEMBERSHIP

## Rule

Every atom with `type: genesis` (a Block Manifest, per `[[CONCEPT--TAXONOMY-V2-3]]`)
MUST satisfy the membership and lifecycle contract that
`[[SPEC--GENESIS-BLOCK-MANIFEST]]` declares:

1. **Block fields present** — the manifest declares `members:`,
   `manifest_version:`, and `daci.driver:` (SPEC §2.2).
2. **Five-dimension core** — `members.core` lists all five EVA 4.0 roles —
   `cognitive`, `algo`, `runbook`, `concept`, `params` — each with ≥1 atom
   id (SPEC §3.1). The five core roles are closed.
3. **Aggregation grammar** — every id under `members.core.*` and
   `members.optional.*` matches the canonical id regex
   (`^[A-Z][A-Z0-9_]*--[A-Z0-9][A-Z0-9_-]*$`), resolves to an existing
   atom, and that atom's `type:` matches the role it is listed under —
   e.g. an id under `members.core.algo` must have `type: algo`,
   `members.optional.guard` must have `type: guard` (SPEC §3.3).
4. **Status cascade** — the block's `status` equals `min(member statuses)`
   under the order `stub < raw < draft < active < stable`. A member that
   is `deprecated` or `superseded` forces the block to a terminal status
   (SPEC §4.2).

## Schema

Reads `atomicIndex: AtomicIndexEntry[]` from the `PredicateContext`. For
each entry whose `type === 'genesis'`, reads the manifest file at
`<repoRoot>/gks/<entry.path>`, parses its YAML frontmatter, and runs the
four checks above. Member resolution and `type:` lookup use the in-memory
`atomicIndex` (no extra disk reads). Each failed check emits one
`severity: 'error'` violation.

## Predicate

```ts
for (const entry of atomicIndex) {
  if (entry.type !== 'genesis') continue
  const fm = parseFrontmatter(await readFile(resolve(gksRoot, entry.path)))
  violations.push(...checkManifest(entry.id, fm, indexById))
  // checkManifest: block-fields → 5-dim core → id/resolution/type → status cascade
}
```

Implementation: `packages/msp/src/validator/proto/genesis-block-membership.ts`.

## Trigger

`msp:validate --all` (runs after the regular validator rules, alongside
the other PROTO predicates).

## Severity

`error` — a Block Manifest is the entry-point of a composite knowledge
unit; a manifest with an incomplete core, a dangling member, or a status
that lies about its members' maturity breaks the aggregation contract
that any future Genesis Block runtime depends on. Better to fail at write
time than to ship a manifest a loader cannot trust.

## Status

`draft` — promoted to `stable` once the first `GENESIS--<NAME>` Block
Manifest atom is authored and the predicate is confirmed correct against
a real manifest. No `type: genesis` atoms exist in the repo yet, so on a
real `--all` run this predicate passes trivially; behavioural coverage
lives in the unit test. Until promotion, violations surface in validator
output but do not fail-exit, per the PROTO loader's draft policy.

## Source

`[[SPEC--GENESIS-BLOCK-MANIFEST]]` §5 explicitly defers machine enforcement of
§2.2 / §3 / §4.2 to "a follow-up `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]`". This
atom is that follow-up.

## Connections
- [[CONCEPT--PROTO-PATTERN]]

