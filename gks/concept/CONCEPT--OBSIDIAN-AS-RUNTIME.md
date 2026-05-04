---
id: CONCEPT--OBSIDIAN-AS-RUNTIME
phase: 1
type: concept
status: stable
vault_id: default
title: Obsidian as MSP/GKS runtime — vault, search, graph, REST, plugins for free
tags:
  - msp
  - gks
  - obsidian
  - runtime
  - architecture
  - foundation
crosslinks: {"references":["FRAME--MSP-ARCHITECTURE-V2"]}
created_at: 2026-05-03T16:55:05.485Z
---

# CONCEPT — Obsidian as runtime

## Problem

If MSP/GKS is a Memory & Soul Passport plus an atomic knowledge store, the agent needs **search, graph traversal, file watching, and a UI for humans to browse the same data**. Building those primitives from scratch in MSP would either reinvent existing OSS or push complexity into the orchestrator. Either way, adoption suffers.

## Hypothesis

The atom files under `gks/` are markdown with YAML frontmatter and `[[wikilink]]` syntax — exactly the format Obsidian has been optimised for over years. If the user's `gks/` directory is **also their Obsidian vault**, Obsidian provides for free:

- **Search** — both built-in (text + tags + frontmatter) and via the Local REST API plugin (HTTP-queryable)
- **Graph view** — visual + filterable; backlinks pane resolves crosslinks live
- **File watching** — re-index on save without MSP code
- **Plugin ecosystem** — Smart Connections (local embeddings), Dataview, Templater, etc., none of which we have to write
- **A UI for humans** — the same vault is browsable; no separate "MSP web UI" needed
- **MCP server** — `obsidian-mcp` exposes vault operations to agents directly

MSP becomes the **passport that consumes Obsidian as the runtime**, not the system that re-implements it.

## What this means concretely

- **GKS atoms** stay as markdown. No DB, no API server of MSP's own for search.
- **Wikilinks/crosslinks** are resolved by Obsidian; MSP just writes them and reads them.
- **Backlinks** — Obsidian's pane is the human view; `backlinks.jsonl` (M3c-1) is the headless mirror for CI / scripts.
- **Semantic search** — Obsidian plugins (Smart Connections) provide it; see `CONCEPT--EMBEDDING-STRATEGY`.

## What MSP still owns

Everything that is **not** about a single atom on disk:

- Sessions JSONL (per-turn log)
- Episodic memory (what mattered, importance scores, summaries)
- Identity / soul (agent profile, voice, preferences)
- Context compression (token-budget-aware summarisation of past episodes)
- Retrieval orchestration that fuses Obsidian results + episodic memory

## Trade-offs

**Positive**
- Zero MSP code for search, graph, file watching, UI.
- Users already running Obsidian get MSP value with one Local REST API plugin install.
- Plugin ecosystem is the scale-up path: vector → Smart Connections; advanced query → Dataview.

**Negative**
- Hard runtime dependency on Obsidian for richest features (semantic search, graph view).
- File-only fallback for headless / CI / no-Obsidian scenarios.
- Plugin choice becomes a project decision (which embedder, which query plugin).

## Source

Architectural clarification for M7+. Supersedes the implicit "MSP builds it all" framing in `FRAME--MSP-ARCHITECTURE-V2` (v1).
