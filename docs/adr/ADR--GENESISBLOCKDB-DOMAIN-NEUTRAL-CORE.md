---
title: "GenesisBlockDB Domain-Neutral Core"
doc_id: "ADR-GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE"
status: proposed
version: "0.1.0+draft"
updated: "2026-08-03"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/BRD--GENESISBLOCKDB.md"
  - "docs/PRD--GENESISBLOCKDB-PLATFORM.md"
  - "docs/SRS--GENESISBLOCKDB.md"
  - "docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md"
---

# ADR: GenesisBlockDB Domain-Neutral Core

## Status

Proposed.

## Context

GenesisBlockDB is used by more than one product. GoVibe uses it as a storage and query backend for GoVibe-owned semantic records. NotiKeeper uses it for a different application domain. Future clients may bring additional schemas and lifecycle rules.

Embedding GoVibe-specific atom types, GKS authority, MSP context rules, planning semantics, or NotiKeeper notification semantics into the database core would make the product a client-specific framework rather than a reusable database.

## Decision

GenesisBlockDB core SHALL remain domain neutral.

The core owns generic storage and execution concepts:

- nodes and edges;
- labels, relation type strings, and properties;
- client IDs and internal keys;
- client namespaces and schema references;
- vector collections and embeddings;
- lexical indexes;
- temporal versions and event order;
- generic provenance and causality references;
- WAL, snapshots, backup, restore, replay, and recovery;
- typed mutation/query contracts and capability reporting.

Clients own domain meaning:

- ontology and taxonomy;
- canonical identity rules;
- authority and promotion policy;
- planning or notification behavior;
- business validation;
- user workflows and projections.

## Architectural boundary

```text
GoVibe domain       NotiKeeper domain       Future client domain
      |                     |                         |
      +-------- client adapters / SDK contracts -----+
                            |
                GenesisBlockDB generic core
```

## Mandatory rules

1. The core SHALL compile and operate without GoVibe or NotiKeeper packages.
2. New client schemas SHALL not require a database-core fork.
3. Internal storage keys SHALL not redefine client-owned identity.
4. Client relation labels SHALL be indexable without the core owning their semantics.
5. Client-specific validation SHALL occur in the client, adapter, or optional plugin/hook boundary.
6. Generic governance-supporting metadata MAY exist, but one client's authority policy SHALL not be mandatory for all clients.
7. Documentation and public APIs SHALL distinguish generic product contracts from example integrations.

## Consequences

### Positive

- GenesisBlockDB remains a standalone product.
- GoVibe and NotiKeeper evolve independently.
- New clients can adopt the database without ontology migration.
- Product requirements remain valid if any one client is removed.

### Negative

- Client adapters must perform semantic validation and mapping.
- The generic property model needs schema references to avoid becoming an undocumented blob store.
- Cross-client semantic interoperability is not automatic.
- Optional client conveniences must not leak into the core contract.

## Rejected alternatives

### Make GoVibe Canonical Semantic IR native database schema

Rejected because it privileges one client and creates product lock-in.

### Create a separate database-core fork for each client

Rejected because it fragments durability, query, SDK, migration, and benchmark behavior.

### Store arbitrary JSON without namespace or schema metadata

Rejected because it preserves technical flexibility while destroying reviewable compatibility.

## Conformance evidence

The decision is satisfied when:

- GoVibe and NotiKeeper fixtures run against one unmodified core;
- a third namespace/schema is accepted without recompilation;
- database tests do not import GoVibe or NotiKeeper domain packages;
- internal keys can change without changing client IDs;
- schema validation modes are explicit;
- README, PRD, SRS, and Master Spec use the same boundary.

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 0.1.0+draft | 2026-08-03 | GenesisBlockDB Architecture | Established the domain-neutral core and client-owned ontology boundary. |