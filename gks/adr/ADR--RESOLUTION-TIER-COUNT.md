---
id: ADR--RESOLUTION-TIER-COUNT
phase: 2
type: adr
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Resolution tier count — 2-tier MVP (FULL + MENTION), 4-tier data model
tags:
  - msp
  - ucf
  - adr
  - resolution
  - retrieval
crosslinks: {"references":["CONCEPT--RESOLUTION-GRADIENT","FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK"]}
created_at: 2026-05-14T18:37:53.303+07:00
---

# ADR — Resolution tier count

> Resolves decision **D-2** in `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §0.

## Context

`[[CONCEPT--RESOLUTION-GRADIENT]]` defines four resolution tiers — `FULL`, `SUMMARY`, `SKELETON`, `MENTION` — at which a Resource can be rendered into context. The question is **how many of those four to ship in the MVP**.

Each tier beyond the minimum costs implementation: a dedicated renderer, additional budget-allocator logic, more UX surface, more test coverage. But too few tiers re-creates the binary-on/off cliff the gradient is meant to eliminate.

The riskiest assumption in the whole gradient idea is **agent autonomy**: that an LLM agent, given a `MENTION` (id + pointer only), will actually call `expand()` to drill in when relevance warrants. If agents do not expand, intermediate tiers are pointless polish; if they do, intermediate tiers add real value.

## Decision

**Ship a 2-tier MVP — `FULL` + `MENTION` + `expand()` — while encoding the full 4-tier data model from day one.**

- **MVP renders only**: `FULL` (complete body) and `MENTION` (id-only pointer).
- **`expand()`** promotes a `MENTION` to `FULL` on demand.
- **The tier enum, scoring, and assignment data model encode all four tiers** — `SUMMARY` and `SKELETON` are valid tier values that simply have no renderer yet. Adding them later is a **renderer addition**, not a rearchitecture.
- **Phase 3.5 gate**: `SUMMARY` / `SKELETON` renderers ship only if Phase 3 telemetry shows `expand()` is called on **≥ 20%** of `MENTION`-tier Resources. Below that threshold, the FULL/MENTION cliff is empirically fine and the renderers are not worth their cost.

## Consequences

Positive:

- MVP proves or kills the highest-risk assumption (agent autonomy) with minimal renderer investment.
- The 4-tier data model means a positive telemetry signal turns into an additive Phase 3.5 deliverable — no rework.
- A negative telemetry signal saves the entire `SUMMARY`/`SKELETON` implementation cost — that is a feature of the gate, not a failure.
- Token savings are already substantial at 2 tiers (FULL for the few, MENTION for the many).

Negative / accepted costs:

- The MVP has a sharp cliff: a Resource is either fully present or a bare pointer. An agent that mis-judges relevance pays a full `expand()` round-trip. Accepted — the round-trip is observable and bounded, and measuring its frequency is exactly the Phase 3.5 gate signal.
- Carrying enum values (`SUMMARY`, `SKELETON`) with no renderer is mild dead-weight in the type system. Accepted — it is the price of making Phase 3.5 additive.

## Alternatives considered

**4-tier from the start.** Rejected: builds `SUMMARY` and `SKELETON` renderers + per-tier budget allocation before any evidence that agents exercise the gradient. If agent-autonomy telemetry is poor, that entire investment is wasted.

**1-tier (FULL only) + drop MENTION.** Rejected: that is just top-K retrieval with extra steps. Without `MENTION` there is no pointer for `expand()`, so the autonomy hypothesis cannot even be tested.

**3-tier (FULL / SUMMARY / MENTION), skip SKELETON.** Rejected for MVP: `SUMMARY` already requires the markdown-parsing renderer, which is the bulk of the intermediate-tier cost. If we are paying for one intermediate renderer we should gate on telemetry and then build both, not pre-commit to one.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §0 (D-2), §6 (tier shapes), §11 Phase 3 / Phase 3.5.
- `[[CONCEPT--RESOLUTION-GRADIENT]]` — the four-tier model and `expand()` semantics.

## Connections
- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]

