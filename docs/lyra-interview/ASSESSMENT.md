# ASSESSMENT - LYRA

## Evidence audit

| claim | tag | citation | risk |
|---|---|---|---|
| HQL currently parses fixed command families plus linear pattern `MATCH`, not arbitrary Cypher. | derived | `src/query/hql.pest:51-93`; `src/query/ast.rs:162-208` | Medium: "Cypher equivalent" can overclaim. |
| HQL execution is direct dispatch, not a logical planner. | derived | `src/lib.rs:3398-3499`; `src/query/mod.rs:4-8` | Medium: selective patterns may need reordering. |
| Pattern matching scans all live nodes unless the anchor has exact `{id}`. | derived | `src/lib.rs:3049-3091` | High for large graphs with late selective predicates. |
| HQL clauses are post-processing, not index pushdown. | derived | `src/lib.rs:2925-2973`; `docs/adr/ADR--GENESISDB-HQL-FILTER-PROJECTION.md:116-120` | High for G1 filter parity against Qdrant. |
| G3 cross-dimension locality has not been benchmarked. | unknown | `docs/genesis-interview/QUESTIONS.md:90-92` | Critical: central moat may be mirage. |
| Qdrant vector numbers exist but compare embedded GenesisBlockDB with localhost client-server Qdrant. | measured | `docs/AUDIT--P20-QDRANT-3WAY-AND-EF-CONFIG.md:34-47`; `BENCHMARKING.md:245-263` | Medium: unfair surface can inflate wins. |
| Graph traversal numbers exist for 100k/800k, fanout 8, depths 1/3/6. | measured | `docs/AUDIT--P31-POST-MARKXIII-REGRESSION.md:38-41`, `docs/AUDIT--P31-POST-MARKXIII-REGRESSION.md:68-88` | Medium: not the same as HQL-vs-Cypher subset coverage. |
| `AS OF` is point valid-time filtering with exclusive `valid_to`; no interval syntax appears. | derived | `src/query/hql.pest:25`, `src/lib.rs:3502-3513` | High if GoVibe needs interval overlap or tx time. |
| GoVibe requires valid-time plus transaction-time fields. | asserted | `G:/govibe/docs/architecture/ERD-GoVibe-Platform-Data-Model.md:221-225` | High: engine edge tx metadata is not queryable via HQL. |
| In-engine RRF/BM25 is planned, not shown in current grammar/executor. | derived | `docs/adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md:91-111`; `src/query/hql.pest:51-93` | Critical for A3/G3. |
| HGMem differentiated retrieval has no measured advantage in the inspected evidence. | unknown | `docs/genesis-interview/QUESTIONS.md:73-77`; `tests/grl_retrieval_tests.rs:135-146` | Medium: could distract from simpler vector+RRF. |

## Target validity

**G1 - vector/retrieval vs Qdrant:** [asserted] The target asks for expressiveness + latency parity or >=10% faster on the same query. Citation: `docs/lyra-interview/LYRA.md:91`. [derived] This is measurable only if the comparison uses the same surface and same filter semantics. Citation: `BENCHMARKING.md:245-263`. [derived] Current `WHERE` is post-retrieval, so Qdrant-style payload filter parity is not established. Citation: `src/lib.rs:2925-2973`; `BENCHMARKING.md:249-251`. [derived] Honest reframing: "vector latency/recall parity on same-surface ANN; filter parity only after pushdown evidence."

**G2 - traversal vs Cypher:** [asserted] The target is limited to the needed subset and <=10% slower. Citation: `docs/lyra-interview/LYRA.md:92`. [derived] This is fair only if the subset is enumerated. Citation: `docs/genesis-interview/GENESIS.md:78-82`. [derived] Current HQL supports fixed linear path patterns but not broad Cypher semantics. Citation: `src/query/hql.pest:55-84`; `docs/adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS.md:101-110`. [derived] Honest reframing: "equivalent for pre-registered fixed-hop, linear, read-only patterns only."

**G3 - cross-dimension:** [asserted] The target is a decisive round-trip + latency win over Qdrant+Neo4j+TS-RRF. Citation: `docs/lyra-interview/LYRA.md:93`. [derived] This is the relevant moat test under the charter because it evaluates cross-dimension integration locality rather than specialist turf. Citation: `docs/genesis-interview/GENESIS.md:74-76`. [unknown] It lacks a numeric decisive margin and has not been benchmarked. Citation: `docs/genesis-interview/QUESTIONS.md:90-92`. [derived] Honest reframing: "fails if HQL does not beat composed p50 and p99 end-to-end latency and round trips at equal correctness; commissioner must set decisive margin."

## Semantic soundness

[derived] Engine validity is point-based: `valid_from <= as_of < valid_to`, with null `valid_to` behaving as open-ended. Citation: `src/lib.rs:3502-3513`. [measured] Tests cover future-valid filtering and retraction visibility for point `AS OF`. Citation: `tests/temporal_queries_tests.rs:65-116`; `tests/retract_edge_tests.rs:120-145`. [derived] HQL has no syntax for Allen relations, interval overlap, application-time ranges, transaction-time `recorded_at`, or `superseded_at` predicates. Citation: `src/query/hql.pest:25`, `src/query/hql.pest:51-84`. [asserted] GoVibe's ERD separates business-valid time from system transaction time. Citation: `G:/govibe/docs/architecture/ERD-GoVibe-Platform-Data-Model.md:221-225`.

[derived] Traversal semantics are BFS with a visited set for `neighbors`, direction flags, relation filters, and optional limit. Citation: `src/lib.rs:3681-3819`. [derived] Pattern semantics are left-to-right expansion and row post-processing. Citation: `src/lib.rs:3034-3179`. [derived] Gap: repeated variable identity, cycles, branching, frontier caps, and variable-length path semantics are not current code-backed semantics. Citation: `docs/adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS.md:101-110`.

## Falsification plan

**G1:** Use 100k dim-1024 synthetic clustered vectors plus a real-doc corpus. Baselines: Qdrant same host and GenesisBlockDB through both embedded and REST surfaces. Queries: vector-only, vector+payload filters, collection-scoped, cold/warm. Metrics: recall@10, p50/p95/p99, max latency, RSS, and CI. Fails if HQL is slower on same-surface same-query latency or cannot express equivalent filters. Target provenance: `docs/lyra-interview/LYRA.md:91`.

**G2:** Use the P31 graph shape: 100k nodes, 800k edges, fanout 8, depths 1/3/6, 200 queries/depth. Citation: `docs/AUDIT--P31-POST-MARKXIII-REGRESSION.md:38-41`. Compare HQL pattern queries to the pre-registered Cypher subset. Fails if any needed query cannot parse, returns different node/path sets, or is >10% slower. Target provenance: `docs/lyra-interview/LYRA.md:92`.

**G3/moat:** Build queries spanning at least two dimensions: vector + hop, hop + `AS OF`, vector + hop + `AS OF`, and fusion once RRF exists. Compare embedded HQL, REST HQL, and Qdrant+Neo4j+TS-RRF. Fails if HQL does not reduce round trips and does not beat composed p50 and p99 end-to-end latency at equal recall/correctness. The numeric "decisive" margin remains an OPEN-QUESTION.

**HGMem:** Compare HGMem/SuperNode retrieval to vector-only, vector+RRF, and graph-expanded RRF with judged relevance and token budget. Fails if recall@k or nDCG@k does not improve at equal or lower p99 latency.

## Red-team of PROPOSAL

not present

## OPEN-QUESTIONS

1. What exact numeric G3 margin means "win decisively"?
2. Which Cypher subset is production-required by GoVibe/GKS?
3. Must HQL support valid-time interval overlap, or is point `AS OF` enough?
4. Must HQL expose transaction-time predicates over `recorded_at` and `superseded_at`?
5. Are timestamp strings guaranteed normalized so lexicographic comparison is sound?
6. Should HQL reorder selective graph patterns, or require users to write the selective anchor first?
7. What are the allowed RRF signals, missing-value behavior, and weight bounds?
8. What corpus and relevance judgments define HGMem success?
9. Is W-scale fan-out supposed to be enforced by HQL/runtime, or only by governance process?
10. Is in-engine FTS5/BM25/RRF a prerequisite for running the G3 benchmark?
