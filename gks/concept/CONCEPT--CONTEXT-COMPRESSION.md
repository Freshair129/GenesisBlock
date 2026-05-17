---
id: CONCEPT--CONTEXT-COMPRESSION
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Context compression — token-budget aware lossy summarisation of episodes
tags: &a1
  - msp
  - compression
  - token-budget
  - episodic
  - m7d
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--CONSOLIDATOR
    - FEAT--CONSOLIDATOR
created_at: 2026-05-05T16:10:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--CONTEXT-COMPRESSION
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Context compression — token-budget aware lossy summarisation of episodes
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-05T16:10:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--CONTEXT-COMPRESSION
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Context compression — token-budget aware lossy summarisation of episodes
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-05T16:10:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      domain: concept
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — context compression

## Problem

Agents have finite context windows. As an agent works through a project:

- Sessions accumulate (M7b consolidator emits Episodes)
- Retrieval (M7c) returns top-K hits — these get injected into the prompt
- Eventually the combined context (system + identity + retrieved + sessions + new turn) exceeds the model's window

Without compression: agent either starts dropping context arbitrarily (loses coherence) or stops mid-task (UX failure).

The compressor (M7d, lives at `src/orchestrator/compressor.ts` per `msp_spec.md` §7d) is the budget-aware shrink-to-fit step run **after retrieval, before LLM call**.

## What it takes / produces

```ts
const compressed = await compress({
  episodes: retrievedEpisodes,    // from M7c hits or session-end consolidation
  budgetTokens: 4000,             // remaining context budget
  preserveOrder: true,            // optional: keep chronological vs by-importance
  llm: createSlmClient({ ... }),  // optional; falls back to deterministic if absent
})
// compressed: CompressedEpisode[] — total tokens ≤ budget
// each entry: { episodeRef, summary, originalTokens, compressedTokens, droppedTurns }
```

The output is **lossy by design** — it's the agent's working memory, not the canonical store. Original episodes remain in `episodic_memory.json`; compression doesn't mutate.

## Three-tier compression strategy

| Tier | What | Cost | When |
|---|---|---|---|
| **Keep** | Episode passes through verbatim | 0 | High-importance + fits budget |
| **Trim** | Drop low-value turns from the episode (greetings, dead ends); keep the rest | trivial | Borderline-importance OR bigger than budget allows whole-keep |
| **Resummarise** | Re-summarise the whole episode at higher compression ratio | LLM call (or deterministic fallback) | Low-importance, AND already has a summary |

Default: greedy fill from highest-importance down, applying the cheapest tier per episode that fits. If the budget can't fit even all summaries, drop the lowest-importance ones from the output.

## Why three tiers (not just resummarise everything)

- Resummarise is expensive (LLM calls × number of episodes).
- Many episodes are already at peak compression (a one-line decision); re-summarising is lossy without benefit.
- Trim catches the common case ("episode is mostly fine but had 3 irrelevant turns").

Per-episode tier choice:
1. If `episode.tokens ≤ budgetSlice`: **keep**
2. Else if episode has ≥ 30% dropable turns (per `tier1Score` re-applied): **trim**
3. Else: **resummarise** (target 60% of original token count)

## Token estimation

MSP doesn't ship a tokeniser (no new deps). Use a **conservative character-count heuristic**:

```ts
estimateTokens(text: string): number = Math.ceil(text.length / 3.5)
```

3.5 chars/token is conservative for mixed-language text (English averages ~4, Thai averages ~3). Better safe than over-budget.

For higher-fidelity counts (e.g. tiktoken), users can inject `opts.tokeniser?: (s: string) => number` per-call.

## Where it sits

```
agent boot
  │
  ▼
recall(query)            ← M7c
  │ Episodes/Hits
  ▼
compress({                ← M7d (this work)
  episodes,
  budgetTokens,
})
  │ CompressedEpisode[]
  ▼
buildPrompt(...)         ← caller (agent harness)
  │
  ▼
LLM call
```

Pure read + transform; no persistence. Episodic store unchanged.

## Invariants

- **Total tokens ≤ budget** — enforced; if even summaries don't fit, drop episodes from output (preserve the highest-importance ones)
- **Provenance preserved** — every compressed entry carries `episodeRef` (sessionId + turnRange or atomId) so caller can fetch the original
- **No mutation** — episodic store / sessions are read-only inputs
- **LLM-optional** — without `opts.llm`, tier-3 (resummarise) falls back to deterministic truncation (drop turns from the end until fits); marked `compressedBy: 'truncated'` in output
- **Idempotent under same budget** — same input + same budget → same output (modulo LLM non-determinism on tier-3, mitigated via `mock` provider in tests)

## Out of scope (deferred)

- **MCP tool wrapping** (`msp_compress`) — M7f
- **Cross-episode dedup** — could be M9 (compute set-cover style)
- **Hierarchical summary-of-summaries** — out of M7
- **Semantic-similarity-based dedup before compression** — depends on M7c vector path; possible follow-up
- **Online learning of compression ratios** — M9+

## Source

`msp_spec.md` §7d, `[[CONCEPT--CONSOLIDATOR]]` (compressor's input shape comes from M7b episodes), `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`.

## Connections
- [[FEAT--CONSOLIDATOR]]

