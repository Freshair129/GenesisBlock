---
id: CONCEPT--GENESIS-GRAPH-BACKEND
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Genesis Graph Backend — an embedded graph backend specialised for atomic
  knowledge
tags:
  - msp
  - gks
  - graph
  - backend
  - genesis-block
  - sovereign-stack
  - cypher
crosslinks:
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
created_at: 2026-05-12T11:55:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  domain: concept
---

# CONCEPT — Genesis Graph Backend

## Problem

The MSP / GKS stack today resolves graph queries through one of two
`GraphBackend` implementations (see `packages/gks/src/memory/graph.ts:375`):

1. The in-memory `GraphStore` (default) — fast for small vaults, but the
   entire edge set is loaded per-process; no columnar layout, no
   secondary indices, no on-disk persistence beyond JSONL append.
2. `PgGraphBackend` — production-grade, but introduces a Postgres
   dependency that breaks the "pkg install + go" experience the
   sovereign-stack ethos asks for.

Both work for the four-layer recall path GKS already ships
(`Atomic + Vector + Episodic + Obsidian`). Neither is optimised for the
specific access pattern an atom vault generates at scale:

- **Highly skewed traversal depth.** A `verify-flow` walk from a
  `FEAT--` typically expands 3–6 hops along `references` / `implements`
  / `supersedes` edges. The graph is sparse but deep.
- **Time-travel reads dominate.** Bi-temporal queries (`asOf:
  <timestamp>`) are first-class (ADR-002). Edges have `valid_from` /
  `valid_to`; a single traversal may need to project the graph at any
  past instant.
- **Wikilink edges are auto-generated at sync time.** The parser walks
  thousands of `.md` files and emits edges; the storage layer must
  accept high-throughput batched writes without locking out reads.
- **OpenCypher would be a natural query surface.** Today every
  graph-shaped question is hand-coded against `neighbors()` /
  `query()`. A real Cypher executor would let MSP express
  Impact-Analysis and Traceability queries as one-liners (`MATCH (a)-[:SUPERSEDES*]->(b) WHERE a.id = $id RETURN b`).

The PRD `genesis-block-node v1.2.1` proposes solving (1)–(4) with an
embedded engine, forked from LadybugDB (MIT, Rust), exposing a single
binary `.db` file with columnar indexing and Cypher query support.

## What "Genesis Graph Backend" means here

> **A specialised Knowledge Engine** that, from GKS's point of view,
> is **a new `GraphBackend` implementation** — nothing more, nothing
> less. The "engine" framing in the upstream PRD describes how the
> internals work (active sync, MCP-aware, RI-at-query-time); from the
> GKS layer it must look like any other backend so existing `retain` /
> `recall` / `reflect` callers do not change.

### Two parts (mirrors PRD §1)

| Part | Lives where | Responsibility |
|---|---|---|
| **Storage Core** | Forked Rust crate; loaded via `napi-rs` N-API addon | Binary `.db` file, columnar indices, Cypher executor, page cache |
| **Logic Layer** | TypeScript adapter in `packages/gks/src/memory/graph/genesis-graph.ts` | Implements `GraphBackend` interface; translates `addNode` / `addEdge` / `query` / `neighbors` into Cypher or native FFI calls |

The Parser (Markdown / YAML / Wikilinks) and the MCP surface stay
**outside** the engine — MSP already owns those. Bundling them inside
the engine would re-create the layer violation `[[ADR--GRAPH-IS-GKS-DOMAIN]]`
warns against.

## Why fork LadybugDB rather than wrap an existing graph DB

The Option-comparison from the design discussion settled on **fork**
for these reasons (recorded here so the ADR can refer to a single
source of truth):

1. **Technology sovereignty.** A wrap couples our roadmap to the
   upstream crate's release cadence. A fork lets us patch the storage
   format for bi-temporal edge representation natively (the upstream
   has none).
2. **Tight coupling at the right layer.** The "tightness" we need is
   between the Cypher executor and our edge encoding, not between
   parser and DB — those still stay loosely coupled across the Node
   process boundary. So "fork" here means *fork the storage / query
   crate*, not "rewrite everything from C++".
3. **RI-at-query-time.** The Resonance Index weighting policy
   (`RI_CHANNEL_WEIGHT_POLICY.yaml`) needs to influence ranking inside
   the executor. Adding a per-edge weight column + a `RANK BY ri()`
   Cypher hook is straightforward in the fork; impossible in a wrap.
4. **Lean profile.** Cluster / enterprise-auth code paths in
   general-purpose graph DBs cost RAM and startup time we don't need.
   The fork can strip them.
5. **MCP/A2A as a citizen of the engine.** A2A handoff payloads can be
   served directly from the engine's process without an extra Node
   round-trip — the engine exposes a thin Unix-socket / stdio
   transport that MSP's MCP server forwards.

## What this CONCEPT does *not* commit to

This atom records intent. It does **not** decide:

- Whether the engine ships as a separate `npm` package or lives inside
  `packages/gks/`. (ADR decides.)
- Whether the existing `GraphStore` / `PgGraphBackend` are deprecated.
  (They are not — pluggability per ADR-003 is preserved.)
- The Cypher subset the executor supports in v0. (BLUEPRINT decides.)
- Whether the engine is the default `GraphBackend` for new `MemoryStore`
  instances. (Out of scope until benchmarks land.)

## Success criteria (for the eventual FEAT)

- A `GenesisGraphBackend` class exported from `@freshair129/gks` that
  passes the entire existing `test/memory/graph/*` suite without
  modification, swapped in via the standard `MemoryStore` config knob.
- `npm install @freshair129/gks` on a fresh machine produces a working
  graph backend with **no external services** (no Postgres, no Docker,
  no Rust toolchain at install time — prebuilt binaries via
  `@napi-rs/cli`).
- A `MATCH (a)-[*1..3]->(b)` Cypher query against a 50k-node /
  500k-edge vault returns in <50 ms p50 on a 2026-era laptop.
- Bi-temporal query (`asOf: <past instant>`) yields the same edge set
  as the in-memory `GraphStore` baseline on a frozen fixture.

## Open questions for the ADR

1. **Boundary of the fork.** Do we fork the whole LadybugDB repo, or
   vendor just the storage / query crates and rewrite the surface?
2. **Cypher coverage scope in v0.** Full OpenCypher is a multi-quarter
   project. What is the minimum subset that unlocks Impact Analysis +
   Traceability?
3. **Native dependency policy.** What is GKS's stance on adding a
   prebuilt `.node` artifact to the published `npm` tarball? (ADR-005
   cut FalkorDB partly to avoid native deps — that decision needs
   to be reconciled.)
4. **Where does `genesis-block-node` live in the monorepo?** Sub-package
   inside `packages/gks/native/`, or peer at `packages/genesis-block/`?

These are decided in `[[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]`.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

