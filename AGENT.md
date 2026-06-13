---
version: "0.2.1b"
created_at: "2026-06-13T19:39:22+07:00,ATHER,9b1ced3"
last_update: "2026-06-14T00:29:45+07:00,ATHER"
status: "beta"
attributes:
  domain: "agent-governance"
  doc_type: "core-directive"
  scope: "Repository"
  target_path: "G:\\GenesisBlock_Dev\\GenesisBlock\\AGENT.md"
---

# AGENT Context: GenesisBlock / GenesisDB

## Mission

GenesisBlock is a local-first hybrid semantic-graph engine for AI agents and human-machine collaboration. Treat GenesisDB as the Rust-native core memory substrate with graph traversal, vector search, bitemporal history, governance, CRDT sync, MCP access, SDKs, and dashboard/Obsidian integrations.

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
- UI integrations:
  - Dashboard: `dashboard/`
  - Obsidian plugin: `obsidian-plugin/`
- Specs and governance docs live in `docs/`, with `docs/C4--GENESISDB-ARCHITECTURE.md` as the architecture entrypoint and `docs/MASTER-SPEC--GENESIS-DB.md` as current parent spec.

## Core Capabilities

- Node/edge ingestion with WAL persistence and signed events.
- Bitemporal nodes/edges using `valid_from`, `valid_to`, `caused_by`, logical clocks, and TTL.
- HNSW-backed vector search with optional language centroids.
- Thai-aware fuzzy ID matching through combining-mark filtering and lexical similarity.
- HQL commands: `SEARCH`, `TRAVERSE`, `MATCH`, `CONTEXT`.
- Graph Retrieval Layer (GRL): H0-H5 context tiers with token-budget fallback to SuperNodes.
- Governance: external agents cannot directly create `MASTER` tier nodes.
- CRDT sync: Lamport clocks, signed events, LWW-style reconciliation.
- Swarm identity: ed25519 identity stored under database path.
- Autonomic maintenance: pruning, meta-graph generation, state persistence.
- REST status and swarm status for dashboard and Obsidian health checks.

## Important Caveats

- Worktree may already contain user changes. Do not overwrite or revert them without explicit approval.
- Some docs contain deprecated notes or encoding artifacts. Prefer the C4 map, current parent docs, and implementation evidence.
- `/v1/query/hql` currently expects a raw JSON string body. Python/Go SDKs appear to send `{ "query": "..." }`, so verify before changing SDK or server behavior.
- `hql.pest` at repo root is more complete than `src/query/hql.pest`; verify actual grammar source before editing HQL.
- `retract_edge` is currently a stub returning `Ok(None)`.
- `execute_batch` exists in core but is not exposed as a REST route in `src/main.rs`.
- `README.md` and `docs/API_REFERENCE.md` should be treated cautiously until cleaned.

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

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.2.1b | 2026-06-14 | beta | Added agent registry SSOT and validation command. | working-tree | ATHER |
| 0.2.0b | 2026-06-14 | beta | Added C4 architecture index entrypoint and clarified SSOT read order for agents. | 4101228 | ATHER |
| 0.1.0b | 2026-06-13 | candidate | Initial AGENT context drafted from repository docs, code, tests, SDKs, MCP, dashboard, and workflow rules. | 9b1ced3 | ATHER |
