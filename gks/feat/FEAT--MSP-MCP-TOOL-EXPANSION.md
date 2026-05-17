---
id: FEAT--MSP-MCP-TOOL-EXPANSION
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: MSP MCP tools — recall / remember / compress / identity-get / identity-set
tags:
  - msp
  - mcp
  - tools
  - m7f
  - user-facing
crosslinks: {"references":["CONCEPT--MSP-MCP-TOOL-EXPANSION","FEAT--MSP-MCP-SERVER","FEAT--CONSOLIDATOR","FEAT--RETRIEVAL-ORCHESTRATION","FEAT--COMPRESSOR","FEAT--IDENTITY-LAYER"]}
linked_symbols:
  - {"file":"packages/msp/src/mcp/server.ts"}
  - {"file":"packages/msp/src/mcp/tools/recall.ts"}
  - {"file":"packages/msp/src/mcp/tools/remember.ts"}
  - {"file":"packages/msp/src/mcp/tools/compress.ts"}
  - {"file":"packages/msp/src/mcp/tools/identity-get.ts"}
  - {"file":"packages/msp/src/mcp/tools/identity-set.ts"}
created_at: 2026-05-05T16:15:00.000+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: "Feature spec"
---

# MSP MCP tools — passport-side surface (M7f)

## User-facing API (MCP tools)

```jsonc
// Tool: msp_recall
{
  "name": "msp_recall",
  "input": { "query": "rate limiting decision", "top_k": 10, "namespace": "evaAI" },
  "output": {
    "hits": [{ "atom_id": "ADR--RATE-LIMIT", "source": "gks-vector", "score": 0.42, "rank": 1, "snippet": "..." }],
    "semantic_available": true, "obsidian_available": false,
    "fallback_reasons": [], "timings": { "vector": 142, "fusion": 3 }
  }
}

// Tool: msp_remember
{
  "name": "msp_remember",
  "input": { "session_id": "sess-2026-05-05", "namespace": "evaAI" },
  "output": { "episodes_emitted": [...], "episodes_persisted": 3, "llm_calls": 1 }
}

// Tool: msp_compress
{
  "name": "msp_compress",
  "input": { "episodes": [...], "budget_tokens": 4000 },
  "output": { "compressed": [...], "total_tokens": 3812, "tier_counts": { "keep": 2, "trim": 1, ... } }
}

// Tool: msp_identity_get
{
  "name": "msp_identity_get",
  "input": { "namespace": "evaAI" },
  "output": { "identity": { "schemaVersion": 1, "profile": {...}, "voice": {...}, "preferences": {...} } }
}

// Tool: msp_identity_set
{
  "name": "msp_identity_set",
  "input": { "kind": "voice", "voice": { "tone": ["analytical"], ... } },
  "output": { "ok": true, "identity": { ...full identity post-write } }
}
```

## Acceptance criteria

- [ ] **5 tools registered** in `src/mcp/server.ts` (added to existing 6)
- [ ] Each tool follows the M6 handler pattern: `({ root, ... }) => async (args) => result`
- [ ] **`msp_recall`** wraps M7c `recall()`; accepts `obsidian` lazily (constructs `createObsidianClient` if env supports it, else passes `undefined`)
- [ ] **`msp_remember`** wraps M7b `consolidate()` + appends emitted episodes via existing `EpisodicWriter`
- [ ] **`msp_compress`** wraps M7d `compress()`; passes `createSlmClient` from input.provider
- [ ] **`msp_identity_get`** wraps M7e `getIdentity()`; optionally calls `prunePreferences()` first when `prune: true`
- [ ] **`msp_identity_set`** dispatches on `kind: 'profile' | 'voice' | 'preference'` to the right setter
- [ ] **All tools accept `namespace?` with `'evaAI'` default** matching existing M6 convention
- [ ] **All tools handle root** via existing `mcpRoot()` helper (or equivalent)
- [ ] **Tool tests** under `test/mcp/tools/` (5 test files), mirroring M6 pattern
- [ ] **Server registration test** verifies all 11 tools (6 M6 + 5 M7f) are listed via MCP `tools/list`
- [ ] Test target ~430 → ~470 (+40)

## Surfaces

| Surface | Form |
|---|---|
| MCP tools | `msp_recall`, `msp_remember`, `msp_compress`, `msp_identity_get`, `msp_identity_set` |
| TS handlers | `src/mcp/tools/{recall,remember,compress,identity-get,identity-set}.ts` |
| Tests | `test/mcp/tools/{recall,remember,compress,identity-get,identity-set}.test.ts` + 1 update to `test/mcp/server.test.ts` |

## Out of scope

- Streaming tool outputs — out of MCP-stdio scope
- Tool versioning beyond what MCP spec offers
- Per-tool auth — orchestrator concern (M9)

## Connections
- [[CONCEPT--MSP-MCP-TOOL-EXPANSION]]
- [[FEAT--MSP-MCP-SERVER]]
- [[FEAT--CONSOLIDATOR]]
- [[FEAT--RETRIEVAL-ORCHESTRATION]]
- [[FEAT--COMPRESSOR]]
- [[FEAT--IDENTITY-LAYER]]

