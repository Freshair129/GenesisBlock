---
title: "GenesisBlockDB Business Requirements Document"
doc_id: "BRD-GENESISBLOCKDB"
status: draft
version: "0.1.0+draft"
updated: "2026-08-03"
owner: "Freshair129 / Product Authority"
source_of_truth: true
related_issue: 84
---

# GenesisBlockDB Business Requirements Document

## 1. Product statement

GenesisBlockDB is a standalone embedded, local-first hybrid graph and vector database product for applications that need durable semantic, relational, temporal, provenance-aware, and retrieval-oriented data inside one operational boundary.

It is not a GoVibe component with one captive client. GoVibe is one client, NotiKeeper is another, and future clients may use GenesisBlockDB with independent schemas, ontologies, policies, and lifecycle rules.

## 2. Business problem

Applications that combine graph relationships, vector retrieval, temporal history, provenance, and local-first operation commonly assemble multiple databases or services. That introduces:

- operational complexity;
- cross-system consistency gaps;
- synchronization and deployment overhead;
- latency between application and remote services;
- duplicated identity and provenance models;
- vendor and network dependence;
- difficult offline or edge deployment.

GenesisBlockDB addresses this by providing one embedded product boundary for graph, vector, lexical, temporal, governance-supporting, and event-sourced capabilities.

## 3. Target users and clients

GenesisBlockDB targets product teams and developers building:

- local-first AI and agent applications;
- knowledge and memory systems;
- notification and event intelligence applications such as NotiKeeper;
- planning and governance platforms such as GoVibe;
- embedded analytics and relationship-heavy applications;
- edge, desktop, private, or self-hosted systems requiring low-latency local data access.

Client category is defined by technical need, not by adopting one ontology.

## 4. Business goals

1. Provide an independent database product usable without GoVibe.
2. Reduce the need to combine separate graph, vector, temporal, and event stores for embedded workloads.
3. Preserve local-first and in-process deployment as a first-class product advantage.
4. Support client-defined namespaces and schemas without database-core forks.
5. Offer measurable performance, durability, retrieval, and recovery behavior.
6. Support multiple language bindings and integration surfaces.
7. Permit each client to retain authority over its own semantic model.

## 5. Business requirements

| ID | Business requirement | Priority |
|---|---|---|
| GB-BR-001 | GenesisBlockDB SHALL be marketed and documented as a standalone product. | MUST |
| GB-BR-002 | The product SHALL support multiple independent clients and domains. | MUST |
| GB-BR-003 | Clients SHALL be able to define namespaces, labels, relation types, properties, and schema references without core recompilation. | MUST |
| GB-BR-004 | The core SHALL remain neutral to GoVibe, NotiKeeper, or any other client ontology. | MUST |
| GB-BR-005 | The product SHALL provide one operational boundary for graph, vector, lexical, temporal, provenance, and durability capabilities included in the supported edition. | MUST |
| GB-BR-006 | Performance and reliability claims SHALL be evidence-backed and reproducible. | MUST |
| GB-BR-007 | The product SHALL support embedded/in-process use as a primary deployment mode. | MUST |
| GB-BR-008 | Server, SDK, and MCP surfaces MAY expose the same core capabilities without redefining client semantics. | SHOULD |
| GB-BR-009 | Client applications SHALL be able to change their schemas independently within declared compatibility rules. | MUST |
| GB-BR-010 | GenesisBlockDB SHALL not require a client to adopt another client's governance or authority model. | MUST |
| GB-BR-011 | The product SHALL support data portability, backup, restore, and version compatibility appropriate to the supported release. | MUST |
| GB-BR-012 | The product SHOULD support offline, private, and self-hosted adoption without mandatory external cloud services. | SHOULD |

## 6. Product differentiation

GenesisBlockDB differentiates through the combination of:

- embedded and local-first operation;
- index-backed graph traversal;
- vector and lexical retrieval;
- temporal/event-sourced history;
- generic provenance and causality support;
- one Rust core exposed through multiple interfaces;
- client-neutral extensibility.

The differentiation is not ownership of GoVibe or NotiKeeper semantics.

## 7. Product boundaries

### In scope

- generic node and edge storage;
- typed client-defined relations and properties;
- vector collections and hybrid retrieval;
- temporal versions and event history;
- provenance fields and query support;
- WAL, snapshot, backup, restore, and recovery;
- APIs, SDKs, query contracts, and optional MCP surface;
- client namespaces and schema references.

### Out of scope

- deciding canonical semantic identity for every client;
- defining GoVibe atom taxonomy or MSP policy;
- defining NotiKeeper notification ontology;
- acting as a project-management product;
- silently promoting client data according to one application's governance rules;
- requiring one client framework for database use.

## 8. Success metrics

- number and diversity of independent client integrations;
- ability to pass client-neutral conformance tests;
- no database-core fork required for GoVibe and NotiKeeper schemas;
- measured graph/vector/ingest/recovery performance;
- durability and restoration success rates;
- compatibility across supported SDKs and interfaces;
- documented schema evolution without client lock-in.

## 9. Risks

| Risk | Mitigation |
|---|---|
| Product becomes coupled to GoVibe terminology | client-neutral core ADR, namespace contract, product PRD/SRS reviews |
| Generic model becomes an unvalidated property blob | optional client schema references and validation hooks |
| Too many capabilities increase complexity | explicit capability matrix and release scope |
| Benchmark narrative outruns evidence | reproducible reports and retraction policy |
| Client schema evolution breaks data | versioned schema references, migrations, compatibility tests |
| Embedded and server modes diverge | one core with conformance tests across interfaces |

## 10. Commercial note

Pricing, packaging, hosted offerings, enterprise support, and service-level commitments remain separate decisions. This BRD establishes product independence and business requirements without inventing commercial evidence that does not yet exist.

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 0.1.0+draft | 2026-08-03 | Product Authority | Initial standalone and client-neutral business requirements. |