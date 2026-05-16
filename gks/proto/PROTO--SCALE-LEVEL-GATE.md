---
id: PROTO--SCALE-LEVEL-GATE
phase: 2
type: proto
status: draft
severity: warning
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--SCALE-LEVEL-GATE — codegen refuses to run L2+ tasks without stable atoms
tags:
  - msp
  - proto
  - codegen
  - scale-gate
  - framework-spec-7.7.2
crosslinks: {"enforces":["FRAMEWORK--MSP-ARCHITECTURE-V2"],"references":["CONCEPT--COGNITIVE-LAYER-FACADE","CONCEPT--CODEGEN-MICROTASK-RUNNER"]}
linked_symbols:
  - {"file":"packages/msp/src/cognitive/scale-gate.ts","symbol":"enforceScaleGate"}
created_at: 2026-05-12T22:52:00.000+07:00
---

# PROTO — SCALE-LEVEL-GATE

## Rule

Before a codegen microtask runs, the runner MUST verify that the atoms required for its scale level exist in `gks/<type>/` AND have `status: stable | active` (`FRAMEWORK_MASTER_SPEC.md` §7.7.2).

| Scale | Required atoms (status: stable / active) |
|---|---|
| L1 (quick task) | — (gate is a no-op) |
| L2 (feature/module) | CONCEPT + ADR + FEAT + BLUEPRINT |
| L3 (major/core) | above + FRAME + FLOW |

The gate walks the BLUEPRINT's `crosslinks.references` closure to find the required atoms. A draft or missing atom in any required type aborts the run with `ScaleLevelGateError` BEFORE any SLM call.

## Schema

The gate consumes the L0 index (`gks/00_index/atomic_index.jsonl`) when present, otherwise walks `gks/<type>/*.md` and parses frontmatter. No DB / network calls.

## Why

Codegen burns tokens. Running an L2 task whose parent BLUEPRINT references a draft ADR is wasted compute at best and a silent contradiction at worst. The gate makes the wasted-token path explicit and rejects it at the cheapest layer — before the SLM is invoked.

## Validator

Severity: `warning` while Phase 0 lands; flip to `error` once L2/L3 callers have migrated their flows. Implemented by `enforceScaleGate()` at `packages/msp/src/cognitive/scale-gate.ts`. Throws `ScaleLevelGateError` with a `missing: string[]` list of the absent atom types.

## Exceptions

L1 (Quick Task) is a no-op — `runTask({ scale: 'L1' })` skips the gate entirely (matches `FRAMEWORK_MASTER_SPEC.md` §7.7.2 checklist).
Hotfixes (§6.4) bypass the gate via `gks hotfix open` — 48 h backfill window applies.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--COGNITIVE-LAYER-FACADE]]
- [[CONCEPT--CODEGEN-MICROTASK-RUNNER]]

