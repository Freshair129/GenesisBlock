---
id: CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS
phase: 1
type: concept
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Symbol graph must understand framework semantics, not just syntactic structure
tags:
  - msp
  - symbol-graph
  - framework
  - concept
  - nextjs
  - prisma
  - mcp
crosslinks: {"references":["FRAME--SYMBOL-GRAPH","CONCEPT--SYMBOL-GRAPH","CONCEPT--PARSER-CHOICE"]}
created_at: 2026-05-12T06:35:00.000+07:00
---

# CONCEPT — Framework-aware symbol indexing

## Problem

The current symbol graph (post-`FEAT--SYMBOLS-MULTI-LANG`) extracts syntactic structure — classes, functions, imports — but treats every symbol as semantically equal. It does not distinguish a Next.js Page from an internal helper, a Prisma model from a DTO, or an MCP tool registration from a plain object literal.

For an agent doing **impact analysis** on a real codebase, this is insufficient:

- "What handles `POST /api/orders`?" — syntactic search returns every function named `post`, not the route handler
- "Which routes touch the `Order` entity?" — requires knowing which functions are routes and which call ORM methods on the Order model
- "What MCP tools does this agent expose?" — requires recognizing the registration pattern, not just listing exports

Without framework awareness, the symbol graph is a **leaf-level call graph** with no business semantics layered on top.

## Hypothesis

Layering **framework recognizers** on top of the existing parser pipeline — each recognizer specialized for one framework (Next.js, Prisma, MCP server, FastAPI, etc.) — produces typed nodes (`Page`, `Route`, `Entity`, `Tool`) and typed edges (`HANDLES`, `RENDERS_AT`, `ACCESSES`, `LOADS_FOR`) that turn the graph into an architecture-aware index.

The recognizer interface is small enough that adding support for a new framework is bounded work (one file, one test fixture), not a rewrite of the parser.

## Scope (first phase)

In scope:
- **Next.js (App Router)**: Page / Layout / Loading / Error / Route / Template / Middleware node kinds; server/client runtime classification; data-fetching detection (`generateStaticParams`, `getServerSideProps`, etc.)
- **Prisma**: Model nodes + relation edges extracted from `schema.prisma`
- **Drizzle ORM**: Schema files producing the same shape as Prisma
- **MCP server**: Tool registrations discovered via `registerTool({...})` pattern in TypeScript

## Out of scope (first phase)

- Vue / Svelte / Solid frameworks (different file conventions; defer until demand)
- Deep UI component graph (component tree analysis; separate concern)
- Authorization / RBAC analysis across routes (security-critical; needs separate ADR)
- Bundler-level analysis (esbuild / vite plugin output; separate tool)
- Languages other than TypeScript for framework recognition (Python FastAPI / Django defer to second phase)

## Verification

- A real Next.js + Prisma sample project, indexed by the framework-aware pipeline, produces:
  - ≥ 1 `Page` node per `app/**/page.tsx` file
  - ≥ 1 `Route` node per `app/**/route.ts` file with `HANDLES` edges to each HTTP-verb function
  - ≥ 1 `Entity` node per `model` in `schema.prisma`
- An agent calling `msp_symbol_neighbors` on a `Route` node returns the handler function plus any `Entity` accessed from that handler
- Zero false-positive framework-typed nodes on a plain `src/utils/*.ts` file (non-framework fixture)

## Source

- `FEAT--SYMBOLS-MULTI-LANG` — parser foundation
- `FRAME--SYMBOL-GRAPH` — overall architecture this concept extends
- Original FEAT--SYMBOLS-FRAMEWORK-AWARENESS (superseded by the decomposition this concept opens; see `ADR--SYMBOLS-FRAMEWORK-AWARENESS`)
- Next.js App Router documentation (Vercel)
- Prisma schema reference
