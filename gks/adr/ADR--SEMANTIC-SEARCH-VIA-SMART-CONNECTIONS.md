---
id: ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS
phase: 2
type: adr
status: stable
vault_id: default
title: Semantic search delegated to Smart Connections; MSP never embeds
tags:
  - msp
  - semantic-search
  - smart-connections
  - obsidian
  - decision
crosslinks: {"references":["CONCEPT--EMBEDDING-STRATEGY","ADR--MSP-OBSIDIAN-INTEGRATION"]}
created_at: 2026-05-03T16:55:06.784Z
---

# ADR — semantic search via Smart Connections

## Context

Three viable architectures for semantic recall of GKS atoms:

1. **MSP-side embedder** — Node ships a model (e.g. `@xenova/transformers`); MSP embeds queries + atoms.
2. **Sidecar service** — Ollama with `bge-m3` or similar; MSP issues HTTP embed calls.
3. **Plugin-resourced** — Smart Connections (Obsidian plugin) embeds inside the GUI process; MSP queries the plugin.

Cost matters: option 1 doubles model memory if user already runs Smart Connections in Obsidian. Option 2 is the GksV3 default but requires Ollama to be configured. Option 3 reuses an embedder the user already chose.

## Decision

**Option 3 — Smart Connections.** MSP never owns embedding logic.

### Constraints this creates

1. **Live semantic recall requires a running Obsidian.** If Obsidian is down, MSP's `recall` returns text-search-only with a flag in the result envelope (`semantic: { available: false, reason: 'obsidian-offline' }`).

2. **Query embedding cannot be done client-side from MSP.** The model + version + tokenizer + normalisation are inside the plugin. MSP gets back hits, not vectors.

3. **Embedding storage is plugin-private.** `.smart-connections/` is documented as the persistence layer but its schema is not stable across plugin versions. MSP **must not** parse those files for vector arithmetic — only for diagnostics.

### Integration mechanism

Three options for talking to Smart Connections, in preference order:

a. **Smart Connections REST endpoint** — if the plugin exposes one (probe at startup; treat as feature flag).
b. **Companion plugin "msp-bridge"** that exposes a stable endpoint over Local REST API — future work; not M7-prep.
c. **Read-only diagnostic** of `.smart-connections/` directly — useful for debug, never for live query.

For M7 implementation, mechanism (a) if available, else fall back to text search per `ADR--MSP-OBSIDIAN-INTEGRATION`.

### Scale-up path (deferred)

When the project outgrows Smart Connections (large vault, batch queries, dedicated infra), the migration is to **swap or augment the plugin** — pgvector / qdrant via a companion plugin that re-uses Smart Connections's embedder. MSP remains untouched: it still calls "the Obsidian endpoint", which now points at a beefier backend.

## Consequences

**Positive**
- Zero MSP-side model deps. No bundled `transformers.js`, no Ollama-required-by-default.
- User picks embedding model in a familiar GUI dropdown.
- Vectors stay local by default (privacy aligns with user's plugin choice).
- Scale-up (vector DB) is a plugin upgrade, not an MSP rewrite.

**Negative**
- Hard runtime dependency on Obsidian for semantic features. Acceptable — `ADR--MSP-OBSIDIAN-INTEGRATION` accommodates this.
- Smart Connections's REST API surface is plugin-defined; we have no control over breaking changes. Mitigation: probe + version-pin via the integration ADR.
- Headless servers (e.g. CI) lose semantic recall. Acceptable — CI shouldn't need it; for production headless deployments, run Obsidian-as-service or build option (b).

## Alternatives considered

1. **Bundle `@xenova/transformers` for ONNX models.** Rejected — adds ~100 MB to MSP install + duplicates user's existing embedder + version drift risk.
2. **Default to Ollama BGE-M3.** Considered. Better for headless but raises the install bar (Ollama + model pull). Smart Connections is friendlier for the typical Obsidian user; Ollama remains a possible adapter via option (b) above.
3. **Skip semantic; do BM25 only.** Rejected — meaningfully worse retrieval; users with Smart Connections already have semantic available.

## What this ADR does NOT change

- **Vector layer in `CONCEPT--MEMORY-VECTOR-BACKLINKS`** — that's about backlinks, not embeddings. Untouched.
- **GKS storage shape** — atoms + frontmatter + crosslinks unchanged.
- **MSP's writers + validator** — they don't care about embeddings either way.

## Source

`CONCEPT--EMBEDDING-STRATEGY` + Smart Connections plugin docs.
