---
id: ADR--MEMORY-BACKLINKS-INDEXER
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Backlinks indexer is full-rebuild only; never incremental
tags:
  - msp
  - memory
  - backlinks
  - indexer
  - decision
crosslinks:
  references:
    - CONCEPT--MEMORY-BACKLINKS-INDEXER
    - CONCEPT--MEMORY-VECTOR-BACKLINKS
created_at: 2026-05-03T14:16:42.336+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — backlinks indexer rebuild strategy

## Context

The indexer could (a) maintain incremental state — track which atoms changed and update only their edges — or (b) full rebuild — read all atoms each invocation and produce the file from scratch.

Incremental sounds tempting (avoid O(N) work for a one-atom edit). But it adds:
- Cache invalidation logic (which atoms touched? which edges did they own?)
- Persistent state (`.indexer-state.json`) that can drift from reality
- Complexity in the hot path (race between editor and indexer)

Full rebuild is O(N atoms × ~5 crosslinks each) — for a typical project (hundreds of atoms) it's milliseconds.

## Decision

**Full rebuild on every invocation.** No incremental state. Same approach the atomic-index re-indexer takes (`scripts/msp/re-indexer.ts`).

### Pipeline

1. Walk `gks/<type>/*.md` recursively (skip `00_index/`).
2. For each atom: parse YAML frontmatter; ignore body.
3. For each `crosslinks.<predicate>: [...]` entry, emit `{ from: atom.id, to: targetId, type: predicate }`.
4. Sort by `(from, to, type)` lex order for deterministic diff.
5. Atomic write to `vector/backlinks.jsonl`.

### Idempotency

Running the indexer twice with no atom changes produces byte-identical output. If diff is non-empty, an atom changed between runs — that's a feature for git review.

### Triggers

- Manual: `npm run msp:backlinks`.
- Automatic: pre-commit hook (M3) when atoms in `gks/` change.
- CI: every PR re-derives + asserts the file matches what's committed (otherwise fail).

## Consequences

**Positive**
- No state to corrupt.
- Same input → same output, always.
- O(N) is fast enough at our scale; if N grows past tens of thousands, revisit.

**Negative**
- Wastes work on minor edits. Acceptable for an interactive tool.
- Must be re-run after every atom change to keep `backlinks.jsonl` fresh. Mitigated by pre-commit hook.

## Alternatives considered

1. **Incremental.** Rejected per cache-invalidation pain.
2. **Compute on-the-fly at every query.** Rejected — query latency suffers; CI can't snapshot.
3. **Store backlinks inside `atomic_index.jsonl`.** Considered. Conflates two concerns (atom metadata vs edges); separate files keep grep-ability.

## Source

`[[CONCEPT--MEMORY-BACKLINKS-INDEXER]]` + `[[CONCEPT--MEMORY-VECTOR-BACKLINKS]]`.
