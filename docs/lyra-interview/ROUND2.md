# ROUND2 - LYRA

## L-R2.1 - Falsify "cloud DBs will not follow local-first"

| Claim | Tag | Citation | Finding |
|---|---|---|---|
| The Round 2 brief frames P1 as a local-first/personal-use tier where cloud DBs "won't" follow, and names SQLite+sqlite-vec, LanceDB, Kuzu/LadybugDB, DuckDB+vss, Chroma embedded, local Postgres+pgvector, and libSQL/Turso as the real competitors. | asserted | `docs/ROUND2-POSITIONING.md:17-24` | The claim is explicitly a positioning thesis, not measured evidence. |
| Qdrant already contests local-first vector search: the Python client has in-memory and disk-persistent local mode without running a Qdrant server, and Qdrant Edge is a beta embedded in-process vector engine with local storage/query and sync hooks. | derived | [Qdrant client local mode](https://github.com/qdrant/qdrant-client#local-mode); [Qdrant Edge](https://qdrant.tech/documentation/edge/) | This falsifies "can't" and weakens "won't": rating = **already contested** for vector-local deployment. |
| Chroma already contests local embedded/persistent vector storage: its docs describe in-memory clients and `PersistentClient` loading/saving a local database path. | derived | [Chroma clients](https://docs.trychroma.com/docs/run-chroma/clients) | Rating = **already contested** for Python local vector memory. |
| Turso/libSQL already contests local-first SQL deployment: embedded replicas serve reads from a local file, support local operation paths, and Turso Sync is positioned for local-first writes with push/pull. | derived | [Turso embedded replicas](https://docs.turso.tech/features/embedded-replicas/introduction) | Rating = **already contested** for SQLite-family local/cloud sync, not proof of Genesis differentiation. |
| Neo4j documents embedded Java usage where application code and Neo4j run in the same JVM. | derived | [Neo4j embedded Java](https://neo4j.com/docs/java-reference/current/java-embedded/) | Rating = **already contested** for embedded graph as a deployment shape, though not necessarily for Genesis' exact bitemporal/signed-WAL model. |
| The defensible moat is not "cloud/local vendors will not follow"; it can only be the narrower G3 bundle: embedded consolidation plus bitemporal cross-dimension query locality in one engine. | derived | `docs/ROUND2-POSITIONING.md:11-13`; `docs/genesis-interview/QUESTIONS.md:90-92` | Rating = **unknown / won't-yet at best** until G3 is benchmarked; the exact bundle may be less contested, but that is not proven. |

## L-R2.2 - Is SQLite+sqlite-vec the null hypothesis?

Yes. [derived] SQLite+sqlite-vec is now the null hypothesis for local agent memory because the brief itself names it the "good enough" king and sharpest baseline, while G3 cross-dimension locality is explicitly never benchmarked. Citation: `docs/ROUND2-POSITIONING.md:20-24`; `docs/genesis-interview/QUESTIONS.md:90-92`.

[derived] The null has credible primitives: sqlite-vec is a vector-search SQLite extension that runs on laptops, servers, mobile devices, WASM browsers, and Raspberry Pis; SQLite documents recursive CTE graph queries; SQLite triggers can react to row insert/update/delete events. Citation: [sqlite-vec](https://alexgarcia.xyz/sqlite-vec/); [SQLite WITH / recursive CTE](https://www.sqlite.org/lang_with.html); [SQLite CREATE TRIGGER](https://sqlite.org/lang_createtrigger.html).

[measured] GenesisBlock's current graph bench corpus has a reusable scale point: 100k nodes, fanout 8, 200 queries per depth, with 686 MB RSS at 100k/800k. Citation: `docs/AUDIT--P31-POST-MARKXIII-REGRESSION.md:38-41`; `docs/AUDIT--P31-POST-MARKXIII-REGRESSION.md:56-63`.

Experiment that settles the null:

1. [derived] Build two implementations of the same pre-registered local G3 workload: `vector + graph-hop + AS OF + fusion`, one in GenesisBlockDB HQL/runtime and one in SQLite+sqlite-vec with recursive CTEs and trigger-maintained temporal history. Citation: `docs/genesis-interview/QUESTIONS.md:84-92`; `src/query/hql.pest:51-93`.
2. [derived] Use the existing 100k/800k graph scale as the first required tier, because it is already measured in this repo; add embeddings, temporal edge history, and identical relevance labels. Citation: `docs/AUDIT--P31-POST-MARKXIII-REGRESSION.md:38-41`; `docs/genesis-interview/QUESTIONS.md:90-92`.
3. [assumed] Provisional kill number: the SQLite null survives if it returns the same correct top-k results and GenesisBlockDB is not **>10% faster on both p50 and p99 end-to-end latency** on the same query, same process boundary, same hardware. Citation: `docs/lyra-interview/LYRA.md:91-93`. Risk: G3 has no numeric margin; 10% is borrowed from G1/G2, not specified for G3.
4. [derived] If SQLite cannot faithfully express `HYBRID + TRAVERSE + AS OF + RANK BY rrf(...)`, the null fails semantically before latency is considered. Citation: `docs/genesis-interview/QUESTIONS.md:84-92`; `docs/genesis-interview/QUESTIONS.md:105-110`.

## L-R2.3 - SQLite contradiction, multi-model claim, and RAM

| Claim | Tag | Citation | Audit |
|---|---|---|---|
| "Build on SQLite while competing against SQLite+sqlite-vec" is coherent only if GenesisBlockDB is positioned as native graph/vector/bitemporal/signed orchestration over SQLite-backed payloads, not as "SQLite but better." | derived | `docs/adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md:34-48`; `docs/adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md:98-100` | Coherent frame: SQLite is the projection/substrate; Genesis owns the signed WAL, graph/vector, bitemporal, governance, and fusion pipeline. |
| The SQLite substrate is planned, not current runtime behavior. | measured | `docs/adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md:108-111`; `docs/SPEC--SQLITE-SUBSTRATE-S0-S1.md:24-30`; `Cargo.toml:48-87` | S0-S3 are unchecked/planned; S0/S1 explicitly exclude SQL-backed HQL and FTS/BM25. Current dependency list shown has no SQLite/rusqlite/libsql dependency. |
| Current node records already accept arbitrary JSON props and store resident `NodeOutput` in `Storage.nodes`. | measured | `src/lib.rs:99-125`; `src/lib.rs:1554-1558` | "Already a document store" is true only at the storage-shape level. |
| Current HQL document/property access is narrow: `prop.<key>` predicates and pattern props, applied after retrieval with no planner. | measured | `src/query/hql.pest:34-48`; `src/query/hql.pest:61-84`; `src/lib.rs:2826-2945` | A broad NoSQL/document query surface is not current code-backed. |
| "Multi-model consolidation" is falsifiable only when tied to a single workload that needs vector + graph + time + fusion in one query. | derived | `docs/ROUND2-POSITIONING.md:39-44`; `docs/genesis-interview/QUESTIONS.md:90-92` | Real capability claim if it reduces latency/round-trips at equal correctness; marketing frame if it expands into document/KV/time-series without a pre-registered query and metric. |
| The RAM weakness does not "disappear" from evidence. The measured repo fact is 686 MB RSS at 100k/800k, 7.1x vs LadybugDB per the SQLite ADR. | measured | `docs/adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md:28`; `docs/AUDIT--P31-POST-MARKXIII-REGRESSION.md:68-88` | The P1 claim that personal scale makes RAM weakness matter less is inferred, not measured. |
| The planned S1 substrate phase treats RAM reduction as a gate and intentionally avoids hardcoded numeric thresholds. | derived | `docs/SPEC--SQLITE-SUBSTRATE-S0-S1.md:151-169` | The repo itself says RAM must be re-measured; no current evidence proves personal-scale sufficiency. |

## OPEN-QUESTIONS

1. [unknown] What exact "personal-scale" corpus size and target device RAM budget should falsify or validate the claim that 686 MB @100k/800k no longer matters? Citation: `docs/ROUND2-POSITIONING.md:22-24`; `docs/AUDIT--P31-POST-MARKXIII-REGRESSION.md:56-63`.
2. [unknown] Does the commissioner approve the provisional >10% p50+p99 G3 margin, or should G3 define a different "decisive" number? Citation: `docs/lyra-interview/LYRA.md:91-93`; `docs/genesis-interview/QUESTIONS.md:90-92`.
3. [unknown] What relevance labels or oracle define "same correct top-k" for the G3 SQLite-vs-Genesis experiment? Citation: `docs/genesis-interview/QUESTIONS.md:84-92`.
4. [unknown] Is a public document/NoSQL query surface actually required, or is JSON `props` plus HQL `prop.<key>` enough for the MSP memory layer? Citation: `src/lib.rs:99-125`; `src/query/hql.pest:34-48`; `docs/ROUND2-POSITIONING.md:39-44`.

**ROUND2 verdict:** [derived] P1 strengthens only after reframing from "cloud will not follow" to "prove embedded G3 beats SQLite/local-specialist baselines"; [derived] the trap is selling planned SQLite-backed multi-model consolidation as current moat; [unknown] the next decision is the pre-registered SQLite+sqlite-vec+CTE G3 benchmark margin and personal-scale RAM budget. Citation: `docs/ROUND2-POSITIONING.md:17-24`; `docs/ROUND2-POSITIONING.md:32-44`; `docs/genesis-interview/QUESTIONS.md:90-92`.
