---
id: CONCEPT--HYBRID-RETRIEVAL-FTS-LAYER
phase: 1
type: concept
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Hybrid Retrieval — adding FTS as layer 2 (FRAMEWORK_MASTER_SPEC §13)
tags: &a1
  - msp
  - retrieval
  - fts
  - hybrid-retrieval
  - framework-spec-13
crosslinks: &a2
  references:
    - CONCEPT--COGNITIVE-LAYER-FACADE
    - CONCEPT--MEMORY-SUBSYSTEM
created_at: 2026-05-12T22:53:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--HYBRID-RETRIEVAL-FTS-LAYER
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: Hybrid Retrieval — adding FTS as layer 2 (FRAMEWORK_MASTER_SPEC §13)
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-12T22:53:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--HYBRID-RETRIEVAL-FTS-LAYER
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: Hybrid Retrieval — adding FTS as layer 2 (FRAMEWORK_MASTER_SPEC §13)
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-12T22:53:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--HYBRID-RETRIEVAL-FTS-LAYER
      phase: 1
      type: concept
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: Hybrid Retrieval — adding FTS as layer 2 (FRAMEWORK_MASTER_SPEC §13)
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-12T22:53:00.000+07:00
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

# CONCEPT — Hybrid Retrieval FTS layer

## Problem

`FRAMEWORK_MASTER_SPEC.md` §13 defines a 4-layer hybrid retrieval pipeline:

1. Atomic — `gks_lookup` (exact id, O(1))
2. **FTS — keyword grep across `gks/<type>/*.md`**
3. Vector — `gks_recall` (semantic via embeddings)
4. Graph — `gks_backlinks` (relationship traversal)

The existing MSP `orchestrator/retrieval/` implements (1) (3) (4) plus an Obsidian source when configured, with RRF reranking on top. **Layer 2 is missing.** Keyword hits that aren't an exact atomic id and don't survive embedding semantics get dropped.

## Hypothesis

A pure-Node FTS over `gks/<type>/*.md` (case-insensitive substring + token-overlap score, no ripgrep dependency) catches the keyword hits the other layers miss without adding a binary install. The §13.2 cheap-cascade still short-circuits on exact-id matches; FTS only fires when atomic missed.

## Scope

In:

- `ftsSearch(gksRoot, query, opts)` returning `RetrievalHit[]` with `metadata.matchedBy = 'fts'`.
- Token-overlap score (matches / total-tokens) so RRF blending in `createCognitiveLayer.recall` has a comparable scalar.
- Strip frontmatter from the snippet so users see body content.
- Pure Node, no rg / external binary.

Out:

- Inverted index — N is small (hundreds of atoms in practice); O(N) scan is fine.
- Multi-language tokenisation (Thai segmentation in particular) — defer to a follow-up if `vault_id` requires it.

## Success criteria

- Hits returned with score ∈ (0, 1] equal to `matches / tokens.length`.
- Empty query / empty vault returns `[]` without throwing.
- Frontmatter never appears in the snippet.
- `limit` parameter is honoured.

## Why

Without FTS the hybrid pipeline is mis-named: it's only 3 layers. Closing the gap to the documented 4 makes the recall path consistent across consumers (EVA, Claude Code, Hermes, openclaw, custom). The cost is ~70 lines of TS and one new test file.

## Connections

- [[CONCEPT--COGNITIVE-LAYER-FACADE]]
- [[CONCEPT--MEMORY-SUBSYSTEM]]
