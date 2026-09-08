---
title: "GenesisBlockDB Documentation Hub"
doc_id: "DOCS-NAVIGATION-HUB-GENESISBLOCKDB"
status: draft
version: "0.2.1+draft"
updated: "2026-09-08"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/DOC-REGISTRY.md"
  - "docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md"
  - "docs/FLOW--GENESISRAG17-PIPELINE.md"
  - "docs/GENESISRAG17-EXTENSION-MAP.md"
---

# GenesisBlockDB Documentation Hub

## Start here

1. Business requirements: `docs/BRD--GENESISBLOCKDB.md`
2. Product requirements: `docs/PRD--GENESISBLOCKDB-PLATFORM.md`
3. Software requirements: `docs/SRS--GENESISBLOCKDB.md`
4. Technical architecture: `docs/MASTER-SPEC--GENESIS-DB.md`
5. Architecture map: `docs/C4--GENESISDB-ARCHITECTURE.md`
6. Active registry: `docs/DOC-REGISTRY.md`
7. GenesisRAG17 TEST integration: `docs/FLOW--GENESISRAG17-PIPELINE.md`

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
- Legacy GKS/MSP MCP promotion compatibility path (separate from GenesisRAG17): `docs/ADR--GKS-MSP-PROMOTION-MCP.md`

## GenesisRAG17 TEST integration

GenesisRAG17 is documented here as a separate client integration and TEST
worker boundary. The native engine remains domain-neutral and pinned in the
worker acceptance to
`e15e35b0093394e0a8880af7f4e6f63cf81223b7`. The worker owns physical graph,
embedding, index and publication operations; MSP relays authenticated calls;
GKS remains the passive canonical and quality authority.

- Architecture decision: [ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md](ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md)
- Executable sequence, lifecycle and recovery: [FLOW--GENESISRAG17-PIPELINE.md](FLOW--GENESISRAG17-PIPELINE.md)
- Future-stage extension seams: [GENESISRAG17-EXTENSION-MAP.md](GENESISRAG17-EXTENSION-MAP.md)
- Worker setup and exact model/runtime manifest: [genesisrag17-worker/README.md](../genesisrag17-worker/README.md)
- Product [GenesisRAG17 architecture decision ADR-071](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/decisions/ADR-071-GENESISRAG17-ISOLATED-EXECUTION-AND-PUBLICATION.md)
- Product [17-stage source specification](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-SPEC.md)
- Product [17-stage execution flow](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-FLOW.md)
- [Pinned historical acceptance report](https://github.com/Freshair129/zuri.ai/blob/b64b46df057d3160c659afa3c34628ee86520257/.brain/reports/GENESISRAG17-ACCEPTANCE.md)

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
| 0.2.1+draft | 2026-09-08 | GenesisBlockDB Architecture | Linked the live zuri GenesisRAG17 ADR-071 and retained the pinned historical acceptance report. |
| 0.2.0+draft | 2026-09-08 | GenesisBlockDB Architecture | Added the GenesisRAG17 TEST integration entrypoints, ownership boundary and external source links. |
| 0.1.0+draft | 2026-08-03 | GenesisBlockDB Architecture | Added the standalone-product documentation entrypoint and client-neutral boundary navigation. |
