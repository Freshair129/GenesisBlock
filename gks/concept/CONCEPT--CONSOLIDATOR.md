---
id: CONCEPT--CONSOLIDATOR
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Consolidator — promotes session turns into durable episodic memory
tags: &a1
  - msp
  - memory
  - consolidator
  - importance
  - summarisation
  - m7b
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--MEMORY-EPISODIC
    - CONCEPT--MEMORY-SESSIONS
created_at: 2026-05-04T17:05:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--CONSOLIDATOR
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Consolidator — promotes session turns into durable episodic memory
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-04T17:05:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--CONSOLIDATOR
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Consolidator — promotes session turns into durable episodic memory
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-04T17:05:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--CONSOLIDATOR
      phase: 1
      type: concept
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Consolidator — promotes session turns into durable episodic memory
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-04T17:05:00.000+07:00
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
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — consolidator

## Problem

Sessions accumulate dozens to hundreds of turns. Most are noise (greetings, clarifications, dead ends). A few capture real decisions, learnings, or context worth keeping around for the next session.

Without a consolidator, two failure modes are inevitable:

1. **Save everything** → episodic store becomes a swamp; recall surfaces low-value matches; vector index grows unbounded.
2. **Save nothing** → agent has no memory across sessions; every turn starts blind.

The consolidator's job is the gate between sessions and episodic. Per `msp_spec.md` §7c (passport aspect), it lives in `src/orchestrator/consolidator.ts` and is triggered at session-end (or explicitly by `msp_remember` MCP tool).

## What it produces

For each session, the consolidator emits **0..N episodes** — each episode is a contiguous span of turns that is:

- coherent (same topic / decision being worked through), AND
- important enough to keep (per scoring below).

Episodes get written via the existing `EpisodicWriter` (`src/memory/episodic/writer.ts`) — so the consolidator does NOT introduce a new persistence layer. It only decides *what* gets persisted.

## The three sub-jobs

| Sub-job | What | Output |
|---|---|---|
| **Importance scoring** | Decide if a chunk of turns is worth keeping | `score: 0..1` + `verdict: 'keep' \| 'drop' \| 'borderline'` |
| **Episode-boundary detection** | Find where one episode ends and the next begins (topic shift) | Array of `[startIdx, endIdx]` ranges |
| **Summarisation** | For kept episodes, generate a 1-3 sentence summary + extract tags | `{ summary, tags[] }` per episode |

## Why hybrid scoring (not pure-deterministic, not pure-LLM)

See `[[ADR--CONSOLIDATOR-HYBRID-SCORING]]` for the full decision. Briefly:

- **Pure deterministic** is too crude. Length + keyword density miss "important but short" turns (e.g. a one-line decision: "we'll use pgvector, not qdrant").
- **Pure LLM** is too expensive. Calling a model for every chunk of every session quickly dominates inference cost — and most sessions are obviously low-importance.
- **Hybrid** uses cheap rules to make the easy decisions (clearly drop / clearly keep) and only invokes the LLM for the borderline cases. Typical mix: ~10–20% LLM-evaluated, the rest deterministic.

## Where it sits in the passport

```
session ends (or msp_remember called)
        │
        ▼
┌───────────────────────────────┐
│ src/memory/sessions/reader    │
│ load session.jsonl turns      │
└──────────┬────────────────────┘
           │
           ▼
┌───────────────────────────────┐
│ src/orchestrator/consolidator │  ← M7b (this work)
│  ├── boundary detector        │
│  ├── importance scorer        │
│  │    ├── deterministic gate  │
│  │    └── LLM (borderline)    │
│  └── summariser               │
└──────────┬────────────────────┘
           │ emits Episode[]
           ▼
┌───────────────────────────────┐
│ src/memory/episodic/writer    │
│ append to episodic_memory.json│
└───────────────────────────────┘
```

## Invariants

- Consolidation is **idempotent** — running the same session through twice produces the same episodes.
- Consolidation is **lossless on the input** — the original session.jsonl is never mutated. Episodes reference back to their source by `session_id` + turn range.
- Borderline cases that fail LLM call (timeout, no provider) **default to keep** — false positives are recoverable; false negatives lose memory.
- Cost is bounded — `maxLlmCalls` opt caps how many borderline chunks may be LLM-evaluated per consolidation pass.

## Out of scope (deferred)

- **Cross-session merging** — collapsing redundant episodes from different sessions (e.g. user asked "how do I X" twice). Goes in M7c (retrieval orchestration) or later.
- **Forgetting / decay** — pruning old episodes by age × importance. Future M9 work.
- **Hierarchical summaries** — summary-of-summaries for long-lived projects. Out of scope for M7b.
- **User-facing review UI** — humans can look at `episodic_memory.json` directly; no UI in M7b.

## Source

`msp_spec.md` §7c (Passport — Consolidator), `[[CONCEPT--MEMORY-EPISODIC]]`, user direction "M7b approach = hybrid (deterministic + LLM borderline)" during M7-prep follow-up cleanup.

## Connections

- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--MEMORY-SESSIONS]]
