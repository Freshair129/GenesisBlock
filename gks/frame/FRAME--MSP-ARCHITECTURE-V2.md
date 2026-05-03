---
id: FRAME--MSP-ARCHITECTURE-V2
phase: 0
type: frame
status: stable
vault_id: default
title: MSP architecture v2 — passport-orchestrator over Obsidian-backed GKS
tags:
  - msp
  - architecture
  - foundation
  - v2
  - supersede
crosslinks: {"references":["CONCEPT--OBSIDIAN-AS-RUNTIME","CONCEPT--EMBEDDING-STRATEGY","ADR--MSP-OBSIDIAN-INTEGRATION","ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS"],"supersedes":["FRAME--MSP-ARCHITECTURE"]}
created_at: 2026-05-03T16:55:07.217Z
---

# FRAME — MSP architecture v2

> Supersedes `FRAME--MSP-ARCHITECTURE` (v1). v1 framed MSP as a "gatekeeper" — a write-path enforcer for atoms going into `gks/`. That framing was correct for what M0–M6 implemented but **incomplete**: MSP's name (Memory & Soul Passport) implies a passport that travels with the agent, carrying memory + identity + retrieval logic. v1 captured the gatekeeping; v2 captures the passport.

## The two-layer mental model

```
                    Agent (Claude / Cursor / EVA / custom)
                            │
                            ▼
                ┌────────────────────────┐
                │     MSP (passport)     │
                │  travels with agent    │
                │                        │
                │  - sessions            │
                │  - episodic memory     │
                │  - consolidator        │
                │  - retrieval orch.     │
                │  - context compression │
                │  - identity / soul     │
                │  - validator (gks adp.)│
                │  - codegen runner      │
                └───────────┬────────────┘
                            │ knowledge queries
                            ▼
                ┌────────────────────────┐
                │   GKS (knowledge)      │
                │  atomic .md + wikilinks│
                │  + atomic_index.jsonl  │
                │  + backlinks.jsonl     │
                └───────────┬────────────┘
                            │ runtime
                            ▼
                ┌────────────────────────┐
                │   Obsidian (vault)     │
                │  - file watching       │
                │  - text search         │
                │  - graph view          │
                │  - Local REST API      │
                │  - Smart Connections   │
                │    (local embeddings)  │
                │  - obsidian-mcp        │
                └────────────────────────┘
                            │ scale-up later
                            ▼
                  [vector DB / graph DB]
```

## Roles

### MSP — passport

Owns everything that **travels with the agent's identity**:

| Concern | Module | Status |
|---|---|---|
| Per-turn log | `src/memory/sessions/` | ✅ M3c-2 |
| Episodes (what mattered) | `src/memory/episodic/` | ✅ M3c-3 |
| Consolidator (importance + summarise) | `src/orchestrator/consolidator.ts` | ⏳ M7b |
| Retrieval orchestration (fuse Obsidian + episodic) | `src/orchestrator/retrieval.ts` | ⏳ M7c |
| Context compression (token-budget) | `src/orchestrator/compressor.ts` | ⏳ M7d |
| Identity / soul | `src/identity/` | ⏳ M7e |
| GKS write-path validator | `src/validator/` | ✅ M2/M3b/M5c/M5d |
| Codegen runner | `src/codegen/` | ✅ M3c-4/M4b/M4c |
| MCP tool surface | `src/mcp/` | ✅ M6 |
| Hooks (pre-commit/pre-push) | `examples/hooks/` | ✅ M3a/M5a/M5b |

### GKS — knowledge

Markdown + wikilinks + index files. **No code at runtime** — Obsidian is the runtime.

### Obsidian — runtime

Provides file watching, text search, graph view, REST API, plugin ecosystem. MSP delegates to it via `ADR--MSP-OBSIDIAN-INTEGRATION`.

### Smart Connections — embedding plugin

Local embeddings inside Obsidian's process. MSP delegates semantic search to it via `ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS`. MSP **never embeds**.

## Data flow

### Write
```
Agent → msp_remember(turn) → sessions writer (append JSONL)
                          ↓ if importance threshold met
                       consolidator → episodic writer (append episode)
                          ↓ if context overflow
                       compressor → summarise old episodes → re-write
```

### Read
```
Agent → msp_recall(query)
        ↓
    retrieval orch.
        ├── Obsidian REST text search (keyword)
        ├── Smart Connections semantic search (if Obsidian + plugin live)
        ├── episodic memory read (MSP-owned)
        └── backlinks.jsonl traversal (graph hop)
        ↓
    RRF merge → ranked top-K → returned with provenance
```

## What v1 got right

- **Inbound queue** as the only write-path to `gks/` — still authoritative.
- **Validator + forbidden-fields + anti-hallucination rules** — still the gatekeeper for atom writes.
- **Doc-to-code phase governance** — still enforces the workflow.
- **Microtask + codegen runner** — still the way SLM output enters `src/`.

These all stay in v2; they just live inside the passport, not as the passport itself.

## What v1 got wrong

- **Treated MSP as primarily a gatekeeper.** That's a *function* of MSP, not its identity. The identity is the passport — memory + soul + retrieval orchestration that travels with the agent. Gatekeeping is one capability among many.
- **Implied search/graph/embedding live "outside" or in some abstract orchestrator.** They don't — they live in Obsidian. The orchestration that fuses them lives in MSP.

## Migration

- v1 atom (`FRAME--MSP-ARCHITECTURE`) marked `status: superseded` + `superseded_by: [FRAME--MSP-ARCHITECTURE-V2]`. Body unchanged for historical reference.
- All atoms previously crosslinked to v1 stay valid; they reference v1 frame for context, and v2 supersedes it.
- M7 implementation work (consolidator, retrieval, compressor, identity, Obsidian client) lands per the table above.

## Source

User's M7-prep architectural clarification. Previous: `FRAME--MSP-ARCHITECTURE` (v1).
