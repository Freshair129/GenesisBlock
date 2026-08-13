---
title: "GenesisBlockDB Document Registry"
doc_id: "DOC-REGISTRY-GENESISBLOCKDB"
status: draft
version: "0.2.0+draft"
updated: "2026-08-13"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/README.md"
  - "docs/DOC-STATUS.md"
---

# GenesisBlockDB Document Registry

## 1. Purpose

This registry identifies the active product, requirements, architecture, contract, evidence, and historical entrypoints for GenesisBlockDB.

It does not replace code and test evidence. Product documents define intent and requirements; implementation status must remain traceable to code, tests, benchmarks, audits, and release evidence.

## 2. Registry rules

- One canonical product definition per document role.
- Product requirements must remain independent of GoVibe, NotiKeeper, or another single client.
- Client-specific schemas live in client repositories or adapters.
- `source_of_truth: false` and superseded documents do not compete with active canonical documents.
- Performance claims must point to reproducible evidence.
- A path in this registry must exist at the registered revision.

The registry is an index of canonical entrypoints, not an exhaustive catalog
of every historical, exploratory, benchmark, interview, or ADR document under
`docs/`. Unregistered documents remain reviewable references, but they do not
claim product-definition or implementation-status authority unless a current
registry row explicitly names them.

## 3. Product and requirements

| Role | Doc ID | Version | Status | Owner | Path |
|---|---|---|---|---|---|
| BRD | `BRD-GENESISBLOCKDB` | `0.1.0+draft` | draft | Freshair129 / Product Authority | `docs/BRD--GENESISBLOCKDB.md` |
| PRD | `PRD-GENESISBLOCKDB-PLATFORM` | `0.1.0+draft` | draft | Freshair129 / Product Authority | `docs/PRD--GENESISBLOCKDB-PLATFORM.md` |
| SRS | `SRS-GENESISBLOCKDB` | `0.1.0+draft` | draft | GenesisBlockDB Engineering | `docs/SRS--GENESISBLOCKDB.md` |

## 4. Architecture and contracts

| Role | Doc ID | Version | Status | Owner | Path |
|---|---|---|---|---|---|
| Architecture composition | `MASTER-SPEC-GENESISBLOCKDB` | `2.1.0` | current | GenesisBlockDB Architecture | `docs/MASTER-SPEC--GENESIS-DB.md` |
| Architecture index | `C4--GENESISDB-ARCHITECTURE` | `0.1.8b` | current | GenesisBlockDB Architecture | `docs/C4--GENESISDB-ARCHITECTURE.md` |
| ADR | `ADR-GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE` | `0.1.0+draft` | proposed | GenesisBlockDB Architecture | `docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md` |
| Client contract | `CONTRACT-CLIENT-NAMESPACE-AND-SCHEMA` | `0.1.0+draft` | draft | GenesisBlockDB Engineering | `docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md` |
| API reference | `API_REFERENCE` | generated | current | GenesisBlockDB Engineering | `docs/API_REFERENCE.md` |

## 5. Product narrative and evidence

| Role | Doc ID | Version | Status | Owner | Path |
|---|---|---|---|---|---|
| Positioning | `POSITIONING-GENESISBLOCKDB` | n/a | current | GenesisBlockDB Product | `docs/POSITIONING.md` |
| Whitepaper | `WHITEPAPER-GENESISBLOCKDB-SEMANTIC-SUBSTRATE` | `0.1.0+draft` | draft | GenesisBlockDB Architecture | `docs/WHITEPAPER--GENESISBLOCKDB-SEMANTIC-SUBSTRATE.md` |
| Database whitepaper | `WHITEPAPER--GENESIS-DB` | n/a | current | GenesisBlockDB Architecture | `docs/WHITEPAPER--GENESIS-DB.md` |
| Performance report | `REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE` | n/a | historical | GenesisBlockDB Engineering | `docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md` |
| Product version | `VERSION` | current | current | GenesisBlockDB Engineering | `docs/VERSION.md` |

## 6. Historical and superseded documents

| Role | Doc ID | Version | Status | Replaced by | Path |
|---|---|---|---|---|---|
| Historical terminology whitepaper | `WHITEPAPER-GENESIS-KNOWLEDGE-SYSTEM-HISTORICAL` | `2.1.0-superseded` | superseded | `WHITEPAPER-GENESISBLOCKDB-SEMANTIC-SUBSTRATE` | `docs/WHITEPAPER--GENESIS-KNOWLEDGE-SYSTEM.md` |
| Historical implementation status snapshot | `DOC-STATUS-GENESISBLOCKDB-HISTORICAL` | `2026.06.21+archived` | superseded | `DOC-REGISTRY-GENESISBLOCKDB` plus current code/test evidence | `docs/DOC-STATUS.md` |

## 7. Client boundary

```text
GoVibe domain       NotiKeeper domain       Future client domain
      |                     |                         |
      +-------- client adapters / SDK contracts -----+
                            |
                GenesisBlockDB generic core
```

The registry records GenesisBlockDB product documents only. GoVibe and NotiKeeper application schemas are external client contracts and must not be registered as native GenesisBlockDB ontology.

## 8. Known follow-up documents

The following documents should be created only when implementation work requires them:

- typed Query IR contract;
- generic node/edge mutation contract;
- WAL durability acknowledgment contract;
- backup/restore and migration contract;
- SDK/interface conformance matrix;
- NotiKeeper adapter conformance report;
- third-client namespace conformance report.

## 9. Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 0.2.0+draft | 2026-08-13 | GenesisBlockDB Architecture | Reconciled registered frontmatter and defined canonical-entrypoint scope for automated validation. |
| 0.1.0+draft | 2026-08-03 | GenesisBlockDB Architecture | Established the active standalone-product registry and separated historical status snapshots. |
