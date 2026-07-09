# Grounding: SQLite-only "hybrid vector+graph+bitemporal agent memory" — what's real as of 2026-07

## 1. sqlite-vec

**Versions/dates** (https://github.com/asg017/sqlite-vec/releases, fetched 2026-07-06):
- Latest **stable: v0.1.9 — 2026-03-31**. Latest prerelease: **v0.1.10-alpha.4 — 2026-05-18** (alpha.1 2026-03-31).

**ANN status — this changed recently, do NOT claim "still brute-force only":**
- v0.1.10-alpha.1 release notes: "Initial alpha release of `sqlite-vec` with new ANN indexes: rescore, ivf (experimental, not enabled), and DiskANN" (https://github.com/asg017/sqlite-vec/releases/tag/v0.1.10-alpha.1). The release itself says "Proper docs/examples coming soon!" and defers details to PRs #276-278 — i.e. **ANN exists only in an undocumented alpha**; the shipped stable line (≤v0.1.9) is still brute-force linear scan.
- The ANN tracking issue #25 (opened 2024-06-21) is **still open**; author states brute force "slows down on large datasets (>1M w/ large dimensions)" and weighed IVF vs HNSW vs DiskANN (favoring LM-DiskANN for simplicity) (https://github.com/asg017/sqlite-vec/issues/25).
- alpha.4 is still fixing basics in the ANN path ("Fix bug that made ALTER TABLE RENAME to fail on vec0 that use the new ivf/diskann features"; cached-statement cleanup bug in DiskANN) — signal of maturity level.

**Quantization:** float32, **int8**, and **binary/bit** vectors in `vec0` tables (README: "Store and query float, int8, and binary vectors in vec0 virtual tables", https://github.com/asg017/sqlite-vec). Binary = "1 bit per element, a 32x reduction"; **Matryoshka** dimension-truncation also supported (v0.1.0 announcement, https://alexgarcia.xyz/blog/2024/sqlite-vec-stable-release/index.html).

**Metadata filtering / partition keys:** added in **v0.1.6 (2024-11-20)** — metadata columns usable in KNN `WHERE`, partition keys shard the index ("3x faster" on the year-partition example), auxiliary `+` columns unindexed. Limitations: only equality, `<,<=,>,>=`, `IN`; "Notably absent: REGEXP, LIKE, GLOB...; Also NULL values are not supported yet" (https://alexgarcia.xyz/blog/2024/sqlite-vec-metadata-release/index.html).

**Published performance (author's own, brute-force, v0.1.0 blog, https://alexgarcia.xyz/blog/2024/sqlite-vec-stable-release/index.html):**
- 100k vectors: 3072-dim f32 ≈ **214 ms**/query; 1536-dim f32 ≈ **105 ms**; 3072-dim bit ≈ 11 ms.
- 1M vectors: 3072-dim f32 ≈ **8.52 s**; 192-dim f32 ≈ 192 ms; bit ≈ 124 ms.
- So at agent-memory-typical dims (768–1024), full-precision brute force at 100k is roughly ~50–100 ms-class, and ~1M is seconds-class (interpolation — the exact 768/1024-dim rows are UNVERIFIED).

**Author's scale guidance:** "most applications of local AI or embeddings aren't working with billions of vectors. Most of my little data analysis projects deal with thousands of vectors, maybe hundreds of thousands" (same v0.1.0 post); issue #25 frames a future custom ANN as what would get it to "low millions"/"tens of millions".

**sqlite-vss:** yes, deprecated in favor of sqlite-vec. README warning: "`sqlite-vss` is not in active development. Instead, my effort is now going towards `sqlite-vec`…" (https://github.com/asg017/sqlite-vss).

Adjacent note: there is now also a third-party `sqlite-vector` extension (sqliteai) and an "sqlite.org/vec1" page surfaced in search (https://sqlite.org/vec1) — existence of an *official* SQLite vec1 extension is UNVERIFIED (not fetched); worth checking before quoting.

## 2. libSQL / Turso native vector search

- **GA, native, no extension needed.** libSQL "implements DiskANN algorithm in order to speed up approximate nearest neighbors queries" (https://turso.tech/blog/turso-brings-native-vector-search-to-sqlite; https://docs.turso.tech/features/ai-and-embeddings).
- **Works embedded/local:** "This works in every build of libSQL. Whether you are connecting to the Turso service, or running an in-memory database without any network connectivity, vectors are available" (Turso blog, above URL).
- Types: FLOAT64/32/16/BF16/FLOAT8/**FLOAT1BIT** (32x compression); index via `CREATE INDEX … (libsql_vector_idx(embedding))`; ANN queried explicitly via `vector_top_k()`; max 65,536 dims; euclidean unsupported on 1-bit; index requires ROWID/simple PK (https://docs.turso.tech/features/ai-and-embeddings).
- Caveat: the **new Rust rewrite "Turso"** (tursodatabase/turso) is a separate lineage — its vector-index story is still being designed in issues (#832 DiskANN, #3778 "first do SIMD brute force") (https://github.com/tursodatabase/turso/issues/832, /issues/3778). The GA DiskANN is in **libSQL** (the C fork), not yet the rewrite. Positioning claims should name libSQL specifically.

## 3. Recursive CTE as graph engine

- **Row-at-a-time semantics are by spec:** SQLite's own docs define recursive CTE evaluation as a queue algorithm that "extract[s] a single row from the queue" per step (https://sqlite.org/lang_with.html — semantics well known; exact wording UNVERIFIED, not fetched this session).
- **Practitioner numbers** (dev.to, "SQLite as a Graph Database… Why We Ditched Neo4j", 2024-03-24, https://dev.to/rohansx/sqlite-as-a-graph-database-recursive-ctes-semantic-search-and-why-we-ditched-neo4j-1ai): "At depth 4 with an average branching factor of 10, you're visiting 10,000 nodes per query. SQLite handles this in milliseconds with proper indexes" — but "At **500k entities with depth 6, you'll feel it**"; sweet spot stated as "knowledge graphs in the tens-of-thousands-of-nodes range"; also caps semantic search at "roughly 50-100k embeddings" before wanting HNSW. Companion repo: https://github.com/shwetarkadam/sqlite-graph.
- Comparative framing: Neo4j-style index-free adjacency gives constant-time hops; recursive CTEs degrade on deep (6+ hop) traversals — "milliseconds vs seconds or timeout" for friends-of-friends-of-friends at millions of users (https://algoroq.io/compare-tech/neo4j-vs-postgresql/ — PostgreSQL CTEs, directionally applicable to SQLite; SQLite-specific head-to-head benchmark vs a real graph DB: **none found — UNVERIFIED/absent in the literature I reached**).
- No rigorous published SQLite-recursive-CTE-vs-graph-DB benchmark surfaced; evidence is anecdotal blog-scale. That itself is a citable gap.

## 4. Bitemporal in raw SQLite

- **No native system-versioning: confirmed.** "SQLite doesn't have built-in temporal tables" / no `AS OF` (https://www.sqliteforum.com/p/sqlite-and-temporal-tables; https://www.ohnekontur.de/2024/02/19/unlocking-time-harnessing-the-power-of-temporal-tables-in-sqlite/). Everything is triggers + history tables.
- **Real implementations, all single-axis (transaction-time/audit), none truly bitemporal:**
  - simonw/sqlite-history — trigger-generated change tracking (https://simonwillison.net/2023/Apr/15/sqlite-history/)
  - ohnekontur (2024-02-19): mirror history table + valid_from/valid_to + I/U/D triggers; author scripts the SQL generation because doing it by hand is tedious; **system-versioning only, not bitemporal** (URL above)
  - bytefish.de audit-log pattern (https://www.bytefish.de/blog/sqlite_logging_changes.html)
  - thatdevsherry/historia — application-layer history (https://github.com/thatdevsherry/historia)
- **Known-hard part, on record:** bytefish.de states flatly: "It's not possible to provide an equivalent to Temporal Tables from within SQLite, because it **lacks a stable transaction time to be used throughout multiple triggers**" — "Functions like CURRENT_TIMESTAMP … are stable only within a *Statement*." Author concedes his pattern enables forensics but can't restore prior state "with 100% certainty" (https://www.bytefish.de/blog/sqlite_logging_changes.html). This is the strongest citable defect for the two-axis case.
- **Interval-overlap integrity:** SQLite has no `WITHOUT OVERLAPS`/exclusion constraints; even PostgreSQL only got PK `WITHOUT OVERLAPS` in v18, and still "does not automatically track system time" (https://lord.technology/2025/01/28/understanding-temporal-primary-keys.html). In SQLite, non-overlap of valid-time intervals per entity must be enforced by triggers or app code. Retraction (closing valid_to on an assertion without destroying transaction-time history) has **no published SQLite implementation I could find — UNVERIFIED/absent**; the canonical bitemporal references (XTDB docs, https://v1-docs.xtdb.com/concepts/bitemporality/) describe the semantics but on purpose-built engines.

## 5. Single-transaction consistency across vec + CTE + temporal WHERE

- **Confirmed.** SQLite's default isolation is SERIALIZABLE, and in WAL mode "all reads made in a transaction see a consistent snapshot of the database that existed at the time the transaction started" (https://sqlite.org/isolation.html). Within one connection, "a query sees all changes … completed on the same database connection prior to the start of the query."
- This covers sqlite-vec too: `vec0` is a virtual table backed by **shadow tables in the same database file**, so a single statement/transaction joining a vec0 KNN, a recursive CTE, and temporal predicates reads one snapshot. (Shadow-table storage: sqlite-vec design discussions in issue #25 reference "shadow tables"; general shadow-table participation in transactions is core SQLite behavior. The specific end-to-end claim "vec0 + CTE + temporal WHERE in one statement is snapshot-consistent" is an inference from these documented properties, not something anyone has written up — mark the composed claim UNVERIFIED-as-a-quote, solid-as-an-inference.)
- Positioning consequence: the "your DIY stack has cross-store consistency races" argument does **not** apply to an all-in-one-SQLite-file assembly; the honest attack surface is instead (a) brute-force/alpha-ANN vector scale, (b) row-at-a-time deep traversal, (c) hand-rolled bitemporal correctness with a documented stable-transaction-time defect.

## Bottom line for positioning
A developer today can assemble: sqlite-vec (stable = exact scan, fine to ~100k vectors at 768-1024 dims; ANN is a March-May 2026 alpha) + recursive CTEs (fine to ~tens of thousands of nodes / depth ≤4-5) + trigger-based history (single-axis only; true bitemporal has no off-the-shelf SQLite implementation and a documented timestamp-stability hazard) — all with genuine single-snapshot consistency. The defensible differentiation claims are scale/latency under combined load and correct-by-construction bitemporality, not consistency.

## Sources
- https://github.com/asg017/sqlite-vec/releases · https://github.com/asg017/sqlite-vec/releases/tag/v0.1.10-alpha.1 · https://github.com/asg017/sqlite-vec/issues/25 · https://github.com/asg017/sqlite-vec
- https://alexgarcia.xyz/blog/2024/sqlite-vec-stable-release/index.html · https://alexgarcia.xyz/blog/2024/sqlite-vec-metadata-release/index.html · https://github.com/asg017/sqlite-vss
- https://turso.tech/blog/turso-brings-native-vector-search-to-sqlite · https://docs.turso.tech/features/ai-and-embeddings · https://github.com/tursodatabase/turso/issues/832 · https://github.com/tursodatabase/turso/issues/3778
- https://dev.to/rohansx/sqlite-as-a-graph-database-recursive-ctes-semantic-search-and-why-we-ditched-neo4j-1ai · https://github.com/shwetarkadam/sqlite-graph · https://algoroq.io/compare-tech/neo4j-vs-postgresql/ · https://sqlite.org/lang_with.html
- https://www.bytefish.de/blog/sqlite_logging_changes.html · https://www.ohnekontur.de/2024/02/19/unlocking-time-harnessing-the-power-of-temporal-tables-in-sqlite/ · https://simonwillison.net/2023/Apr/15/sqlite-history/ · https://github.com/thatdevsherry/historia · https://lord.technology/2025/01/28/understanding-temporal-primary-keys.html · https://v1-docs.xtdb.com/concepts/bitemporality/
- https://sqlite.org/isolation.html