---
id: AUDIT--GENESIS-BLOCK-INTEGRATION
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: AUDIT — Genesis Block integration (P3.1–P3.4)
tags:
  - msp
  - gks
  - graph
  - backend
  - genesis-block
  - rust
  - napi
  - cypher
  - audit
crosslinks:
  references:
    - BLUEPRINT--GENESIS-GRAPH-INTEGRATION
    - ADR--GENESIS-GRAPH-AS-GKS-BACKEND
    - ADR--GENESIS-BLOCK-STORAGE-LAYOUT
    - ADR--GENESIS-BLOCK-CYPHER-V0-SURFACE
    - SPEC--GENESIS-GRAPH-BACKEND
    - PROTOCOL--GENESIS-GRAPH-FFI
created_at: 2026-05-19T02:00:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Execution audit
cluster: implementation_flow
role: Execution audit
---

# AUDIT — Genesis Block integration (P3.1–P3.4)

## Context

`[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` phases P3.1 through P3.4 landed across four PRs on `main`: **#148** (Scaffold), **#152** (HALT GATE 1 — `[[ADR--GENESIS-BLOCK-STORAGE-LAYOUT]]`), **#153** (Storage MVP + Bi-temporal + Cypher v0 implementation), and **#154** (HALT GATE 2 — `[[ADR--GENESIS-BLOCK-CYPHER-V0-SURFACE]]`). Phase P3.5 (Benchmarks) and P3.6 (ADR promotion) are tracked elsewhere — this AUDIT covers the integration work, not the final promotion. The end state is a working native Rust backend wired into GKS as an optional drop-in replacement for the pure-TS `GenesisGraphBackend`.

## What shipped

| Phase | PR | What landed |
|---|---|---|
| **P3.1 Scaffold** | #148 | `packages/gks/native/genesis-block/Cargo.toml` crate, `napi-build` toolchain, `schemaVersionSync` / `engineNameSync` constant exports, CI matrix across 4 target triples (`linux-x64-gnu`, `darwin-x64`, `darwin-arm64`, `win32-x64-msvc`). |
| **⛔ HALT GATE 1** | #152 | `[[ADR--GENESIS-BLOCK-STORAGE-LAYOUT]]` — JSONL event log chosen over columnar pages. Matches the pure-TS backend's on-disk shape so workspaces are bit-portable between the two. |
| **P3.2 + P3.3 + P3.4** | #153 | ~554 LOC of Rust (`packages/gks/native/genesis-block/src/lib.rs`: storage MVP, bi-temporal columns, Cypher v0 parser/executor) plus ~223 LOC of TS adapter (`packages/gks/src/memory/graph/genesis-graph.ts` factory + `NativeGenesisGraphBackend` class) plus `optionalDependency` wiring in `packages/gks/package.json`. |
| **⛔ HALT GATE 2** | #154 | `[[ADR--GENESIS-BLOCK-CYPHER-V0-SURFACE]]` — narrowed v0 surface frozen ahead of benchmarking so the perf budget is anchored to a stable query grammar. |

## How it works at runtime

`createGenesisGraphBackend({ path })` is the single entry point. It tries to `require('@freshair129/gks-genesis-block-native')`; on success it returns a `NativeGenesisGraphBackend` wrapping `GenesisDatabase.open({ path })` from the Rust crate; on failure (missing prebuilt `.node` for the host triple, mismatched glibc, etc.) it logs one info-level message and falls back to the pure-TS `GenesisGraphBackend`. Both classes implement `GenesisGraphBackendApi = GraphBackend & { cypher(...) }`, so callers cannot tell which path they got beyond the constructor name.

The persistence layout is identical between backends: `<path>/manifest.json` carries the schema version + engine name, `<path>/genesis-graph.jsonl` carries the event log. A workspace created by the Rust backend is readable by the pure-TS one and vice versa, which is what makes the optional-dependency strategy safe — uninstalling the native binary downgrades performance but never strands data.

## Deviations from BLUEPRINT

- **Phases P3.2–P3.4 shipped in one PR** instead of three. PR #153 bundles Storage MVP + bi-temporal columns + Cypher v0 because another agent (co-Sonnet via Antigravity) committed all three layers together as `b06015a` before a phased separation could happen. Net effect: AUDIT coverage and benchmark gating are simpler, and the boundary tests still exercise each phase independently (storage-only fixtures vs `asOf` fixtures vs Cypher fixtures).
- **Cypher v0 surface narrowed** vs the BLUEPRINT's `§Cypher v0 scope`. See `[[ADR--GENESIS-BLOCK-CYPHER-V0-SURFACE]]` §"Narrowed scope" for the 4 narrowings: seed pattern restricted to `{id: 'literal'}` only; target pattern restricted to label-only; multi-`WHERE` predicates not joined with `AND` (must be separate clauses); edge direction restricted to `->`. The narrowed grammar from PR #154 onward is the canonical v0.
- **`optionalDependency` wired as `file:./native/genesis-block`** rather than `workspace:*`. `packages/gks/native/genesis-block` is nested *inside* the `packages/gks` workspace and is not itself a top-level workspace under `packages/*` / `apps/*`, so `workspace:*` could not resolve. The `file:` reference creates the correct symlink under root `node_modules` and lets `require('@freshair129/gks-genesis-block-native')` work from the pure-TS adapter without any further plumbing.

## Tests

- 28 graph / Cypher integration tests in `packages/gks/test/memory/genesis-graph-cypher.test.ts` pass against the native backend — the factory automatically picks native when the prebuilt `.node` is present in `node_modules`.
- `packages/gks/test/memory/genesis-graph.test.ts` and `memory-store-genesis-graph.test.ts` cover the non-Cypher methods (`addNode`, `addEdge`, `retractEdge`, `query`, `neighbors`) across both backends.
- Rust-side unit tests via `cargo test --lib` cover the schema version constant + engine name plus internal parser cases.
- The CI matrix builds the native crate for 4 target triples: `linux-x64-gnu`, `darwin-x64` (macos-13 runner), `darwin-arm64`, `win32-x64-msvc`.

## Known limitations

- **`NativeGenesisGraphBackend.size()` is adapter-local**: it counts `addNode` calls observed by *this* process instance and does not replay the on-disk log on construction. The pure-TS backend's `size()` reflects fully-loaded state. Tests asserting post-load size against the native backend need fixture-driven seeding inside the same process. Tracked but not blocking — `size()` is not on the hot path for any orchestrator query.
- **The two Cypher parsers are not bit-identical.** The Rust path (`MATCH_RE` / `WHERE_RE` / `RETURN_RE` regexes) and the pure-TS `parseCypherV0` accept slightly different grammars at the edges; queries that exercise the narrowed-vs-wider drift only run cleanly against one path. Documented in `[[ADR--GENESIS-BLOCK-CYPHER-V0-SURFACE]]` §"Neutral" consequences. Callers should stay inside the narrowed v0 surface to keep both paths in sync.

## Next phases

- **P3.5 Benchmarks** — `npm run bench:graph` against a 50k-node / 500k-edge fixture, with the **<50 ms p50** target from `[[CONCEPT--GENESIS-GRAPH-BACKEND]]`. Report will land under `packages/gks/benchmarks/genesis-block/`. The benchmark harness work is tracked in parallel with this AUDIT.
- **P3.6 Promotion** — `[[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]` is already `stable`, so no status flip is required; this AUDIT replaces the formal promotion step. The only remaining work is to confirm P3.5 benchmarks pass the success criterion and to write `AUDIT--GENESIS-GRAPH-BENCHMARK-RESULTS` once P3.5 lands. **This current AUDIT does not block on P3.5.**

## Connections

- [[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]
- [[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]
- [[ADR--GENESIS-BLOCK-STORAGE-LAYOUT]]
- [[ADR--GENESIS-BLOCK-CYPHER-V0-SURFACE]]
- [[SPEC--GENESIS-GRAPH-BACKEND]]
- [[PROTOCOL--GENESIS-GRAPH-FFI]]
