---
id: FEAT--MSP-SYMBOL-MCP
phase: 2
type: feat
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: msp_symbol_* MCP tool surface — 5 read-only tools for in-session symbol-graph queries
tags:
  - msp
  - symbol-graph
  - mcp
  - feat
  - tool
crosslinks: {"implements":["ADR--SYMBOL-GRAPH-PERSISTENCE"],"references":["FRAME--SYMBOL-GRAPH","CONCEPT--SYMBOL-GRAPH","FEAT--MSP-MCP-SERVER"]}
linked_symbols:
  - {"file":"src/mcp/tools/symbol-lookup.ts"}
  - {"file":"src/mcp/tools/symbol-neighbors.ts"}
  - {"file":"src/mcp/tools/symbol-impact.ts"}
  - {"file":"src/mcp/tools/symbol-community.ts"}
  - {"file":"src/mcp/tools/symbol-search.ts"}
  - {"file":"src/mcp/server.ts"}
created_at: 2026-05-09T17:52:00.000+07:00
---

# FEAT — msp_symbol_* MCP tool surface

## User-facing behaviour

Five read-only MCP tools that an agent calls during a session to query the Symbol Graph built by `msp:graph build`. All require the graph to exist; if `meta.last_built_at` is missing, every tool returns `errorResult("graph not built — run 'npm run msp:graph build' first")`.

Registered in `src/mcp/server.ts` `TOOLS` array alongside the existing 11 → total **16 MSP MCP tools** after this PR's implementation lands (PR-4).

| Tool | Input | Output |
|---|---|---|
| `msp_symbol_lookup` | `{ name: string, kind?: string, root?: string }` | `{ ok, hits: Symbol[] }` — exact-match first, then prefix matches, ranked by `exported` then file depth |
| `msp_symbol_neighbors` | `{ id: string, depth?: number=1, edge_types?: string[], root?: string }` | `{ ok, center: Symbol, nodes: Symbol[], edges: Edge[] }` — k-hop BFS bounded at depth ≤ 3 |
| `msp_symbol_impact` | `{ id: string, root?: string }` | `{ ok, callers: Array<{symbol, distance}>, count }` — reverse closure on `calls ∪ references` |
| `msp_symbol_community` | `{ id: string, root?: string }` | `{ ok, community: {id,label,size,modularity}, members: Symbol[] }` |
| `msp_symbol_search` | `{ query: string, limit?: number=20, root?: string }` | `{ ok, hits: Array<Symbol & {score:number}> }` — substring + token-fuzzy across `name + signature` |

## Why these 5 (and not more)

These cover the four common queries from `CONCEPT--SYMBOL-GRAPH`:

| Question | Tool |
|---|---|
| "What is `recall` and where is it?" | `msp_symbol_lookup` (exact) or `msp_symbol_search` (fuzzy) |
| "What does `recall()` reach?" | `msp_symbol_neighbors(id, depth=1)` |
| "What calls `recall()`?" | `msp_symbol_impact(id)` |
| "What else is in the same logical module?" | `msp_symbol_community(id)` |

A 6th tool `msp_symbol_subgraph(community_id)` was considered but deferred — `msp_symbol_community` already returns the member list; full subgraph is a CLI-tier query.

## Verification

- Unit tests in `test/mcp/symbol-tools.test.ts` (PR-4): each tool registered, returns expected shape on a fixture SQLite graph
- Integration test: end-to-end via `bin.test.ts`-style spawn — tools/list returns 16, each `msp_symbol_*` callable
- Tool count assertion in `test/mcp/server.test.ts` updated from 11 → 16

## Out of scope

- Write tools — symbol graph is read-only knowledge, never mutated by agents
- Embeddings or semantic search over symbols — covered by `ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS` boundary
- Cross-repo symbol queries — Phase 2 (after upstream to GKS)

## Source

- `FRAME--SYMBOL-GRAPH`, `CONCEPT--SYMBOL-GRAPH`
- Existing tool shape: `src/mcp/tools/recall.ts` (zod inputSchema, `handler(ctx)` returning `ToolTextResult`)
- Tool registration pattern: `src/mcp/server.ts:TOOLS` array (currently 11 entries)
