---
version: "0.2.5b"
created_at: "2026-06-13T19:39:22+07:00,ATHER,9b1ced3"
last_update: "2026-10-10T12:00:00+07:00,Codex"
status: "beta"
attributes:
  domain: "agent-governance"
  doc_type: "core-directive"
  scope: "Repository"
  target_path: "G:\\GenesisBlock_Dev\\GenesisBlock\\AGENT.md"
---

# AGENT Context: GenesisBlock / GenesisBlockDB

## Mission

GenesisBlock is a local-first hybrid semantic-graph database engine for AI agents and human-machine collaboration. Treat GenesisBlockDB as the Rust-native backend substrate and the application's one database boundary: storage layer, signed WAL durability, engine-owned SQLite projection, graph/vector execution, retriever, HQL/AST, community detection, governance, and CRDT sync.

## Operating Workflow

The repository-wide agent policy (R1-R10), approval workflow, risk classification, and Definition of Done are canonical in [`AGENTS.md`](AGENTS.md). Read and follow that file before acting. This document provides GenesisBlock-specific architecture and capability context; it does not redefine shared workflow rules.

## Execution DAG coordination

The approved local guard contract is [Execution DAG Guard V1](docs/SPEC--EXECUTION-DAG-GUARD-V1.md).
Use `npm run workflow:plan` to check the effective W1 planning overlay and
`npm run workflow:test` for guard fault tests. Historical queue/model fields and
the advisory tiered-swarm router never authorize dispatch. W1 remains disabled;
workflow-hardening approval authorizes only the guard and synthetic pilot.

For guarded runs, keep an immutable plan and durable ledger outside the candidate
checkout. Reserve through the guard before dispatch, record the actual provider
session and requested model/effort, and submit hashed evidence through the adapter.
Receiver ACK is separate from PASS. Only the integrated candidate with fresh,
independent Verify, Review and final Audit receipts may reach Final Gate.

Use GPT-6 Luna Max for Explore, Research, Worker, Verify and Review. Use GPT-6.1
Sol for Decision, critical Debug, Integration, Audit and Orchestration. Author,
integrator and final auditor identities must be separate where specified by the
contract. Three code attempts, one infrastructure retry and two repeated findings
are hard bounds; preserve counters and quarantine expired writers until stop proof.
The CLI checks accepted transitions; it does not sandbox arbitrary processes or
authenticate the model provider. The trusted coordinator owns that boundary.

## Architecture Map

Agent registry SSOT: `.agents/agent-registry.yaml`. Use `agent_id` as the stable identity and `label` as the human-facing name.

Start architecture discovery from `docs/C4--GENESISDB-ARCHITECTURE.md`. Treat it as the architecture index and SSOT map for C1-C4 navigation. The authoritative parent spec remains `docs/MASTER-SPEC--GENESIS-DB.md`.

- Rust core: `src/lib.rs`
  - storage, WAL, HNSW, graph indices, governance, HQL execution, GRL, CRDT, consensus.
- Standalone REST server: `src/main.rs`
  - Axum routes under `/v1/*`, default port `3000`.
- Node/N-API package: `index.js`, `index.d.ts`
  - native binding wrapper and TypeScript surface.
- MCP server: `mcp/server.js`
  - tools: `query_hql`, `retrieve_tiered_context`, `add_knowledge`.
- SDKs:
  - Python: `genesisdb-python/`
  - Go: `genesisdb-go/`
- Optional client/ops surfaces:
  - Dashboard: `dashboard/` (operational consumer, not core runtime)
  - Obsidian plugin: `obsidian-plugin/` (human-facing bridge, not DB ownership)
- Specs and governance docs live in `docs/`, with `docs/C4--GENESISDB-ARCHITECTURE.md` as the architecture entrypoint and `docs/MASTER-SPEC--GENESIS-DB.md` as current parent spec.

## Core Capabilities

- Node/edge ingestion with WAL persistence and signed events.
- Bitemporal nodes/edges using `valid_from`, `valid_to`, `caused_by`, logical clocks, and TTL.
- HNSW-backed vector search over **per-model/dim vector collections** (a `default` always exists; query dim validated per collection). HNSW indexing is **asynchronous** (off the write path; eventually searchable — `flush_index`/`index_lag`). Optional language centroids.
- Thai-aware fuzzy ID matching through combining-mark filtering and lexical similarity.
- HQL commands: `SEARCH`, `TRAVERSE`, `MATCH`, `CONTEXT`.
- Graph Retrieval Layer (GRL): H0-H5 context tiers with token-budget fallback to SuperNodes.
- Governance: external agents cannot directly create `MASTER` tier nodes.
- CRDT sync: Lamport clocks, signed events, LWW-style reconciliation.
- Swarm identity: ed25519 identity stored under database path.
- Autonomic maintenance: pruning, meta-graph generation, state persistence.
- REST status and swarm status expose engine health to optional dashboard and Obsidian consumers.
- SQLite is an internal relational projection for props, labels, joins, and app tables; callers must not treat it as a separate app-owned database.
- Future internal stores should join the Genesis transaction and WAL model instead of reintroducing external dual-write database ownership.

## Current Engine Model (shipped 2026-06-22)

- **One boundary, multiple internal files.** `genesis-graph.wal` is the mutation durability authority. Files such as `state.json`, `nodes.bin`, `edges.bin`, `vec_<name>.bin`, `fvec_<name>.bin`, and `projection.sqlite` are filesystem-backed materialized state, snapshots, or rebuildable projections owned by the engine.
- **SQLite is a derived projection.** `projection.sqlite` exists for props, labels, and relational app data under Genesis ownership. It is rebuildable from Genesis-owned state and must never be treated as an external caller-writable source of truth.
- **Edges keyed by `u128` hash.** `edges: DashMap<u128, EdgeOutput>` keyed by `Storage::edge_key(id) = trunc128(SHA256(id))`; `out_idx`/`in_idx: DashMap<u32, HashSet<u128>>`. Key is derived from `EdgeOutput.id` (never stored authoritatively); legacy u64-keyed snapshots load transparently. Edge id strings are **not** interned into `id_to_u32` (nodes only). `ADR--GENESISDB-EDGE-NUMERIC-KEYS`.
- **Per-collection vector spaces.** No global arena/HNSW/`vector_dim`; `Storage.collections: DashMap<String, Arc<VectorCollection>>` (+ `default_collection`). `NodeInput.collection` routes a node's embedding; `HybridSearchInput.collection` scopes + dim-validates search. Snapshot = per-collection `vec_<name>.bin`/`meta_<name>.bin` + a `collections` manifest in `state.json`; legacy single-space DBs migrate to `default`. New REST `/v1/collection/create`, `/v1/collections`; NAPI `createCollection`/`listCollections`. `ADR--GENESISDB-MULTI-COLLECTION`.
- **Async HNSW indexing.** `add_node`/`execute_batch` stage the vector (durable via WAL) and enqueue the HNSW insert onto a per-`Storage` indexing thread — vectors are *eventually searchable*. `flush_index()` for read-your-write; `index_lag()` for backlog; compaction/rebuild flush first. `ADR--GENESISDB-ASYNC-INDEXING`.

## Important Caveats

- Worktree may already contain user changes. Do not overwrite or revert them without explicit approval.
- Some docs contain deprecated notes or encoding artifacts. Prefer the C4 map, current parent docs, and implementation evidence.
- `/v1/query/hql` currently expects a raw JSON string body. Python/Go SDKs appear to send `{ "query": "..." }`, so verify before changing SDK or server behavior.
- HQL grammar SSOT is `src/query/hql.pest` (`src/query/ast.rs` declares `#[grammar = "query/hql.pest"]`). The old root `hql.pest` no longer exists — do not re-create it; keep grammar changes in sync with `src/query/ast.rs`.
- `retract_edge` performs a bitemporal soft-delete (sets `valid_to`); `neighbors` hides retracted edges from the current view unless `include_invalid = true`. Exposed at REST `/v1/edge/retract`.
- `execute_batch` exists in core but is not exposed as a REST route in `src/main.rs`.
- `docs/API_REFERENCE.md` was regenerated from `src/main.rs` (2026-06-22) and is current; `README.md` is high-level — both fine to cite now.
- Do not propose app-facing direct SQLite, graph, or vector store ownership. The intended topology is `application -> GenesisBlockDB -> internal WAL + projections/indexes`.

## Common Commands

```bash
cargo test
cargo check
cargo run --bin genesis-db-server
node --test __test__/*.mjs
npm run build
npm run mcp:start
npm run agents:sync
npm run agents:validate
```

Dashboard:

```bash
cd dashboard
npm run dev
npm run build
npm run lint
```

## Verification Policy

- Rust core changes: run `cargo test`; use targeted tests first if scope is narrow.
- N-API/MCP changes: run `npm test`.
- REST/server changes: run `cargo check` and relevant integration tests.
- Dashboard changes: run dashboard lint/build and Playwright where applicable.
- Performance-sensitive changes: run relevant benchmark/audit binary before claiming no regression.

## Version Diff

| From | To | Change |
|---|---|---|
| none | 0.1.0b | New repository agent-context document proposed from docs/codebase review. |
| 0.1.0b | 0.2.0b | Added C4 architecture index as the required architecture discovery entrypoint and promoted context to beta. |
| 0.2.0b | 0.2.1b | Added agent registry SSOT and registry validation command to the repository context. |
| 0.2.1b | 0.2.2b | Clarified GenesisBlockDB as backend/runtime engine first; dashboard/Obsidian are optional client surfaces. |
| 0.2.2b | 0.2.3b | Added "Current Engine Model" (u64 edge keys, per-collection vector spaces, async HNSW indexing); refreshed Core Capabilities + the API_REFERENCE caveat. |
| 0.2.3b | 0.2.4b | Clarified one-boundary storage ownership: WAL authority, SQLite as internal derived projection, filesystem snapshot/materialization roles, and no external dual-write store pattern. |
| 0.2.4b | 0.2.5b | Added approved execution DAG guard coordination and delegated shared R1-R10 governance to canonical `AGENTS.md`; retained engine-specific architecture context here. |

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.2.5b | 2026-10-10 | beta | Added approved guard/pilot workflow, retained the separate W1 dispatch boundary, and delegated shared agent governance to `AGENTS.md`. | working-tree | Codex |
| 0.2.4b | 2026-07-23 | beta | Clarified WAL authority, SQLite projection ownership, and single-boundary storage topology for agents. | working-tree | Codex |
| 0.2.3b | 2026-06-22 | beta | Aligned with shipped engine: u64 edge keys, multi-collection vector spaces, async indexing. | working-tree | Claude |
| 0.2.2b | 2026-06-14 | beta | Clarified backend/runtime ownership and downgraded UI surfaces to optional consumers. | working-tree | ATHER |
| 0.2.1b | 2026-06-14 | beta | Added agent registry SSOT and validation command. | working-tree | ATHER |
| 0.2.0b | 2026-06-14 | beta | Added C4 architecture index entrypoint and clarified SSOT read order for agents. | 4101228 | ATHER |
| 0.1.0b | 2026-06-13 | candidate | Initial AGENT context drafted from repository docs, code, tests, SDKs, MCP, dashboard, and workflow rules. | 9b1ced3 | ATHER |
