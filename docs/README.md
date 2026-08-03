---
title: "GenesisBlockDB Documentation Hub"
doc_id: "DOCS-NAVIGATION-HUB-GENESISBLOCKDB"
status: draft
version: "0.1.0+draft"
updated: "2026-08-03"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/DOC-REGISTRY.md"
---

# GenesisBlockDB Documentation Hub

## Start here

1. Business requirements: `docs/BRD--GENESISBLOCKDB.md`
2. Product requirements: `docs/PRD--GENESISBLOCKDB-PLATFORM.md`
3. Software requirements: `docs/SRS--GENESISBLOCKDB.md`
4. Technical architecture: `docs/MASTER-SPEC--GENESIS-DB.md`
5. Architecture map: `docs/C4--GENESISDB-ARCHITECTURE.md`
6. Active registry: `docs/DOC-REGISTRY.md`

## Product boundary

GenesisBlockDB is a standalone embedded, local-first hybrid graph and vector database product.

```text
GoVibe domain       NotiKeeper domain       Future client domain
      |                     |                         |
      +-------- client adapters / SDK contracts -----+
                            |
                GenesisBlockDB generic core
```

GoVibe and NotiKeeper are independent clients. They retain ownership of their own schemas, ontology, authority, workflow, and projections.

## Current boundary documents

- Domain-neutral core ADR: `docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md`
- Client namespace/schema contract: `docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md`
- Client-neutral whitepaper: `docs/WHITEPAPER--GENESISBLOCKDB-SEMANTIC-SUBSTRATE.md`
- Historical GKS terminology document: `docs/WHITEPAPER--GENESIS-KNOWLEDGE-SYSTEM.md`
- Historical implementation status snapshot: `docs/DOC-STATUS.md`

## Evidence and references

- Performance report: `docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md`
- API reference: `docs/API_REFERENCE.md`
- Product version: `docs/VERSION.md`
- Positioning: `docs/POSITIONING.md`
- Interactive benchmark dashboard: `docs/perf-comparison-dashboard.html`

## Documentation rules

- BRD defines business need and product independence.
- PRD defines product users, scope, goals, non-goals, and capabilities.
- SRS defines implementation-facing SHALL requirements.
- Master Spec defines technical architecture composition.
- ADRs define significant decisions.
- Contracts define external boundaries.
- Code, tests, benchmarks, audits, and reports provide implementation evidence.
- Client schemas must not become mandatory database-core ontology.
- Historical documents must remain visibly superseded and must not compete with current product definitions.

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 0.1.0+draft | 2026-08-03 | GenesisBlockDB Architecture | Added the standalone-product documentation entrypoint and client-neutral boundary navigation. |