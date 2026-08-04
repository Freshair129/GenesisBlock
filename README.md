# GenesisBlockDB

GenesisBlockDB is a **standalone, embedded, local-first hybrid graph + vector database product** for AI, agent, knowledge, notification, analytics, and other relationship-heavy applications.

Applications should treat GenesisBlockDB as the only database handle or endpoint they open for Genesis-owned data — an embedded SQLite relational projection (properties, labels, joins) lives inside the engine's WAL-durable boundary, not as a caller-managed store. Do not dual-write to separate SQLite, graph, or vector stores behind the engine.

A single in-process Rust core combines storage + WAL, HNSW vector indexes, an index-backed property graph, bitemporal/event-sourced history, generic provenance/governance-supporting primitives, and optional CRDT synchronization. The core compiles to a Node.js N-API addon and an Axum REST server.

GenesisBlockDB is client neutral. **GoVibe is one client, NotiKeeper is another, and future clients may use independent namespaces, schemas, ontologies, and policies without recompiling the database core.** GoVibe-specific GKS/MSP/planning semantics and NotiKeeper-specific notification semantics remain client-owned.

Nearest comparators are embedded engines such as Kuzu, DuckDB combined with graph extensions, and RocksDB-based graph systems. Neo4j and Qdrant are references, not the product category.

**Benchmarked, not narrated** — see the [consolidated performance report](docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md) and the [interactive benchmark dashboard](docs/perf-comparison-dashboard.html).

**New here?** → [Documentation Hub](docs/README.md) · [5-minute Quickstart (Node.js)](QUICKSTART.md) · [Why GenesisBlockDB](docs/POSITIONING.md)

## Product Boundary

```text
GoVibe domain       NotiKeeper domain       Future client domain
      |                     |                         |
      +-------- client adapters / SDK contracts -----+
                            |
                GenesisBlockDB generic core
```

GenesisBlockDB owns generic database behavior:

- node, edge, property, vector, lexical and temporal storage;
- client namespaces and client schema references;
- generic provenance and causality metadata;
- query, durability, backup, restore and recovery contracts;
- SDK, REST, MCP and embedded interfaces.

Clients own:

- ontology and taxonomy;
- canonical identity rules;
- authority and promotion policy;
- planning, notification, or other business workflows;
- application validation and user-facing projections.

## Measured performance

The current evidence-backed report records, on its documented SSD environment:

- **Vector k-NN** (bge-m3 1024-dim, 100k): recall@10 0.984 at approximately 1.1 ms p50, at parity with Chroma on the measured recall/latency frontier.
- **Graph traversal**: 1-hop p50 approximately 22 µs, O(neighborhood) rather than O(N), and 7–185× faster than server Neo4j on the measured k-hop workloads.
- **Incremental K-Impact**: O(V_affected), approximately 1.7 µs flat on the measured workload.
- **Durable ingest**: approximately 2,000 vectors/second bulk and 839 TPS in the measured concurrent write scenario.

Do not reuse these values outside the report's workload, hardware, configuration, and caveats.

## Documentation Entrypoints

### Navigation and ownership

- Documentation hub: [docs/README.md](docs/README.md)
- Active document registry: [docs/DOC-REGISTRY.md](docs/DOC-REGISTRY.md)
- Historical 2026-06-21 implementation-status snapshot: [docs/DOC-STATUS.md](docs/DOC-STATUS.md)

### Product and requirements

- Business requirements: [docs/BRD--GENESISBLOCKDB.md](docs/BRD--GENESISBLOCKDB.md)
- Product requirements: [docs/PRD--GENESISBLOCKDB-PLATFORM.md](docs/PRD--GENESISBLOCKDB-PLATFORM.md)
- Software requirements: [docs/SRS--GENESISBLOCKDB.md](docs/SRS--GENESISBLOCKDB.md)
- Client namespace and schema contract: [docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md](docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md)
- Domain-neutral core decision: [docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md](docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md)

### Architecture and evidence

- Quickstart: [QUICKSTART.md](QUICKSTART.md)
- Positioning: [docs/POSITIONING.md](docs/POSITIONING.md)
- Architecture index / C4 map: [docs/C4--GENESISDB-ARCHITECTURE.md](docs/C4--GENESISDB-ARCHITECTURE.md)
- Technical architecture composition: [docs/MASTER-SPEC--GENESIS-DB.md](docs/MASTER-SPEC--GENESIS-DB.md)
- GenesisBlockDB semantic-substrate whitepaper: [docs/WHITEPAPER--GENESISBLOCKDB-SEMANTIC-SUBSTRATE.md](docs/WHITEPAPER--GENESISBLOCKDB-SEMANTIC-SUBSTRATE.md)
- Historical GKS terminology whitepaper: [docs/WHITEPAPER--GENESIS-KNOWLEDGE-SYSTEM.md](docs/WHITEPAPER--GENESIS-KNOWLEDGE-SYSTEM.md)
- Database whitepaper: [docs/WHITEPAPER--GENESIS-DB.md](docs/WHITEPAPER--GENESIS-DB.md)
- API reference: [docs/API_REFERENCE.md](docs/API_REFERENCE.md)
- Version SSOT: [docs/VERSION.md](docs/VERSION.md)
- Performance and competitive report: [docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md](docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md)
- Benchmark dashboard: [docs/perf-comparison-dashboard.html](docs/perf-comparison-dashboard.html)

Agent context: [AGENT.md](AGENT.md) · Contributor workflow: [CONTRIBUTING.md](CONTRIBUTING.md)

## Core Capabilities

- Durable generic node and edge ingestion through signed-WAL-backed storage.
- One application-facing database boundary over WAL, SQLite projection, and native graph/vector indexes — clients should not dual-write to separate stores.
- Client-defined namespaces, labels, relation types, properties and schema references.
- HNSW-backed semantic search with per-model/dimension vector collections and asynchronous indexing.
- Thai-aware lexical matching and documented cross-lingual behavior.
- HQL query execution for search, traversal, hybrid retrieval and context; typed Query IR is the long-term public boundary.
- Graph Retrieval Layer for tiered or bounded context packages without requiring one client authority model.
- Bitemporal node evolution through supersession rather than destructive overwrite.
- Embedded SQLite projection for node properties, labels, app-defined relational schemas, joins, and SQL-backed filtering/text retrieval.
- Generic provenance, causality, governance-supporting and consensus primitives.
- REST, N-API, MCP, Python SDK and Go SDK interfaces over the Rust engine.

## Storage Model

- `genesis-graph.wal` is the internal durability authority and mutation source of truth.
- Snapshot/state files such as `state.json`, `nodes.bin`, `edges.bin`, `vec_<name>.bin`,
  and `fvec_<name>.bin` are materialized on-disk state used for fast reload and recovery.
- `projection.sqlite` is an engine-owned, rebuildable relational projection. It is not a
  caller-owned database and should not be written directly.
- If GenesisBlockDB gains more internal stores in the future, they should join the same
  Genesis transaction and replay model rather than forcing applications to manage an
  extra external database.

## Quick Start

Build the Rust workspace:

```bash
cargo build
```

Run the standalone server:

```bash
cargo run --features bins --bin genesis-db-server
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

- Start product work from the BRD and PRD.
- Start implementation requirements from the SRS.
- Start architecture work from the C4 index and Master Spec, then follow ADRs, feature specs and code anchors.
- Client schemas belong to clients or adapters; do not add GoVibe or NotiKeeper ontology to the database core.
- Use `docs/DOC-REGISTRY.md` for current document ownership; `docs/DOC-STATUS.md` is historical only.
- This repository follows Documentation-Driven Development and Root Cause Analysis.
- Generated artifacts such as dashboard build output and Playwright reports are ignored.
