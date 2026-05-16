---
id: CONCEPT--MEMORY-SUBSYSTEM
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: MSP memory subsystem — sessions / episodic / vector for audit-grade recall
tags:
  - msp
  - memory
  - sessions
  - episodic
  - vector
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2"]}
created_at: 2026-05-03T14:01:52.815+07:00
---

# CONCEPT — MSP memory subsystem

MSP keeps three independent memory stores under `.brain/msp/projects/<ns>/`. They are **for audit, not for full context reload** — the rule is that agents access memory selectively through ID references, never by replaying entire history (that would explode token budgets).

## Three layers

| Layer | Path | Shape | Use |
|---|---|---|---|
| **Sessions** | `sessions/<episodicId>.jsonl` | linear append-only JSONL, one row per turn | replay an exact conversation in order |
| **Episodic** | `memory/episodic_memory.json` | rich JSON with summary + key_decisions + tags | jump to a high-importance event by ID |
| **Vector / Backlinks** | `vector/backlinks.jsonl` | edge list `{from, to, type}` | hybrid retrieval (RRF) per `FRAMEWORK_MASTER_SPEC.md` §13 |

## Read patterns

| Need | Tool | Result |
|---|---|---|
| "Show me turn 42 of session sess_001" | grep `sessions/sess_001.jsonl` for `turnId: 42` | one JSONL row |
| "What were the key decisions of episode ep_007?" | read `episodic_memory.json` filter by `episodicId` | one JSON object |
| "Find atoms semantically near `query`" | hybrid query → vector + obsidian + atomic + episodic, fused via RRF | top-K hits |
| "What references this atom?" | walk `backlinks.jsonl` reversed | edge list |

## What does NOT live here

- The canonical SSOT (atoms) — that's `gks/`
- The validator state — stateless, runs each invocation
- Pending proposals — `.brain/msp/projects/<ns>/inbound/`

## Why "for audit, not full reload"

If every agent turn pulled the entire session JSONL, a 50-turn conversation would re-feed ~50,000 tokens before the first reply. The rule is: agents fetch by ID (a specific turn / episode / atom), not by sequence.

## Source

`msp_spec.md` §7 (Memory Subsystem).

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

