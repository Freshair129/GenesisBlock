---
title: "GenesisBlockDB Technical Architecture and Capability Composition"
doc_id: "MASTER-SPEC-GENESISBLOCKDB"
status: current
version: "2.3.33b"
updated: "2026-10-03"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/BRD--GENESISBLOCKDB.md"
  - "docs/PRD--GENESISBLOCKDB-PLATFORM.md"
  - "docs/SRS--GENESISBLOCKDB.md"
  - "docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md"
  - "docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md"
  - "docs/SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md"
  - "docs/adr/ADR--GENESISDB-HQL2-PATTERN-CONSTRAINTS.md"
  - "docs/adr/ADR--GENESISDB-HQL2-P8-COMPLETION-ADDENDUM.md"
  - "docs/adr/ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY.md"
  - "docs/SPEC--GENESISDB-TYPED-QUERY-IR-V1.md"
  - "docs/SPEC--WAVE-A-COMMIT-CORRECTNESS.md"
  - "docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md"
  - "docs/FLOW--GENESISRAG17-PIPELINE.md"
  - "docs/GENESISRAG17-EXTENSION-MAP.md"
---

# GenesisBlockDB Technical Architecture and Capability Composition

The owner-approved [HQL2 execution boundary](adr/ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY.md)
adds a staged, explicitly selected v2 query pipeline. Existing HQL, Query IR v1,
storage and transport contracts remain the current compatibility surfaces.
P7 reference-interpreter evidence does not imply P8 production execution.
The [P8 core checkpoint](REPORT--HQL2-P8-CORE-2026-09-28.md) records the
separately approved Rust pipeline, revision-bound scans, annotation lookup,
constrained graph expansion, exact KNN/Original Rerank, lease-bound
HistoryScan/ChangeScan, root Match anchors, and structural unsigned parameters.
Approved addendum D1-D5 now has local implementation evidence: pre-parse P6
ACL denial, registered exact-scan lexical and scalar-token ContextPack profiles,
Sequence node/edge exact-JSON properties under the lease, and contextual
NULL/list/JSON literals. The focused completion target passes 10/10, including
an edge-property-before-SHORTEST regression. The approved D7 bridge accepts
differential-tested actor-scoped zero-hop and bounded one-hop HQL1 ID
projections through `Storage::query_v2`, including one endpoint-ID exact string
equality filter; parser resources are preflighted and reserved before legacy
AST construction. Its nine focused tests cover legacy and HQL2 differential
behavior, pre-parse authorization/namespace checks, malformed and unlisted
forms, and resource rejection. HQL `JOIN TABLE` now lowers to the existing
closed RowScan/Join pipeline for Inner/Left/Semi/Anti, with bare JOIN defaulting
to Inner; HQL, typed IR and independent P7 differential results match for
duplicate, missing-property NULL and JSON-null cases. The latest root-HQL2
regression sweep passes 382/0/1 across 33 targets; the separate 11-target
P6/schema-v6/compatibility group passes 194/0/0. Storage-backed HQL/typed-IR
`Values`/`UnionAll` results match independent P7 for 169 nullable bag pairs
and 338 Storage executions, preserving NULL and duplicate multiplicity.
HistoryScan bags for Node, Edge, Row, Vector and Annotation match independent
P7 catalogs assembled from WAL-derived revision facts and captured frontiers/
valid-time windows, including endpoint, vector-owner and annotation-reference
ACL dependencies. Artifact HistoryScan remains unsupported; broad ChangeScan,
P8/P13, hosted CI and review remain open. The
H2-D11 R4/P6 annotation grant regression confirms:
Annotation subjects require explicit Annotation(Read) separately from the
Namespace(Read) query grant, while same-namespace target/evidence references
remain covered by the broad namespace grant. HQL2 and typed-IR
`tx_as_of` now select one historical frontier across source scans, graph/vector/
annotation operators, hydration and `Snapshot.tx`, with no current-state
fallback and the P6 generation/current policy still pinned. Five focused targets
pass 56/56. Independent review
of D7 remains pending; earlier read-only review found no concrete static defect,
and focused re-review confirmed the corrected D5
JSON-literal size preflight. Compact constraints remain fail-closed. Broad
exact-oracle, shared HQL1/HQL2/IR, resource/cancellation, P8 and P13 gates
remain open; these local results do not close the full HQL2/P8 gate. A separate
selected HQL2/durability/authority verification now passes 40 targets,
including schema-v6 WAL-only recovery; this is fixture evidence, not release or
consumer qualification.
The final locked/offline no-default-features Rust suite passed with no failures;
the explicitly selected `probe_vs_recall` test remains NOT_RUN. Strict all-target
Clippy passed with default and no-default features. These local results do not
close independent review, full HQL2/P8/P13, release or consumer gates.

## 1. Role of this document

This document is the authoritative technical architecture composition for GenesisBlockDB. It explains how implemented and declared product capabilities fit together.

It is not the Business Requirements Document, Product Requirements Document, or Software Requirements Specification. Those are maintained separately:

```text
BRD--GENESISBLOCKDB
  -> PRD--GENESISBLOCKDB-PLATFORM
  -> SRS--GENESISBLOCKDB
  -> MASTER-SPEC / C4 / ADR / feature specs
  -> code, tests and benchmarks
```

## 2. Abstract

GenesisBlockDB is a high-performance, embedded, local-first hybrid graph and vector database engine written in Rust. It provides a unified substrate for structured graph relationships, vector embeddings, lexical retrieval, temporal/event history, generic provenance, and durability.

GenesisBlockDB is a standalone product with multiple independent clients. GoVibe, NotiKeeper, and future applications own their own ontology, authority, workflow, and projection semantics. The database core stores and executes generic client-defined records through namespaces, schema references, typed graph records, vector collections, temporal metadata, and query contracts.

The architecture SHALL remain valid if GoVibe or NotiKeeper is removed from the ecosystem.

## 3. Product-neutral architecture boundary

```text
Client application
  -> client-owned domain/schema/authority
  -> client adapter or SDK
  -> GenesisBlockDB typed API / Query IR
  -> generic graph, vector, lexical, temporal, provenance and durability core
```

### 3.1 Core ownership

GenesisBlockDB owns:

- generic nodes, edges, labels, properties and client identifiers;
- client namespaces and schema references;
- vector collections, embeddings and retrieval indexes;
- lexical indexes;
- temporal versions, event order, supersession and generic causality references;
- WAL, snapshots, backup, restore, replay and recovery;
- query, mutation, capability and SDK contracts;
- optional generic governance-supporting and consensus primitives.

Clients own:

- atom or record taxonomy;
- relation business meaning;
- canonical identity policy;
- authority, promotion and context rules;
- planning, notification or other workflows;
- application validation and user-facing views.

GoVibe-specific GKS/MSP/planning contracts and NotiKeeper-specific notification contracts SHALL NOT become mandatory database-core ontology.

## 4. Core Architecture

### 4.1 Storage Model

GenesisBlockDB uses a **Log-Structured Merge-Friendly** architecture based on a Write-Ahead Log (WAL).

- **Primary Log:** `genesis-graph.wal` (JSONL format) stores mutation events.
- **Persistence:** high-durability append-only logic with batched group commits.
- **Unified operational boundary:** applications open, mutate, query, back up and restore GenesisBlockDB as one database. SQLite is an internal relational projection; native graph/vector indexes are not separate application-managed databases.
- **Relational projection:** embedded SQLite (`rusqlite`, bundled) stores node properties, normalized labels and U2 app-defined tables. Versioned additive schemas, idempotent typed mutation batches and bounded named joins are available through Genesis APIs. SQLite remains internal and rebuildable from the signed WAL. Unified cross-domain transactions preflight relational constraints before WAL append. Supported query reads and graph/projection publication share a reentrant commit boundary; failed durable apply requires reopen before further queries, writes or checkpoint. Concurrent reads serialize; ANN visibility still requires the existing flush barrier. See [Wave A contract and verification limits](SPEC--WAVE-A-COMMIT-CORRECTNESS.md).
- **In-memory state:**
  - `DashMap<u32, NodeOutput>`: lean primary node records; `props` are hydrated from SQLite rather than retained on the traversal path.
  - `DashMap<u128, EdgeOutput>`: primary edge storage. Edges are keyed by deterministic `u128 = trunc128(SHA256(id))`; the key is derived from `EdgeOutput.id` and is not client identity.
  - `Adjacency Indices`: forward (`out_idx`) and backward (`in_idx`) indexes for O(1)-class adjacency access.

Internal numeric or hashed keys are implementation details. Public contracts preserve client-provided IDs.

### 4.2 Client namespace and schema metadata

Generic client records may carry:

```yaml
client_namespace: string
schema_ref: string
schema_version: string
client_record_id: string
client_mutation_id: string | null
```

The database preserves and indexes this metadata according to the client namespace/schema contract. Validation may be performed by the client, adapter, or optional hook. The core does not hard-code one client ontology.

### 4.3 Semantic Hybrid Indexing

GenesisBlockDB bridges lexical and semantic search through:

1. **Lexical Index:** Thai-aware trigram/bigram behavior that strips combining marks for high-recall fuzzy matching.
2. **Vector Index:** named `VectorCollection`s, each with its own model, dimension, metric, arena and HNSW index. A `default` collection exists. HNSW insertion is asynchronous; `flush_index` forces a drain.
3. **Neural Bridge:** multilingual support using language centroids and mean-centering.

Embeddings and similarity are retrieval data. They do not define client canonical identity by themselves.

### 4.4 Graph Retrieval Layer

The Graph Retrieval Layer (GRL) provides generic tiered or bounded graph retrieval:

- a resolver maps configured tiers or scopes to graph expansion;
- a budget manager estimates result size/token cost and may compress results;
- an orchestrator combines vector anchors with bounded graph expansion.

A client may map these primitives to its own context policy. The GRL does not make GoVibe MSP rules mandatory for NotiKeeper or other clients.

### 4.5 GenesisRAG17 TEST integration boundary

The isolated GenesisRAG17 integration is a client adapter around this public
engine boundary. The separate `genesisrag17-worker/` process owns physical
Stage 13 graph writes, Stage 15 CPU embeddings, Stage 16 index/readback and
atomic publication. It uses one native store owner and a worker-owned SQLite
FTS5 lexical sidecar; the sidecar is an adapter implementation, not a new
engine authority. MSP authenticates and relays pipeline calls, while GKS owns
canonical decisions, Stage 14 enrichment and the Stage 17 quality verdict.

The physical sequence is graph-only write and receipt -> GKS enrichment ->
embedding -> six-lane index/readback -> quality gate -> atomic pointer update ->
publication receipt. A prepared snapshot is not query-visible, and a gate
without a matching publication receipt cannot finish a source run. This TEST
adapter does not add GKS ontology, MSP policy, source schemas or a second
database to GenesisBlockDB. Its native engine pin is
`e15e35b0093394e0a8880af7f4e6f63cf81223b7`; its model and contract pins,
runtime variables, lane statuses and extension seams are recorded in the
[GenesisRAG17 ADR](ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md),
[pipeline flow](FLOW--GENESISRAG17-PIPELINE.md), [extension
map](GENESISRAG17-EXTENSION-MAP.md) and [worker README](../genesisrag17-worker/README.md).

## 5. Data Model and Bitemporality

### 5.1 Generic node schema

| Field | Type | Description |
|---|---|---|
| `id` | String | Stable external/client-facing identifier. |
| `labels` | Vec<String> | Client-defined classification labels. |
| `props` | JSON | Generic client properties. |
| `impact` | f64 | Optional derived importance score. |
| `embedding` | Vec<f64> | Optional vector data; collection metadata applies. |
| `valid_from` | RFC3339 | Logical start time. |
| `valid_to` | Option<RFC3339> | Logical end time. |
| `expires_at` | Option<RFC3339> | Optional TTL expiration. |
| `caused_by` | Option<String> | Generic causality/provenance reference. |
| `clock` | LogicalClock | Lamport timestamp where CRDT behavior applies. |
| `client_namespace` | String/optional by deployment contract | Client/domain namespace. |
| `schema_ref` | String/optional by validation mode | Client-controlled schema reference. |
| `schema_version` | String/optional | Schema version metadata. |

### 5.2 Generic edge schema

A generic edge preserves:

- stable external edge ID;
- source and target external IDs;
- client-defined relation type;
- namespace and schema metadata;
- generic properties;
- provenance/causality metadata;
- temporal validity;
- internal numeric/hash key as a private optimization only.

### 5.3 Bitemporal philosophy

GenesisBlockDB follows an immutable-by-default update pattern. Updates use supersession:

1. mark the existing version with `valid_to`;
2. insert a new version with `valid_from` and updated properties;
3. link the mutation to generic causality/provenance metadata where supplied.

Clients decide whether a supersession is a semantic correction, business update, notification state change, or another domain event.

## 6. Reasoning and Autonomic Substrate

### 6.1 K-Impact Model

Node importance may be calculated through the documented K-Impact formula and inputs such as dependency depth, configured strictness/governance metadata and source stability.

The core may compute generic scores. Clients decide how or whether those scores affect authority or workflows.

### 6.2 Structural Insight Engine

The maintenance loop may perform:

- community detection;
- supernode or cluster-summary generation;
- structural gap detection;
- vector/centroid drift tracking.

Outputs are analytical candidates or derived data. They do not automatically become client canonical truth.

## 7. Governance-Supporting and Consensus Primitives

GenesisBlockDB may expose generic tiers, guards, signatures, proposals and votes. These are database/runtime primitives, not a universal application authority model.

A client may map them to:

- GoVibe canonical promotion;
- NotiKeeper notification approval;
- a future client's independent governance policy.

The core SHALL not require one mapping for all clients.

## 8. Distributed Synchronization

Where enabled, synchronization uses documented logical-clock, reconciliation and CRDT behavior.

- Lamport timestamps provide deterministic event ordering inputs.
- LWW or other configured reconciliation behavior must be documented as a storage conflict rule, not a substitute for client semantic conflict policy.
- Local clock advancement follows the documented protocol.

Client-level semantic conflicts may require review even when storage-level reconciliation succeeds.

## 9. HQL and Typed Query IR

The accepted public-query architecture is a versioned typed Query IR. Its
partial executor/API slice implements `search`, `traverse`, linear `match_path`
and target-id `context` in core, N-API and REST. Query-vector/temporal context
and relational named-query remain unsupported or planned; see the capability
manifest and `SPEC--GENESISDB-TYPED-QUERY-IR-V1`.

GenesisBlockDB currently exposes HQL as a compatibility frontend for graph, vector and context
operations. HQL is not storage authority and is not required to grow into general-purpose SQL or
Cypher. Free-form NL interpretation belongs in a separate client-owned Agent Query Adapter, which
must produce validated Query IR before invoking the engine.

```text
NL intent -> external agent adapter -> typed Query IR -> GenesisBlockDB typed executor
HQL text -> compatibility parser --------------------^
```

### 9.1 Search

```sql
SEARCH ~target SIMILAR TO [v1, v2, ...] K 5 IN "code" LANGUAGE "th" AS OF "2026-01-01T00:00:00Z"
```

The optional `IN <collection>` clause scopes search to a named vector collection; omitted means `default`.

### 9.2 Traverse

```sql
TRAVERSE FROM seed DEPTH 2 REL INFER(depends_on) AS OF "..."
```

Relation labels are client-defined data. Query execution does not grant them universal business meaning.

### 9.3 Hybrid

```sql
MATCH target SIMILAR TO [...] ALPHA 0.4 LANGUAGE "en"
```

Query contracts should support namespace, collection, temporal/revision and bounded traversal scope where implemented.

## 10. Deployment and Connectivity

### 10.1 Deployment modes

The same Rust core supports in-process embedding and a single-node self-hosted Axum server. Multi-node HA, distributed SQL and automatic failover are not v1 claims unless separately implemented and evidenced.

### 10.2 Model Context Protocol

GenesisBlockDB may provide a native MCP server for bounded database operations.

Current tools include or may expose:

- HQL/query execution;
- bounded/tiered context retrieval;
- generic knowledge/record insertion with provenance.

MCP tools SHALL not assume GoVibe canonical authority or NotiKeeper workflow semantics.

### 10.3 Python SDK

The Python SDK provides typed generic node, edge, query and retrieval bindings. Client-specific schema wrappers belong in client packages or adapters.

### 10.4 Go SDK

The Go SDK provides concurrent-safe generic database access. References to “full mapping of GKS schemas” should be interpreted or revised as optional client adapters, not the product-neutral core contract.

## 11. Conformance and evidence

The architecture is conformant when:

- GoVibe and NotiKeeper adapters run against one unmodified core;
- a third client namespace/schema can be added without recompilation;
- public IDs survive WAL/snapshot/restore/query paths;
- internal key changes do not alter client identity;
- client-specific validation is outside mandatory core ontology;
- implemented/partial/proposed status remains evidence-backed;
- benchmark claims include workload and environment.

## 12. Document responsibility

- BRD defines business need and product independence.
- PRD defines user-facing product scope and major capabilities.
- SRS defines SHALL requirements.
- This Master Spec defines technical architecture composition.
- ADRs define significant decisions.
- Feature specs and code/tests define implementation detail and evidence.

Wave B (R-02/R-03, approved 2026-09-08) journals immutable collection definitions
including empty/default spaces and preserves calibration and exact rerank rows
through fold/recovery. Disk schema is 4; derived SQLite projection schema is 5.
Edge replacement rewires both adjacency directions and `edge_versions` selects
replica-local transaction intervals. History is available only from the reported
edge-history floor. New-reader preflight rejects unsupported complete frames;
old-engine use of a manually stripped journal-only v4 copy is unsupported.
See [Wave B contract and verification record](SPEC--WAVE-B-DURABLE-COLLECTIONS-EDGE-HISTORY.md).
These are local implementation contracts, not deployment or consumer migration evidence.

P6 (architecture correction approved 2026-09-22) is the approved target for signed generation
receipts after index flush and fail-closed snapshot validation, opaque generation-bound read
leases, explicit temporal selectors, and signed revision-checked access-policy events. Its target
disk schema is now 6 (the prior documented Wave B/P6 baselines are v4/v5); readers that do not
understand schema 6 must reject it. Fixture-backed v5-to-v6 migration and markerless WAL recovery
pass their focused tests, but no user database migration or consumer compatibility qualification
has been performed. Current graph/vector records still lack entity namespace fields, so the
P6 ACL contract does not claim migrated tenant isolation. See [P6 specification](SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md).

Schema-v6 selection remains authority-bearing when the snapshot marker is lost. A missing
`state.json` requires journal preflight before projection replay: schema 6 is selected only by a
valid signed local activation record and, for migrated stores, its complete migration and P6
receipt proof. Schema-5-compatible WAL remains replayable as v5; v6-only events without that
proof fail with `RECOVERY_REQUIRED` instead of silently downgrading. See H2-D11 ADR R6b and the
P6 WAL-only recovery contract. Runtime implementation is fixture-verified; no
user database was migrated, and broader P6/P8/P13 qualification remains open.

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 2.3.33b | 2026-10-03 | GenesisBlockDB Architecture | Record test-only HQL/typed-IR Values/UnionAll P7 differential for 169 nullable bag pairs and 338 Storage executions; HQL2 382/0/1 across 33 targets, P6/compatibility 194/0/0; no runtime/schema/transport change; retain broad exact-oracle, P8/P13 and qualification gates |
| 2.3.32b | 2026-10-03 | GenesisBlockDB Architecture | Record storage-backed P7 HistoryScan differentials for Node/Edge/Row/Vector/Annotation; History/Change 15/15, Annotation source 7/7, P7 130/130, HQL2 379/0/1 and P6/schema-v6/compatibility 194/0/0; Artifact, broad ChangeScan/P8/P13, hosted CI and review remain open |
| 2.3.31b | 2026-10-03 | GenesisBlockDB Architecture | Record independent P7 differential for H2-D11 Vector HistoryScan from WAL-derived revisions; P7 130/130, History/Change 14/14, HQL2 377/0/1 and P6/schema-v6/compatibility 194/0/0. At prior PR #194 head 9ada855, Windows cargo plus four worker checks fail and reviews are absent; hosted verification of this source revision and full acceptance/merge remain open |
| 2.3.30b | 2026-10-03 | GenesisBlockDB Architecture | Record H2-D11 R4/P6 annotation ACL conformance and local HQL2 375/0/1 plus selected P6/schema-v6 43/0/0; possible ChangeScan budget side channel, prior PR worker failures and cancelled Windows Rust, hosted checks/review and full acceptance/merge remain open |
| 2.3.29b | 2026-10-03 | GenesisBlockDB Architecture | Record PR #194 run 37083654705 at docs-only head 43cc6e8: Windows HQL/IR Join budget failure and four worker bootstrap failures; local Join reproduction 5/5, full acceptance and merge remain open |
| 2.3.28b | 2026-10-03 | GenesisBlockDB Architecture | Synchronize the explicit HQL2 regression sweep to 374/0/1 across 32 targets and current PR #194 CI state: core checks pass, four worker checks fail at fresh schema-v6 bootstrap; preserve worker-fix approval, review, broad P8/P13 and release gates. |
| 2.3.27b | 2026-10-03 | GenesisBlockDB Architecture | Implement HQL2/IR `tx_as_of` across sources, graph/vector/annotation operators, hydration and `Snapshot.tx` using one no-fallback frontier; record focused 56/56, HQL2 373/0/1 and P6/compatibility 194/0/0; retain broad P8/P13, review, worker-CI, soak, security and release gates. |
| 2.3.26b | 2026-10-03 | GenesisBlockDB Architecture | Record HQL JOIN lowering through approved RowScan/Join contract, independent HQL/IR/P7 parity and 365/0/1 across 31 HQL2 targets; retain broad P8/P13, review, worker-CI, soak, security and release gates. |
| 2.3.25b | 2026-10-02 | GenesisBlockDB Architecture | Record full local Rust regression and strict Clippy pass for HQL2/H2-D11 integration; keep `probe_vs_recall` NOT_RUN, no user migration, and independent-review/P8/P13/release gates open. |
| 2.3.24b | 2026-10-02 | GenesisBlockDB Architecture | Record fixture-verified H2-D11 R6b markerless schema-v6 recovery and upstream Query IR V1 1.0.3 linear match_path; 40 selected HQL2/durability/authority targets pass; no user DB migration, full P8/P13 gates open. |
| 2.3.23b | 2026-10-02 | GenesisBlockDB Architecture | Synchronize owner-approved H2-D11 R6b/P6 schema-v6 WAL-only recovery authority; implementation and recovery tests pending. |
| 2.3.22b | 2026-10-02 | GenesisBlockDB Architecture | Add D7's differential-proven one-hop endpoint-ID exact string filter; record 9/9 focused adapter tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 compatibility targets; retain shared-runtime, independent-review and broad P8/P13 gates. |
| 2.3.21b | 2026-10-02 | GenesisBlockDB Architecture | Extend D7 with differential-proven one-hop HQL1 projections and pre-parse resource reservation; record 8/8 focused tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 compatibility targets; retain shared-runtime, independent-review and broad P8/P13 gates. |
| 2.3.20b | 2026-10-02 | GenesisBlockDB Architecture | Record D7's initial actor-scoped HQL1 projection, 5/5 focused adapter tests and 354/0/1 across 27 HQL2 targets; retain shared-runtime, independent-review and broad P8/P13 gates. |
| 2.3.19b | 2026-10-02 | GenesisBlockDB Architecture | Record D4 edge-property-before-SHORTEST regression, 10/10 P8 completion tests and 349/0/1 across 26 HQL2 targets; independent static review has no concrete finding; retain broad P8/P13 gates. |
| 2.3.18b | 2026-10-02 | GenesisBlockDB Architecture | Truth-sync approved P8 D1-D5 runtime and 528/0/1 across 37 explicit regression targets; retain JSON-size correction review and broad P8/P13 gates. |
| 2.3.17b | 2026-10-02 | GenesisBlockDB Architecture | Register owner-approved P8 addendum D1-D6 for bounded text operators, Sequence property constraints, contextual literals and P6 ACL-fixture correction; runtime evidence pending. |
| 2.3.16b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync lease-bound HQL/typed-IR Sequence node ID/labels and 338/0/1 across 25 HQL2 targets; retain properties/Compact, broader P8 and release gates. |
| 2.3.15b | 2026-09-30 | GenesisBlockDB Architecture | Record delegated HQL/typed-IR Sequence node-ID/conjunctive-label contract under P6; runtime implementation pending, properties/Compact fail-closed. |
| 2.3.14b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync HQL root-Match P7 graph differential and 331/0/1 across 24 root HQL2 targets; retain broad HQL2/P8 and release gates. |
| 2.3.13b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync HQL/IR exact-KNN and Original Rerank differentials against independent P7 ranking and 330/0/1 across 23 root HQL2 targets; retain broad HQL2/P8 and release gates. |
| 2.3.12b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync HQL/IR exact-KNN differential against the independent P7 oracle and 329/0/1 across 23 root HQL2 targets; retain broad HQL2/P8 and release gates. |
| 2.3.11b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync checked HQL `%`/typed-IR `rem`, same-type I64/F64 semantics and 328/0/1 across 22 local targets; retain remaining HQL2/P8 and release gates. |
| 2.3.10b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync exact DecimalU64 structural HQL parameters and HQL2 NULLS LAST default; record 325/0/1 across 22 local targets; retain pattern-constraint, remaining HQL2/P8 and release gates. |
| 2.3.9b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync exact DecimalU64 structural HQL parameter binding and 323/0/1 across 22 local HQL2 targets; retain pattern-constraint, remaining HQL2/P8 and release gates. |
| 2.3.8b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync validated typed-IR root Match anchors and 319/0/1 across 22 local HQL2 targets; retain pattern-constraint, remaining HQL2/P8 and release gates. |
| 2.3.7b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync lease-bound HQL/IR HistoryScan/ChangeScan and 318/0/1 local HQL2 evidence; retain Lexical/Context, constrained-pattern, transport and P8 gates. |
| 2.3.6b | 2026-09-29 | GenesisBlockDB Architecture | Truth-sync exact HQL/IR KNN and Original Rerank, H2-D11 collection-space binding and 307/0/1 local HQL2 evidence; retain explicit history, transport and P8 gates. |
| 2.3.5b | 2026-09-29 | GenesisBlockDB Architecture | Truth-sync structural HQL/IR Sequence Expand and the 206-pass/21-target fixture matrix; keep pattern constraints, full HQL2 and release gates open. |
| 2.3.4b | 2026-09-29 | GenesisBlockDB Architecture | Truth-sync the partial P8 checkpoint for constrained HQL/IR graph expansion and the 199-pass/20-target fixture matrix; keep full HQL2 and release gates open. |
| 2.3.2b | 2026-09-28 | GenesisBlockDB Architecture | Register accepted explicit HQL2 execution boundary; P7 fixtures do not imply production completion. |
| 2.3.3b | 2026-09-28 | GenesisBlockDB Architecture | Register the approved P8 scalar/core checkpoint and remaining storage/surface gates. |
| 2.3.1b | 2026-09-22 | GenesisBlockDB Architecture | Registered the owner-approved P6 durability, generation/lease, temporal and ACL target with explicit schema-compatibility and implementation-status limits. |
| 2.3.0b | 2026-09-08 | GenesisBlockDB Architecture | Added the separate GenesisRAG17 TEST adapter boundary, ordered publication flow and extension references while retaining the client-neutral core. |
| 2.2.2 | 2026-09-08 | GenesisBlockDB Architecture | Reflected approved Wave B durable collections, schema compatibility and edge version intervals. |
| 2.2.1 | 2026-09-08 | GenesisBlockDB Architecture | Reflected approved Wave A preflight, publication and recovery-required behavior with verification limits. |
| 2.2.0 | 2026-08-14 | GenesisBlockDB Architecture | Approved typed Query IR as the primary query boundary, retained HQL compatibility, and placed NL conversion outside the engine. |
| 2.1.0 | 2026-08-03 | GenesisBlockDB Architecture | Separated BRD/PRD/SRS roles, established standalone client-neutral boundary, added client namespace/schema metadata, and removed GoVibe-specific authority from the core definition. |
| 2.0.0 | previous | GenesisBlockDB Architecture | Previous master specification. |
