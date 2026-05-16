---
id: CONCEPT--COGNITIVE-LAYER-FACADE
phase: 1
type: concept
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Cognitive Layer facade — one-line memoryOS entry for any agent
tags:
  - msp
  - cognitive-layer
  - facade
  - memoryos
  - agent-agnostic
crosslinks: {"references":["CONCEPT--AGENT-AGNOSTIC","CONCEPT--AGENT-INTEGRATION-PATTERNS","FRAMEWORK--MSP-ARCHITECTURE-V2"]}
created_at: 2026-05-12T22:45:00.000+07:00
---

# CONCEPT — Cognitive Layer facade

## Problem

`[[CONCEPT--AGENT-AGNOSTIC]]` declares that MSP must be pluggable into any cognitive-layer client (EVA, Claude Code, Gemini CLI, Hermes, openclaw, Antigravity, custom MCP). Today wiring is correct but **scattered**: a new consumer must construct `MemoryStore`, register `getIdentity`, instantiate the MCP server, and import `runTask` from `codegen/runner.ts` by hand. The seams that the FRAMEWORK_MASTER_SPEC documents (§7.5 Memory-for-Audit, §7.7.2 Scale-Level gate, §9.6 AUTO-GENERATED marker, §13 hybrid retrieval, §14.1 SSOT hierarchy, §17.3 T1/T2/T3 routing) sit in code already, but no single entry point makes them visible.

## Hypothesis

A thin facade in `packages/msp/src/cognitive/index.ts` that returns one object — `{ recall, remember, consolidate, runTask, verifyFlow, hotfix, resolveSSOT, mcpServer, store, graph }` — lets every cognitive-layer agent adopt the stack with **one factory call** while still honouring the seven §-points without imposing new policy. The facade re-uses the existing implementations; it does not introduce new primitives.

## Scope

In:
- `createCognitiveLayer(opts)` factory returning the unified surface.
- Hybrid 4-layer recall (FTS layer 2 added; atomic / vector / graph already shipped).
- T1/T2/T3 tier routing for `runTask` (default T1 = local Ollama + qwen2.5-coder).
- §7.7.2 Scale-Level gate as `enforceScaleGate()` called before SLM invocation.
- §7.5 Memory-for-Audit stamping on episodic recall hits.
- §9.6 AUTO-GENERATED marker helper.
- §14.1 `resolveSSOT()` for citation conflict resolution.
- `hotfix.{open,list,close,check}` re-exports.
- `mcpServer()` pre-wired with the existing 19 MSP MCP tools — no new tools added.

Out:
- New MCP tools.
- Atom-write paths from the facade — atoms still go through `msp_candidate` + PR ([[ADR--AGENT-WRITE-BOUNDARIES]]).
- Full §8.4 slot/layout grammar for the deterministic composer — only the AUTO-GENERATED marker portion of §8.5/§9.6 is implemented in Phase 0.

## Why this matters

EVA / Hermes / openclaw / Claude Code can now drop into the same stack with:

```ts
import { createCognitiveLayer } from 'msp'
const layer = await createCognitiveLayer({ root: process.cwd() })
```

This is the answer to "what makes MSP plug into a cognitive layer" — the seam that `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` v2 promised but `docs/AGENT-INTEGRATION.md` had to teach by example. The facade reduces consumer-side wiring from ~50 lines to one.

## Success criteria (for the eventual FEAT)

- A `createCognitiveLayer` import from `msp` exposes `recall/remember/consolidate/runTask/verifyFlow/hotfix/resolveSSOT/mcpServer/store/graph` in one call.
- Episodic recall hits carry `audit_only: true` (§7.5).
- `runTask` with `scale: 'L2'` refuses a missing-atoms blueprint with `ScaleLevelGateError` (§7.7.2).
- Default `slm.tier: 'T1'` resolves to Ollama + qwen2.5-coder (§17.3).
- `mcpServer()` returns a McpServer instance with the existing 19 tools wired.

## Connections
- [[CONCEPT--AGENT-INTEGRATION-PATTERNS]]

