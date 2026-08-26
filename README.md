# GenesisBlockDB

GenesisBlockDB is a **standalone, embedded, local-first hybrid graph + vector database product** for AI, agent, knowledge, notification, analytics, and other relationship-heavy applications.

Applications should treat GenesisBlockDB as the only database handle or endpoint they open for Genesis-owned data — an embedded SQLite relational projection (properties, labels, joins) lives inside the engine's WAL-durable boundary, not as a caller-managed store. Do not dual-write to separate SQLite, graph, or vector stores behind the engine.

A single in-process Rust core combines storage + WAL, HNSW vector indexes, an index-backed property graph, bitemporal/event-sourced history, generic provenance/governance-supporting primitives, and optional CRDT synchronization. The core compiles to a Node.js N-API addon and an Axum REST server.

GenesisBlockDB is client neutral. **GoVibe is one client, NotiKeeper is another, and future clients may use independent namespaces, schemas, ontologies, and policies without recompiling the database core.** GoVibe-specific GKS/MSP/planning semantics and NotiKeeper-specific notification semantics remain client-owned.

Nearest comparators are embedded engines such as Kuzu, DuckDB combined with graph extensions, and RocksDB-based graph systems. Neo4j and Qdrant are references, not the product category.

**Benchmarked, not narrated** — see the [consolidated performance report](docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md) and the [interactive benchmark dashboard](docs/perf-comparison-dashboard.html).

**New here?** → [Documentation Hub](docs/README.md) · [5-minute Quickstart (Node.js)](QUICKSTART.md) · [Why GenesisBlockDB](docs/POSITIONING.md)

## Current Version

| Field | Value |
|---|---|
| **Version** | `0.2.3` (crate, npm, and `modules.json` in lock-step; `npm run version:check` gates CI) |
| **Milestone** | Mobile SDK — iOS/Android/React Native SDKs published and live (GitHub Releases, GitHub Packages, npm), iOS on-device acceptance verified in CI; GNSE bitemporal line complete (WP-0.1 → WP-3.3) |
| **Status** | Advanced prototype — durable, benchmarked, full Rust + Node suites green |

Version SSOT: [docs/VERSION.md](docs/VERSION.md) · Detailed history: [CHANGELOG.md](CHANGELOG.md)

### What it can do today

- **Hybrid queries in one engine**: HNSW vector search, index-backed graph traversal, and SQL-projected property filtering fused in-process — no cross-store glue code.
- **Bitemporal time travel on two axes**: `valid_at` (when a fact was true in the world) and `tx_as_of` (what the database believed at a past commit) over a framed, signed journal; `recorded_at` is queryable and `caused_by` provenance chains automatically on supersede. Correctness is pinned by a dedicated matrix suite (`tests/bitemporal_matrix_wp31_tests.rs`).
- **Retention profiles**: `frontier_only` (default — folds history at every checkpoint, cost-neutral) or `full` (keeps the replayable journal for time travel); the fold is the single history-destruction boundary, and questions beyond the retained horizon fail loudly (`beyond_horizon`) instead of returning silently wrong answers.
- **Measured cross-dimension advantage**: fused vector+graph+AS-OF queries run **115–188× faster** than the equivalent DIY single-file SQLite assembly at 100k nodes × 1024 dims ([moat verdict](docs/REPORT--G3-MOAT-VERDICT.md)) — disclosed honestly: bulk ingest is currently slower than SQLite's, and the corpus is synthetic (real-corpus run scheduled).
- **Vector memory that scales down**: per-collection vector spaces (own model/dim/metric), asynchronous HNSW indexing, F16/SQ8/BQ quantization with an off-RAM rerank sidecar, and WAL compaction/checkpointing.
- **Embedded everywhere**: Node.js N-API addon, standalone Axum REST server, MCP server, Python and Go SDKs — plus a C FFI (`genesisdb_*`) powering a published iOS `GenesisBlockDB.xcframework` (Swift, GitHub Release asset), an Android `genesisdb-android` `.aar` (GitHub Packages), and a React Native package (npm), all from the same crate. iOS on-device acceptance — the published xcframework consumed via `.binaryTarget` and executed in the iOS Simulator — is CI-verified (see [docs/SPEC--MOBILE-SDK.md](docs/SPEC--MOBILE-SDK.md)).

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
- **Cross-dimension moat bench** (2026-08, 100k×1024 synthetic): fused vector+graph+AS-OF queries 114.9–187.9× versus the DIY single-file SQLite assembly, both sides in-process — see [docs/REPORT--G3-MOAT-VERDICT.md](docs/REPORT--G3-MOAT-VERDICT.md) for the honest caveats (ingest, synthetic corpus, skipped FTS axis).

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

### Storage engine

- **Framed, signed journal as the durability authority** (`wal/active.gwal` + sealed, checksummed history segments): every mutation is a sequenced frame (`frame_seq`); acked writes replay from the durable frontier after any crash. Legacy `genesis-graph.wal` databases migrate transparently on open.
- **Snapshot instant-load**: materialized state files (`state.json`, `nodes.bin`, `edges.bin`, per-collection `vec_<name>.bin`) let reopen skip full replay; the journal remains the source of truth and rebuildable projections never outrank it.
- **Retention profiles** chosen at `open()`: `frontier_only` (default — fold at every checkpoint, cost-neutral), `full` (keep the replayable tx-time history), or `budget:<bytes>`. The fold is the single history-destruction boundary; time-travel questions past the retained horizon fail loudly with `beyond_horizon`.
- **WAL compaction / checkpointing** to live state, embedded SQLite projection for properties/labels/app tables, and governance/consensus primitives (tiers, ed25519-signed events, CRDT sync).

### Query surface

- **Typed Query IR is the primary machine contract** (`query-ir.v1`): versioned request envelope with `search` (vector / hybrid / lexical) and `traverse` operations, temporal selectors (`valid_at` valid-time + `tx_as_of` transaction-time), per-request consistency (`eventual` / `read_your_write`), strict unknown-field rejection, and a `capabilities` endpoint that discloses exactly what is implemented — wired across core, N-API, REST, and the C FFI/JNI. HQL (`SEARCH` / `TRAVERSE` / Cypher-style `MATCH` / `CONTEXT`) remains a compatibility frontend that lowers onto the same engine paths.
- **HNSW-backed semantic search** with per-model/dimension vector collections, asynchronous indexing (`flush_index()` for read-your-write), and an exact-scan floor guarding recall.
- **Graph Retrieval Layer** for tiered or bounded context packages; Thai-aware lexical matching with documented cross-lingual behavior.
- **Bitemporal node evolution** through supersession rather than destructive overwrite: two-axis time travel (`valid_at` + `tx_as_of`), queryable `recorded_at`, automatic `caused_by` provenance chains, and a per-node tx-time version chain (`node_versions`).

### Boundary & interfaces

- One application-facing database boundary over journal, SQLite projection, and native graph/vector indexes — clients should not dual-write to separate stores.
- Client-defined namespaces, labels, relation types, properties and schema references; provenance, causality and governance metadata stay generic.
- REST, N-API, MCP, Python SDK, Go SDK, and C FFI (Android/React Native) interfaces over one Rust engine.

## Storage Model

- The **framed journal** (`wal/active.gwal` plus sealed history segments) is the internal
  durability authority and mutation source of truth. Every mutation is a sequenced,
  checksummed frame; recovery replays from the durable frontier. Databases written by
  pre-frame versions (`genesis-graph.wal`) migrate transparently on open.
- Snapshot/state files such as `state.json`, `nodes.bin`, `edges.bin`, `vec_<name>.bin`,
  and `fvec_<name>.bin` are materialized on-disk state used for fast reload and recovery —
  they never outrank the journal in the durability contract.
- The **retention profile** decides how much transaction-time history the journal keeps
  (`frontier_only` default / `full` / `budget:<bytes>`); folding history is the only
  operation that destroys it, and queries past the retained horizon fail loudly.
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
