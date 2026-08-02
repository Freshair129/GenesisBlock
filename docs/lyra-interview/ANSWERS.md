# ANSWERS - LYRA evidence audit

## A1
**Verdict:** [unknown] The moat is a falsifiable hypothesis, not a measured result.

**Evidence:** [unknown] The input brief says the cross-dimension gate - one in-engine `vector + graph-hop + asOf + fusion` round trip versus Qdrant + Neo4j + TS-RRF - has "Never benchmarked" status. Citation: `docs/genesis-interview/QUESTIONS.md:90-92`. [measured] Component benchmarks exist but do not settle the moat: the Qdrant comparison used 100k synthetic clustered vectors and Qdrant over localhost gRPC, while GenesisBlockDB was embedded. Citation: `docs/AUDIT--P20-QDRANT-3WAY-AND-EF-CONFIG.md:34-47`. [derived] The repo benchmark guide warns not to compare embedded p50 directly to networked p50. Citation: `BENCHMARKING.md:245-263`.

**Falsifier / what would settle it:** Run the same query set through (1) HQL in-process, (2) HQL REST, and (3) Qdrant + Neo4j + TS-RRF with identical result payloads and recall criteria. The moat fails if HQL is not lower in end-to-end p50 and p99 after confidence intervals, or if its recall/correctness is below the composed baseline. The "decisive" margin is unknown because the target gives no numeric G3 margin. Citation: `docs/genesis-interview/GENESIS.md:74-76`.

**Open questions:** What exact margin makes a G3 win "decisive" rather than a tie?

## A2
**Verdict:** [derived] HQL today is a fixed-shape dispatch language; planner pressure begins at selective joins, variable-length paths, and fusion pipelines.

**Evidence:** [derived] The grammar parses `SEARCH`, `TRAVERSE`, hybrid `MATCH`, Cypher-style linear `MATCH`, and `CONTEXT`. Citation: `src/query/hql.pest:51-93`. [derived] The AST has matching command variants only. Citation: `src/query/ast.rs:162-208`. [derived] `execute_hql` dispatches those variants directly to `hybrid_search`, `neighbors`, `retrieve_context`, or `match_pattern`. Citation: `src/lib.rs:3398-3499`. [derived] Pattern matching anchors by exact `{id}` or scans all nodes, expands left-to-right, then applies `WHERE`. Citation: `src/lib.rs:3049-3095`, `src/lib.rs:3172-3179`. [asserted] Current ADR scope excludes branching patterns, identity cycles, relationship alternation in patterns, OR, and aggregation. Citation: `docs/adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS.md:101-110`.

**Falsifier / what would settle it:** [derived] A concrete planner-breaking shape is `MATCH (a:User)-[:SENT]->(m)-[:MENTIONS]->(t {id:"x"}) WHERE m.prop.lang = "th" RETURN a.id`: the current executor must scan candidate `a` rows before seeing the selective `t` anchor. Citation: `src/lib.rs:3049-3095`, `src/lib.rs:3172-3179`. If latency grows with all `User` nodes rather than the selective `t` neighborhood, direct dispatch has crossed into planner territory.

**Open questions:** Is the intended user contract "write the selective anchor first", or should HQL reorder pattern evaluation?

## A3
**Verdict:** [unknown] Caller-parameterized RRF is not proven safe; it removes the default-policy footgun only if the engine enforces checkable signal rules.

**Evidence:** [derived] Current HQL has no `RANK BY rrf(...)` grammar. Citation: `src/query/hql.pest:51-93`. [asserted] The K-Impact ADR documents a NotiKeeper misuse: `alpha: 1.0` was intended as vector-only but meant K-Impact-only. Citation: `docs/adr/ADR--GENESISDB-KIMPACT-AS-SIGNAL.md:42-46`. [derived] Current scoring still accepts caller `alpha` and blends similarity with `impact`. Citation: `src/lib.rs:3621-3643`.

**Falsifier / what would settle it:** Minimal checkable rule: every `RANK BY rrf` signal must name a declared source, direction, normalization/rank extraction, missing-value behavior, and weight range; queries with unknown signals, all-zero weights, duplicate aliases, or no retrieval source must error. Concrete misuse still possible without this: `rrf(vector:0, hops:1000)` silently becomes graph-hop ranking while looking like hybrid retrieval.

**Open questions:** Which signals are first-class engine signals, and what are their allowed weight ranges?

## A4
**Verdict:** [unknown] HGMem value is unproven; the repo shows clustering/meta-graph machinery, not differentiated hyperedge retrieval evidence.

**Evidence:** [asserted] The interview brief calls HGMem a candidate and says its `merge` step is a stub. Citation: `docs/genesis-interview/QUESTIONS.md:73-77`. [derived] Current code implements community/meta structures, SuperNodes, MetaEdges, and structural gap suggestions. Citation: `src/lib.rs:3976-4014`, `src/lib.rs:5637-5694`. [measured] GRL tests verify H0/H1/H2 scope and tiny-budget SuperNode fallback, not superiority over vector+RRF. Citation: `tests/grl_retrieval_tests.rs:88-104`, `tests/grl_retrieval_tests.rs:135-146`. [unknown] No evidence found that HGMem beats plain vector + RRF retrieval.

**Falsifier / what would settle it:** Compare HGMem retrieval against vector-only, vector+RRF, and graph-expanded RRF on the same judged relevance set. HGMem fails if it does not improve recall@k or nDCG@k at equal or lower p99 latency and token budget.

**Open questions:** What corpus and relevance judgments define HGMem success?

## B1
**Verdict:** [unknown] Seven high-damage assumptions remain open.

**Evidence:**  
1. [unknown] G3 locality beats app composition. No benchmark yet. Citation: `docs/genesis-interview/QUESTIONS.md:90-92`. Cheapest resolution: run the C1 G3 harness.  
2. [assumed] G3 "decisive" has a shared numeric meaning. The target lacks a margin. Citation: `docs/lyra-interview/LYRA.md:91-93`. Resolution: commissioner sets a pre-registered margin.  
3. [assumed] HQL can stay planner-free while adding selective multi-hop pattern semantics. Current executor is left-to-right. Citation: `src/lib.rs:3034-3041`, `src/lib.rs:3094-3179`. Resolution: benchmark selective-late-predicate patterns.  
4. [assumed] RRF weights can be caller-owned without recreating ranking-policy errors. K-Impact misuse shows weight semantics can be misunderstood. Citation: `docs/adr/ADR--GENESISDB-KIMPACT-AS-SIGNAL.md:42-52`. Resolution: reject invalid weight/signal specs and run misuse tests.  
5. [unknown] GoVibe hallucination-safety follows from hop scoping. The concept asserts prevention. Citation: `G:/govibe/docs/CONCEPT--HYBRID-JIT-CONTEXT.md:49-53`. Resolution: run answer-quality/hallucination evals by hop level.  
6. [unknown] In-engine RRF/BM25 exists. Current substrate ADR lists it as S3, not current code. Citation: `docs/adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md:91-111`. Resolution: implement or declare out of scope before benchmarking G3.  
7. [unknown] Point `AS OF` is enough for the GoVibe interval model. GoVibe requires valid-time and tx-time fields. Citation: `G:/govibe/docs/architecture/ERD-GoVibe-Platform-Data-Model.md:221-225`. Resolution: define interval-overlap and transaction-time query requirements.

**Falsifier / what would settle it:** Each assumption is settled by the cheapest experiment or commissioner question listed above.

**Open questions:** Should any of these assumptions block HQL-redefinition before code work continues?

## C1
**Verdict:** [derived] G1 and G2 are measurable but easily unfair; G3 is the right moat test but needs a numeric margin.

**Evidence:** [asserted] G1 asks for Qdrant parity or >=10% faster, G2 asks for needed Cypher subset within <=10% slower, and G3 asks for decisive round-trip + latency win. Citation: `docs/lyra-interview/LYRA.md:88-93`. [measured] Existing vector benchmarks use 100k, dim 1024 synthetic clustered vectors with exact L2 ground truth. Citation: `docs/AUDIT--P20-QDRANT-3WAY-AND-EF-CONFIG.md:34-47`. [measured] Existing graph harness uses 100k nodes, fanout 8, depths 1/3/6, and 200 queries/depth. Citation: `docs/AUDIT--P31-POST-MARKXIII-REGRESSION.md:38-41`.

**Falsifier / what would settle it:** Dataset: 100k/800k graph + 100k dim-1024 vectors plus a small real-doc corpus. Queries: vector+filter, fixed and variable hop traversal, `AS OF`, and cross-dimension fusion. Baselines: Qdrant, Cypher engine, and Qdrant+Neo4j+TS-RRF. Metrics: correctness/recall@k, p50/p95/p99, max latency, round trips, RAM/RSS, ingest, warm/cold, and 95% CI over repeated runs. G1 fails if HQL lacks Qdrant-equivalent filter expressiveness or is slower on same-surface same-query latency. G2 fails if the needed Cypher subset is not expressible or is >10% slower. G3 fails if HQL does not beat composed p50 and p99 end-to-end latency and round trips.

**Open questions:** Which Cypher subset is "needed", and what numeric G3 margin replaces "decisive"?

## D1
**Verdict:** [derived] Current `AS OF` is a valid-time point projection, not a full bitemporal interval system.

**Evidence:** [derived] Engine nodes expose `valid_from`/`valid_to`; edges expose `valid_from`/`valid_to` plus `recorded_at`/`superseded_by`. Citation: `src/lib.rs:121-138`, `src/lib.rs:156-168`. [derived] HQL grammar only exposes `AS OF "<ts>"`, not interval overlap or transaction-time predicates. Citation: `src/query/hql.pest:25`, `src/query/hql.pest:51-84`. [derived] `is_valid_as_of` implements `valid_from <= as_of < valid_to` by string comparison and treats null `valid_to` as open-ended. Citation: `src/lib.rs:3502-3513`. [measured] Tests cover point `AS OF` and retraction visibility. Citation: `tests/temporal_queries_tests.rs:65-116`, `tests/retract_edge_tests.rs:120-145`. [asserted] GoVibe separates valid time from system transaction time. Citation: `G:/govibe/docs/architecture/ERD-GoVibe-Platform-Data-Model.md:221-225`.

**Falsifier / what would settle it:** Add interval-overlap and transaction-time conformance tests. Current model fails full bitemporal soundness if users need Allen relations, tx-time `recorded_at/superseded_at`, or non-normalized timestamp comparisons.

**Open questions:** Are timestamps guaranteed normalized RFC3339 UTC strings, and must HQL support tx-time queries?

## D2
**Verdict:** [derived] HQL is a convenient graph/vector subset, not Cypher equivalence.

**Evidence:** [derived] HQL can express vector `SEARCH`, depth-bounded `TRAVERSE`, hybrid vector+impact `MATCH`, fixed linear graph `MATCH`, and tiered `CONTEXT`. Citation: `src/query/hql.pest:51-93`. [derived] Pattern syntax supports node/edge variables, labels, exact inline props, direction, `WHERE` with AND, `ORDER BY`, `LIMIT`, `RETURN`, and `AS OF`. Citation: `src/query/hql.pest:55-84`. [asserted] The ADR excludes branching patterns, identity cycles, variable-length paths, OR, and aggregation in current pattern scope. Citation: `docs/adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS.md:101-110`. [asserted] HQL v2 target still records no cost planner, no branching/comma patterns, no optional match, no path variables, no shortest path, and limited aggregation. Citation: `docs/SPEC--HQL-V2.md:169-171`.

**Falsifier / what would settle it:** Run the needed-query inventory through parser tests. Any required `WITH`, aggregation, negation, optional match, path variable, identity-cycle join, or variable-length path query that cannot parse falsifies "equivalence" beyond the subset.

**Open questions:** Which GoVibe/GKS production queries require Cypher features outside the current subset?

## LYRA — verdict
[derived] The programme rests on evidence for component capabilities - HNSW search, BFS traversal, point `AS OF`, SuperNode fallback - but [unknown] on hope for the cross-dimension moat. Citation: `src/lib.rs:3516-3655`, `src/lib.rs:3681-3819`, `src/lib.rs:4257-4318`, `docs/genesis-interview/QUESTIONS.md:90-92`. [derived] The single most important next act is a pre-registered G3 benchmark with identical payloads, recall checks, p50/p99, round trips, and a commissioner-approved numeric margin for "decisive".
