---
id: CONCEPT--GENESIS-BLOCK-RUNTIME
phase: 1
type: concept
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: Genesis Block Runtime — composite execution layer for GENESIS-- manifests
tags:
  - msp
  - genesis-block
  - runtime
  - composition
  - phase-e5
crosslinks:
  references:
    - SPEC--GENESIS-BLOCK-MANIFEST
    - CONCEPT--AGENT-AGNOSTIC
    - CONCEPT--AGENT-TIER-ROUTING
    - BLUEPRINT--AGENT-DISPATCHER
created_at: 2026-05-14T03:30:00.000+07:00
---

# CONCEPT — Genesis Block Runtime

## Problem

`[[SPEC--GENESIS-BLOCK-MANIFEST]]` defines a Genesis Block as a *composite knowledge unit*: a `GENESIS--<NAME>` manifest atom plus the member atoms it aggregates across the 5-dimension EVA 4.0 core (Algo + Concept + Cognitive + Runbook + Params, plus optional Guard/Safety/Stack/Protocol/Mod/Spec).

The SPEC is descriptive — it pins the *shape* of a block manifest but says nothing about how an agent should *use* one at runtime. Specifically:

- How does an agent **load** a Genesis Block from a manifest id?
- How does it **materialise** the member atom bodies so the LLM can reason over them?
- How does it **execute** the block — i.e. take a user prompt and produce a behavior that composes all 5 dimensions?

Without a runtime, a Genesis Block is just paperwork. The whole point of crystallising knowledge into a 5-dimension block is that it can be invoked as a *unit*, not as a manual scavenger hunt through `gks/`.

## Hypothesis

A Genesis Block Runtime is a thin, composable layer that:

1. Takes a `blockId` (e.g. `IDENTITY-ENGINE`)
2. Loads `GENESIS--<blockId>` from `gks/genesis/`
3. Resolves each member id under `members.*` to the actual atom on disk
4. Concatenates the member bodies into a single LLM prompt, grouped by dimension
5. Dispatches that prompt through the existing tier router (`packages/msp/src/agents/dispatch.ts`)
6. Returns the LLM output plus metadata (tier used, member count, duration)

The runtime owns *composition*. It does **not** own:

- Tier routing — that's `dispatch()`'s job (`[[BLUEPRINT--AGENT-DISPATCHER]]`)
- Atom storage — that's the resolver's job (`packages/msp/src/brain/`)
- Manifest validation — that's the validator's job (eventually a future `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]`)

## Why not just call dispatch() directly?

A reasonable question: if the agent has a prompt and a tier router, why bother with a "Genesis Block"? The answer is reuse + provenance.

**Reuse.** Five-dimension blocks crystallise a behavior into a *named, citable unit*. Instead of pasting the same Cognitive lens + Algorithm + SOP + Params into every prompt by hand, the agent loads `[[GENESIS--IDENTITY-ENGINE]]` once, and the runtime composes the prompt deterministically every time. The block becomes addressable — by id, in conversation, in other atoms via `crosslinks.references`.

**Provenance.** Every execution carries the block id + tier used. The episode atom written by `dispatch()` (via `result-recorder.ts`) records *which Genesis Block produced this output*, so an audit can ask: "show me everything `[[GENESIS--IDENTITY-ENGINE]]` did this week".

**Separation of concerns.** Authors maintain the 5 dimension atoms in `gks/`; the runtime composes them at execution time. Edit the `ALGO--` atom and the next invocation picks up the change — no prompt copy-paste hell.

## Composition rule

The composed prompt is mechanical, not clever. For each dimension that has at least one member, emit a labeled section with the bodies concatenated. Missing dimensions are skipped silently — the runtime does not fail if e.g. `optional.protocol` is absent.

```
## Context (Cognitive)
<COGNITIVE--... body>
<COGNITIVE--... body>

## Algorithm
<ALGO--... body>

## Concept
<CONCEPT--... body>

## Runbook
<RUNBOOK--... body>

## Params
<PARAMS--... body>

## User Request
<userPrompt>
```

Order is fixed: Cognitive → Algorithm → Concept → Runbook → Params → UserRequest. Cognitive comes first because it frames how the agent should *interpret* the rest. UserRequest comes last so the LLM reads everything before the live ask.

## Tier choice

The runtime defaults to `dispatch({type: 'codegen', severity: 'regular', ...})`. The `codegen` type biases routing toward T2 (Gemini) — appropriate because Genesis Block execution is typically structured work (apply this lens + algorithm to this input), not a quick one-shot summarise (T1) or a critical multi-step decision (T3).

The caller can override via `opts.tier`, which becomes `dispatch`'s `budget_hint`. That hint is *advisory*; `dispatch` will still throw if the tier violates cost policy (e.g. T3 on a non-critical task — see `dispatch.ts` step 2).

## Trade-offs

**Positive**
- One named entry point (`executeBlock`) for any 5-dimension knowledge unit
- Provenance is automatic via episode recording
- Authors edit atoms in `gks/`; runtime picks up changes with no rebuild
- Extends cleanly: future runtime versions can add caching, multi-agent fan-out, or alternate composition strategies without changing the manifest contract

**Negative**
- Composition is naive: every member body goes into the prompt, no per-member token budget. Large blocks may overrun T1/T2 context windows.
- No conflict resolution: if two member atoms disagree, both bodies still ship to the LLM. Author discipline is the only guard.
- Tier routing is delegated to `dispatch()`; the runtime cannot inspect or override mid-flight escalation decisions.

## What this CONCEPT does not cover

- The frontmatter contract for `GENESIS--<NAME>` — owned by `[[SPEC--GENESIS-BLOCK-MANIFEST]]`
- Validator enforcement of member resolution / status cascade — deferred to a future `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]`
- Per-member token budgeting or block-level cache — a future enhancement, out of Phase E5 scope
- Multi-block composition (chaining two Genesis Blocks together) — a future runtime feature

## Source

Phase E5 of the agentic-monorepo plan. Pins the runtime contract before code lands so the implementation in `packages/msp/src/genesis/` can be reviewed against this CONCEPT and the `[[BLUEPRINT--GENESIS-BLOCK-RUNTIME]]` plan.

## Connections
- [[CONCEPT--AGENT-AGNOSTIC]]
- [[CONCEPT--AGENT-TIER-ROUTING]]

