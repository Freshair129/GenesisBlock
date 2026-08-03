---
title: "Historical Whitepaper: Genesis Knowledge System"
doc_id: "WHITEPAPER-GENESIS-KNOWLEDGE-SYSTEM-HISTORICAL"
status: superseded
version: "2.1.0-superseded"
updated: "2026-08-03"
owner: "GenesisBlockDB Architecture"
source_of_truth: false
superseded_by:
  - "docs/WHITEPAPER--GENESISBLOCKDB-SEMANTIC-SUBSTRATE.md"
  - "docs/PRD--GENESISBLOCKDB-PLATFORM.md"
related_issue: 84
---

# Historical Whitepaper: Genesis Knowledge System

## Supersession notice

This document is retained as historical evidence but is no longer the canonical product definition for GenesisBlockDB.

The previous version used **Genesis Knowledge System (GKS)** as a broad name for the database engine, distributed semantic substrate, governance model, memory engine, and client-facing knowledge system. That wording creates an ownership collision with client architectures such as GoVibe, where GKS is a logical canonical knowledge and relation authority above a swappable persistence backend.

GenesisBlockDB is now defined independently as a standalone, client-neutral database product.

Use these current documents:

- `docs/BRD--GENESISBLOCKDB.md`
- `docs/PRD--GENESISBLOCKDB-PLATFORM.md`
- `docs/SRS--GENESISBLOCKDB.md`
- `docs/MASTER-SPEC--GENESIS-DB.md`
- `docs/WHITEPAPER--GENESISBLOCKDB-SEMANTIC-SUBSTRATE.md`
- `docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md`
- `docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md`

## Current terminology

### GenesisBlockDB

A standalone embedded, local-first hybrid graph and vector database product. It owns generic storage, graph, vector, lexical, temporal, provenance, durability, query, recovery, and integration capabilities.

### Client knowledge systems

A client application may define a logical knowledge system, canonical identity model, authority model, context policy, or workflow above GenesisBlockDB.

Examples:

- GoVibe may use GenesisBlockDB through an adapter while retaining GoVibe-owned GKS/MSP semantics.
- NotiKeeper may use GenesisBlockDB through its own adapter and notification/event schema.
- Future clients may define unrelated ontologies.

## Historical claims

Performance claims from the previous document remain governed by their original benchmark reports and audit evidence. Supersession of terminology does not validate, invalidate, or update benchmark results.

The previous detailed narrative is available through repository history at revisions before this supersession. It should not be copied into new architecture or product documents as current terminology.

## Reason for supersession

- separate the database product from individual client authority models;
- prevent GoVibe-specific semantics from becoming mandatory database ontology;
- allow NotiKeeper and future clients to evolve independently;
- distinguish product requirements, technical architecture, and application semantics;
- remove the implication that GenesisBlockDB and every client knowledge system are the same thing.

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 2.1.0-superseded | 2026-08-03 | GenesisBlockDB Architecture | Superseded the GKS/database product terminology collision and redirected to client-neutral product documents. |
| 2.0.0 | 2026-06-21 | Rwang | Previous Mark VIII evidence-backed whitepaper. |