---
id: FEAT--RESOLUTION-EXPAND-ON-DEMAND
phase: 2
type: feat
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Resolution expand-on-demand — 2-tier recall + expand() to promote MENTION
  to FULL
tags:
  - msp
  - ucf
  - feat
  - resolution
  - retrieval
  - mcp
crosslinks:
  references:
    - CONCEPT--RESOLUTION-GRADIENT
    - ADR--RESOLUTION-TIER-COUNT
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
created_at: 2026-05-14T19:42:02.796+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  domain: feat
---

# FEAT — Resolution expand-on-demand

> The user-facing API contract for the MVP of `[[CONCEPT--RESOLUTION-GRADIENT]]`, scoped by `[[ADR--RESOLUTION-TIER-COUNT]]` to 2 tiers (FULL + MENTION) plus `expand()`.

## User-facing behaviour

`recall()` results carry a per-hit `resolution` tier. In the MVP, every hit is `FULL` or `MENTION`:

```ts
interface RecallHit {
  id: string
  resolution: 'FULL' | 'SUMMARY' | 'SKELETON' | 'MENTION'   // MVP emits only FULL | MENTION
  body?: string            // present iff resolution === 'FULL'
  title: string            // always present
  score: number
}
```

A new facade method + MCP tool `expand`:

```ts
// Facade
await layer.expand(id, { to: 'FULL', reason: 'investigating contradiction' }): RecallHit

// MCP tool: msp_expand
//   args: { id, to?, reason }
//   returns: the promoted hit at the requested tier
```

Behaviour contract:

- **`recall()` returns mixed tiers**: the top-scoring hits at `FULL`, the long tail at `MENTION` (id + title only). Tier assignment uses `score = w1·similarity + w2·1/(1+hops)` (working weights 0.7 / 0.3 per spec §14 OQ-1).
- **`expand(id)` promotes** a `MENTION` to `FULL` and returns the full hit. Token cost is predictable and returned to the caller.
- **`expand()` re-runs ABAC**: a Resource may be `MENTION`-only because the PDP denied `FULL`, not because it scored low. `expand()` on such a Resource returns `{ denied_reason }`, not a body.
- **Per-vault `expand_limit`** caps expansions per task to prevent runaway loops; exceeding it returns a soft error advising the agent to narrow its query.
- **The tier enum carries all four values** (`SUMMARY`, `SKELETON` valid but unrendered in MVP per `[[ADR--RESOLUTION-TIER-COUNT]]`) — adding renderers later is additive.
- **Every `expand()` is audit-logged** with `id`, `to`, `reason`, and resulting token count — this log is the Phase 3.5 telemetry that gates SUMMARY/SKELETON renderers.

## Verification

- `recall()` on a representative query returns top-N at `FULL` and the remainder at `MENTION`; `body` is present iff `FULL`.
- `expand('id', { to: 'FULL' })` returns the full body; a second `recall()` in the same session reflects the promotion.
- `expand()` on a Resource the PDP denies `FULL` returns `{ denied_reason: 'policy' }`, no body.
- Exceeding `expand_limit` returns a soft error, not a throw.
- Token consumption on the standard query set is ≥60% below flat top-K at the same K.
- Audit log contains one entry per `expand()` with reason + token count.

## Out of scope

- `SUMMARY` / `SKELETON` renderers — gated on Phase 3.5 telemetry per `[[ADR--RESOLUTION-TIER-COUNT]]`.
- Per-tier budget allocator (50/30/15/5 split) — only meaningful at 4 tiers; deferred with the renderers.
- Hop-metric weight tuning (`w1` / `w2`) — spec §14 OQ-1; MVP ships the working defaults.
- Adaptive learning ("user expanded this often → boost to FULL") — Phase 5+.
- Cross-vault resolution caps — carried by `[[FEAT--VAULT-COMPOSITION]]`'s `resolution_policy` field, applied here but not designed here.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §6 — tier shapes, scoring, expand-on-demand.
- `[[CONCEPT--RESOLUTION-GRADIENT]]` — the concept this FEAT implements (MVP slice).
- `[[ADR--RESOLUTION-TIER-COUNT]]` — 2-tier MVP, 4-tier data model, Phase 3.5 gate.

## Connections
- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]

