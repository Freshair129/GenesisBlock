---
id: CONCEPT--MEMORY-EPISODIC-WRITER
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Memory episodic writer — emit episode summaries from session ranges
tags:
  - msp
  - memory
  - episodic
  - writer
crosslinks: {"references":["CONCEPT--MEMORY-EPISODIC","CONCEPT--MEMORY-SESSIONS","FEAT--MEMORY-SESSIONS-WRITER"]}
created_at: 2026-05-03T07:16:39.910Z
---

# CONCEPT — memory episodic writer

## Problem

Session JSONLs grow unboundedly. After 50+ turns, an agent that wants to recall "what did we decide about X" can't reasonably scan the full file. The episodic layer (`CONCEPT--MEMORY-EPISODIC`) defines the rich-summary shape; what's missing is the writer that produces episodes from session ranges.

## Hypothesis

A writer that takes `(episodicId, turnRange, summary?, importance_score)` and either accepts a manual summary or generates one via configured LLM, then validates the schema and appends to `episodic_memory.json`, gives the orchestrator a clean append path. Hybrid-retrieval (RRF) gets a real source to embed; session JSONLs stay untouched.

## Scope

In:
- Append episodes to `episodic_memory.json` keyed by `episodicId`.
- Schema validation per `CONCEPT--MEMORY-EPISODIC` (importance_score 0..1 enforced).
- Pluggable summariser: manual, heuristic, or LLM-backed.
- Idempotency: appending the same `episodicId` twice updates instead of duplicating.

Out:
- The retrieval side (vector search, RRF) — orchestrator concern.
- Importance-score heuristics — pluggable; default `0.5`; orchestrator can override.

## Source

Implements `CONCEPT--MEMORY-EPISODIC`. Spec §7.2.
