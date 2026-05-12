---
id: ADR--SYMBOLS-FRAMEWORK-AWARENESS
phase: 2
type: adr
status: active
tier: architecture
source_type: axiomatic
vault_id: default
title: ADR — Framework-aware symbol indexing — decompose into CONCEPT + ADR + ALGO + PROTO; initial scope Next.js + Prisma + MCP
tags:
  - msp
  - symbol-graph
  - framework
  - adr
  - decision
  - decomposition
crosslinks: {"references":["CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS","FRAME--SYMBOL-GRAPH"],"supersedes":["FEAT--SYMBOLS-FRAMEWORK-AWARENESS"]}
created_at: 2026-05-12T06:36:00.000+07:00
---

# ADR — Symbol graph framework awareness

## Context

`CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS` establishes the problem and hypothesis: extend the symbol graph beyond syntactic structure with pluggable framework recognizers.

The existing `FEAT--SYMBOLS-FRAMEWORK-AWARENESS` (`status: active`, `created_at: 2026-05-12T05:48:00.000+07:00`) bundled four concerns into one atom:
1. The motivation / scope (CONCEPT material)
2. The decision to add framework awareness + which frameworks to support (ADR material)
3. The recognition algorithms per framework (ALGO material)
4. The structural invariants on emitted nodes/edges (PROTO material)
5. The user-observable behavior (FEAT material)

This violates atom-type semantics per `KNOWLEDGE-TYPES.md` and per the contradiction policy in `MASTER--ATOM-CONTRADICTION-POLICY` (atoms should hold one type of claim each).

## Decision

### 1. Decompose the bundled FEAT into 4 atoms

| New atom | Purpose |
|---|---|
| `CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS` (P1) | Motivation, scope, hypothesis |
| `ADR--SYMBOLS-FRAMEWORK-AWARENESS` (P2, this) | Decision: decompose + select frameworks + recognizer interface |
| `ALGO--SYMBOLS-FRAMEWORK-RECOGNITION` (P2) | Recognition algorithms per framework |
| `PROTO--SYMBOLS-FRAMEWORK-INVARIANTS` (P2) | Validator-checkable invariants on emitted nodes/edges |

The original FEAT is **superseded** (`status: superseded`, reciprocal `superseded_by` pointing at the 4 atoms above).

### 2. Initial framework scope (first implementation phase)

- **Next.js (App Router)** — Page, Layout, Loading, Error, Route, Template, Middleware kinds
- **Server / Client components** — `'use client'` / `'use server'` runtime classification
- **Data fetching** — `generateStaticParams`, `generateMetadata`, `getServerSideProps`, `getStaticProps`, `getStaticPaths`, per-HTTP-verb route handler exports
- **Prisma** — model nodes + relation edges from `schema.prisma`
- **Drizzle ORM** — schema files producing same node/edge shape as Prisma
- **MCP server** — tool registrations via `registerTool({...})` pattern in TypeScript

### 3. Recognizer interface (binding contract for ALGO)

Every framework recognizer in `src/symbols/framework/<framework>.ts` exports:

```ts
export interface FrameworkRecognizer {
  /** Stable identifier (e.g. 'nextjs-app-router', 'prisma', 'mcp-tools'). */
  readonly id: string

  /** Predicate — should this recognizer process this file? */
  matches(absolutePath: string, sourceCode?: string): boolean

  /** Emit framework-typed nodes + edges. */
  recognize(absolutePath: string, repoRoot: string, sourceCode: string): Promise<{
    nodes: FrameworkNode[]
    edges: FrameworkEdge[]
  }>
}
```

Registry in `src/symbols/framework/index.ts` collects recognizers and dispatches per file. Recognizer outputs are merged into the same `GraphStore` as syntactic symbols (no separate store).

### 4. No FRAME atom for this work (deferred)

We considered authoring `FRAME--SYMBOLS-FRAMEWORK-AWARENESS` to encode the recognizer-as-strategy pattern. **Deferred** because the pattern is self-evident from the ALGO atom + recognizer interface above; adding a FRAME would be redundant. Revisit if a future framework requires special handling that the current interface cannot express.

### 5. Hard ADR-required enforcement (general policy)

Concurrent with this ADR, promote the FEAT→ADR linkage check in `src/validator/proto/scaling-level-gate.ts` from `severity: 'warning'` to `severity: 'error'` for atoms `created_at >= 2026-05-12T00:00:00Z`. Older atoms grandfathered (warning only) until retrofitted per the Phase-2 handoff.

Rationale: the rule has been on the books since `PROTO--SCALING-LEVEL-GATE` was authored but was unenforced as warning. Decomposing this FEAT exposed how easily the bundle-everything-in-one-FEAT antipattern recurs. Hardening the rule prevents future regressions.

## Consequences

### Positive
- **Granularity**: each atom edits independently; future changes touch one concern
- **Reusability**: CONCEPT can be cited by other framework work; ALGO is per-framework swappable; PROTO is machine-enforced
- **Validator hardening**: hard ADR-required rule prevents the same bundling antipattern from recurring (no more "soft warning that everyone ignores")
- **Backward-compat**: old FEAT remains in the index as `status: superseded` with reciprocal links — history preserved

### Negative
- **More atoms to maintain**: 4 atoms instead of 1. Mitigation: the scaffold-atom script (Phase-2 handoff PR-C) reduces friction
- **Validator rule may surface debt on existing FEATs**: grandfather clause shields them initially; retrofit work tracked in Phase-2 handoff PR-D

### Neutral
- **No FRAME atom for now**: revisit if recognizer interface needs to evolve

## Alternatives considered

### A. Keep as a single FEAT
Rejected: violates atom-type semantics per KNOWLEDGE-TYPES.md. The original FEAT mixed motivation, decision, algorithms, and invariants — making targeted edits risky and reuse impossible.

### B. Decompose into 3 atoms (no ADR)
The user's first instinct (CONCEPT + ALGO + PROTO). Rejected: every decision-bearing atom needs an explicit ADR per `PROTO--SCALING-LEVEL-GATE`. Without an ADR, the *decision* to use a particular recognizer interface, or to scope to Next.js + Prisma + MCP, is undocumented.

### C. Decompose into 5 atoms (add FRAME)
Considered, but the recognizer interface is well-defined by the ALGO atom's contract. Adding FRAME would be redundant documentation. Deferred to future ADR if the interface needs to evolve in a way the ALGO atom cannot capture.

### D. Multiple ALGO atoms (per recognizer)
Instead of one `ALGO--SYMBOLS-FRAMEWORK-RECOGNITION` covering all recognizers, split into `ALGO--NEXTJS-DETECTION`, `ALGO--PRISMA-EXTRACTION`, `ALGO--MCP-DISCOVERY`. Deferred to implementation time: if any single recognizer grows past ~200 LOC of spec, split it out. Initial scope keeps them unified.

## What this ADR does NOT change

- `FRAME--SYMBOL-GRAPH` — overall architecture untouched
- `FEAT--SYMBOLS-MULTI-LANG` — parser foundation unchanged (this work builds on top)
- `BLUEPRINT--SYMBOL-GRAPH-CORE` — existing core blueprint untouched
- Existing 5 MCP tools — `symbol_search`, `symbol_lookup`, `symbol_neighbors`, `symbol_impact`, `symbol_community` continue working with the new framework-typed nodes (they just see more node kinds)

## Source

- `CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS` (motivation)
- `FEAT--SYMBOLS-FRAMEWORK-AWARENESS` (superseded; what this ADR replaces)
- `FRAME--SYMBOL-GRAPH` (architectural backdrop)
- `PROTO--SCALING-LEVEL-GATE` (the rule this ADR hardens)
- `MASTER--ATOM-CONTRADICTION-POLICY` (decomposition justification)
- `HANDOFF-SYMBOLS-EXPANSION-PHASE-2.md` (implementation handoff to follow this PR)
