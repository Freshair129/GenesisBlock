---
id: FRAMEWORK--MSP-ARCHITECTURE-V2
phase: 0
type: framework
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: MSP architecture v2 — passport-orchestrator over Obsidian-backed GKS
tags:
  - msp
  - architecture
  - foundation
  - v2
  - supersede
crosslinks: {"references":["CONCEPT--OBSIDIAN-AS-RUNTIME","CONCEPT--EMBEDDING-STRATEGY","CONCEPT--AGENT-AGNOSTIC","CONCEPT--AGENT-INTEGRATION-PATTERNS","ADR--MSP-OBSIDIAN-INTEGRATION","ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS","ADR--GLOBAL-VS-WORKSPACE"],"supersedes":["FRAMEWORK--MSP-ARCHITECTURE"]}
created_at: 2026-05-03T23:55:07.217+07:00
aliases:
  - FRAMEWORK
  - implementation_flow
  - Governance / architectural framework
cluster: implementation_flow
role: "Governance / architectural framework"
---

# FRAMEWORK — MSP architecture v2

> Supersedes `[[FRAMEWORK--MSP-ARCHITECTURE]]` (v1). v1 framed MSP as a "gatekeeper" — a write-path enforcer for atoms going into `gks/`. That framing was correct for what M0–M6 implemented but **incomplete**: MSP's name (Memory & Soul Passport) implies a passport that travels with the agent, carrying memory + identity + retrieval logic. v1 captured the gatekeeping; v2 captures the passport.
>
> **Updated 2026-05-10 (post-Phase-A–D)**: MSP is now explicitly **agent-agnostic** (`[[CONCEPT--AGENT-AGNOSTIC]]`); the 2-layer mental model below is the MSP-internal split, sandwiched under any cognitive-layer agent (EVA / Hermes / openclaw / Claude Code / Gemini CLI / Antigravity / Cursor / custom). Storage now splits global (`~/.msp/`) vs workspace (`./.brain/msp/projects/<ns>/`) per `[[ADR--GLOBAL-VS-WORKSPACE]]`. MCP surface is **19 tools** (passport + projects + symbols + candidates).
>
> **Updated 2026-05-13 (post-v2.3 taxonomy)**: This atom was renamed from `[[FRAME--MSP-ARCHITECTURE-V2]]` per `[[ADR--TAXONOMY-V2-3-MIGRATION]]`. The prefix `FRAME--` now means **Block Manifest** (runtime entry-point of a Genesis Block, contract: `[[SPEC--GENESIS-BLOCK-MANIFEST]]`); `FRAMEWORK--` carries the prior governance/architecture meaning. The body still uses "FRAME" historically — read as "FRAMEWORK" post-v2.3. Note: "Genesis Block" in this document, when referring to a composite knowledge unit (`[[CONCEPT--GENESIS-GRAPH-BACKEND]]` aside), means **Genesis Block** in v2.3 vocabulary. See `[[CONCEPT--TAXONOMY-V2-3]]` for the full prefix map.

## The three-layer ecosystem

```
┌─────────────────────────────────────────────────────────────────┐
│ COGNITIVE LAYER (agents — one of):                              │
│   EVA  │  Hermes  │  openclaw  │  Claude Code  │  Gemini CLI   │
│         │  Antigravity │ Cursor │ custom (MCP-bridged)          │
└──────────────────────────┬──────────────────────────────────────┘
                           │ uses (agent-agnostic API; see CONCEPT--AGENT-AGNOSTIC)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ MEMORY OS — MSP (this repo, passport)                           │
│   sessions / episodic / consolidator / retrieval / compress     │
│   identity (global+override) / candidates / validator           │
│   19 MCP tools  •  6 CLI bins  •  msp-mcp-server                │
└──────────────────────────┬──────────────────────────────────────┘
                           │ uses (storage primitives)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ KNOWLEDGE BASE — GKS (@freshair129/gks)                         │
│   atomic / vector / episodic / obsidian / graph + audit         │
└──────────────────────────┬──────────────────────────────────────┘
                           │ runtime
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ Obsidian (vault — file watching, search, graph, REST API)       │
└─────────────────────────────────────────────────────────────────┘
```

## The MSP-internal mental model

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
                │  - Meta Learning (MLL) │
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
| Consolidator (importance + summarise) | `src/orchestrator/consolidator.ts` | ✅ M7b |
| Retrieval orchestration (fuse Obsidian + episodic) | `src/orchestrator/retrieval.ts` | ✅ M7c |
| Context compression (token-budget) | `src/orchestrator/compressor.ts` | ✅ M7d |
| Identity / soul (global + per-project override) | `src/identity/` + `src/lib/msp-home.ts` | ✅ M7e + Phase B |
| Project registry / resolution | `src/projects/` | ✅ Phase B |
| GKS write-path validator | `src/validator/` | ✅ M2/M3b/M5c/M5d |
| Codegen runner | `src/codegen/` | ✅ M3c-4/M4b/M4c |
| Symbol graph (parser + impact analysis) | `src/symbols/` | ✅ Symbol Graph PR-1..6 |
| Candidates pipeline (writer + reviewer) | `src/memory/candidates/` | ✅ Inbound→Candidates Phase 1-3 |
| MCP tool surface (19 tools) | `src/mcp/` | ✅ M6 + M7f + Phase B |
| Meta Learning Loop (MLL) | `src/learning/` (proposed) | 🛠️ In-Design ([[FEAT--MLL]]) |
| Hooks (pre-commit/pre-push) | `examples/hooks/` | ✅ M3a/M5a/M5b |

### GKS — knowledge

Markdown + wikilinks + index files. **No code at runtime** — Obsidian is the runtime.

### Obsidian — runtime

Provides file watching, text search, graph view, REST API, plugin ecosystem. MSP delegates to it via `[[ADR--MSP-OBSIDIAN-INTEGRATION]]`.

### Smart Connections — embedding plugin

Local embeddings inside Obsidian's process for the **human browse path**. The agent path uses GKS's `createNomicEmbedder()` directly (per `[[ADR--EMBEDDING-MODEL-PARITY]]`). Both surfaces lock to `nomic-embed-text-v1.5` so they share a vector space.

## Storage layout (post-Phase-B)

Per `[[ADR--GLOBAL-VS-WORKSPACE]]`:

| Concern | Location |
|---|---|
| Identity, preferences, projects registry, cross-project audit | `~/.msp/` (global; overridable via `MSP_HOME`) |
| Sessions, episodic memory, candidates, vector/backlinks | `./.brain/msp/projects/<ns>/` (workspace) |
| Per-project identity override (sparse) | `./.brain/msp/projects/<ns>/identity.override.json` |

Resolution mirrors `git`'s global-vs-local config. `MSP_PROJECT` env or `.mspconfig` selects the project; default is `evaAI` until the named-project registry (`[[CONCEPT--NAMED-PROJECT-REGISTRY]]`) ships fully.

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

- v1 atom (`[[FRAMEWORK--MSP-ARCHITECTURE]]`) marked `status: superseded` + `superseded_by: [[[FRAMEWORK--MSP-ARCHITECTURE-V2]]]`. Body unchanged for historical reference.
- All atoms previously crosslinked to v1 stay valid; they reference v1 frame for context, and v2 supersedes it.
- M7 implementation work (consolidator, retrieval, compressor, identity, Obsidian client) lands per the table above.

## Source

User's M7-prep architectural clarification. Previous: `[[FRAMEWORK--MSP-ARCHITECTURE]]` (v1).

## Connections
- [[CONCEPT--OBSIDIAN-AS-RUNTIME]]
- [[CONCEPT--EMBEDDING-STRATEGY]]
- [[CONCEPT--AGENT-INTEGRATION-PATTERNS]]
- [[ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS]]

