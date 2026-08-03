---
title: "GenesisBlockDB Product Requirements Document"
doc_id: "PRD-GENESISBLOCKDB-PLATFORM"
status: draft
version: "0.1.0+draft"
updated: "2026-08-03"
owner: "Freshair129 / Product Authority"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/BRD--GENESISBLOCKDB.md"
  - "docs/SRS--GENESISBLOCKDB.md"
  - "docs/MASTER-SPEC--GENESIS-DB.md"
---

# PRD: GenesisBlockDB Platform

## 1. Product vision

GenesisBlockDB is an embedded, local-first hybrid graph and vector database product for applications that require low-latency relational traversal, semantic retrieval, durable local storage, temporal history, provenance, and event-oriented evolution in one database boundary.

GenesisBlockDB is an independent product. GoVibe, NotiKeeper, and future applications are clients. Each client owns its own ontology, schema semantics, authority model, workflow, and user-facing behavior.

## 2. Product principles

1. **Client neutral:** the database core does not prescribe GoVibe, NotiKeeper, or another client ontology.
2. **Embedded first:** in-process use is a primary product category, not a reduced server mode.
3. **One operational boundary:** graph, vector, lexical, temporal, provenance, and durability capabilities are managed as one database product.
4. **Evidence backed:** performance and reliability claims require reproducible tests.
5. **Schema extensible:** clients can declare namespaces and schema references without recompiling the core.
6. **History preserving:** mutation, supersession, provenance, and temporal access are explicit.
7. **Interface consistent:** Rust, Node, REST, MCP, Python, and Go surfaces should converge on typed product contracts.

## 3. Target users

- developers building local-first AI and agent systems;
- desktop, edge, private, and self-hosted application teams;
- knowledge, notification, planning, analytics, and memory products;
- teams needing graph and vector capabilities without operating separate remote services;
- integrators requiring a generic semantic and temporal data substrate.

## 4. Representative clients

### 4.1 GoVibe

Uses GenesisBlockDB through an adapter to persist and query GoVibe-owned canonical semantic records. GoVibe retains authority over its atom taxonomy, planning model, GKS rules, MSP policy, and projection contracts.

### 4.2 NotiKeeper

Uses GenesisBlockDB for NotiKeeper-owned notification, event, relation, history, and retrieval models. NotiKeeper does not need to adopt GoVibe schemas.

### 4.3 Future clients

May define independent namespaces, labels, edge types, schema references, vector collections, and application policies.

These examples are integrations, not mandatory product modules.

## 5. Goals

- provide durable generic nodes, edges, properties, vectors, temporal versions, and provenance;
- provide low-latency index-backed graph traversal;
- provide vector, lexical, and hybrid retrieval;
- support client namespaces and versioned schema references;
- support embedded and self-hosted deployment modes from one Rust core;
- expose typed and testable query/mutation contracts;
- preserve data through WAL, snapshot, backup, restore, and recovery paths;
- support application-specific adapters without client ontology in the core.

## 6. Non-goals

- owning GoVibe Canonical Semantic IR;
- owning NotiKeeper domain meaning;
- acting as an autonomous knowledge authority for all clients;
- requiring one governance tier model for all applications;
- replacing every relational, distributed, or cloud database category;
- claiming multi-node HA or distributed SQL unless implemented and evidenced;
- turning HQL into a mandatory general-purpose language when typed Query IR is sufficient.

## 7. Core capabilities

### 7.1 Generic graph records

- client-defined node labels and properties;
- client-defined typed, directed relations;
- stable IDs and optional client-provided IDs;
- forward and reverse adjacency traversal;
- bounded and filtered graph queries.

### 7.2 Vector and lexical retrieval

- named vector collections with model, dimension, metric, and index configuration;
- asynchronous index maintenance with explicit flush semantics;
- lexical and language-aware retrieval;
- hybrid graph/vector/lexical ranking.

### 7.3 Temporal and event history

- valid-time and transaction/event-time metadata;
- supersession without destructive history loss;
- causality/provenance references;
- revision or snapshot-consistent reads where supported.

### 7.4 Durability and recovery

- WAL-backed mutations;
- snapshots and replay;
- backup and restore;
- atomic or explicitly scoped mutation batches;
- integrity checks and recovery reporting.

### 7.5 Client schema and namespace support

- `client_namespace` or equivalent tenant/domain boundary;
- `schema_ref` and schema version metadata;
- optional validation hooks or adapter validation;
- no mandatory GoVibe or NotiKeeper ontology in the core;
- compatibility and migration metadata.

### 7.6 Query and integration surfaces

- typed Query IR as the long-term public contract;
- HQL as a compatibility/query frontend;
- embedded Rust and Node bindings;
- REST server mode;
- optional MCP tools;
- Python and Go SDKs.

### 7.7 Operational visibility

- status and health endpoints;
- index and queue state;
- WAL/snapshot/recovery visibility;
- capability and version reporting;
- optional dashboard as a client, not the core runtime.

## 8. Product requirements

| ID | Requirement | Priority |
|---|---|---|
| GB-PR-001 | GenesisBlockDB SHALL function without GoVibe installed. | MUST |
| GB-PR-002 | GenesisBlockDB SHALL support multiple independent client namespaces. | MUST |
| GB-PR-003 | Clients SHALL define labels, relation types, properties, and schema references without core recompilation. | MUST |
| GB-PR-004 | GoVibe and NotiKeeper schemas SHALL be independent client contracts. | MUST |
| GB-PR-005 | The database SHALL expose generic graph, vector, lexical, temporal, provenance, and durability capabilities. | MUST |
| GB-PR-006 | The database SHALL expose a capability/version manifest. | MUST |
| GB-PR-007 | Embedded operation SHALL remain a primary supported mode. | MUST |
| GB-PR-008 | Server and SDK surfaces SHALL preserve core contract semantics. | SHOULD |
| GB-PR-009 | Client-provided canonical IDs SHALL be preservable without replacement by backend-native IDs. | MUST |
| GB-PR-010 | Schema and data evolution SHALL be versioned and auditable. | MUST |
| GB-PR-011 | Unsupported features SHALL fail explicitly rather than silently degrading semantic guarantees. | MUST |
| GB-PR-012 | Product claims SHALL distinguish implemented, partial, proposed, and superseded capabilities. | MUST |

## 9. Success criteria

- GoVibe and NotiKeeper can both use one unmodified database core;
- a third client can define a new namespace and schema without importing either existing client ontology;
- supported interfaces pass shared conformance tests;
- measured graph, vector, ingest, and recovery targets are reported with reproducible evidence;
- client IDs, provenance, and temporal metadata survive storage/query round-trips;
- documentation clearly separates product requirements from architecture internals and client semantics.

## 10. Product architecture boundary

```text
Client application
  -> client-owned domain and schema
  -> client adapter / SDK
  -> GenesisBlockDB typed API or Query IR
  -> generic storage, graph, vector, temporal, provenance, and durability core
```

The database may provide generic governance-supporting primitives. A client decides how those primitives map to its own authority or promotion model.

## 11. Release documentation requirements

Each release SHALL publish or link:

- product version;
- capability matrix;
- schema/storage migration notes;
- API/SDK compatibility;
- implemented/partial/proposed status;
- benchmark environment and evidence;
- known limitations and retractions.

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 0.1.0+draft | 2026-08-03 | Product Authority | Initial standalone, client-neutral GenesisBlockDB PRD. |