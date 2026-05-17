---
id: FRAMEWORK--AGENT-DISPATCH
phase: 0
type: framework
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: FRAMEWORK — Agent Dispatch — three-tier agent runtime (T1/T2/T3) for MSP
tags:
  - msp
  - framework
  - agents
  - dispatch
  - t1
  - t2
  - t3
crosslinks:
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--AGENT-AGNOSTIC
    - CONCEPT--AGENT-TIER-ROUTING
    - ADR--AGENT-TIER-COST-POLICY
    - BLUEPRINT--AGENT-DISPATCHER
created_at: 2026-05-14T03:05:00.000+07:00
aliases:
  - FRAMEWORK
  - implementation_flow
  - Governance / architectural framework
cluster: implementation_flow
role: Governance / architectural framework
attributes:
  domain: framework
---

# FRAMEWORK — Agent Dispatch

Sister framework to `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`: defines the **agent-dispatch sub-system** — the runtime that picks which LLM CLI to invoke for a given task, runs it, escalates on failure, and records the outcome as an episode atom.

## What it is

A three-tier agent runtime, with one adapter per tier:

| Tier | Backend | Role | Cost |
|---|---|---|---|
| **T1** | Qwen CLI (local SLM) | First-pass for `summarize` / `classify` / `format` | free |
| **T2** | Gemini CLI (cloud) | Default cloud tier; huge context window | ~free under quota |
| **T3** | Claude Code (cloud) | Critical-severity + complex reasoning | paid |

The dispatcher is the **only** code path that picks tiers; callers state intent (`task.type`, `task.severity`, optional `budget_hint`), not a tier name (unless they force one).

## Where it fits

Slots between the application/CLI/MCP layer and the actual LLM CLIs — below MSP's passport, parallel to retrieval orchestration:

```
┌─────────────────────────────────────────────────────────────┐
│ Caller (Claude Code / Cursor / Antigravity / msp-dispatch)  │
└──────────────────────────┬──────────────────────────────────┘
                           │ dispatch(task)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Dispatcher (packages/msp/src/agents/dispatch.ts)            │
│   1. routing.pick(task) → initial tier                      │
│   2. cost-policy.enforceTierCap() → cap or downgrade        │
│   3. adapter.healthcheck() → one-step fallback up           │
│   4. adapter.run() → output                                 │
│   5. cost-policy.canEscalate() → optional re-run            │
│   6. result-recorder → EPISODE--AGENT-RUN-<ts> atom         │
└──────────────────────────┬──────────────────────────────────┘
                           │ TierAdapter.run(prompt, opts)
       ┌───────────────────┼───────────────────┐
       ▼                   ▼                   ▼
┌────────────┐      ┌────────────┐      ┌────────────┐
│ T1: qwen   │      │ T2: gemini │      │ T3: claude │
│ subprocess │      │ subprocess │      │ subprocess │
└────────────┘      └────────────┘      └────────────┘
```

## Surface

| Path | What |
|---|---|
| `dispatch(task) → DispatchResult` | Public TS entry — accepts `DispatchTask`, returns `{tier_used, output, duration_ms, escalated_from?}` |
| `npx msp-dispatch [--tier --type --severity] "<prompt>"` | CLI wrapper for ad-hoc invocation + scripting |
| `msp_brain_resolve` (MCP) | Memory-side counterpart: resolves a query against the dispatched-agent's memory layer |
| `[[EPISODE--AGENT-RUN]]-<ts>` | Immutable atom emitted per run (recorded by `result-recorder.ts`) |

Internals (not exported): `routing.pick()`, `cost-policy.{canEscalate,enforceTierCap}`, `registry.getAdapter()`, three `TierAdapter` implementations.

## Why three tiers

Capability and cost lie on the same axis. A single-tier system either (a) burns budget on tasks a smaller model would handle (always-T3), or (b) loops on retries when the cheap tier under-performs (always-T1). Three tiers let routing apply *pre-emptive* cost control via task shape (`task.type`, `severity`, `context_size_tokens`) instead of post-hoc retry. Two would be sufficient if either local SLMs or cloud quotas changed by an order of magnitude; for now three is the empirical sweet spot.

## Invariants

1. **Adapter interchangeability** — every backend implements `TierAdapter { name, healthcheck, run }`. Adding a new backend never touches `dispatch.ts`.
2. **Deterministic routing** — given the same `DispatchTask` and the same cost policy, `routing.pick()` returns the same tier. No randomness, no learned weights at this layer.
3. **Immutable episodes** — `result-recorder.ts` appends; it never updates or deletes a prior `[[EPISODE--AGENT-RUN]]-*` atom.
4. **Non-throwing healthcheck** — adapter `healthcheck()` returns `false` instead of throwing on missing CLI / network errors; `dispatch.ts` wraps in `safeHealthcheck()` as belt-and-braces.
5. **One-step escalation** — failure escalates exactly once (T1→T2 or T2→T3), gated by `cost-policy.canEscalate()`. No unbounded fallback chain.
6. **Best-effort recording** — `recordEpisode()` errors are logged to stderr but never fail dispatch.

## Boundaries (what dispatch does NOT do)

- **Pick a model within a tier** — that's the CLI's job. T2 picks Gemini Flash vs Pro; T3 picks Sonnet vs Opus.
- **Sync or replicate episodes** — episodes are workspace-local; cross-project sharing is the Two-Brain layer's concern (`[[CONCEPT--TWO-BRAIN-ARCHITECTURE]]`).
- **Automatically fall back to weaker tiers** — escalation only goes *up* the tier ladder. A T3 failure surfaces to the caller; it does not silently run again on T1.
- **Cache outputs** — same prompt at the same tier runs again. Caching is a future ADR-level decision (see `[[BLUEPRINT--AGENT-DISPATCHER]]` open questions).
- **Multi-agent ensembles** — running T1 + T2 in parallel and picking the best is deferred.

## Related work

- **`[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`** — parent framework; agent-dispatch is one sub-system of the passport.
- **`[[CONCEPT--TWO-BRAIN-ARCHITECTURE]]`** — memory-side counterpart; dispatch writes episodes that the brain layer later consolidates.
- **`[[CONCEPT--AGENT-AGNOSTIC]]`** — sibling principle the dispatcher concretises: any agent CLI can be added as a `TierAdapter` without API breakage.
- **MCP tool surface** — `msp_brain_resolve` (memory) and a future `msp_dispatch` tool will expose the dispatcher to remote agents.
- **Master Block promotion** (`MASTER--*`, `[[BLUEPRINT--MASTER]]-*`) — knowledge composition over time; episodes feed the consolidator that eventually proposes master-block updates.

## Connections
- [[CONCEPT--AGENT-TIER-ROUTING]]
- [[ADR--AGENT-TIER-COST-POLICY]]

