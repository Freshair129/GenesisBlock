---
id: PROTO--SAMPLE-RULE
phase: 2
type: proto
status: draft
severity: warning
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--SAMPLE-RULE — trivial demo predicate (M8a)
tags:
  - msp
  - proto
  - sample
  - demo
  - m8a
crosslinks: {"enforces":["FRAME--MSP-ARCHITECTURE-V2"],"references":["CONCEPT--PROTO-PATTERN","FEAT--PROTO-LOADER"]}
linked_symbols:
  - {"file":"src/validator/proto/sample.ts"}
created_at: 2026-05-05T19:00:00.000+07:00
---

# PROTO — SAMPLE-RULE

## Rule

Every project should have at least one `FRAME--*` atom describing its architecture.

## Schema

Reads `atomic_index.jsonl` from the validator's `PredicateContext`. No file
I/O beyond what the index already provides.

## Predicate

```ts
const hasFrame = ctx.atomicIndex.some(a => a.type === 'frame')
return hasFrame ? { ok: true, ... } : { ok: false, violations: [...] }
```

Implementation: `src/validator/proto/sample.ts`.

## Trigger

`msp:validate --all` (runs after the regular validator rules).

## Severity

`warning` — non-blocking. A project early enough not to have a FRAME yet
shouldn't be CI-blocked just by this demo.

## Status

`draft` — this is the bootstrap demo for the M8a loader. It will not be
promoted to `stable`. Real PROTOs (M8b–f) follow this shape but enforce
real governance.
