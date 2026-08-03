---
title: "Software Requirements Specification: GenesisBlockDB"
doc_id: "SRS-GENESISBLOCKDB"
status: draft
version: "0.1.0+draft"
updated: "2026-08-03"
owner: "GenesisBlockDB Engineering"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/BRD--GENESISBLOCKDB.md"
  - "docs/PRD--GENESISBLOCKDB-PLATFORM.md"
  - "docs/MASTER-SPEC--GENESIS-DB.md"
  - "docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md"
---

# SRS: GenesisBlockDB

## 1. Purpose

Define implementation-facing requirements for GenesisBlockDB as a standalone, client-neutral embedded hybrid graph and vector database product.

This SRS intentionally avoids GoVibe-specific or NotiKeeper-specific ontology. Application semantics remain client-owned.

## 2. System boundary

```text
Client application and client-owned schema
  -> SDK / adapter / REST / MCP / embedded API
  -> typed mutation and query contracts
  -> GenesisBlockDB generic core
  -> WAL, snapshot, graph, vector, lexical, temporal and relational projections
```

## 3. Functional requirements

### 3.1 Client independence and namespaces

| ID | Requirement | Priority |
|---|---|---|
| GB-SRS-CLI-001 | GenesisBlockDB SHALL operate without GoVibe, NotiKeeper, or another application framework installed. | MUST |
| GB-SRS-CLI-002 | GenesisBlockDB SHALL support independent client namespaces. | MUST |
| GB-SRS-CLI-003 | A client SHALL be able to provide labels, relation types, properties, and schema references without database-core recompilation. | MUST |
| GB-SRS-CLI-004 | The core SHALL NOT require GoVibe atom types, MSP policy, GKS authority rules, or NotiKeeper notification types. | MUST |
| GB-SRS-CLI-005 | Client-provided canonical identifiers SHALL be preservable as authoritative client IDs. | MUST |
| GB-SRS-CLI-006 | Backend-native internal identifiers SHALL NOT silently replace client-provided IDs in external contracts. | MUST |

### 3.2 Generic graph storage

| ID | Requirement | Priority |
|---|---|---|
| GB-SRS-GRAPH-001 | The system SHALL persist generic nodes with stable IDs, labels, properties, temporal metadata, and optional provenance. | MUST |
| GB-SRS-GRAPH-002 | The system SHALL persist typed directed edges with stable IDs, endpoints, properties, temporal metadata, and optional provenance. | MUST |
| GB-SRS-GRAPH-003 | The system SHALL support forward and reverse adjacency traversal. | MUST |
| GB-SRS-GRAPH-004 | Traversal SHALL support bounded depth and relation filtering. | MUST |
| GB-SRS-GRAPH-005 | The core SHALL NOT require table-per-client-type storage. | MUST |
| GB-SRS-GRAPH-006 | Duplicate mutation handling SHALL be idempotent when a client mutation ID is supplied. | MUST |

### 3.3 Properties and client schema references

| ID | Requirement | Priority |
|---|---|---|
| GB-SRS-SCHEMA-001 | Records SHALL support a client namespace and schema reference. | MUST |
| GB-SRS-SCHEMA-002 | Schema references SHALL be versionable. | MUST |
| GB-SRS-SCHEMA-003 | GenesisBlockDB MAY expose validation hooks but SHALL NOT make one client ontology mandatory for all clients. | MUST |
| GB-SRS-SCHEMA-004 | Schema incompatibility SHALL return an explicit error when validation is requested. | MUST |
| GB-SRS-SCHEMA-005 | Unknown client properties SHALL be preserved according to the declared storage contract. | SHOULD |
| GB-SRS-SCHEMA-006 | Schema metadata SHALL survive backup, restore, snapshot, and export/import paths. | MUST |

### 3.4 Vector and lexical retrieval

| ID | Requirement | Priority |
|---|---|---|
| GB-SRS-VEC-001 | The system SHALL support named vector collections with declared model, dimension, metric, and index configuration. | MUST |
| GB-SRS-VEC-002 | Vector insertion SHALL validate collection compatibility. | MUST |
| GB-SRS-VEC-003 | Asynchronous index maintenance SHALL expose completion or flush semantics. | MUST |
| GB-SRS-VEC-004 | The system SHALL support top-k vector retrieval for supported collections. | MUST |
| GB-SRS-VEC-005 | Lexical retrieval SHALL preserve documented language behavior. | SHOULD |
| GB-SRS-VEC-006 | Hybrid retrieval SHALL expose ranking parameters and capability support explicitly. | SHOULD |
| GB-SRS-VEC-007 | Embedding data SHALL NOT be used as the only authoritative client identity. | MUST |

### 3.5 Temporal, provenance and causality

| ID | Requirement | Priority |
|---|---|---|
| GB-SRS-TEMP-001 | Records SHALL support valid-time metadata. | MUST |
| GB-SRS-TEMP-002 | The system SHALL preserve mutation/event ordering metadata sufficient for documented replay behavior. | MUST |
| GB-SRS-TEMP-003 | Supersession SHALL preserve prior record history according to the temporal model. | MUST |
| GB-SRS-TEMP-004 | The system SHALL support `as-of` retrieval for implemented temporal query surfaces. | MUST |
| GB-SRS-TEMP-005 | Generic provenance and causality references SHALL be storable without imposing client semantics. | MUST |
| GB-SRS-TEMP-006 | Provenance SHALL be queryable by generic APIs where the capability is declared. | SHOULD |

### 3.6 Durability, atomicity and recovery

| ID | Requirement | Priority |
|---|---|---|
| GB-SRS-DUR-001 | Mutations SHALL be recorded through the documented WAL path before durability is acknowledged. | MUST |
| GB-SRS-DUR-002 | Batch mutation atomicity SHALL match the published capability contract. | MUST |
| GB-SRS-DUR-003 | Snapshot and replay SHALL preserve externally visible record identity and supported metadata. | MUST |
| GB-SRS-DUR-004 | Backup and restore SHALL produce an integrity report. | MUST |
| GB-SRS-DUR-005 | Recovery SHALL distinguish successful replay, partial recovery, corruption, and unsupported-version failure. | MUST |
| GB-SRS-DUR-006 | A failed atomic commit SHALL NOT be reported as durable success. | MUST |

### 3.7 Query contracts

| ID | Requirement | Priority |
|---|---|---|
| GB-SRS-QRY-001 | The long-term public query boundary SHALL be representable as typed Query IR. | SHOULD |
| GB-SRS-QRY-002 | HQL MAY provide a compatibility frontend but SHALL NOT define storage authority. | MUST |
| GB-SRS-QRY-003 | Queries SHALL expose namespace and revision/temporal scope where supported. | MUST |
| GB-SRS-QRY-004 | Unsupported query capabilities SHALL fail explicitly. | MUST |
| GB-SRS-QRY-005 | Query results SHALL preserve client IDs and declared schema metadata. | MUST |
| GB-SRS-QRY-006 | Bounded joins or relational projections SHALL remain inside the unified database boundary when exposed. | SHOULD |

### 3.8 Interfaces and SDKs

| ID | Requirement | Priority |
|---|---|---|
| GB-SRS-API-001 | Embedded, server, and SDK surfaces SHALL report product and capability versions. | MUST |
| GB-SRS-API-002 | Shared operations SHALL have conformance tests across supported interfaces. | MUST |
| GB-SRS-API-003 | Interface adapters SHALL NOT redefine client ontology. | MUST |
| GB-SRS-API-004 | API errors SHALL distinguish validation, capability, query, conflict, durability, and integrity failures. | MUST |
| GB-SRS-API-005 | MCP tools, when enabled, SHALL expose bounded database operations and SHALL NOT assume GoVibe authority semantics. | MUST |

### 3.9 Operational visibility

| ID | Requirement | Priority |
|---|---|---|
| GB-SRS-OPS-001 | The system SHALL expose health and version status. | MUST |
| GB-SRS-OPS-002 | The system SHALL expose relevant WAL, snapshot, index, queue, and recovery state for supported modes. | SHOULD |
| GB-SRS-OPS-003 | The optional dashboard SHALL remain a client of the runtime rather than a required core dependency. | MUST |
| GB-SRS-OPS-004 | Implemented, partial, proposed, and superseded capability status SHALL be distinguishable in release documentation. | MUST |

## 4. Non-functional requirements

| ID | Requirement | Target or rule |
|---|---|---|
| GB-SRS-NFR-001 | Data integrity | No acknowledged durable write may disappear after supported recovery without an explicit integrity failure |
| GB-SRS-NFR-002 | Client neutrality | GoVibe and NotiKeeper conformance fixtures run on one unmodified core |
| GB-SRS-NFR-003 | Determinism | Repeated deterministic queries at the same snapshot/revision produce equivalent results |
| GB-SRS-NFR-004 | Performance evidence | Claims include workload, dataset, hardware, configuration and raw evidence |
| GB-SRS-NFR-005 | Compatibility | Schema/storage/API changes include version and migration impact |
| GB-SRS-NFR-006 | Portability | Client-owned semantic meaning does not depend on internal numeric keys |
| GB-SRS-NFR-007 | Local-first operation | Core functionality does not require mandatory external cloud service |
| GB-SRS-NFR-008 | Security boundary | Namespace and authorization behavior are explicit per interface/deployment mode |

## 5. Required product contracts

- client namespace and schema contract;
- generic node and edge mutation schema;
- typed Query IR;
- capability manifest;
- WAL and durability acknowledgment contract;
- snapshot/backup/restore contract;
- version and migration policy;
- SDK/interface conformance matrix;
- error taxonomy.

## 6. Verification

Requirements SHALL be verified through:

- Rust unit and integration tests;
- WAL, snapshot, recovery and corruption tests;
- graph traversal and mutation tests;
- vector collection compatibility tests;
- temporal/as-of tests;
- SDK and REST conformance tests;
- namespace and schema independence fixtures;
- GoVibe and NotiKeeper client-adapter tests that do not modify the database core;
- reproducible benchmark harnesses.

## 7. Out of scope

- defining application-specific canonical knowledge authority;
- defining GoVibe Canonical Semantic IR;
- defining NotiKeeper notification behavior;
- multi-node HA or distributed SQL unless separately specified and implemented;
- silent acceptance of incompatible client schema mutations.

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 0.1.0+draft | 2026-08-03 | GenesisBlockDB Engineering | Initial standalone and client-neutral SRS. |