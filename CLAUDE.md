# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

<!-- GENERATED_AGENT_POLICY_START -->
<!-- Generated from AGENTS.md v1.0.0; do not edit this section. -->

## Shared agent policy (R1-R10)

### R1 — Think before action

- State assumptions explicitly. If multiple interpretations exist, present them. Ask when uncertainty materially affects implementation, and push back when a simpler approach is sufficient.
- When assumptions exist, use an `[ASSUMPTIONS]` list before acting.

### R2 — Simplicity first

- Write the minimum that solves the request. Do not add speculative features, one-use abstractions, or unrequested flexibility. Simplify code when a shorter solution is equally clear and correct.

### R3 — Surgical changes

- Touch only what the request requires. Preserve unrelated behavior, documentation, tests, and user work. Do not perform opportunistic cleanup.

### R4 — Goal-driven execution

- Define acceptance, success, and exit criteria before acting. Give a brief plan with a verification check for each step, then continue until the criteria are met.

### R5 — Documentation first; wait for approval

- Before code changes, inspect the high-level parent documents and same-level peer contracts.
- Output the proposed documentation changes, stop, and ask for approval. Do not write code in the same response as the proposal.

### R6 — Root-cause analysis first

- Do not fix a bug by guessing. Support the root cause with evidence and record the RCA in `.brain/rca/` with symptom, evidence, root cause, why detection failed, and prevention.

### R7 — Definition of Done

- Complete only when acceptance, success, and exit criteria are met; relevant tests pass; documentation is updated; and no known regressions remain. Report unverified items explicitly.

### R8 — Change risk assessment

- Classify non-trivial changes before implementation: LOW for isolated changes without schema impact; MEDIUM for cross-module or public API changes; HIGH for architecture, migration, or security-sensitive changes.

### R9 — Scope boundary

- Implement only the approved request. Record unrelated findings separately; do not expand scope to fix them.

### R10 — Complexity-based execution

- Choose the lowest complexity level that preserves correctness, safety, and maintainability. When uncertain, choose the higher level.
- Classify every task: C-1 is direct implementation for trivial changes; C-2 is documentation-driven work; C-3 is architecture-driven work requiring documentation and diagrams.
- Escalate C-1 to C-2 or C-2 to C-3 when uncertainty increases. Do not downgrade after approval without explaining why.
- Verify C-1 with validation, C-2 with tests and documentation review, and C-3 with tests, documentation review, and architecture review.

This shared policy is mandatory. Tool-specific instruction files may add constraints but must not weaken these rules.

### Hotfix exception and workflow

- The doc-first gate may be bypassed only for trivial, non-structural syntax, typo, or basic lint fixes. When uncertain, use doc-first.
- For new features or changes, propose documentation, analyze parent/peer impact, and wait for approval before code.
- For bugs, state `[ROOT CAUSE]` with supporting evidence and the proposed fix first. Wait for approval for complex fixes; apply only the bounded hotfix exception above.
- For new features, end the proposal with: “Please review and approve this documentation. I will generate the code once approved.”
<!-- GENERATED_AGENT_POLICY_END -->

## What this is

GenesisBlock / GenesisBlockDB is a local-first **hybrid semantic-graph database engine** written in Rust. A single Rust crate (`genesis-block-native`) compiles to two artifacts from the same core: a `cdylib` Node.js native addon (via NAPI-RS) and an `rlib` consumed by a standalone Axum REST server. Everything else (Python SDK, Go SDK, MCP server, dashboard, Obsidian plugin) is a client over one of those two surfaces.

Treat GenesisBlockDB as the application's one database boundary. Callers should not open or own separate SQLite, graph, or vector databases for Genesis-managed data.

The whole engine lives in `src/lib.rs` (~6,000 lines): storage, WAL persistence, per-collection HNSW vector indexes (async indexing), graph indices, bitemporal node/edge model, governance tiers, HQL execution, Graph Retrieval Layer (GRL), CRDT sync, and ed25519-signed consensus. The storage core is one large file by design — the only split-out module is `src/query/` (the HQL `pest` grammar + AST).

## Build & run

```bash
cargo build                                       # build cdylib + rlib (bins are off by default)
cargo build --release                             # LTO release (slow; see Cargo.toml profile)
cargo run --features bins --bin genesis-db-server # REST server, listens on :3000, routes under /v1/*
npm run build                                     # napi build --platform --release -> index.win32-x64-msvc.node
npm run build:debug                               # debug native addon (faster iteration)
cargo build --no-default-features --features mobile             # mobile core (no napi/bins/sysinfo)
cargo build --no-default-features --features "mobile ffi" --target aarch64-apple-ios  # iOS staticlib
```

All `[[bin]]` targets (the REST server plus the benchmark/audit harnesses) are gated behind the off-by-default `bins` feature, so default `cargo build` / `napi build` compiles **only** the `.node` cdylib — pass `--features bins` to build or run any of them.

**Mobile / FFI surface (MARK XVI, v0.2.0):** the *same crate* also targets iOS/Android embedded — `--no-default-features --features mobile` drops napi, bins, and `sysinfo` (the RSS probe is bench-only; it'd fail in mobile sandboxes). `--features "mobile ffi"` adds `src/ffi.rs`, a `#[cfg(feature = "ffi")]` C ABI (17 `genesisdb_*` symbols over the sync `Storage` core — the count grew well past the original 8 as the relational/QueryIr surface landed, so check `include/genesisdb.h` rather than trusting a number in prose) behind the shipped iOS xcframework and Android JNI bridge. Desktop and mobile share one engine version — they are one source tree, different feature flags (see `docs/SPEC--MOBILE-SDK.md`). Cross-compile is validated by `.github/workflows/mobile-build.yml` (iOS/Android can't be built on the Windows dev host). Note: `sysinfo` is `optional` + `bins`-only, and `autobenches = false` keeps `cargo clippy --all-targets` from synthesizing phantom bench targets that ignore `required-features`.

**core/napi split (#161):** the napi bindings are gated behind the default-on `napi-bindings` feature. The storage core (`Storage` + all its methods) uses a tiny `Error`/`Result` shim when that feature is off, so the core, the REST server, and the integration tests build as **plain native binaries with no `napi_*` symbols** — which is what lets them link on Linux. Build the standalone server with `cargo build --no-default-features --features bins --bin genesis-db-server`; run the Rust tests anywhere (incl. Linux CI) with `cargo test --no-default-features`. The default build (napi on) is unchanged and still produces the cdylib. The `#[napi]`/`#[napi(object)]` attributes are `cfg_attr`-gated; `GenesisDatabase` (the async wrapper) is the only napi-only type.

There is **no install/prepare build script** (napi-rs convention): `npm install` does not compile anything. A fresh clone must build the addon explicitly with `npm run build` (release) or `npm run build:debug` before `npm test`. Registry **consumers** receive the prebuilt platform addon through napi's `optionalDependencies` (no compile). On publish, `prepublishOnly` (`napi prepublish -t npm`) publishes the per-platform packages; the binaries come from the release-workflow build matrix (`napi artifacts`), not from a publish-time rebuild — so the main package carries no platform `.node`.

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
cargo bench --bench ldbc_lite                            # Criterion bench ([[bench]], not gated)
cargo run --release --features bins --bin industrial-audit   # also: scientific-audit, snb-ingestion,
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
- **One operational boundary, multiple internal stores.** The app-facing contract is GenesisBlockDB itself, not a bundle of databases the caller manages. Signed WAL is the durability authority; native graph/vector structures and `projection.sqlite` are internal materializations or projections owned by the engine.
- **WAL authority vs snapshot/projection files.** `genesis-graph.wal` is the mutation source of truth. Files such as `state.json`, `nodes.bin`, `edges.bin`, `vec_<name>.bin`, `fvec_<name>.bin`, and `projection.sqlite` are filesystem-backed materialized state used for reload, querying, and rebuild. Recovery may trust a durable snapshot for instant load, but rebuildable projections must never outrank the WAL in the durability contract.
- **SQLite is internal.** `projection.sqlite` is an embedded relational projection for props, labels, and app tables. It is rebuildable from Genesis-owned state, and no caller should write to it directly or treat it as an external app database.
- **HQL pipeline.** Query text → `pest` grammar → `src/query/ast.rs` (`HqlCommand`) → `Storage::execute_hql` pattern-matches the command and dispatches **directly** to Storage methods (`hybrid_search`, `neighbors`, `retrieve_context`). There is no intermediate logical plan — the old `LogicalPlanner`/`QueryPlan`/`PlanStep` were dead code, removed in MARK XIV. Commands: `SEARCH`, `TRAVERSE`, `MATCH <pattern>` (Cypher-style linear paths), `MATCH…SIMILAR`/`HYBRID`, `CONTEXT`.
- **Grammar source of truth: `src/query/hql.pest`.** `src/query/ast.rs` declares `#[grammar = "query/hql.pest"]`, so that file is what `pest_derive` loads. The old root `hql.pest` no longer exists — do not re-create it. Keep grammar changes in sync with `src/query/ast.rs`.
- **Bitemporal, append-mostly.** Nodes evolve by `supersede_node` (new version + `caused_by` link), not destructive update. Edges have `valid_from`/`valid_to`. `retract_edge` is a bitemporal soft-delete: it sets `valid_to` (default now) and advances the edge clock; the edge stays in the log and is visible to time-travel queries (`as_of` before the retraction, or `include_invalid = true`) but `neighbors` hides it from the current view. REST: `/v1/edge/retract`.
- **HQL command families.** The current grammar/executor surface distinguishes five families: `SEARCH`, `TRAVERSE`, `MATCH (<pattern>)`, `MATCH <target> SIMILAR ...` (hybrid), and `CONTEXT`. Colon-bearing ids such as `user:5` parse **unquoted** in command `target`/`seed` positions (`qualified_id` in `src/query/hql.pest`); quoted string ids stay accepted. Inside pattern syntax `:` still introduces a label — `(a:Label)` — so it is not a qualified id there.
- **Vector spaces are per-collection.** There is no single global vector arena/HNSW/`vector_dim`. `Storage.collections: DashMap<String, Arc<VectorCollection>>` (each owns arena+metadata+HNSW+metric+dim); a `default` collection always exists. `NodeInput.collection` routes a node's embedding; `HybridSearchInput.collection` scopes search and validates query dim. Snapshot = per-collection `vec_<name>.bin`/`meta_<name>.bin` + a `collections` manifest in `state.json`; legacy single-space DBs migrate to `default`. See `ADR--GENESISDB-MULTI-COLLECTION`.
- **HNSW indexing is async.** `add_node`/`execute_batch` stage the vector into the arena (durable via WAL) and **enqueue** the HNSW insert onto a per-`Storage` indexing thread; vectors are *eventually searchable*. Call `flush_index()` for read-your-write (tests do); `index_lag()` reports the backlog. Compaction/rebuild flush first. See `ADR--GENESISDB-ASYNC-INDEXING`.
- **Edges keyed by u128 hash.** `edges: DashMap<u128, EdgeOutput>` keyed by `Storage::edge_key(id) = trunc128(SHA256(id))`; `out_idx`/`in_idx` are `DashMap<u32, HashSet<u128>>`. The key is always derived from `EdgeOutput.id`, never stored authoritatively — legacy u64-keyed snapshots load transparently (key re-derived). Edge id strings are **not** interned into `id_to_u32` (only nodes are). See `ADR--GENESISDB-EDGE-NUMERIC-KEYS`.
- **Governance tiers** (`Tier`/`ScalingTier` enums): external agents cannot directly create `MASTER`-tier nodes. Governance is enforced in the engine, not the transport layer — test it via `tests/governance_tests.rs`.
- **CRDT sync & consensus**: Lamport/logical clocks, ed25519-signed `SignedEvent`s, LWW reconciliation, Merkle root. Swarm identity (ed25519 keypair) is persisted under the database path.

## Gotchas

- `/v1/query/hql` accepts **both** body shapes — a raw JSON string (`"SEARCH ..."`) and the object the Python/Go SDKs send (`{ "query": "..." }`) — via the `#[serde(untagged)] HqlBody` enum in `src/router.rs` (covered by `tests/rest_api_tests.rs`). Keep that untagged enum if you touch the route.
- `execute_batch` exists in the core but is **not** exposed as a REST route.
- Do not design new features around caller-managed direct SQLite access. If a future relational, document, or KV subsystem is added, it should join the Genesis transaction/WAL model as an internal store, not reopen external dual-write architecture.
- Docs under `docs/` are governance/spec-heavy and some contain stale notes or encoding artifacts. Prefer code evidence and the C4 map over prose.

## Where to read next

- `AGENTS.md` — canonical shared policy R1-R10, approval workflow, and verification policy.
- `AGENT.md` — GenesisBlock architecture context, project-specific commands, and caveats.
- `docs/C4--GENESISDB-ARCHITECTURE.md` — architecture index / C4 navigation (the SSOT map).
- `.agents/agent-registry.yaml` — agent identity SSOT (`agent_id` stable, `label` human-facing).
