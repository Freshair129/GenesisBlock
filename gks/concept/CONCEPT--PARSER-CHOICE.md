---
id: CONCEPT--PARSER-CHOICE
phase: 1
type: concept
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Parser choice — TypeScript Compiler API in v1, tree-sitter deferred to Phase 2
tags: &a1
  - msp
  - symbol-graph
  - parser
  - tree-sitter
  - typescript-compiler-api
  - tradeoff
crosslinks: &a2
  references:
    - FRAMEWORK--SYMBOL-GRAPH
    - CONCEPT--SYMBOL-GRAPH
created_at: 2026-05-09T16:32:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--PARSER-CHOICE
  phase: 1
  type: concept
  status: stable
  tier: genesis
  source_type: axiomatic
  vault_id: default
  title: Parser choice — TypeScript Compiler API in v1, tree-sitter deferred to
    Phase 2
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-09T16:32:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--PARSER-CHOICE
    phase: 1
    type: concept
    status: stable
    tier: genesis
    source_type: axiomatic
    vault_id: default
    title: Parser choice — TypeScript Compiler API in v1, tree-sitter deferred to
      Phase 2
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-09T16:32:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      domain: concept
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — Parser choice for Symbol Graph

## Problem

To extract symbols + edges from `src/**/*.ts` and `web/src/**/*.tsx`, the Symbol Graph layer needs an AST parser with:

1. **Full type/name resolution** — to resolve `foo()` calls to their definition site, including imports
2. **Cross-file binding** — `import { x } from './y'` must link to `y.ts`'s exported `x`
3. **CI-stable install** — works on Node 20 + 22 Ubuntu runners without prebuilt-binary mismatches
4. **Reasonable performance** — ~70 files in MSP today, growing to maybe ~200; build should finish in seconds, not minutes

Two candidates fit MSP's current scope: **TypeScript Compiler API** and **tree-sitter**.

## Comparison

| Dimension | TypeScript Compiler API (`typescript`) | tree-sitter (`tree-sitter` + `tree-sitter-typescript`) |
|---|---|---|
| Already a dep | ✅ (`devDependencies`) | ❌ — adds 2 native packages |
| Install on CI | Pure JS, zero risk | Native module via node-gyp; prebuilds may mismatch Ubuntu Node 20+22 |
| TS support | Native, type-aware | Syntax-only — no type resolution |
| Multi-language | TS + JS + JSX + TSX only | 100+ grammars (Python, Go, Rust, …) |
| Cross-file binding | Built-in via `Program.getTypeChecker()` | Manual scope tracking required |
| Speed | ~2× slower than tree-sitter on huge repos | Fast (incremental parsing) |
| Maturity | Microsoft-maintained | Active (GitHub-funded), but the JS bindings are newer |
| Failure mode | Throws on invalid TS — easy to catch per-file | Recovers gracefully; partial AST on errors |

## Decision (v1)

Use **TypeScript Compiler API** for Phase 1.

Reasoning:

1. **Zero install risk** is decisive for our CI matrix. We've burned time on native-module flakes before (CLAUDE.md "Worktree mode caveat" mentions a related class of issue). Risk-free install matters more than raw speed for a 70-file repo.
2. **Type-aware resolution out of the box** — `program.getTypeChecker().getSymbolAtLocation(node)` resolves a call site to its definition across files for free. With tree-sitter we'd reimplement scope graphs.
3. **Already a dep** — no `package.json` churn, no new deprecation surface.
4. **Speed margin** — ~2× slower than tree-sitter, but our build target is "seconds, not minutes." Tree-sitter's speed advantage is irrelevant at this scale.

## Phase 2 escape hatch

The parser interface is hidden behind:

```typescript
interface SymbolParser {
  parseFile(path: string): { symbols: Symbol[]; edges: Edge[] }
}
```

So swapping engines is a one-file change. When MSP grows beyond TypeScript (e.g. a Python sub-tool, or absorbing a Go service), the tree-sitter parser is added as a **sibling** behind the same interface, and a `meta.parser` field records which engine ran. Both parsers can run in the same build (TS files via TS Compiler API, Python via tree-sitter-python).

The `tree-sitter` + `tree-sitter-typescript` packages are NOT added in v1 — even as `optionalDependencies`. They join the project only when Phase 2 is approved.

## Cost of being wrong

If TS Compiler API turns out to be too slow at MSP's eventual scale (>500 files), we eat the cost of:

- One additional npm dep + native build setup
- Reimplementing the same `parseFile()` method against tree-sitter's grammar
- One PR's worth of work — small

The interface design absorbs this cost; we're not painting into a corner.

## What this concept does NOT decide

- Specific symbol kinds extracted (function vs method vs arrow-const vs class member) — see `[[BLUEPRINT--SYMBOL-GRAPH-CORE]]` (PR-3)
- Which AST nodes count as a `calls` edge vs `references` — see same blueprint
- File include/exclude globs — exposed as a CLI flag with a sensible default

## Source

- User design dialogue 2026-05-09 — Tree-sitter proposed; trade-off analysis is this concept
- npm registry inspection 2026-05-09: `tree-sitter@0.25.0` (native), `tree-sitter-typescript@0.23.2` (native)
- Existing `typescript` dependency in `package.json` `devDependencies`

## Connections
- [[FRAMEWORK--SYMBOL-GRAPH]]
- [[CONCEPT--SYMBOL-GRAPH]]

