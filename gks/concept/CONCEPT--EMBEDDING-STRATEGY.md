---
id: CONCEPT--EMBEDDING-STRATEGY
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Embedding strategy — GKS canonical writer, Smart Connections in-Obsidian browse
tags:
  - msp
  - gks
  - embedding
  - semantic-search
  - smart-connections
  - obsidian
crosslinks: {"references":["CONCEPT--OBSIDIAN-AS-RUNTIME","ADR--EMBEDDING-MODEL-PARITY","ADR--GRAPH-IS-GKS-DOMAIN"]}
created_at: 2026-05-03T23:55:05.902+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — embedding strategy

> **Updated 2026-05-04 (M7-prep follow-up)**: original framing said "MSP delegates to Smart Connections; never embeds". Audit against GksV3 3.6.0 found GKS now ships `createNomicEmbedder()` — Node-side local embedder. Reframed below: GKS is the canonical writer; Smart Connections is the in-Obsidian browse path. Both must use the same model — see `[[ADR--EMBEDDING-MODEL-PARITY]]`.

## Problem

Semantic recall (find atoms similar to a free-text query) requires embeddings. The choices are:

1. **MSP ships its own embedder** — extra binary or Node-side model, double-resourced, version skew across processes.
2. **External vector DB** (pgvector, qdrant, ...) — needs infra; embedding pipeline still has to live somewhere.
3. **GKS embeds canonically; Smart Connections (Obsidian plugin) embeds in-Obsidian for human browse** — the chosen path. Both surfaces use the same model so vectors are conceptually interchangeable.

## Decision

Two surfaces, one model:

### Path A — GKS canonical writer (agent-facing)

- GKS 3.6.0+ ships `createNomicEmbedder()` running `nomic-ai/nomic-embed-text-v1.5` locally via `@huggingface/transformers`.
- Vectors persisted via `VectorBackend` (default JSONL at `.brain/msp/projects/<ns>/vector/atomic.jsonl`; HNSW or pgvector swappable).
- This is what `msp_recall` (M7c) queries. Agents never touch Smart Connections directly.

### Path B — Smart Connections (human browse, in-Obsidian)

- User opens vault in Obsidian → Smart Connections plugin builds its own index in `.smart-connections/`.
- Powers the Smart View pane, "find similar notes" UI, etc. — for humans, not agents.
- Must be configured to use the **same model** as GKS (see `[[ADR--EMBEDDING-MODEL-PARITY]]`).
- MSP **does not** parse `.smart-connections/` files — schema is plugin-version-private.

### Why two indexes if model is the same

- GKS owns its `VectorBackend` interface; Smart Connections owns `.smart-connections/`.
- Reading SC's storage from MSP would couple MSP to plugin internals — fragile.
- M10a ("msp-bridge" companion plugin) is the future deduplication path: SC reads GKS's store directly, single index. Until then, 2× storage cost is accepted at typical vault sizes (< 5,000 atoms).

## Implications for MSP

- MSP **wraps** GKS's embedder for query embedding (M7c). It does not bundle its own model.
- For live semantic recall: query embedded by GKS adapter → GKS vector store search → ranked atoms.
- For offline / no-Obsidian scenarios: semantic search still works (GKS vector store is independent of Obsidian). Smart Connections's path is unavailable, but agent-facing recall is not affected.
- "Vector DB scale-up" path = swap GKS's `VectorBackend` (JSONL → HNSW → pgvector); MSP stays untouched.

## Why GUI-resourced embedding is acceptable for path B

Smart Connections runs in the Obsidian Electron process the user is already paying for. For human browse (rare, interactive), embedding latency + memory cost are absorbed by Obsidian's footprint. Agent-facing path (A) is independent and runs without Obsidian.

## What MSP must do

- **Use** GKS's embedder for canonical write + query (path A).
- **Detect** whether Obsidian + Smart Connections are reachable for path B (Smart View, browse-side recall).
- **Document** the canonical model so users configure SC correctly.
- **Never** ship a competing embedder.

## Source

User architectural direction in M7-prep + audit during M7-prep follow-up. GksV3 3.6.0 CHANGELOG. Smart Connections plugin docs. See `[[ADR--EMBEDDING-MODEL-PARITY]]` for the model lock decision.

## Connections
- [[CONCEPT--OBSIDIAN-AS-RUNTIME]]
- [[ADR--GRAPH-IS-GKS-DOMAIN]]

