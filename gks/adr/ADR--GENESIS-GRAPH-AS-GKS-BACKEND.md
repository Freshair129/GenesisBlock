---
id: ADR--GENESIS-GRAPH-AS-GKS-BACKEND
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Genesis Graph Backend ships as a GraphBackend implementation, not a
  parallel engine
tags: &a1
  - msp
  - gks
  - graph
  - backend
  - genesis-block
  - cypher
  - rust
  - napi
  - decision
crosslinks: &a2
  references:
    - CONCEPT--GENESIS-GRAPH-BACKEND
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - PROTOCOL--GENESIS-GRAPH-FFI
created_at: 2026-05-12T11:57:00.000+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--GENESIS-GRAPH-AS-GKS-BACKEND
  phase: 2
  type: adr
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Genesis Graph Backend ships as a GraphBackend implementation, not a
    parallel engine
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-12T11:57:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--GENESIS-GRAPH-AS-GKS-BACKEND
    phase: 2
    type: adr
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Genesis Graph Backend ships as a GraphBackend implementation, not a
      parallel engine
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-12T11:57:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--GENESIS-GRAPH-AS-GKS-BACKEND
      phase: 2
      type: adr
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Genesis Graph Backend ships as a GraphBackend implementation, not a
        parallel engine
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-12T11:57:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Architecture decision record
      attributes:
        domain: adr
      domain: adr
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — Genesis Graph Backend as a GKS backend

## Context

`[[CONCEPT--GENESIS-GRAPH-BACKEND]]` frames the desire for a sovereign,
embedded, Cypher-capable graph engine forked from LadybugDB. The
remaining question is **how it integrates with the existing stack**.

Three options were considered.

| Option | What it means | Cost | Risk |
|---|---|---|---|
| **A — Peer package** | New `packages/genesis-block/` peer of GKS; MSP chooses which storage to talk to at config time. | New MCP surface, duplicates `retain` / `recall` / `reflect`, two source of truth for bi-temporal logic. | High — re-implements GKS contracts; drift inevitable. |
| **B — Replace GKS** | Migrate `packages/msp/` to talk to the new engine; delete `packages/gks/`. | Rewrite all four GKS layers (Atomic / Vector / Episodic / Obsidian). Migrate 60+ ADRs / FRAMEs. | Very high — multi-quarter; breaks every dependent. |
| **C — Backend of GKS** (chosen) | New engine implements `GraphBackend` (`packages/gks/src/memory/graph.ts:375`); GKS still owns `MemoryStore` and the public API. | Single Rust crate + thin TS adapter (~600 LoC). | Low — additive; ADR-003 already authorises new backends. |

Option C is the only one that honours
[`ADR-003`](../../docs/gks/adr/003-pluggable-backends.md)
(*pluggable backends are first-class*), and avoids contradicting
[`ADR-008`](../../docs/gks/adr/008-gks-storage-engine-scope.md)
(*GKS is the storage engine — orchestration lives above*).

## Decision

**The Genesis Graph Backend is implemented as a new GKS `GraphBackend`.**

Concretely:

1. **Code lives at** `packages/gks/src/memory/graph/genesis-graph.ts`
   (TypeScript adapter) and `packages/gks/native/genesis-block/`
   (Rust crate, forked from LadybugDB).
2. **The adapter implements the full `GraphBackend` surface** — five
   methods declared at `packages/gks/src/memory/graph.ts:375`:
   `load`, `addNode`, `addEdge`, `retractEdge`, `query`, `neighbors`.
3. **The Rust crate exposes its surface via `napi-rs`** as a single
   `.node` native addon, prebuilt for `darwin-x64`, `darwin-arm64`,
   `linux-x64-gnu`, `linux-arm64-gnu`, `win32-x64-msvc` and shipped
   in the `@freshair129/gks` npm tarball under `native/prebuild/`.
4. **No new MCP tools are added.** The existing 13 GKS MCP tools call
   into `MemoryStore`, which calls the configured `GraphBackend` —
   Genesis Block sits below that line and inherits the surface for
   free.
5. **The default `GraphBackend` flips to `GenesisGraphBackend` after
   P3.5 benchmarks** (see BLUEPRINT). Until then, new `MemoryStore`
   instances use the in-memory `GraphStore` to preserve the zero-dep
   quickstart promise. After the benchmark gate, `GenesisGraphBackend`
   becomes the default for persisted memory; `GraphStore` remains
   available as an explicit opt-in for tests and ephemeral sessions.
   The default-flip is documented in BLUEPRINT phase P3.6 and requires
   a separate ADR amendment PR — it is **not** an automatic consequence
   of this ADR landing.
6. **`PgGraphBackend` is reframed as the *reference baseline* for
   correctness testing**, not a parallel production target. During
   P3.2–P3.5, every test in `test/memory/graph/` runs against both
   `PgGraphBackend` (oracle) and `GenesisGraphBackend` (subject under
   test) and asserts result equivalence. After the default-flip,
   `PgGraphBackend` remains available for two narrow scenarios:
   (a) multi-process / multi-machine deployments where embedded
   single-writer semantics do not fit, (b) organisations with
   existing Postgres infrastructure that prefer SQL-level admin
   tooling. Deprecation is **out of scope** for this ADR.
7. **OpenCypher is exposed as an opt-in extension method**, not part
   of the base `GraphBackend` interface. Callers that want to issue
   Cypher use a downcast: `(backend as GenesisGraphBackend).cypher(q)`.
   The MSP `verify-flow` and Impact-Analysis paths gain a Cypher
   path; everything else continues to use `neighbors()` / `query()`.
8. **The FFI contract between the TS adapter and the Rust crate is
   pinned by `[[PROTOCOL--GENESIS-GRAPH-FFI]]`** (companion atom). Every
   `#[napi]` export in `packages/gks/native/genesis-block/` and every
   TS method on `GenesisGraphBackend` must match that PROTOCOL; ABI
   drift across the boundary is a runtime crash, not a type error.

### Reconciling with ADR-005

[`ADR-005`](../../docs/gks/adr/005-cut-falkordb.md) cut
FalkorDB partly to remove a Redis-module native dependency. Adding a
`.node` addon at first glance re-introduces native code — but the
trade-off is different:

- FalkorDB required a **running Redis server** at runtime; users had
  to install and start a separate process.
- Genesis Block ships as a **process-local addon** loaded once at
  startup. No separate process, no port, no daemon.
- Prebuilt binaries are now standard practice for the Node ecosystem
  (`@napi-rs/cli`, `prebuildify`, `node-pre-gyp`). The install-time
  failure modes that drove ADR-005 (compilation toolchain dependence)
  are mitigated.

ADR-005 stays in force for **server-class native dependencies**;
this ADR carves out a narrow exception for process-local prebuilt
addons.

### Reconciling with ADR-001

[`ADR-001`](../../docs/gks/adr/001-file-based-vector-store.md)
chose JSONL as the default vector store for diff-ability and git
friendliness. This ADR does **not** touch the vector layer. The graph
backend's `.db` file is opaque binary; that is acceptable because:

- The source of truth for atom content remains the `.md` files in
  `gks/<type>/`.
- The graph backend's `.db` is a **derived index**, like a SQLite
  cache. Losing it is recoverable by replaying the parser over the
  Markdown vault.
- The user-visible `git diff` story is preserved because the `.db`
  file is gitignored; only the Markdown atoms it indexes are
  committed.

A `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` task tracks adding a
`.gitignore` entry for `*.gks-graph.db` in the published templates.

### PgGraphBackend → reference baseline (transition plan)

`PgGraphBackend` (currently a peer production target in
`packages/gks/src/memory/graph/pg.ts`) is repositioned in three
stages, tied to BLUEPRINT phases:

| Phase | `PgGraphBackend` role |
|---|---|
| Today → P3.4 | Production peer (unchanged behaviour) |
| P3.2 → P3.5 | **Oracle** in the parametrised graph test suite — every assertion runs against PG first, then `GenesisGraphBackend`, and the two outputs must agree |
| P3.6+ | Optional backend retained for two narrow scenarios: (a) multi-process / multi-machine deployments, (b) orgs with existing Postgres + SQL-admin requirements |

Full deprecation is **out of scope**. If single-process scenarios
dominate after one quarter of production GenesisBlock usage, a
separate ADR may propose retiring `PgGraphBackend`; that decision
belongs to its own ADR, not this one.

## Consequences

### Positive

- **No public-API churn.** `retain` / `recall` / `reflect` / MCP tools
  / CLI all unchanged. Existing tests in `packages/gks/test/memory/`
  cover the new backend by parameterisation.
- **Cypher as opt-in superpower.** MSP's Impact-Analysis path can
  migrate to one-line Cypher queries; everything else stays.
- **Sovereignty preserved.** The fork is the source of truth for the
  storage format; we control the roadmap.
- **Performance ceiling raised.** A columnar, on-disk store with a
  proper page cache outperforms the in-memory `GraphStore` once edge
  count exceeds a few hundred thousand.

### Negative

- **New native build pipeline.** GKS's CI must now build and publish
  prebuilt `.node` artifacts across five target triples. Adds ~10 min
  to release time.
- **Rust toolchain becomes part of GKS's developer onboarding.**
  Contributors who only touch TypeScript are unaffected
  (`npm install` pulls the prebuilt binary), but anyone modifying
  the engine needs `rustup`.
- **Schema migrations gain a second axis.** The `.db` format will
  evolve; we need a version byte at the front and a migration
  routine, mirroring the manifest-version policy GKS already has
  (`src/lib/schema-version.ts`).

### Neutral

- The PRD's framing of an "active engine" with Parser + Syncer +
  Analytic + Interface collapses to: Parser stays in MSP, Syncer is
  the existing `npm run msp:index`, Analytic is `verify-flow` /
  `validateLinks` / Impact-Analysis (some moving to Cypher),
  Interface is the existing MCP server. The engine itself is
  passive — same as the rest of GKS per ADR-008.

## Status

**Draft.** This ADR is the artifact for the P2 decision step of
`[[CONCEPT--GENESIS-GRAPH-BACKEND]]`. Promotion to `stable` requires:

1. A green CI run against the existing graph test suite with the
   `GenesisGraphBackend` swapped in via `parametrize`.
2. A benchmark run on the 50k/500k fixture from CONCEPT's success
   criteria.
3. A reviewer sign-off on the ADR-005 / ADR-001 reconciliation
   sections above.

## Connections

- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
