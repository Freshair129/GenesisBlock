---
title: "Historical GenesisBlockDB Documentation Status Snapshot"
doc_id: "DOC-STATUS-GENESISBLOCKDB-HISTORICAL"
status: superseded
version: "2026.06.21+archived"
updated: "2026-08-03"
owner: "GenesisBlockDB Architecture"
source_of_truth: false
superseded_by: "DOC-REGISTRY.md"
related_issue: 84
---

# Historical GenesisBlockDB Documentation Status Snapshot

## Supersession notice

This file preserves the best-effort implementation-status snapshot recorded on 2026-06-21. It is not the active document registry and must not be used as the current product-definition source of truth.

Use:

- `docs/DOC-REGISTRY.md` for active document ownership and lifecycle;
- `docs/BRD--GENESISBLOCKDB.md` for business requirements;
- `docs/PRD--GENESISBLOCKDB-PLATFORM.md` for product requirements;
- `docs/SRS--GENESISBLOCKDB.md` for software requirements;
- current code, tests, audits, benchmarks, and release evidence for implementation status.

The historical classifications below are retained for traceability. They may be stale after 2026-06-21.

## Historical status legend

- **Implemented** — present in `src/lib.rs` and exercised by tests/benchmarks at the snapshot date.
- **Partial** — core present, with documented stubs/gaps at the snapshot date.
- **Proposed** — design/spec not fully built at the snapshot date.
- **Superseded** — historical and replaced by newer evidence/decisions.
- **Reference** — process/record documents rather than feature specifications.

> Caveat: most `SPEC--*` and `TDD--*` files ended with review/approval language and were authored as proposals. The historical classification reflected observed code evidence rather than the document's own claim.

## Implemented at snapshot date

| Doc | Evidence recorded on 2026-06-21 |
|---|---|
| SPEC--AXIOMATIC-GUARDS, SPEC--MARK-IV-MASTER | `validate_governance`, `Tier` (P24) |
| SPEC--BATCH-ATOMICITY | `execute_batch` (`batch_atomicity_tests`) |
| SPEC--CAUSALITY-CHAINS | `supersede_node`, `caused_by` |
| SPEC--TEMPORAL-QUERIES | HQL `AS OF`, `is_valid_as_of` (`temporal_queries_tests`) |
| SPEC--KIMPACT-AND-INFERENCE | `compute_impact`, incremental refresh, `INFER` (P25) |
| SPEC--GRAPH-RETRIEVAL-LAYER, SPEC--LOGIC-GATED-CONTEXT | `retrieve_context`, `get_ranked_context` (`grl_retrieval_tests`) |
| SPEC--GRAPH-CLUSTERING, SPEC--HIERARCHICAL-REASONING, SPEC--STATE-TRANSITION-REASONING | `detect_communities`, `generate_meta_graph`, `meta_history` |
| SPEC--INDEX-COMPACTION | `compact` / `perform_index_compaction` (`compaction_tests`) |
| SPEC--CROSS-LINGUAL-MAPPING | `set_language_centroid`, centroid mean-centering (`thai_fuzzy_tests`) |
| SPEC--CRYPTOGRAPHIC-SWARM | ed25519 identity, signed events |
| DESIGN--HNSW-HYBRID-INDEX | `hnsw_rs` per-collection index, async indexing (P14–P21) |
| SPEC--MULTI-COLLECTION-VECTOR-SPACE | per-collection vector spaces P-C/P-D (`multi_collection_tests`) |
| ADR--GENESISDB-EDGE-ID-INTERNING, ADR--GENESISDB-EDGE-NUMERIC-KEYS | lean numeric edge keys and recorded RAM reduction |
| ADR--GENESISDB-ASYNC-INDEXING | deferred HNSW indexing and recorded latency change |
| DESIGN--TRANSITIVE-INFERENCE | `INFER(...)` unbounded traversal |
| TDD--GENESISDB-DUAL-TRACK | WAL plus binary snapshot, load and replay |
| TDD--NEURAL-CONSENSUS-PROTOCOL | proposal/vote quorum paths |
| TDD--STRUCTURAL-INSIGHT-ENGINE | structural-gap calculation |
| SPEC--MCP-SERVER | `mcp/server.js`; historical note recorded tool-list drift |

## Partial at snapshot date

| Doc | Gap recorded on 2026-06-21 |
|---|---|
| SPEC--COLLABORATIVE-SYNC, SPEC--P2P-GOSSIP-PROTOCOL | discovery and PushDelta work; anti-entropy PullRequest stub |
| SPEC--PYTHON-SDK, SPEC--GO-SDK plus guides | SDK/server request-shape mismatch |
| SPEC--SELF-OPTIMIZING-SUBSTRATE | autonomic loop present; per-cluster `ef` tuning not wired |
| ADR--PHASE-13-QUERY-PLANNER | `LogicalPlanner` existed but was not connected to `execute_hql` |
| SPEC--OBSIDIAN-UI-INTEGRATION | plugin present; maintenance/503 contract not implemented |

## Proposed at snapshot date

The snapshot recorded no active proposed item after multi-collection vector support shipped. It listed HQL collection scoping and same-node multi-vector follow-ups as deferred.

## Superseded at snapshot date

- `EXPANSION-SPEC--GENESIS-DB` was explicitly deprecated.
- Audits P7–P13 were superseded by the later measured audit/report program.
- Earlier competitive-roadmap, scalability-validation, and benchmark-suite ADRs were superseded by measured evidence and refined positioning.

## Historical reference set

The snapshot treated whitepapers, Master Spec, C4, API reference, version record, performance report, incidents, change requests, guides, flows, frameworks, TDD governance documents, audits, and architecture ADRs as reference/process records rather than feature-status proof by themselves.

## Historical dangling references

The snapshot recorded these references for later governance repair:

- root `hql.pest`;
- `src/query/planner.rs`;
- `src/sync/mod.rs`;
- `AGENTS.md`;
- `docs/specs/SPEC-Genesis-Block.md`;
- `docs/GO-SDK-GUIDE.md`.

Their current status must be verified against the present branch rather than inferred from this historical file.

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 2026.06.21+archived | 2026-08-03 | GenesisBlockDB Architecture | Converted the 2026-06-21 implementation-status SSOT claim into an explicitly superseded historical snapshot and redirected active ownership to DOC-REGISTRY. |
