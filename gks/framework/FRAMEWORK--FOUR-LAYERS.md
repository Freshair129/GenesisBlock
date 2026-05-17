---
id: FRAMEWORK--FOUR-LAYERS
phase: 0
type: framework
status: stable
created_at: 2026-05-13T12:00:00+07:00
vault_id: GKS-CORE
tier: genesis
title: Four-layer storage model
tags: [architecture, framework, core]
crosslinks:
  references: [CONCEPT--MEMORY-STORE]
linked_symbols:
  - { file: "packages/gks/src/memory/gks.ts" }
  - { file: "packages/gks/src/memory/vector/index.ts" }
  - { file: "packages/gks/src/memory/episodic.ts" }
  - { file: "packages/gks/src/memory/obsidian-mcp.ts" }
aliases:
  - FRAMEWORK
  - implementation_flow
  - Governance / architectural framework
cluster: implementation_flow
role: "Governance / architectural framework"
---

# FRAME — Four-layer storage model

GKS organises memory into **four cooperating storage layers**, each
answering a distinct *kind* of question. `recall(query)` fans out to
all four in parallel.

| Layer | Question | Latency | Source of truth |
|---|---|---|---|
| **Atomic** | Canonical definition of `[[CONCEPT--X]]`? | < 1 ms | `gks/` curated markdown |
| **Vector** | Docs semantically near this query? | tens of ms | embedder + manifest |
| **Episodic** | What did we discuss in this session? | tens of ms | session traces + summaries |
| **Obsidian** | What does the external graph say? | tens-hundreds ms | external Obsidian vault |

## Why four (not three, not five)

- Three would force semantic + canonical into one bucket, losing the
  "never approximates" guarantee of the Atomic layer.
- Five would split episodic into trace+summary, but that's an internal
  sub-structure of one layer; doesn't deserve its own merge stage.

## Merge contract (recall)

1. Each layer returns hits with normalised
   `(id, source, score, path?, title?, snippet, metadata?)` shape.
2. Dedup by `(path \|\| id)` — keep highest score per key.
3. `STABLE_BOOST = 0.05` added to hits whose `metadata.status === 'stable'`.
4. Optional rerank pass; default BM25-lite.
5. Cap at `maxTotal` (default 10).

## Pluggability

Each layer has an interface — swap the implementation without touching
`retain` / `recall` / `reflect` callers:

- **Vector**: JSONL / HNSW / pgvector backends
- **Graph** (atomic-note backlinks): in-memory / Postgres tables
- **Reranker**: BM25-lite / HTTP cross-encoder / custom fn
- **Obsidian**: REST / MCP-stdio / mock
- **Embedder**: Ollama / OpenAI / mock
- **LLM** (consolidator): Anthropic

## Connections
- [[CONCEPT--MEMORY-STORE]]

