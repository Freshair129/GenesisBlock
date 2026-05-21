---
id: CONCEPT--MEMORY-BACKLINKS-INDEXER
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Memory backlinks indexer — derive vector/backlinks.jsonl from atom crosslinks
tags: &a1
  - msp
  - memory
  - backlinks
  - vector
  - indexer
crosslinks: &a2
  references:
    - CONCEPT--MEMORY-VECTOR-BACKLINKS
    - FRAMEWORK--CROSSLINKS-VOCABULARY
created_at: 2026-05-03T14:16:41.740+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--MEMORY-BACKLINKS-INDEXER
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Memory backlinks indexer — derive vector/backlinks.jsonl from atom crosslinks
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T14:16:41.740+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--MEMORY-BACKLINKS-INDEXER
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Memory backlinks indexer — derive vector/backlinks.jsonl from atom
      crosslinks
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T14:16:41.740+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--MEMORY-BACKLINKS-INDEXER
      phase: 1
      type: concept
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Memory backlinks indexer — derive vector/backlinks.jsonl from atom
        crosslinks
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-03T14:16:41.740+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Strategic intent / PRD
      attributes:
        domain: concept
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# CONCEPT — backlinks indexer

## Problem

`[[CONCEPT--MEMORY-VECTOR-BACKLINKS]]` describes the on-disk shape (`{from, to, type}` JSONL). But maintaining it by hand is impossible — every atom mutation touches multiple edges. Today there's no derivation pipeline, so the file stays empty (or worse, drifts from canonical crosslinks).

## Hypothesis

A derivation pipeline that walks every atom in `gks/`, emits one edge per `crosslinks.*` entry, and writes a sorted JSONL to `.brain/msp/projects/<ns>/vector/backlinks.jsonl` makes the graph layer always-fresh and impossible to drift. Same input → same output (sorted by `from`); diff-friendly so PRs surface graph changes naturally.

## Scope

In:

- Walk `gks/<type>/*.md`, parse frontmatter, emit edges from `crosslinks.*`.
- Deterministic output: sort by `from`, then `to`, then `type`.
- Atomic write (`.tmp` + rename).
- One CLI invocation re-derives the entire graph from scratch (no incremental state to corrupt).

Out:

- Reverse-index lookup (computed in-memory at query time).
- Hybrid retrieval / RRF (orchestrator).
- Vector embedding (orchestrator + embedder of choice).

## Source

Implements `[[CONCEPT--MEMORY-VECTOR-BACKLINKS]]`. Spec §7.3.

## Connections

- [[FRAMEWORK--CROSSLINKS-VOCABULARY]]
