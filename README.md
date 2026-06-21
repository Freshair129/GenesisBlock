# GenesisDB

GenesisDB is an **embedded, local-first hybrid graph + vector engine for AI agent
memory and analytics**. A single in-process Rust core (storage + WAL, HNSW vector
index, index-backed property graph, bitemporal/event-sourced model, governance
tiers, optional CRDT sync) compiles to a Node.js NAPI addon and an Axum REST
server. Nearest comparators are embedded engines (Kuzu, DuckDB+graph,
RocksDB+graph); Neo4j/Qdrant are references, not the category.

**Benchmarked, not narrated** — see the [consolidated performance report](docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md) (audits P14–P25).

## Measured performance (2026-06-21, on SSD; see report)

- **Vector k-NN** (bge-m3 1024-dim, 100k): recall@10 0.984 @ ~1.1 ms p50 — at
  parity with Chroma on the same recall↔latency frontier.
- **Graph traversal**: 1-hop p50 ~22 µs (~42k/s), **O(neighborhood) not O(N)**
  across 10k→1M; **7–185× faster than server Neo4j** on k-hop.
- **Incremental K-Impact**: O(V_affected), ~1.7 µs flat — up to 398,000× faster
  than a full O(V) recompute. **Governance guard**: <0.1% of a write.
- **Durable ingest**: ~2,000 vec/s bulk; 839 TPS concurrent (12 writers).

## Documentation Entrypoints

- Performance & competitive report: [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md](docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md)
- Architecture index / C4 map: [docs/C4--GENESISDB-ARCHITECTURE.md](docs/C4--GENESISDB-ARCHITECTURE.md)
- Authoritative parent specification: [docs/MASTER-SPEC--GENESIS-DB.md](docs/MASTER-SPEC--GENESIS-DB.md)
- Whitepapers: [docs/WHITEPAPER--GENESIS-DB.md](docs/WHITEPAPER--GENESIS-DB.md), [docs/WHITEPAPER--GENESIS-KNOWLEDGE-SYSTEM.md](docs/WHITEPAPER--GENESIS-KNOWLEDGE-SYSTEM.md)
- Agent context: [AGENT.md](AGENT.md) · Contributor workflow: [CONTRIBUTING.md](CONTRIBUTING.md)
- _Note: `docs/API_REFERENCE.md` is stale/corrupted (contains a leaked LLM transcript) — pending regeneration from code; trust `src/main.rs` routes + the report meanwhile._

## Core Capabilities

- Durable node and edge ingestion through WAL-backed storage.
- HNSW-backed semantic search with Thai-aware lexical matching.
- HQL query execution for search, traversal, hybrid retrieval, and context.
- Graph Retrieval Layer (GRL) for tiered agent context packages.
- Bitemporal node evolution through supersession rather than destructive overwrite.
- Governance and consensus primitives for multi-agent knowledge workflows.
- REST, N-API, MCP, Python SDK, and Go SDK interfaces over the Rust engine.

## Quick Start

Build the Rust workspace:

```bash
cargo build
```

Run the standalone server:

```bash
cargo run --bin genesis-db-server
```

The server listens on port `3000` by default and exposes routes under `/v1/*`.

Run the Node/MCP test suite:

```bash
node --test __test__/*.mjs
```

Run the dashboard locally:

```bash
cd dashboard
npm install
npm run dev
```

## Optional Dashboard

The dashboard is an optional operational client under `dashboard/`. It is not the core runtime. It reads server status from:

- `GET /v1/status`
- `GET /v1/swarm/status`

Useful dashboard checks:

```bash
cd dashboard
npm run lint
npm run build
```

## Repository Notes

- Start architecture work from the C4 index, then follow links to the master spec, ADRs, feature specs, and code anchors.
- This repository follows Documentation-Driven Development (DDD) and Root Cause Analysis (RCA).
- Generated artifacts such as dashboard build output and Playwright reports are ignored.
