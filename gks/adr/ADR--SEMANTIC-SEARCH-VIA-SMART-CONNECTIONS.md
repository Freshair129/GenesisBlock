---
id: ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Semantic search — GKS canonical, Smart Connections in-Obsidian browse
  (parity-locked model)
tags: &a1
  - msp
  - gks
  - semantic-search
  - smart-connections
  - obsidian
  - decision
crosslinks: &a2
  references:
    - CONCEPT--EMBEDDING-STRATEGY
    - ADR--EMBEDDING-MODEL-PARITY
    - ADR--MSP-OBSIDIAN-INTEGRATION
created_at: 2026-05-03T23:55:06.784+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS
  phase: 2
  type: adr
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Semantic search — GKS canonical, Smart Connections in-Obsidian browse
    (parity-locked model)
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T23:55:06.784+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS
    phase: 2
    type: adr
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Semantic search — GKS canonical, Smart Connections in-Obsidian browse
      (parity-locked model)
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T23:55:06.784+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS
      phase: 2
      type: adr
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Semantic search — GKS canonical, Smart Connections in-Obsidian browse
        (parity-locked model)
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-03T23:55:06.784+07:00
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

# ADR — semantic search topology

> **Updated 2026-05-04 (M7-prep follow-up)**: original decision said "MSP never embeds; Smart Connections is canonical". GksV3 3.6.0 audit found GKS now ships `createNomicEmbedder()` and a built-in vector store. Reframed: **GKS** is the canonical embedder + writer for agent-facing recall; **Smart Connections** remains for in-Obsidian human browse. Both pinned to the same model by `[[ADR--EMBEDDING-MODEL-PARITY]]`. Original alternatives still useful for context — preserved below.

## Context

Three viable architectures for semantic recall of GKS atoms:

1. **MSP-side embedder** — Node ships a model; MSP embeds queries + atoms.
2. **GKS-side embedder + Smart Connections companion for in-Obsidian browse** — GKS owns canonical vectors; SC owns the human-facing browse pane in Obsidian; both use the same model.
3. **Smart Connections only (plugin-resourced everywhere)** — original M7-prep choice; obsoleted by GksV3 3.6.0 shipping its own embedder.

Cost matters: option 1 doubles model memory if user runs Obsidian + SC. Option 3 forces every agent-facing recall through Obsidian. Option 2 lets agents work headless (no Obsidian) and humans browse through familiar SC UI — best of both.

## Decision

**Option 2 — GKS canonical + Smart Connections companion.**

### Two surfaces

| Surface | Owner | Used by | Storage |
|---|---|---|---|
| Agent-facing semantic recall | **GKS** (`createNomicEmbedder()` + `VectorBackend`) | `msp_recall` (M7c), MCP tools, headless CI | `.brain/.../vector/atomic.jsonl` (or HNSW / pgvector) |
| Human browse in Obsidian | **Smart Connections** | Smart View pane, "find similar notes" | `.smart-connections/` (plugin-private) |

### Constraints this creates

1. **Both surfaces must use the same embedding model.** Locked by `[[ADR--EMBEDDING-MODEL-PARITY]]` to `nomic-ai/nomic-embed-text-v1.5`. User configures Smart Connections via plugin settings; GKS uses it by default in 3.6.0.

2. **Agent-facing recall does NOT require Obsidian.** GKS vector store is independent. MSP works headless (CI, server, no GUI).

3. **Smart Connections's storage is plugin-private.** `.smart-connections/` schema is plugin-version-specific. MSP **must not** parse those files for vector arithmetic — only for diagnostics.

4. **Embedding storage is duplicated until M10a.** GKS persists vectors via `VectorBackend`; Smart Connections persists in `.smart-connections/`. ~2× storage cost accepted at typical scale (< 5,000 atoms). M10a "msp-bridge" companion plugin is the dedup path: SC reads GKS's store directly.

### Integration mechanism

For agent path (canonical):

a. **GKS adapter** — MSP imports `createNomicEmbedder()` from `@evaai/gks` (or equivalent), wraps it in M7c retrieval orchestrator. No network hop.

For human path (browse):

b. **Smart Connections plugin in Obsidian** — pre-installed by user; configured to use the canonical model.

c. **Probe at startup** — M7a checks if Obsidian REST + SC are reachable; if so, MSP can surface "open this in Smart View" deep links. Not a requirement for agent recall.

### Scale-up path

When the project outgrows JSONL (large vault, batch queries, sub-100ms latency requirement), swap GKS's `VectorBackend` to HNSW or pgvector. **No MSP change required**. Smart Connections's storage upgrade is plugin-side (M10a or equivalent) — also no MSP change.

## Consequences

**Positive**

- Headless agent recall works (CI, server boot, no Obsidian needed).
- Human browse is unchanged — SC users get the familiar pane.
- Single model = vectors interchangeable in principle; opens M10a dedup.
- Scale-up = swap GKS backend, no rewrites.

**Negative**

- 2× storage until M10a — accepted at current scale.
- User must configure SC plugin to match the canonical model. Recoverable failure: agents always get correct results (GKS canonical); only Obsidian Smart View shows divergent neighbours if user picks a different model in SC — drift visible to humans, not destructive.
- M7c adds GKS as a hard runtime dep for semantic features (was Obsidian before this update). GKS is an npm package, easier to vend than a GUI app.

## Alternatives considered

1. **Bundle `@xenova/transformers` in MSP for ONNX models.** Rejected — duplicates GKS's bundle; ~100 MB install bloat; version drift.
2. **Default to Ollama BGE-M3.** Considered. Better for headless once Ollama installed, but raises the install bar (Ollama + model pull). GKS's nomic via `@huggingface/transformers` runs without Ollama.
3. **Skip semantic; do BM25 only.** Rejected — meaningfully worse retrieval; GKS already ships embeddings for free.
4. **Smart Connections as canonical (original M7-prep choice).** Rejected on audit — forces every agent recall through Obsidian; defeats headless.

## What this ADR does NOT change

- **MSP write contract** — atoms + frontmatter + crosslinks unchanged.
- **MSP validator** — doesn't care about embeddings either way.
- **GKS storage shape** — atomic markdown unchanged; only the vector backend choice.
- **Authority / promotion / hooks** — orthogonal.

## Source

`[[CONCEPT--EMBEDDING-STRATEGY]]` + GksV3 3.6.0 CHANGELOG (`createNomicEmbedder`) + `[[ADR--EMBEDDING-MODEL-PARITY]]` + audit during M7-prep follow-up.

## Connections

- [[ADR--MSP-OBSIDIAN-INTEGRATION]]
