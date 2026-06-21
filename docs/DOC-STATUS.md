# Documentation Status Index (SSOT)

Central status manifest (best-effort from code evidence, 2026-06-21) — a single
index instead of editing 40+ spec frontmatters. Status legend:

- **Implemented** — present in `src/lib.rs` and exercised by tests/benchmarks.
- **Partial** — core present, with documented stubs/gaps.
- **Proposed** — design/spec not (fully) built.
- **Superseded** — historical; replaced by newer evidence/decisions.
- **Reference** — process/record docs (audits, ADRs, guides, reports).

> Caveat: most `SPEC--*`/`TDD--*` files end with "review and approve" and were
> authored as proposals; the classification below reflects what the **code**
> actually does, not the doc's own claim.

## Implemented (verified)
| Doc | Evidence |
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
| DESIGN--HNSW-HYBRID-INDEX | `hnsw_rs` arena index (P14–P21) |
| DESIGN--TRANSITIVE-INFERENCE | `INFER(...)` unbounded traversal |
| TDD--GENESISDB-DUAL-TRACK | WAL + binary snapshot, instant-load + replay (P14) |
| TDD--NEURAL-CONSENSUS-PROTOCOL | `propose_consensus`/`submit_vote` quorum (`crdt_sync_tests`) |
| TDD--STRUCTURAL-INSIGHT-ENGINE | `calculate_structural_gaps` |
| SPEC--MCP-SERVER | `mcp/server.js` (3 tools; the doc's 4-tool list is drift) |

## Partial (core present, gaps)
| Doc | Gap |
|---|---|
| SPEC--COLLABORATIVE-SYNC, SPEC--P2P-GOSSIP-PROTOCOL | gossip discovery + PushDelta work; **anti-entropy PullRequest is a stub** |
| SPEC--PYTHON-SDK, SPEC--GO-SDK + guides | SDKs exist but send `{"query":…}` to `/v1/query/hql` (server wants a raw string) |
| SPEC--SELF-OPTIMIZING-SUBSTRATE | autonomic loop runs; per-cluster `ef` tuning not wired (ef is now global-configurable via `set_index_params`) |
| ADR--PHASE-13-QUERY-PLANNER | `LogicalPlanner` exists but is **dead code** (`execute_hql` matches directly) |
| SPEC--OBSIDIAN-UI-INTEGRATION | plugin present; `503`/maintenance contract not implemented |

## Proposed (not built)
- SPEC--MULTI-COLLECTION-VECTOR-SPACE (P-B embedding dedup landed; per-collection P-C/P-D open)

## Superseded
- **EXPANSION-SPEC--GENESIS-DB** — explicitly DEPRECATED.
- **AUDIT--P7…P12**, AUDIT--P13 — pre-correction numbers (P12 retracted artifacts);
  superseded by **AUDIT--P14–P25** + `REPORT--2026-06-21`.
- **ADR--GENESISDB-COMPETITIVE-ROADMAP, ADR--GENESISDB-SCALABILITY-VALIDATION,
  ADR--GENESISDB-BENCHMARK-SUITE** — pre-pivot/aspirational; superseded by the
  measured P14–P25 program and `ADR--GENESISDB-MARKET-POSITIONING` (refined).

## Reference / process (not feature specs)
WHITEPAPER--GENESIS-DB, WHITEPAPER--GENESIS-KNOWLEDGE-SYSTEM, MASTER-SPEC--GENESIS-DB,
C4--GENESISDB-ARCHITECTURE, API_REFERENCE, VERSION, REPORT--2026-06-21,
INCIDENT--*, CR--*, MCP-GUIDE, PYTHON-SDK-GUIDE, FLOW--*, FRAMEWORK--*,
TDD--DOCUMENTATION-GOVERNANCE-SSOT-ENFORCEMENT, AUDIT--P14…P25,
ADR--GENESISDB-{GOVERNANCE-LOGIC,HNSW-HYBRID-INDEXING,KIMPACT-ALGORITHM,
TEMPORAL-MODEL,SEGREGATION-STRATEGY,CSR-MUTATION-STRATEGY,MARKET-POSITIONING},
ADR--PHASE-11-INDUSTRIAL-HARDENING, ADR--PHASE-13-WAL-GROUP-COMMIT.

## Known dangling references (fix in doc-governance pass)
- `docs/MASTER-SPEC--GENESIS-DB.md` cited as authoritative parent — verify it is current.
- Root `hql.pest`, `src/query/planner.rs`, `src/sync/mod.rs`, `AGENTS.md`,
  `docs/specs/SPEC-Genesis-Block.md`, `docs/GO-SDK-GUIDE.md` are referenced but
  do not exist.
