---
id: BLUEPRINT--GENESIS-GRAPH-TS-FIRST
phase: 3
type: blueprint
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Genesis Block Phase 0 TS-first backend (staged ahead of the Rust crate)
tags:
  - msp
  - gks
  - graph
  - backend
  - genesis-block
  - typescript
  - blueprint
crosslinks: {"references":["BLUEPRINT--GENESIS-GRAPH-INTEGRATION","ADR--GENESIS-GRAPH-AS-GKS-BACKEND","CONCEPT--GENESIS-GRAPH-BACKEND"]}
linked_symbols:
  - { file: "packages/gks/src/memory/graph/genesis-graph.ts" }
  - { file: "packages/gks/src/memory/graph/cypher-v0.ts" }
  - { file: "packages/gks/src/memory/graph/genesis-graph-errors.ts" }
  - { file: "packages/gks/src/memory/index.ts" }
created_at: 2026-05-12T22:49:00.000+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: "Implementation plan"
---

# BLUEPRINT — Genesis Block Phase 0 (TS-first)

## Why a Phase 0 staged ahead of the Rust crate

`[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` (P3.1 → P3.6) ships the Rust + napi-rs crate. That's a multi-PR effort. Meanwhile callers asking for "complete and ready to use right now" need a `GenesisGraphBackend` that:

1. Implements the full `GraphBackend` surface so existing `MemoryStore` tests pass with no caller changes.
2. Persists across restarts (JSONL event log + manifest version byte, same idiom as `GraphStore`).
3. Exposes the v0 Cypher subset from [[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]] §"Cypher v0 scope" so MSP's `verify-flow` / Impact-Analysis paths can adopt it incrementally.
4. Uses the **same on-disk directory** that the Rust crate will eventually own, so the upgrade is invisible to consumers.

This blueprint covers (1)–(4) in pure TypeScript. The Rust phases stay as written.

## Architectural pattern

```
packages/gks/src/memory/graph/
  genesis-block.ts          ← GenesisGraphBackend class (this BLUEPRINT)
  genesis-block-errors.ts   ← GenesisGraphUnsupportedCypher, GenesisGraphSchemaMismatchError
  cypher-v0.ts              ← hand-written recursive-descent parser for the v0 subset
  pg.ts                     ← unchanged
```

On-disk layout (same dir the Rust crate will adopt):

```
<path>/
  manifest.json          schema_version: '1.0.0'
  genesis-block.jsonl    append-only event log (node | edge | edge_retract)
```

## Active-engine framing (alignment with [[CONCEPT--GENESIS-GRAPH-BACKEND]] §1B)

The PRD framed Genesis Block as Parser + Syncer + Analytic + Interface. These already exist in the monorepo — they were just scattered. With this blueprint they line up cleanly:

| PRD role | Where it lives |
|---|---|
| Parser | `scripts/msp/re-indexer.ts` + `packages/msp/src/symbols/` |
| Syncer | `npm run msp:index` |
| Analytic | `packages/msp/src/memory/backlinks/`, `gks verify-flow`, `validateLinks`, the new `cognitive/recall.ts` hybrid pipeline |
| Interface | `createMspMcpServer` (19 tools) + the new `createCognitiveLayer` facade |
| Storage Core | **this BLUEPRINT** — TS Phase 0 today, Rust later |

The split honours `[[ADR--GRAPH-IS-GKS-DOMAIN]]` ("storage stays in GKS; orchestration lives above the contract").

## Cypher v0 subset (recap from [[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]])

```
MATCH (a:Label {id: 'literal'})-[r:rel1|rel2*N..M]->(b:Label)
[WHERE b.prop = 'literal' [AND b.other = 'lit']]
RETURN b.id [, length(r) AS hops]
```

Anything outside the subset raises `GenesisGraphUnsupportedCypher` with the offending fragment. This includes `OPTIONAL MATCH`, `UNION`, `WITH`, `UNWIND`, `CREATE` / `DELETE` / `SET` / `MERGE`, sub-queries, path-property functions, and pattern quantifiers `+` / `?` — exactly the same boundary the Rust crate will respect.

## Module breakdown

| File | Responsibility |
|---|---|
| `genesis-block.ts` | `class GenesisGraphBackend implements GraphBackend`. Event-replay load(), addNode/addEdge/retractEdge/query/neighbors mirror `GraphStore`. Adds `cypher(query)`. |
| `cypher-v0.ts` | `parseCypherV0(query): CypherV0Plan`. Hand-written recursive descent; ~150 LoC. Throws `GenesisGraphUnsupportedCypher` on anything outside the subset. |
| `genesis-block-errors.ts` | `GenesisGraphUnsupportedCypher` (carries the fragment) + `GenesisGraphSchemaMismatchError`. |

## Wire-up

| File | Change |
|---|---|
| `packages/gks/src/memory/index.ts` | Add `export { createGenesisGraphBackend, GenesisGraphBackend }`. |
| `packages/gks/.gitignore` | Add `*.genesis-block.jsonl` + `genesis-block.jsonl` + `.brain/**/graph/*.jsonl`. |

## Test strategy

1. `test/memory/genesis-block.test.ts` — full contract suite mirroring `graph.test.ts`.
2. `test/memory/genesis-block-cypher.test.ts` — every v0 shape + the unsupported-feature error path.
3. The existing `graph.test.ts` for `GraphStore` is unchanged — both backends must keep passing.

## What this BLUEPRINT does **not** cover

- The Rust crate at `packages/gks/native/genesis-block/`. Tracked in [[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]] P3.1–P3.6.
- Migrating MSP queries to Cypher in production. The cypher() surface is opt-in.
- Benchmarks against the in-memory `GraphStore` — the TS Phase 0 backend is **not** expected to be faster than `GraphStore`; it exists for persistence + the public contract.

## Risks & open questions

| Risk | Mitigation |
|---|---|
| Cypher parser drifts from the Rust grammar | Tests pin the v0 subset; the Rust crate inherits the same test fixtures when it lands. |
| TS implementation paints us into a corner on layout | The directory shape (`manifest.json` + `genesis-block.jsonl`) is identical to what the Rust crate will adopt; only the file format swaps. |
| Schema-version drift between TS and future Rust | `enforceSchemaCompatibility()` already raises `SchemaVersionMismatchError` on major-version mismatch. |

## Connections
- [[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]

