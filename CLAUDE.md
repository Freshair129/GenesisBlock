# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

GenesisBlock / GenesisBlockDB is a local-first **hybrid semantic-graph database engine** written in Rust. A single Rust crate (`genesis-block-native`) compiles to two artifacts from the same core: a `cdylib` Node.js native addon (via NAPI-RS) and an `rlib` consumed by a standalone Axum REST server. Everything else (Python SDK, Go SDK, MCP server, dashboard, Obsidian plugin) is a client over one of those two surfaces.

The whole engine lives in `src/lib.rs` (~2500 lines): storage, WAL persistence, per-collection HNSW vector indexes (async indexing), graph indices, bitemporal node/edge model, governance tiers, HQL execution, Graph Retrieval Layer (GRL), CRDT sync, and ed25519-signed consensus. There is no module split for the core — it is one large file by design.

## Build & run

```bash
cargo build                              # build rlib + bin
cargo build --release                    # LTO release (slow; see Cargo.toml profile)
cargo run --bin genesis-db-server        # REST server, listens on :3000, routes under /v1/*
npm run build                            # napi build --platform --release -> index.win32-x64-msvc.node
npm run build:debug                      # debug native addon (faster iteration)
```

`npm install` is wired to run `npm run build` (the native addon) via the `install` script — be aware it triggers a Rust compile.

## Tests

Two independent test surfaces — run the one matching your change, both for cross-cutting work:

```bash
cargo test                               # Rust integration tests in tests/*.rs
cargo test --test governance_tests       # single test FILE (each tests/*.rs is its own crate)
cargo test --test governance_tests -- master_tier   # single test by name substring
npm test                                 # node --test __test__/*.mjs  (NAPI + MCP surface)
node --test __test__/mcp.test.mjs        # single Node test file
```

Rust tests are **integration tests only** — `src/lib.rs` contains no `#[test]` blocks; everything lives under `tests/` (e.g. `governance_tests.rs`, `crdt_sync_tests.rs`, `grl_retrieval_tests.rs`, `thai_fuzzy_tests.rs`, `temporal_queries_tests.rs`). The `tests/test_*/` subdirectories are fixture databases (WAL + `state.json`) used by those tests, not test code.

## Benchmarks / audits

Several `[[bin]]` targets in `Cargo.toml` are load/audit harnesses, plus a Criterion bench. Run these before claiming no perf regression on storage/index/HQL changes:

```bash
cargo bench --bench ldbc_lite
cargo run --release --bin industrial-audit     # also: scientific-audit, snb-ingestion,
                                               # snb-bulk-ingestion, shadow-sync-stress, hql-query-stress
```

## Other surfaces

```bash
npm run mcp:start            # MCP server (mcp/server.js) — tools: query_hql, retrieve_tiered_context, add_knowledge
npm run agents:validate      # validate .agents/agent-registry.yaml
cd dashboard && npm run dev  # optional operational dashboard (reads /v1/status, /v1/swarm/status)
```

## Architecture notes that span files

- **One core, two front-ends.** `GenesisDatabase` (NAPI class in `src/lib.rs`) wraps `Arc<Storage>` and exposes every operation as an `async` method that offloads to `tokio::task::spawn_blocking`. `src/main.rs` re-wraps the *same* `Storage` as Axum handlers under `/v1/*`. When you add an engine capability, you usually wire it in **both** places (NAPI method + REST route) — they can drift, so check both.
- **HQL pipeline.** Query text → `pest` grammar → `src/query/ast.rs` (`HqlCommand`) → `src/query/mod.rs` `LogicalPlanner::plan` (produces `QueryPlan` of `PlanStep`s) → executed against `Storage`. Commands: `SEARCH`, `TRAVERSE`, `MATCH`/`HYBRID`, `CONTEXT`.
- **Grammar source of truth: `src/query/hql.pest`.** `src/query/ast.rs` declares `#[grammar = "query/hql.pest"]`, so that file is what `pest_derive` loads. The old root `hql.pest` no longer exists — do not re-create it. Keep grammar changes in sync with `src/query/ast.rs`.
- **Bitemporal, append-mostly.** Nodes evolve by `supersede_node` (new version + `caused_by` link), not destructive update. Edges have `valid_from`/`valid_to`. Note `retract_edge` is currently a stub returning `Ok(None)`.
- **Vector spaces are per-collection.** There is no single global vector arena/HNSW/`vector_dim`. `Storage.collections: DashMap<String, Arc<VectorCollection>>` (each owns arena+metadata+HNSW+metric+dim); a `default` collection always exists. `NodeInput.collection` routes a node's embedding; `HybridSearchInput.collection` scopes search and validates query dim. Snapshot = per-collection `vec_<name>.bin`/`meta_<name>.bin` + a `collections` manifest in `state.json`; legacy single-space DBs migrate to `default`. See `ADR--GENESISDB-MULTI-COLLECTION`.
- **HNSW indexing is async.** `add_node`/`execute_batch` stage the vector into the arena (durable via WAL) and **enqueue** the HNSW insert onto a per-`Storage` indexing thread; vectors are *eventually searchable*. Call `flush_index()` for read-your-write (tests do); `index_lag()` reports the backlog. Compaction/rebuild flush first. See `ADR--GENESISDB-ASYNC-INDEXING`.
- **Edges keyed by u64 hash.** `edges: DashMap<u64, EdgeOutput>` keyed by `Storage::edge_key(id) = trunc64(SHA256(id))`; `out_idx`/`in_idx` are `DashMap<u32, HashSet<u64>>`. Edge id strings are **not** interned into `id_to_u32` (only nodes are). See `ADR--GENESISDB-EDGE-NUMERIC-KEYS`.
- **Governance tiers** (`Tier`/`ScalingTier` enums): external agents cannot directly create `MASTER`-tier nodes. Governance is enforced in the engine, not the transport layer — test it via `tests/governance_tests.rs`.
- **CRDT sync & consensus**: Lamport/logical clocks, ed25519-signed `SignedEvent`s, LWW reconciliation, Merkle root. Swarm identity (ed25519 keypair) is persisted under the database path.

## Gotchas

- `/v1/query/hql` expects a **raw JSON string** body, but the Python/Go SDKs send `{ "query": "..." }`. Verify the actual contract before changing either side.
- `execute_batch` exists in the core but is **not** exposed as a REST route.
- Docs under `docs/` are governance/spec-heavy and some contain stale notes or encoding artifacts. Prefer code evidence and the C4 map over prose.

## Where to read next

- `AGENT.md` — operating workflow (Documentation-Driven Development + Root Cause Analysis), complexity levels (C-1/C-2/C-3), verification policy, and caveats. Read this before non-trivial changes.
- `docs/C4--GENESISDB-ARCHITECTURE.md` — architecture index / C4 navigation (the SSOT map).
- `.agents/agent-registry.yaml` — agent identity SSOT (`agent_id` stable, `label` human-facing).
