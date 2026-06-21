---
version: "0.2.3b"
created_at: "2026-06-13T19:39:22+07:00,ATHER,9b1ced3"
last_update: "2026-06-22T01:30:00+07:00,Claude"
status: "beta"
attributes:
  domain: "agent-governance"
  doc_type: "core-directive"
  scope: "Repository"
  target_path: "G:\\GenesisBlock_Dev\\GenesisBlock\\AGENT.md"
---

# AGENT Context: GenesisBlock / GenesisBlockDB

## Mission

GenesisBlock is a local-first hybrid semantic-graph database engine for AI agents and human-machine collaboration. Treat GenesisBlockDB as the Rust-native backend substrate: storage layer, WAL persistence, DB engine, retriever, in-memory embedding arena, graph traversal, hybrid/vector search, HNSW, symbolic graph reasoning, HQL/AST, community detection, governance, and CRDT sync.

## Operating Workflow

- Default mode: Documentation-Driven Development (DDD) + Root Cause Analysis (RCA).
- Before code changes, inspect parent docs and same-level specs.
- For bugs, identify root cause with evidence and document RCA in `.brain/rca/` before fixing.
- Use the lowest safe complexity level:
  - `C-1`: trivial direct change.
  - `C-2`: feature/API/workflow change, doc approval required.
  - `C-3`: architecture/distributed/dataflow change, doc + diagrams required.
- Keep changes surgical. Do not refactor adjacent code unless required.
- Definition of Done: acceptance criteria met, tests pass, docs updated, no known regressions.

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

## Current Engine Model (shipped 2026-06-22)

- **Edges keyed by `u64` hash.** `edges: DashMap<u64, EdgeOutput>` keyed by `Storage::edge_key(id) = trunc64(SHA256(id))`; `out_idx`/`in_idx: DashMap<u32, HashSet<u64>>`. Edge id strings are **not** interned into `id_to_u32` (nodes only). `ADR--GENESISDB-EDGE-NUMERIC-KEYS`.
- **Per-collection vector spaces.** No global arena/HNSW/`vector_dim`; `Storage.collections: DashMap<String, Arc<VectorCollection>>` (+ `default_collection`). `NodeInput.collection` routes a node's embedding; `HybridSearchInput.collection` scopes + dim-validates search. Snapshot = per-collection `vec_<name>.bin`/`meta_<name>.bin` + a `collections` manifest in `state.json`; legacy single-space DBs migrate to `default`. New REST `/v1/collection/create`, `/v1/collections`; NAPI `createCollection`/`listCollections`. `ADR--GENESISDB-MULTI-COLLECTION`.
- **Async HNSW indexing.** `add_node`/`execute_batch` stage the vector (durable via WAL) and enqueue the HNSW insert onto a per-`Storage` indexing thread — vectors are *eventually searchable*. `flush_index()` for read-your-write; `index_lag()` for backlog; compaction/rebuild flush first. `ADR--GENESISDB-ASYNC-INDEXING`.

## Important Caveats

- Worktree may already contain user changes. Do not overwrite or revert them without explicit approval.
- Some docs contain deprecated notes or encoding artifacts. Prefer the C4 map, current parent docs, and implementation evidence.
- `/v1/query/hql` currently expects a raw JSON string body. Python/Go SDKs appear to send `{ "query": "..." }`, so verify before changing SDK or server behavior.
- HQL grammar SSOT is `src/query/hql.pest` (`src/query/ast.rs` declares `#[grammar = "query/hql.pest"]`). The old root `hql.pest` no longer exists — do not re-create it; keep grammar changes in sync with `src/query/ast.rs`.
- `retract_edge` is currently a stub returning `Ok(None)`.
- `execute_batch` exists in core but is not exposed as a REST route in `src/main.rs`.
- `docs/API_REFERENCE.md` was regenerated from `src/main.rs` (2026-06-22) and is current; `README.md` is high-level — both fine to cite now.

## Common Commands

```bash
cargo test
cargo check
cargo run --bin genesis-db-server
node --test __test__/*.mjs
npm run build
npm run mcp:start
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

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.2.3b | 2026-06-22 | beta | Aligned with shipped engine: u64 edge keys, multi-collection vector spaces, async indexing. | working-tree | Claude |
| 0.2.2b | 2026-06-14 | beta | Clarified backend/runtime ownership and downgraded UI surfaces to optional consumers. | working-tree | ATHER |
| 0.2.1b | 2026-06-14 | beta | Added agent registry SSOT and validation command. | working-tree | ATHER |
| 0.2.0b | 2026-06-14 | beta | Added C4 architecture index entrypoint and clarified SSOT read order for agents. | 4101228 | ATHER |
| 0.1.0b | 2026-06-13 | candidate | Initial AGENT context drafted from repository docs, code, tests, SDKs, MCP, dashboard, and workflow rules. | 9b1ced3 | ATHER |
