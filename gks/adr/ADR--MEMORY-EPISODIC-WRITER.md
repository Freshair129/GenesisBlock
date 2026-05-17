---
id: ADR--MEMORY-EPISODIC-WRITER
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Episodic writer is JSON-array, idempotent on episodicId, with pluggable
  summariser
tags:
  - msp
  - memory
  - episodic
  - writer
  - decision
crosslinks:
  references:
    - CONCEPT--MEMORY-EPISODIC-WRITER
    - CONCEPT--MEMORY-EPISODIC
created_at: 2026-05-03T14:16:40.386+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — episodic writer storage shape

## Context

`episodic_memory.json` could be (a) a single JSON array of episode objects, (b) a JSONL file (one episode per line), or (c) one file per episode under `memory/episodes/<episodicId>.json`. Each has trade-offs for read/append/diff.

Pluggable summariser also needs a decision: do we write the writer to *generate* summaries (calling an LLM) or to *accept* pre-generated ones?

## Decision

### Storage: single JSON array, atomic-rewrite append

Use **option (a) — single JSON array**, with **atomic-rewrite append**:

1. On `appendEpisode(episode)`:
   - Read entire file (or `[]` if missing).
   - If `episode.episodicId` already exists, replace it (idempotent).
   - Else push the new episode.
   - Write to `<file>.tmp` then `rename` (atomic on POSIX).
2. File stays diff-friendly (one episode per JSON object, sorted by `timestamp`).

Why not JSONL: episodes are mutated (importance can be re-rated; associations grow). Append-only doesn't fit; JSONL diff is uglier.
Why not file-per-episode: too many small files; complicates "scan all episodes for vector embed" workflows.

### Summariser: accept-only, with optional plugin

Writer **does not** call any LLM. It accepts:
- A manual `summary` string, OR
- A `summariser` plugin function `(turns: SessionTurn[]) => Promise<EpisodeContent>`.

Default plugin is **heuristic** (first non-trivial assistant message + last decision-shaped sentence). LLM-backed summariser is an orchestrator concern; the writer stays pure.

### Importance score validation

`importance_score` is required, must be a number in `[0, 1]`. Invalid → `EpisodicSchemaError` with the offending value.

## Consequences

**Positive**
- Single file = one read for vector index rebuild.
- Atomic rewrite = no torn writes; no lock needed.
- Plugin boundary keeps writer testable without LLM mocks.

**Negative**
- O(N) read+write on every append. Acceptable until N > ~10,000; M3 may add chunked storage.
- Mutation requires reading the whole file. Same constant.

## Alternatives considered

1. **JSONL with periodic compaction.** Considered. Adds compaction job + reader complexity. Defer.
2. **SQLite.** Rejected — too heavy for a single-tenant local file.
3. **Writer calls LLM directly.** Rejected — couples writer to model; same anti-pattern as putting LLM in the validator.

## Source

`[[CONCEPT--MEMORY-EPISODIC-WRITER]]` + `[[CONCEPT--MEMORY-EPISODIC]]`.
