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
| **Engine version** | See [canonical version record](docs/VERSION.md) |
| **Milestone** | Mobile SDK — iOS/Android/React Native SDKs shipped; Android and React Native have package-manager distribution, iOS has a published xcframework release artifact |
| **Status** | Advanced prototype — durable and benchmarked; core Rust + Node suites green, GenesisRAG17 model-backed tests require the pinned external fixture |

Version policy and intended SSOT: [docs/VERSION.md](docs/VERSION.md) · Detailed history: [CHANGELOG.md](CHANGELOG.md)

> **Distribution truth rule:** an install method is documented as **Published** only when a clean external consumer can resolve the required artifact without depending on an unmentioned monorepo checkout. Source-only and planned paths are labeled explicitly.

## Installation & Distribution

GenesisBlockDB can be used in three broad modes:

- **Embedded** — the database runs inside your application process, similar to SQLite/Kuzu/DuckDB.
- **Server** — run the Axum REST server and connect from any language over HTTP.
- **Agent/mobile bindings** — use MCP, Android, iOS, or React Native adapters over the same Rust engine.

### Installation matrix

| Surface | Status | Install / consume path | Notes |
|---|---|---|---|
| **Node.js / TypeScript embedded** | ✅ Published | `npm install @freshair129/gks-genesis-block-native` | Primary embedded desktop/server package; Node.js `>=20` |
| **Rust core** | ✅ Source only | clone repo + `cargo build --release` | Root crate intentionally remains `publish = false` until a stable public Rust API exists; see the accepted [crates.io ADR](docs/adr/ADR--GENESISDB-RUST-CRATES-IO-DISTRIBUTION.md) |
| **Standalone REST server** | ✅ Source only | clone repo + Cargo server command | Binary and container release workflows are merged, but no server binary or GHCR image has been published yet |
| **MCP server** | ✅ Source only | clone repo + `npm install` + `npm run mcp:start` | The merged CLI payload change is not in the currently published npm release yet |
| **Python SDK** | ✅ Source only | `python -m pip install ./genesisdb-python` | REST client SDK; wheel/sdist and live-server CI are merged, but no PyPI release is published |
| **Go SDK** | ✅ Source only | use `genesisdb-go/` from the monorepo | Canonical module path resolves at commits in CI; no versioned submodule tag has been published |
| **Android** | ✅ Published | Maven Central: `io.github.freshair129:genesisdb-android:0.1.1` | Preferred Android path; resolves anonymously |
| **Android raw `.aar`** | ✅ Published | GitHub Releases | Manual/fallback integration path |
| **React Native** | ✅ Published | `npm install react-native-genesisdb` | Android uses Maven Central; iOS uses CocoaPods + published xcframework during install |
| **iOS binary** | ✅ Published | `GenesisBlockDB.xcframework.zip` from GitHub Releases | General public SPM package URL is not yet the canonical path |
| **C FFI** | ✅ Source only | build Rust with `ffi` feature + use `include/genesisdb.h` | Suitable for C/C++/Swift/other FFI hosts |
| **Docker / OCI** | 🟡 Pipeline ready; not published | — | Docker build and persistence CI pass, but no server release asset or GHCR package is published |
| **PyPI** | 🟡 Packaging ready; not published | — | Clean wheel and live-server checks pass in CI; no registry release exists yet |
| **crates.io** | 🟡 Not published; decision resolved | — | Root crate remains `publish = false` until a stable public Rust API exists; see the accepted [ADR](docs/adr/ADR--GENESISDB-RUST-CRATES-IO-DISTRIBUTION.md) |
| **Public Go module distribution** | 🟡 Commit resolution verified; versioned release pending | — | External CI resolves a commit, but no `genesisdb-go/v...` version tag exists yet |
| **Homebrew** | 🟡 Planned | — | Gated on stable server binaries appearing in a published release |
| **winget / Scoop** | 🟡 Planned | — | Gated on stable Windows server binaries appearing in a published release |

Distribution completion and acceptance requirements are tracked in **[Issue #166 — Distribution & Installation](https://github.com/Freshair129/GenesisBlock/issues/166)**.

### 1. Node.js / TypeScript — embedded database

This is the simplest supported embedded installation path.

```bash
npm install @freshair129/gks-genesis-block-native
```

Requirements:

- Node.js `>=20`
- Supported native targets declared by the package:
  - Linux x64 GNU
  - Windows x64 MSVC
  - macOS x64
  - macOS arm64 (Apple Silicon)

Example:

```js
import binding from '@freshair129/gks-genesis-block-native'

const { GenesisDatabase } = binding

const db = GenesisDatabase.open({
  path: './agent-memory',
  vectorDim: 1024,
})
```

The Node package is an N-API binding over the Rust engine. It runs **in-process**; you do not need a separate GenesisBlockDB server for this mode.

If a compatible prebuilt native package cannot be resolved, installation may require a Rust toolchain (`cargo`) so the native addon can be built locally. See [QUICKSTART.md](QUICKSTART.md).

### 2. Rust — build the engine from source

The root Rust crate is currently intentionally **not published to crates.io** (`publish = false`). Use a source checkout for Rust development today.

```bash
git clone https://github.com/Freshair129/GenesisBlock.git
cd GenesisBlock
cargo build --release
```

Development build:

```bash
cargo build
```

Run Rust tests:

```bash
cargo test
```

The accepted publication boundary is documented in the [crates.io ADR](docs/adr/ADR--GENESISDB-RUST-CRATES-IO-DISTRIBUTION.md). The root crate remains unpublished until its stable public API gates are met; do not document or rely on `cargo install genesis-block-native` as a supported consumer path.

### 3. Standalone REST server — any language over HTTP

Use this mode when GenesisBlockDB should run as its own process and multiple applications or languages need to connect to it.

```bash
git clone https://github.com/Freshair129/GenesisBlock.git
cd GenesisBlock
cargo run --release --no-default-features --features bins --bin genesis-db-server
```

The server listens on port `3000` by default and exposes routes under `/v1/*`.

Conceptually:

```text
Python ─┐
Go ─────┤
Node ───┤ HTTP /v1/*
Agent ──┤
        ▼
GenesisBlockDB REST server
        │
        ▼
   Rust engine
```

The Dockerfile, standalone server release workflow, and GHCR publishing workflow are implemented. CI validates target builds and the persistence restart smoke, but no server binaries are attached to a published release and no GHCR package is available yet. Continue using the source build above until a release publishes those artifacts.

### 4. MCP server — AI agents

GenesisBlockDB includes an MCP server in this repository.

Current source installation:

```bash
git clone https://github.com/Freshair129/GenesisBlock.git
cd GenesisBlock
npm install
npm run mcp:start
```

Equivalent entrypoint:

```bash
node mcp/server.js
```

The `genesisblock-mcp` executable is declared in the main npm package, and CI verifies the CLI and server files in the packed payload. The currently published npm release predates that change, so registry installation is not available yet; use the repository command above until a package version containing the CLI is published.

The CLI is versioned with the main package; there is no separate MCP npm package. See the [MCP guide](docs/MCP-GUIDE.md) for the registry-based client configuration to use after publication.

### 5. Python SDK — source install, REST client

The Python SDK is currently a client for the standalone REST server. It is **not** the embedded Rust engine.

Install from this repository:

```bash
git clone https://github.com/Freshair129/GenesisBlock.git
cd GenesisBlock
python -m pip install ./genesisdb-python
```

Start GenesisBlockDB separately:

```bash
cargo run --release --no-default-features --features bins --bin genesis-db-server
```

Then connect:

```python
from genesisdb import GenesisClient

client = GenesisClient("http://localhost:3000")
```

See [Python SDK Guide](docs/PYTHON-SDK-GUIDE.md).

The merged packaging workflow builds a wheel and sdist, installs the wheel in a clean environment, and exercises it against a live server. A Trusted Publishing workflow exists, but no `genesisblockdb-client` release is published to PyPI yet. Continue using the source installation above; see the [Python SDK Guide](docs/PYTHON-SDK-GUIDE.md) for release requirements.

### 6. Go SDK — source/monorepo use

The Go REST client currently lives under:

```text
genesisdb-go/
```

For development and tests:

```bash
git clone https://github.com/Freshair129/GenesisBlock.git
cd GenesisBlock/genesisdb-go
go test ./...
```

The canonical module path is:

```text
github.com/Freshair129/GenesisBlock/genesisdb-go
```

A clean external consumer resolves this module from a commit in CI and completes a live-server round trip. No `genesisdb-go/v...` release tag exists yet, so a versioned `go get` path is not published.

For local development from another Go module, use a local replacement deliberately:

```bash
go mod edit -replace github.com/Freshair129/GenesisBlock/genesisdb-go=../GenesisBlock/genesisdb-go
go get github.com/Freshair129/GenesisBlock/genesisdb-go
```

See the [Go SDK specification](docs/SPEC--GO-SDK.md) for the canonical path and version-tag requirements.

### 7. Android — Maven Central (preferred)

Add Maven Central if your project does not already have it:

```kotlin
repositories {
    mavenCentral()
}
```

Then add:

```kotlin
dependencies {
    implementation("io.github.freshair129:genesisdb-android:0.1.1")
}
```

Supported `.aar` ABIs:

- `arm64-v8a` — modern physical devices
- `armeabi-v7a` — older 32-bit physical devices
- `x86_64` — Android Studio emulator

The Maven groupId is `io.github.freshair129`, while the Kotlin/JNI package remains `dev.genesisblock`. These are intentionally different identifiers.

See [android/README.md](android/README.md).

### 8. Android — raw `.aar` from GitHub Releases

A CI-built `.aar` is also attached to GitHub Releases for manual/fallback integration.

Release page:

- [GenesisBlockDB Releases](https://github.com/Freshair129/GenesisBlock/releases)

Prefer Maven Central for normal applications. The raw `.aar` path is useful for offline/manual integration, local repositories, artifact inspection, or environments where Maven resolution is not appropriate.

Legacy GitHub Packages publication may still exist for compatibility, but Maven Central is preferred because public consumers do not need a GitHub PAT to resolve it.

### 9. React Native — npm

Install:

```bash
npm install react-native-genesisdb
```

Current package expectation:

- React Native `>=0.71.0`
- Android resolves GenesisBlockDB from Maven Central.
- iOS uses the package podspec; `pod install` fetches and verifies the published xcframework and compiles the vendored Swift SDK sources.

For iOS React Native applications:

```bash
cd ios
pod install
```

See [react-native-genesisdb/README.md](react-native-genesisdb/README.md) for current native integration details.

### 10. iOS / Swift — published xcframework artifact

A CI-built binary framework is available as:

```text
GenesisBlockDB.xcframework.zip
```

Published release asset:

- [v0.2.0 — GenesisBlockDB.xcframework.zip](https://github.com/Freshair129/GenesisBlock/releases/tag/v0.2.0)

The artifact contains device + simulator slices and is consumed by the repository's external acceptance fixture.

For source development of the Swift SDK itself, build the Rust FFI library first:

```bash
cargo build --no-default-features --features "mobile ffi"
mkdir -p ios/genesisdb/Sources/CGenesisDBFFI/include
cp include/genesisdb.h ios/genesisdb/Sources/CGenesisDBFFI/include/genesisdb.h
cd ios/genesisdb
swift test
```

This requires macOS/Xcode.

A public root-level Swift Package Manager dependency is not provided. The accepted [iOS SPM ADR](docs/adr/ADR--GENESISDB-IOS-SPM-DISTRIBUTION.md) keeps the release xcframework as the canonical binary path until version, checksum, and simulator acceptance gates are satisfied.

See [ios/README.md](ios/README.md).

### 11. C FFI — source build

GenesisBlockDB exposes a C ABI through `src/ffi.rs` and the public header:

```text
include/genesisdb.h
```

Build without Node N-API bindings and enable the FFI surface:

```bash
git clone https://github.com/Freshair129/GenesisBlock.git
cd GenesisBlock
cargo build --release --no-default-features --features ffi
```

Consume the generated native library together with:

```text
include/genesisdb.h
```

This is the underlying path used by the iOS native SDK and can also be integrated from C/C++ or other languages capable of calling a C ABI.

### 12. Docker / GHCR — pipeline ready; not published

The production Dockerfile, server release workflow, GHCR workflow, and persistence restart smoke are implemented and CI-tested. No server binaries are attached to the latest release and no GHCR package is published, so this is not yet a released install path.

The intended GHCR image reference (not yet published) is:

```text
ghcr.io/freshair129/genesisblock:<version>
```

The release workflow publishes server artifacts only on qualifying release tags. PR CI validates builds and persistence behavior but does not publish an image; use the source build above until an actual release contains the server artifact.

### 13. PyPI — package-ready; not published

The Python SDK installs from source today. The merged distribution package is named `genesisblockdb-client`; CI builds a wheel and sdist, tests a clean install, and runs a live-server integration job. No PyPI package release is published yet.

Implemented packaging and validation:

- `pyproject.toml` with distribution name `genesisblockdb-client` and import namespace `genesisdb`
- wheel + sdist builds and clean-environment install tests
- live-server integration tests

Publishing still requires a PyPI Trusted Publisher binding, a `python-v<version>` tag, and a successful upload followed by a clean registry install. No PyPI release is published yet. See the [Python SDK Guide](docs/PYTHON-SDK-GUIDE.md).

### 14. crates.io — decision resolved; unpublished

The root crate currently contains:

```toml
publish = false
```

Therefore there is no supported crates.io install command today.

The accepted [crates.io ADR](docs/adr/ADR--GENESISDB-RUST-CRATES-IO-DISTRIBUTION.md) decides to keep the root crate unpublished until a stable public Rust consumer boundary exists.

### 15. Homebrew / Windows package managers — planned

Homebrew and `winget`/Scoop remain planned. The release workflow exists, but no stable server/CLI binaries are present in a published release; wrappers remain gated on actual checksummed release assets.

Planned order:

1. Homebrew formula/tap for macOS/Linux.
2. Windows `winget` manifest or Scoop bucket.
3. Debian/RPM packages only if operational demand justifies them.

Tracked in [#166](https://github.com/Freshair129/GenesisBlock/issues/166).

## Choosing an installation mode

| You are building... | Recommended path |
|---|---|
| Node.js / TypeScript app with local embedded DB | npm embedded package |
| Rust application / engine development | source + Cargo |
| Multi-language backend or shared DB process | standalone REST server |
| AI agent / MCP client integration | MCP server from repo today |
| Python backend | Python SDK + REST server |
| Go backend | Go SDK + REST server; source/monorepo until public module distribution is fixed |
| Native Android app | Maven Central |
| React Native app | npm `react-native-genesisdb` |
| Native iOS app | published xcframework / source SDK depending on integration needs |
| C/C++ or custom native binding | C FFI build |
| Containerized deployment | wait for official Docker/GHCR distribution or build a private image from source knowingly |

### What it can do today

- **Hybrid queries in one engine**: HNSW vector search, index-backed graph traversal, and SQL-projected property filtering fused in-process — no cross-store glue code.
- **Bitemporal time travel on two axes**: `valid_at` (when a fact was true in the world) and `tx_as_of` (what the database believed at a past commit) over a framed, signed journal; `recorded_at` is queryable and `caused_by` provenance chains automatically on supersede. Correctness is pinned by a dedicated matrix suite (`tests/bitemporal_matrix_wp31_tests.rs`).
- **Retention profiles**: `frontier_only` (default — folds history at every checkpoint, cost-neutral) or `full` (keeps the replayable journal for time travel); the fold is the single history-destruction boundary, and questions beyond the retained horizon fail loudly (`beyond_horizon`) instead of returning silently wrong answers.
- **Measured cross-dimension advantage**: fused vector+graph+AS-OF queries run **115–188× faster** than the equivalent DIY single-file SQLite assembly at 100k nodes × 1024 dims ([moat verdict](docs/REPORT--G3-MOAT-VERDICT.md)) — disclosed honestly: bulk ingest is currently slower than SQLite's, and the corpus is synthetic (real-corpus run scheduled).
- **Vector memory that scales down**: per-collection vector spaces (own model/dim/metric), asynchronous HNSW indexing, F16/SQ8/BQ quantization with an off-RAM rerank sidecar, and WAL compaction/checkpointing.
- **Embedded everywhere**: Node.js N-API addon, standalone Axum REST server, MCP server, Python and Go SDKs — plus a C FFI (`genesisdb_*`) powering iOS and Android native surfaces and a React Native package, all from the same crate.

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

### GenesisRAG17 TEST integration

GenesisRAG17 is a separate client integration around the client-neutral
GenesisBlockDB engine. The [separate-worker/publication ADR](docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md),
[pipeline flow](docs/FLOW--GENESISRAG17-PIPELINE.md) and
[extension map](docs/GENESISRAG17-EXTENSION-MAP.md) document the TEST boundary:
MSP relays authenticated calls, GKS remains the passive semantic and quality
authority, and the worker owns physical Stage 13/15/16 writes plus atomic
publication. The native engine remains pinned to
`e15e35b0093394e0a8880af7f4e6f63cf81223b7`; the worker uses the pinned CPU
`intfloat/multilingual-e5-small` revision
`614241f622f53c4eeff9890bdc4f31cfecc418b3`.

- Worker setup and runtime/model manifest: [genesisrag17-worker/README.md](genesisrag17-worker/README.md)
- Product [GenesisRAG17 architecture decision ADR-073](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/decisions/ADR-073-GENESISRAG17-ISOLATED-EXECUTION-AND-PUBLICATION.md)
- Product [17-stage source specification](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-SPEC.md)
- Product [17-stage execution flow](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-FLOW.md)
- [Pinned historical acceptance report](https://github.com/Freshair129/zuri.ai/blob/b64b46df057d3160c659afa3c34628ee86520257/.brain/reports/GENESISRAG17-ACCEPTANCE.md)

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

### Node.js embedded

```bash
npm install @freshair129/gks-genesis-block-native
```

See [QUICKSTART.md](QUICKSTART.md).

### Rust workspace

```bash
cargo build
```

### Standalone server

```bash
cargo run --release --no-default-features --features bins --bin genesis-db-server
```

### MCP from repo

```bash
npm install
npm run mcp:start
```

### Dashboard

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
- Distribution and installation expansion is tracked in [#166](https://github.com/Freshair129/GenesisBlock/issues/166).
- This repository follows Documentation-Driven Development and Root Cause Analysis.
- Generated artifacts such as dashboard build output and Playwright reports are ignored.
