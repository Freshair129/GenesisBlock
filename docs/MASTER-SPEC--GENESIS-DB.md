---
title: "GenesisBlockDB Technical Architecture and Capability Composition"
doc_id: "MASTER-SPEC-GENESISBLOCKDB"
status: current
version: "2.1.0"
updated: "2026-08-03"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/BRD--GENESISBLOCKDB.md"
  - "docs/PRD--GENESISBLOCKDB-PLATFORM.md"
  - "docs/SRS--GENESISBLOCKDB.md"
  - "docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md"
  - "docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md"
---

# GenesisBlockDB Technical Architecture and Capability Composition

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
- **Relational projection:** embedded SQLite (`rusqlite`, bundled) stores node properties, normalized labels and U2 app-defined tables. Versioned additive schemas, idempotent typed mutation batches and bounded named joins are available through Genesis APIs. SQLite remains internal and rebuildable from the signed WAL. Unified cross-domain commit sequencing remains U3.
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

GenesisBlockDB exposes HQL as a compatibility/query frontend for graph, vector and context operations. The canonical future public contract is typed Query IR. HQL is not storage authority and is not required to grow into general-purpose SQL or Cypher.

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

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 2.1.0 | 2026-08-03 | GenesisBlockDB Architecture | Separated BRD/PRD/SRS roles, established standalone client-neutral boundary, added client namespace/schema metadata, and removed GoVibe-specific authority from the core definition. |
| 2.0.0 | previous | GenesisBlockDB Architecture | Previous master specification. |