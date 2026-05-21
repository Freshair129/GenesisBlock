---
id: ADR--COMPRESSOR-THREE-TIER
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Compressor — three-tier (keep / trim / resummarise) greedy fill from
  importance order
tags: &a1
  - msp
  - compression
  - tiered
  - importance
  - decision
  - m7d
crosslinks: &a2
  references:
    - CONCEPT--CONTEXT-COMPRESSION
    - ADR--CONSOLIDATOR-HYBRID-SCORING
created_at: 2026-05-05T16:10:30.000+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--COMPRESSOR-THREE-TIER
  phase: 2
  type: adr
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Compressor — three-tier (keep / trim / resummarise) greedy fill from
    importance order
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-05T16:10:30.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--COMPRESSOR-THREE-TIER
    phase: 2
    type: adr
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Compressor — three-tier (keep / trim / resummarise) greedy fill from
      importance order
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-05T16:10:30.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--COMPRESSOR-THREE-TIER
      phase: 2
      type: adr
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Compressor — three-tier (keep / trim / resummarise) greedy fill from
        importance order
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-05T16:10:30.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Architecture decision record
      attributes:
        domain: adr
      domain: adr
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# ADR — compressor three-tier strategy

## Context

`compress({ episodes, budgetTokens })` (M7d) needs to fit `Episode[]` into a token budget. Three viable strategies:

1. **Resummarise everything** — uniform LLM call per episode targeting `budget/N` tokens each. Predictable but expensive (N LLM calls) and lossy even when not needed.
2. **Hard-cut at budget** — sort by importance, take top-K until budget exhausted. Simple but loses everything past the cutoff.
3. **Three-tier (keep / trim / resummarise)** — apply the cheapest action per episode that fits.

User direction (during all-M planning): use option 3 — pragmatic, cost-bounded, preserves high-importance content verbatim.

## Decision

Greedy fill from importance-descending order. Per episode pick the cheapest tier that fits remaining budget:

| Tier | Action | Cost | Output marker |
|---|---|---|---|
| **keep** | passthrough verbatim | 0 | `compressedBy: 'keep'` |
| **trim** | drop low-importance turns (re-apply tier-1 score from M7b) | µs | `compressedBy: 'trim'` |
| **resummarise** | LLM call targeting 60% of original | one LLM call | `compressedBy: 'resummarise'` |
| **resummarise (fallback)** | truncate to fit (deterministic) | trivial | `compressedBy: 'truncated'` |
| **drop** | skip this episode | 0 | not emitted |

### Per-episode tier choice

```
remaining = budgetTokens - tokensUsedSoFar

if remaining < estimateTokens(episode.summary):
  → drop  (can't even fit the summary)

if estimateTokens(episode.fullText) ≤ remaining:
  → keep

dropable = turns in episode where tier1Score < TRIM_THRESHOLD (default 0.3)
if dropable.length / episode.turns.length ≥ 0.30
   AND estimateTokens(episode.text without dropable) ≤ remaining:
  → trim

if opts.llm available:
  → resummarise (target 0.6 × original or remaining, whichever smaller)
else:
  → truncate (drop last turns until fits)
```

### Iteration order

Episodes processed in **importance-descending order** (`episode.score` desc). This way:

- Highest-importance episodes get the verbatim/trim path while budget is roomy
- Lower-importance episodes get the resummarise/truncate path or get dropped
- Final output preserves the most-important content with highest fidelity

If `opts.preserveOrder = true`, the **output array** is reordered chronologically (by `turnRange[0]`) AFTER selection. Selection order is still importance-driven; only presentation order changes.

### Deterministic fallback (no LLM)

When `opts.llm` is undefined OR LLM call fails (timeout / parse error):

- Tier 3 falls back to **truncate**: keep turns from the **end** of the episode (most recent typically most relevant), drop earlier ones until total ≤ budget slice
- Marked `compressedBy: 'truncated'` so caller can show "(truncated)" tag in UI
- This makes the compressor headless-safe — works in CI / no-network setups

### Token estimation

Use char-count heuristic: `Math.ceil(text.length / 3.5)`. Conservative for mixed-language text. Caller can inject a real tokeniser via `opts.tokeniser?: (s: string) => number` (e.g. tiktoken when running in a Node app that already has it).

### Trim algorithm (within an episode)

```
turns = episode.turns
scored = turns.map(t => ({ t, score: tier1ScoreSingleTurn(t) }))
scored.sort by score asc                  // lowest first = most droppable
target = remaining
result = [...turns]
for each turn in scored where score < TRIM_THRESHOLD:
  if estimateTokens(joinTurns(result minus this turn)) ≤ target:
    result = result minus turn
return result preserving original chronological order
```

`tier1ScoreSingleTurn` is the M7b scorer applied to a 1-turn chunk. Re-uses existing infrastructure.

## Consequences

**Positive**

- Cost-bounded — at most N LLM calls (one per resummarise-tier episode); typically far fewer because keep/trim handle the easy cases
- Lossless for high-importance content — keep verbatim
- Headless-safe via truncate fallback
- Pluggable tokeniser
- Provenance-preserved — every output carries `episodeRef`

**Negative**

- Adds latency proportional to resummarise-tier count × LLM call time. Mitigated by tier preference (cheapest first).
- Truncate fallback can produce mid-sentence cuts. Mitigated: cut at turn boundary, never mid-text.
- Tier choice depends on accurate token estimates; the char/3.5 heuristic underestimates for some scripts. Mitigated by `opts.tokeniser` injection.

## Alternatives considered

1. **Resummarise all** — rejected, expensive + over-compresses easy cases.
2. **Hard cutoff** — rejected, drops important content past cutoff with no recovery option.
3. **Vector-similarity dedup before compression** — considered. Adds dependency on M7c embedder. Out of scope; could be follow-up.
4. **Hierarchical summary-of-summaries** — out of scope. Useful for very long-lived projects but premature.

## What this ADR does NOT decide

- **Default `budgetTokens`** — caller's choice; no default
- **Tokeniser library** — no built-in; use heuristic by default
- **Cross-episode dedup** — out of scope
- **Snippet-level extractive summarisation** — out of scope (use whole-turn drop)

## Source

`msp_spec.md` §7d, `[[CONCEPT--CONTEXT-COMPRESSION]]`, M7b consolidator infrastructure (re-used for tier-1 single-turn scoring).

## Connections

- [[ADR--CONSOLIDATOR-HYBRID-SCORING]]
