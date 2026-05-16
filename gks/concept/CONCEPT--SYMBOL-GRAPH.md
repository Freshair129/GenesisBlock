---
id: CONCEPT--SYMBOL-GRAPH
phase: 1
type: concept
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Symbol Graph — extracted structural knowledge over source code
tags:
  - msp
  - symbol-graph
  - structural
  - prior-art
crosslinks: {"references":["FRAMEWORK--SYMBOL-GRAPH","FRAMEWORK--MSP-ARCHITECTURE-V2"]}
created_at: 2026-05-09T16:31:00.000+07:00
---

# CONCEPT — Symbol Graph

## Problem

The MSP repo has 183 atoms describing the system at the conceptual level (what each piece is supposed to do, why we chose this approach, what was shipped). It does **not** have a queryable representation of how the actual code is wired:

- `recall()` is called from N call sites in the orchestrator, but nothing tells you N or where
- A class hierarchy spans 4 files; no one document captures the inheritance tree
- Renaming an exported symbol affects its imports in unknown locations until `grep` confirms
- The 70 source files cluster into ~10 logical groups (validator, MCP transport, memory writers, …) but the cluster boundaries are implicit

These are **structural** questions. The atom graph can't answer them — atoms describe intent, not implementation topology. Every agent / engineer working on MSP today does manual grep + read to reconstruct the structure each time.

## What "Symbol Graph" means

A **directed graph** where:

- **Nodes** are source-code **symbols**: functions, methods, classes, interfaces, types, enums, top-level consts, modules (the file itself)
- **Edges** are **structural relations**:
  - `defines(module, symbol)` — declaration site
  - `calls(fn_a, fn_b)` — call expression resolved to a definition
  - `extends(class_a, class_b)` — inheritance
  - `implements(class, interface)` — interface satisfaction
  - `imports(module, external_symbol)` — `import { x } from "y"`
  - `references(any, type_or_const)` — non-call usage (type position, value reference)

Edges carry a **weight** (uniform `1.0` in v1; reserved for future call-frequency or semantic weighting) and a **resolved** flag (0 when the parser can't statically resolve the target, e.g. dynamic require, computed call).

After the graph is built, **Leiden community detection** is run to assign each symbol a `community_id`. Communities approximate "logical modules" — clusters of densely-connected symbols. The MSP repo will probably yield 8–15 communities (validator core, MCP transport, memory writers, codegen runner, retrieval orchestration, web UI, etc.).

## Why a graph and not a list

A flat symbol table is what `ctags` provides — fast lookup by name. The graph adds **traversal**:

- **k-hop neighbors** — answer "what does `recall()` reach?" and "what reaches `recall()`?"
- **Reverse closure** — blast-radius / impact analysis
- **Community membership** — "what else is in the same logical module as `X`?"
- **Cross-edge queries** — "which classes implement `Predicate` AND extend `BaseRule`?"

These are first-class graph operations. Without the graph, every such query is a recursive grep.

## Prior art

| System | What it does | Why we don't just use it |
|---|---|---|
| **ctags / Universal Ctags** | Symbol index (name → file:line) | Flat — no relations, no communities |
| **sourcegraph SCIP** | LSIF / SCIP index format — full LSP-grade graph | External tooling; heavy; cloud-leaning. SCIP is a great schema reference though — we may borrow the index shape later. |
| **scope-graphs / stack-graphs** | Statix-style scope graphs for name resolution | Research-grade; Rust; over-engineered for our 70-file MSP repo |
| **comby** | AST-based search/rewrite | Search-only; doesn't model relations |
| **TypeScript Compiler API** | Native AST + checker for `.ts` | This is what we'll build on — it gives us names, types, references, and import resolution out of the box |
| **tree-sitter** | Multi-language AST parser | Phase 2 — when we want Python/Go/Rust support |

The Symbol Graph isn't novel; the value is in **integrating it as a first-class MSP layer** alongside the atom graph, exposed through the same MCP / CLI / Web UI surfaces engineers already use.

## What this concept does NOT define

- Specific persistence format — see `[[ADR--SYMBOL-GRAPH-PERSISTENCE]]` (PR-2)
- Specific parser choice — see `[[CONCEPT--PARSER-CHOICE]]` (sibling, this PR)
- Specific MCP tool contracts — see `[[FEAT--MSP-SYMBOL-MCP]]` (PR-2)
- Edge weight scheme beyond "uniform 1.0 in v1"
- Hierarchical community detection (top-level only in v1)

## Source

- User design dialogue 2026-05-09 — Tree-sitter + Leiden proposal
- `[[FRAMEWORK--SYMBOL-GRAPH]]` — parent frame
- Prior art: ctags, sourcegraph SCIP, stack-graphs, comby (researched 2026-05-09)

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

