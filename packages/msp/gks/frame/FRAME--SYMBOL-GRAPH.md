---
id: FRAME--SYMBOL-GRAPH
phase: 0
type: frame
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Symbol Graph — structural code knowledge axis (orthogonal to 3-tier atom model)
tags:
  - msp
  - symbol-graph
  - architecture
  - structural
  - tree-sitter
  - leiden
  - upstream-gks
crosslinks: {"references":["FRAME--MSP-ARCHITECTURE-V2","FRAME--KNOWLEDGE-3-TIER","CONCEPT--KNOWLEDGE-LAYERS-V2"]}
created_at: 2026-05-09T17:30:00.000+07:00
---

# FRAME — Symbol Graph

## Two graphs, one repo

`FRAME--KNOWLEDGE-3-TIER` defined the **conceptual** axis of MSP: atoms in `gks/<type>/` connected by `crosslinks` (references / supersedes / implements / parent_blueprint / resolves) and tagged with Safety / Master / Genesis tier. That graph captures **why** code exists.

This frame adds an orthogonal **structural** axis: a graph over **source-code symbols** (functions, classes, types, imports) connected by `calls` / `extends` / `implements` / `imports` / `references` / `defines` edges, clustered into communities by Leiden algorithm. That graph captures **how** the code is wired.

```
                      ┌─────────────────────────────────┐
  Conceptual axis ───▶│ Atom Graph (gks/<type>/)        │  why
  (3-tier model)      │  - 183 markdown atoms           │
                      │  - crosslinks.{references,…}    │
                      │  - tier: master/genesis/safety  │
                      └────────────┬────────────────────┘
                                   │ link via
                                   │ linked_symbols[]
                                   ▼
                      ┌─────────────────────────────────┐
  Structural axis ───▶│ Symbol Graph (src/**/*.ts)      │  how
  (this FRAME)        │  - ~5000 symbols                │
                      │  - edges.{calls,extends,…}      │
                      │  - community_id (Leiden)        │
                      └─────────────────────────────────┘
```

The two graphs are **independently queryable** and **link via** `linked_symbols[]` on FEAT atoms. An atom can point to its implementation symbols; a symbol can be looked up to find the atoms that govern it.

## Why this matters

Every MSP session today asks structural questions the 3-tier model can't answer:

- **"What calls `recall()`?"** — reverse-call closure
- **"Which symbols cluster as a logical module?"** — Leiden community
- **"What's the blast radius of changing `edgesFromAtom()`?"** — impact analysis
- **"Where is `ProtoMeta` used?"** — type-position references

Today these require manual `grep` + `git blame` + reading files. The Symbol Graph layer makes them O(1) lookups.

## Hard boundaries

| In scope | Out of scope |
|---|---|
| Read-only structural knowledge over `src/**/*.ts` + `web/src/**/*.tsx` | Refactor / rename tools (IDE territory) |
| Deterministic graph build (sort + seed) | Real-time file-watch — batch CLI only |
| 5 MCP tools + CLI + Web UI tab | Cross-repo symbol linking (post-upstream) |
| Leiden community detection (top-level only in v1) | Hierarchical Leiden levels (Phase 2) |
| TypeScript Compiler API parser (Phase 1) | Multi-language — Python/Go/Rust (Phase 2 via tree-sitter) |
| SQLite + JSONL exports | Embeddings over symbols (semantic search — separate boundary) |

## Path to GKS upstream

Per `CLAUDE.md` two-repo sync rule: prototype in MSP, validate API, then file `upstream/gks-proposals/05-symbol-graph.md` so GKS absorbs the layer (mirroring how GKS today owns `atomic_index.jsonl` + `backlinks.jsonl`). MSP becomes a thin wrapper once GKS ships symbol-graph in a major release.

The 6-PR sequence (PR-1 atoms → PR-6 audit + upstream proposal) is the implementation route. Each PR follows the doc-to-code workflow.

## What this frame does NOT decide

- Specific edge weight scheme (uniform 1.0 in v1; revisit in Phase 2)
- Specific persistence format details — see `ADR--SYMBOL-GRAPH-PERSISTENCE` (PR-2)
- Specific MCP tool contracts — see `FEAT--MSP-SYMBOL-MCP` (PR-2)
- Whether tree-sitter or TS Compiler API is the parser — see `CONCEPT--PARSER-CHOICE` (sibling)

## Source

- User design dialogue 2026-05-09 — proposed Tree-sitter + Leiden combination
- `FRAME--MSP-ARCHITECTURE-V2` — base architecture; this frame extends it orthogonally
- `FRAME--KNOWLEDGE-3-TIER` — conceptual axis sibling
- Prior art: ctags, sourcegraph SCIP, scope-graphs (stack-graphs), comby AST-based search
