---
id: CONCEPT--EMBEDDING-STRATEGY
phase: 1
type: concept
status: stable
vault_id: default
title: Embedding strategy — Smart Connections (Obsidian plugin), local, GUI-resourced
tags:
  - msp
  - embedding
  - semantic-search
  - smart-connections
  - obsidian
crosslinks: {"references":["CONCEPT--OBSIDIAN-AS-RUNTIME"]}
created_at: 2026-05-03T16:55:05.902Z
---

# CONCEPT — embedding strategy

## Problem

Semantic recall (find atoms similar to a free-text query) requires embeddings. The choices are:

1. **MSP ships its own embedder** — extra binary or Node-side model, double-resourced if Obsidian also embeds, version skew across processes.
2. **External vector DB** (pgvector, qdrant, ...) — needs infra; embedding pipeline still has to live somewhere.
3. **Delegate to the Obsidian plugin layer** — Smart Connections does this already, locally, with a user-chosen model, sharing GUI process resources.

## Decision (recorded in this CONCEPT, refined in `ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS`)

Use **Smart Connections** as the canonical embedding source. It:

- Lets the user pick the embedding model from a GUI dropdown (sentence-transformers variants, BGE, OpenAI-compatible APIs if configured, etc.)
- Embeds **locally by default** — vectors do not leave the machine unless the user explicitly chooses an API model.
- **Reuses the Obsidian (Electron renderer) process** — no separate embedder daemon, no MSP-side model bundle.
- Persists embeddings under `.smart-connections/` inside the vault — file-readable for headless / inspection.
- Indexes incrementally on file save — no MSP indexing logic needed.

## Implications for MSP

- MSP **does not embed**. It cannot — different model + version would produce incompatible vectors.
- For live semantic recall, MSP must talk to a running Obsidian (Smart Connections REST endpoint, if exposed; or file-based + careful query handling).
- For offline / no-Obsidian scenarios, semantic search degrades to text search (keyword via Obsidian REST or grep fallback).
- "Vector DB scale-up" path = swap or augment Smart Connections's storage backend (a future plugin or a sidecar service); MSP stays untouched.

## Why GUI-resourced is acceptable

Smart Connections runs on the user's machine inside the same Electron process they're already paying for. For interactive agent use (~one query per turn), embedding latency + memory cost are absorbed by the existing Obsidian footprint. Batch / high-volume querying is **not** the use case.

## What MSP must do

- **Detect** whether Obsidian is reachable (REST endpoint or vault filesystem).
- **Prefer** Smart Connections's exposed endpoint if available.
- **Fall back** to text/keyword search if not.
- **Cache** embedding-derived results when reasonable (RRF inputs are stable across queries within a session).
- **Never** ship a competing embedder.

## Source

User architectural direction in M7-prep discussion. Smart Connections plugin docs.
