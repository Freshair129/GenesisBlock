---
id: BLUEPRINT--GENESIS-GRAPH-INTEGRATION
phase: 3
type: blueprint
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Genesis Block backend integration plan
tags:
  - msp
  - gks
  - graph
  - backend
  - genesis-block
  - rust
  - napi
  - cypher
  - blueprint
  - implementation
crosslinks: {"references":["ADR--GENESIS-GRAPH-AS-GKS-BACKEND","CONCEPT--GENESIS-GRAPH-BACKEND","FRAMEWORK--MSP-ARCHITECTURE-V2","PROTOCOL--GENESIS-GRAPH-FFI"]}
linked_symbols:
  - { file: "packages/gks/src/memory/index.ts" }
  - { file: "packages/gks/src/memory/graph/genesis-graph.ts" }
  - { file: "packages/gks/src/memory/graph/cypher-v0.ts" }
  - { file: "packages/gks/src/memory/graph/genesis-graph-errors.ts" }
created_at: 2026-05-12T11:59:00.000+07:00
---

# BLUEPRINT — Genesis Block backend integration

```yaml
metadata:
  title: "Genesis Block — Rust fork of LadybugDB, wired in as a GKS GraphBackend"
  parent_adr: ADR--GENESIS-GRAPH-AS-GKS-BACKEND
  parent_concept: CONCEPT--GENESIS-GRAPH-BACKEND
```

## Architectural pattern

A single Rust crate behind a thin TypeScript adapter. The adapter is
the only thing GKS imports; the crate is loaded via `napi-rs` as a
native addon. The five `GraphBackend` methods map 1:1 to FFI calls;
Cypher is exposed as a sixth, opt-in method via a downcast.

```
┌──────────────────────────────────────────────────────────────────┐
│  packages/gks/src/memory/index.ts          ← public API (no change)│
│    retain() / recall() / reflect() / MemoryStore                  │
└────────────────────────────┬─────────────────────────────────────┘
                             │ calls
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│  packages/gks/src/memory/graph/genesis-graph.ts                  │
│    class GenesisGraphBackend implements GraphBackend             │
│      load() / addNode() / addEdge() / retractEdge()              │
│      query() / neighbors()                                       │
│      cypher(q: string)   ← opt-in extension                      │
└────────────────────────────┬─────────────────────────────────────┘
                             │ napi-rs N-API
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│  packages/gks/native/genesis-block/   (Rust crate, fork of       │
│                                        LadybugDB MIT)            │
│    Cargo.toml, src/lib.rs, src/storage/, src/cypher/             │
│    prebuild/ (CI-produced .node binaries, per target triple)     │
└──────────────────────────────────────────────────────────────────┘
```

## Module breakdown

### TypeScript side — `packages/gks/src/memory/graph/genesis-graph.ts`

| Export | Shape | Responsibility |
|---|---|---|
| `createGenesisGraphBackend(opts)` | factory returning `GraphBackend` | Loads the prebuilt `.node` for the host triple, opens / creates the `.db` file at `opts.path`, returns the adapter |
| `GenesisGraphBackend` (class) | `implements GraphBackend` | All five base methods + `cypher(q: string): Promise<unknown[]>` |
| `GenesisGraphBackendOptions` | type | `{ path: string; readOnly?: boolean; pageCacheMB?: number }` |

### Rust side — `packages/gks/native/genesis-block/src/`

| Module | Responsibility |
|---|---|
| `storage/` | Columnar page layout, free-list, page cache. Forked + simplified from upstream. |
| `temporal/` | `valid_from` / `valid_to` columns on every edge row; range predicate pushdown for `asOf` queries. |
| `cypher/` | Cypher parser → physical plan; covers the v0 subset only (`MATCH`, `WHERE`, `RETURN`, variable-length paths). |
| `napi/` | N-API surface — one exported function per `GraphBackend` method, plus `cypher`. |
| `migrations/` | Schema version byte + migration routine matching `src/lib/schema-version.ts` policy. |

## Phase plan

| Phase | Output | Definition of done |
|---|---|---|
| **P3.1 Scaffold** | `packages/gks/native/genesis-block/Cargo.toml` builds locally; one trivial exported `napi` function returns a constant. CI matrix builds for the five target triples. | `cargo test` green; `napi build` produces a `.node` artifact loadable from Node. |
| **⛔ HALT GATE 1** | Human reviewer signs off the **storage layout direction** before any persistent on-disk format is committed. | Reviewer comment with `gate-1: approved` label on the P3.1 PR, **OR** a new `[[ADR--GENESIS-BLOCK-STORAGE-LAYOUT]]` if the approach diverges from the LadybugDB fork. Required because storage format is the most expensive bit to migrate after data lands. |
| **P3.2 Storage MVP** | `addNode` / `addEdge` / `query` over a single-column page layout, no temporal awareness yet. | Existing `test/memory/graph/` suite passes with the new backend parametrised in. |
| **P3.3 Bi-temporal** | `valid_from` / `valid_to` columns; `retractEdge`; `query({ asOf })` honoured. | The bi-temporal sub-suite in `test/memory/graph/` passes. |
| **P3.4 Cypher v0** | Parser + executor for the minimal subset (see "Cypher v0 scope" below). | New test file `test/memory/graph/genesis-block-cypher.test.ts` covers: exact-id `MATCH`, single-hop, variable-length `[*1..N]`, `WHERE` on properties, `RETURN`. |
| **⛔ HALT GATE 2** | Human reviewer signs off the **Cypher v0 surface** before benchmarking locks in performance assumptions. | Reviewer comment with `gate-2: approved` label on the P3.4 PR, **OR** an addendum atom extending the Cypher v0 scope. Required because adding Cypher constructs after benchmark targets are committed forces re-runs. |
| **P3.5 Benchmarks** | `npm run bench:graph` numbers against the existing `GraphStore` baseline, on a 50k-node / 500k-edge fixture. | Report committed under `packages/gks/benchmarks/genesis-block/`; meets the <50 ms p50 success criterion from CONCEPT. |
| **P3.6 Promote ADR** | All above green; ADR-005 / ADR-001 reconciliation re-reviewed; status flipped `draft` → `stable`. | Single follow-up PR flipping atom status + writing `[[AUDIT--GENESIS-BLOCK-INTEGRATION]]`. |

Phases P3.1–P3.2 are the minimum for a "drop-in" experience. P3.3 is
needed before MSP's `verify-flow` can rely on it. P3.4 is what unlocks
the Impact-Analysis migration. P3.5 / P3.6 close out the BLUEPRINT.

### HALT gate rationale

Two human-review checkpoints are mandatory between sub-phases — the codegen
runner must exit non-zero and wait for a reviewer label before continuing:

- **Gate 1 (after P3.1, before P3.2)** — storage layout commitment.
  Storage format is the single hardest thing to migrate after data lands;
  approve the layout before MVP writes the first persistent page.
- **Gate 2 (after P3.4, before P3.5)** — Cypher v0 surface freeze.
  Benchmark numbers presume a fixed query surface; expanding Cypher v0
  after P3.5 forces a re-run and invalidates the success criterion.

These gates apply to **autonomous executor** runs (Gemini yolo, codegen
runner). Solo human contributors may sign off in the PR description itself.

## Cypher v0 scope

Minimum subset, just enough to express the queries `verify-flow` and
Impact-Analysis need today:

```
MATCH (a:Atom {id: $id})-[r:references|implements|supersedes*1..6]->(b:Atom)
WHERE b.status = 'stable'
RETURN b.id, length(r) AS hops
```

Specifically:

- Node patterns with a label and a property map (one label, one
  property at most in v0).
- Edge patterns with a relationship-type union (`r:a|b|c`) and a
  variable-length range (`*N..M`).
- A single `WHERE` clause with conjunctions of equality predicates.
- `RETURN` with a property projection and an aggregate `length()`.

Out of scope for v0 (tracked as separate atoms when needed):

- `OPTIONAL MATCH`, `UNION`, `WITH`, `UNWIND`, parameterised
  sub-queries, `CALL { ... }`, `CREATE` / `DELETE` / `SET` /
  `MERGE`. Write paths stay on the typed `addNode` / `addEdge` /
  `retractEdge` methods.
- Path-property functions (`relationships(p)`, `nodes(p)`).
- Pattern quantifiers (`+`, `?`).

## Test strategy

1. **Reuse existing graph tests** by parametrising the suite over
   the three backends (`GraphStore`, `PgGraphBackend`,
   `GenesisGraphBackend`). The factory function makes this a
   one-line change in `test/memory/graph/setup.ts`.
2. **New Cypher suite** at `test/memory/graph/genesis-block-cypher.test.ts`
   — driven from a fixture vault of ~30 atoms covering every
   crosslink type GKS recognises (`references`, `implements`,
   `supersedes`, `superseded_by`, `derives_from`).
3. **Benchmarks** under `packages/gks/benchmarks/genesis-block/`,
   following the existing `bench:locomo` / `bench:longmemeval`
   pattern: JSON + Markdown output, git SHA stamped.
4. **CI matrix** must build the Rust crate and run the parametrised
   graph suite on `ubuntu-latest` and `macos-latest`, Node 20 + 22.
   The Windows triple is built but not tested in CI v0.

## Integration touch-points (what other code learns)

| File / area | Change |
|---|---|
| `packages/gks/package.json` | Add `optionalDependencies` entries for the per-triple prebuild packages (`@freshair129/gks-genesis-block-linux-x64-gnu`, etc.). Add `napi.binaryName` config. |
| `packages/gks/src/memory/index.ts` | Add `export { createGenesisGraphBackend, GenesisGraphBackend, GenesisGraphBackendOptions } from './graph/genesis-block.js'`. No change to anything else. |
| `packages/gks/.gitignore` | Add `*.gks-graph.db`. |
| `packages/gks/README.md` | One row in the "Backends" table: `GenesisGraphBackend` under the Graph column. |
| `packages/msp/src/orchestrator/impact-analysis/` | (Follow-up, not this BLUEPRINT.) Migrate one query to the new `cypher()` method as proof-of-life. |
| Existing MCP server | **No change.** It calls `MemoryStore`, which routes through the configured backend. |

## What this BLUEPRINT does *not* cover

- **Migrating MSP queries to Cypher.** That is a separate
  CONCEPT / ADR / BLUEPRINT pair to be authored once P3.4 is green.
- **Deprecating the in-memory `GraphStore`.** It stays the default
  per ADR. Whether to switch the default is a benchmark-gated
  decision tracked in a future ADR.
- **A2A handoff transport on the engine side.** PRD §1B mentions
  A2A, but A2A is an MSP concern (passport-level). Routing A2A
  payloads through the engine's process is a follow-up CONCEPT, not
  part of v0.
- **Vector layer.** `pgvector` / `HNSW` / JSONL all unchanged.

## Risks & open questions

| Risk | Mitigation |
|---|---|
| Prebuilt `.node` size bloats `npm install` | Use `optionalDependencies` per-triple so users only download their host's binary. |
| Rust toolchain becomes a contributor barrier | Document `npm install` skipping the build when prebuilt binary exists; only Rust-touching contributors need `rustup`. |
| LadybugDB upstream relicenses or stalls | Fork is hard-pinned at commit `<sha-tbd>`; we already control the patched copy. |
| Cypher parser bugs become hard-to-diagnose recall failures | Cypher path is opt-in; default `neighbors()` / `query()` path stays as ground truth. |
| Schema migration disasters | Version byte + read-then-migrate routine, identical pattern to existing `src/lib/schema-version.ts`. |

## Connections
- [[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]
- [[CONCEPT--GENESIS-GRAPH-BACKEND]]
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[PROTOCOL--GENESIS-GRAPH-FFI]]

