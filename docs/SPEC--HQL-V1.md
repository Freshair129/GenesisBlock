---
proposed_id: SPEC--HQL-V1
type: spec
status: current
tier: process
cluster: implementation_flow
role: "HQL v1 command contract with historical audit appendix"
date: 2026-11-02
related:
  - adr/ADR--GENESISDB-HQL-FILTER-PROJECTION
  - adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS
  - PLAN--HQL-REFINEMENT
  - SPEC--HQL-V2
  - AUDIT--HQL-FUZZ
---

# SPEC — HQL v1 (current, as-shipped)

The command contract below includes the P0 grammar refinements and the
[K-Impact retirement](adr/ADR--GENESISDB-RETIRE-KIMPACT.md). The July 2026
defect register and measurements at the end are historical evidence.

**Source of truth:** grammar `src/query/hql.pest`, parser `src/query/ast.rs`,
and executor `Storage::execute_hql` in `src/lib.rs`, with the HQL integration
and fuzz tests. Source line numbers vary as the engine evolves.

---

## 1. Overview

HQL (Hybrid Query Language) is GenesisBlockDB's text query language: **five fixed-shape commands that dispatch directly to one Storage method each, then optionally post-process the result list**. There is no logical plan, no cost-based planner, no join reordering — this is a deliberate, ADR-committed invariant (the old `LogicalPlanner` was removed in MARK XIV).

| Command | Dispatches to | Returns |
|---|---|---|
| `SEARCH` | `hybrid_search` (vector relevance) | node list |
| `TRAVERSE` | `neighbors` | node list |
| `MATCH <target> [SIMILAR TO …]` | `hybrid_search` (vector relevance) | node list |
| `MATCH (pattern)` (Cypher-style) | `match_pattern` | binding rows |
| `CONTEXT` | `retrieve_context` (GRL) | `ContextPackage` |

### 1.1 Surfaces (one funnel)

All three client surfaces call the same `execute_hql(&str) -> serde_json::Value`:

- **NAPI:** `executeHql(query: string): Promise<any>` (offloaded via `spawn_blocking`).
- **REST:** `POST /v1/query/hql` — accepts **both** a raw JSON string body (`"SEARCH …"`) and an object (`{"query": "SEARCH …"}`) via the `#[serde(untagged)] HqlBody` enum in `src/router.rs`.
- **MCP:** `query_hql` tool (`mcp/server.js`).

Grammar changes therefore land on all three surfaces simultaneously; only doc text (index.d.ts docstring, MCP tool description) can drift.

### 1.2 Lexical rules

- Keywords are **case-insensitive** (`^"…"` pest rules); `search`, `Search`, `SEARCH` are equivalent.
- Identifiers contain ASCII alphanumerics, `_` and `-`. Colon-qualified IDs such
  as `user:5` are accepted in command target/seed positions; pattern labels
  retain their existing colon grammar. Quoted string IDs are also accepted.
- `string_lit` = double-quoted, no escape sequences (`(!"\"" ~ ANY)*` — a literal `"` cannot appear inside).
- `number` = optional sign, digits, optional decimal part. Digit-only rules (`k`, `depth`, `budget`, `limit_n`) are **atomic** — the fix for the historical "DEPTH always parses to 1" whitespace bug (see ADR--GENESISDB-HQL-FILTER-PROJECTION).
- Whitespace: space/tab/CR/LF between tokens.

---

## 2. Commands

### 2.1 SEARCH — pure vector search

```
SEARCH [~]<target> [SIMILAR TO [ <n>(,<n>)* ]] K <k> [EF <n>] [OVERSAMPLE <n>]
       [IN <collection>] [LANGUAGE "<lang>"] [AS OF "<ts>"] [<clauses>]
```

- Executes vector retrieval through `hybrid_search`. Retired importance options are rejected.
- `K <k>` is **required** and sets the retrieval pool size.
- An explicit vector supplies the query. When it is omitted, the target node's
  stored embedding supplies it; fuzzy target resolution applies to that lookup.
- `IN <collection>` scopes the search to a named vector collection and validates query dimension against it; default = `default` collection.
- `LANGUAGE "<lang>"` adds the named language centroid to the query vector before search (if that centroid exists).
- `AS OF "<ts>"` applies bitemporal validity filtering (§4.2).
- `EF` and `OVERSAMPLE` expose the per-query HNSW and rerank controls.

### 2.2 TRAVERSE — k-hop graph traversal

```
TRAVERSE FROM [~]<seed> DEPTH <d> REL <rel | INFER(<rel>)>
         [DIRECTION <out|in|both>] [AS OF "<ts>"] [<clauses>]
```

- Executes `neighbors` (BFS over adjacency indices); direction defaults to `out`
  and may be explicitly `in` or `both`.
- Relation alternation such as `LINK|REF` and `ANY` are supported;
  `INFER(<rel>)` requests inferred-relationship traversal.
- No retrieval limit is passed (`limit: None`) — `LIMIT` in the clauses truncates only **after** the full BFS materializes. `DEPTH` is the only retrieval bound.
- A fuzzy seed resolves the traversal's starting ID.
- BFS semantics: node-deduplicated (`visited` set — each node is reported once, via its first-discovered path), results carry the full path.

### 2.3 MATCH … SIMILAR TO — vector retrieval

```
MATCH [~]<target> [SIMILAR TO [ <vector> ]] [K <k>] [EF <n>] [OVERSAMPLE <n>]
      [IN <collection>] [LANGUAGE "<lang>"] [AS OF "<ts>"] [<clauses>]
```

- Executes vector retrieval. `ALPHA` is no longer part of the grammar.
- `K` controls the candidate count; omitting it uses the existing default of 10.
- Target/vector selection follows SEARCH. No importance score participates.
- Keyword collision note: `MATCH (` routes to the pattern command (§2.4); `MATCH <ident|string> SIMILAR` routes here. The grammar orders `match_pattern` before `hybrid`; PEG backtracking makes this unambiguous.

### 2.4 MATCH (pattern) — Cypher-style linear path matching

```
MATCH (a:Label {k:v})-[r:REL]->(b) … [AS OF "<ts>"]
      [WHERE <var.field> <op> <value> (AND …)*]
      [ORDER BY <var.field> [ASC|DESC]] [LIMIT <n>]
      [RETURN * | <var|var.field>, …]
```

- **Node pattern** `(var? :Label? {k:v,…}?)` — all parts optional; `()` matches any node. Inline `{k:v}` props are exact-equality; the key `id` addresses the node's top-level id (not `props`).
- **Edge pattern** `-[var? :Type?]->` (out), `<-[…]-` (in), `-[…]-` (both); detail-free forms `-->`, `<--`, `--`.
- **Execution** (`match_pattern`, `src/lib.rs:3037`): anchor = full scan of all live nodes filtered by the first pattern's constraints; each hop expands every row through `out_idx`/`in_idx`, Cartesian over surviving neighbors; rows bind named variables to full node/edge JSON.
  - **Anchor is O(N) even for `{id:"…"}` anchors** — no direct-lookup fast path, no label index. *(Perf debt. Refinement: P0-T7, P2-T3.)*
  - **No frontier cap** — hub fan-out can grow intermediate rows without bound; `LIMIT` truncates only the final rows. *(Guardrail gap. Refinement: P1-T2.)*
  - Every bound variable is **eagerly serialized to JSON per row** during expansion, whether or not any clause references it. *(Perf debt. Refinement: P1-T3.)*
  - The retraction check recomputes `Utc::now().to_rfc3339()` **per edge candidate** (`src/lib.rs:3098`; `neighbors` shares the pattern at `:3696`). *(Perf debt. Refinement: P0-T6.)*
- **Variable semantics:** a repeated variable name binds **independently per position** — `(a)-->(b)-->(a)` does *not* require the two `a`s to be the same node. *(v1 non-goal. Refinement: P1-T4.)*
- **v1 scope limits (per ADR):** linear paths only — no variable-length hops (`*1..d`), no branching/comma patterns, no rel alternation (`:R1|R2`), no `OR`, no aggregation.
- **Qualified clause fields:** `a` (whole entity) | `a.id` | `a.label` | `a.prop.<key>`. On an **edge** variable, `.label` resolves to its `rel` string and `.prop.<key>` into edge props. `score`/`depth` resolve to `null` in pattern rows. Node `.label` predicates use membership semantics over the labels array (`=` contains / `!=` not-contains; other ops false).

### 2.5 CONTEXT — GRL tiered retrieval

```
CONTEXT FOR [~]<target> TIER <H0…H5> [BUDGET <n>]
```

- Executes `retrieve_context` (Graph Retrieval Layer): tier = context scaling level, `BUDGET` = token budget (default 32000 on overflow).
- Returns a `ContextPackage`, **not** a node list — the clause system does not apply. *(v1 non-goal; revisit is P2-T4, droppable.)*

---

## 3. The clause system (SEARCH / TRAVERSE / hybrid)

Optional trailing clauses, applied to the already-materialized result list **in this fixed order**: `WHERE → ORDER BY → LIMIT → RETURN`. The grammar accepts them only in that order.

- **WHERE** — conjunction only (`AND`); predicate = `<field> <op> <value>`.
  - `field` := `id` | `label` | `score` | `depth` | `prop.<key>`
  - `op` := `=` `!=` `<` `<=` `>` `>=` `CONTAINS` `STARTSWITH`
  - `value` := string literal | number
  - `label` uses membership over the node's labels array. `CONTAINS`/`STARTSWITH` are string-only. Comparison ops coerce numerically when both sides parse as numbers, else compare as strings.
  - **SQL-style null handling:** missing field / JSON `null` / type mismatch ⇒ predicate is **false for every operator, including `!=`**. Never errors.
- **ORDER BY** — one field, default `ASC`; **nulls sort last regardless of direction**.
- **LIMIT** — truncates after WHERE + ORDER BY. Distinct from `K`/`DEPTH`, which bound *retrieval*; a `LIMIT` larger than the pool is a no-op. Overflowed literals saturate to `usize::MAX` (deliberate: "absurdly large = no practical cap", never "clause dropped").
- **RETURN** — omitted or `*` keeps the full `NeighborOutput` shape; a field list reshapes each hit into a flat object keyed by leaf name (`prop.text` → `"text"`).

**WHERE is post-retrieval.** A selective filter can return fewer than `K` rows; callers must over-fetch (`K 100 WHERE … LIMIT 10`). Push-down does not exist in v1.

---

## 4. Cross-cutting semantics

### 4.1 Fuzzy targets (`~`)

`~<target>` resolves via `find_fuzzy_id` (`src/lib.rs:2336`): exact-id short-circuit → Thai-aware trigram index + jaro-winkler (threshold 0.85) → neural vector fallback. **Honored by TRAVERSE and CONTEXT; computed-then-discarded by SEARCH and hybrid** (§2.1 defect).

### 4.2 Bitemporal (`AS OF`)

`AS OF "<ts>"` (RFC3339 string compare) filters nodes and edges by `valid_from`/`valid_to`. Without `AS OF`, edges retracted in the past are hidden from the current view (mirroring `neighbors`); time-travel queries can see them.

### 4.3 Collections

`IN <collection>` routes vector commands to a named per-collection HNSW space with dimension validation; a dim mismatch is a hard error (never silent garbage ranking).

### 4.4 Return shapes

- Node-list commands: array of `NeighborOutput` `{node, path, depth, score}` — or flat projected objects under `RETURN`.
- Pattern command: array of binding rows `{<var>: <node|edge JSON>, …}` — or flat projected objects keyed `a`, `a.id`, `a.<propkey>`.
- CONTEXT: `ContextPackage` object.

### 4.5 Error model and numeric validation

- Unparseable queries return `Err("HQL Parse Error: <pest error with position>")` — a string error over every surface (REST maps to an error response; NAPI rejects the promise). **Zero panics** across a 5,000+ input fuzz corpus (AUDIT--HQL-FUZZ, 34/34).
- Numeric overflow in K, DEPTH, BUDGET and vector components returns a parse
  error instead of substituting a value. LIMIT retains its documented saturation.
- A retired ALPHA clause is rejected by the grammar.

---

## 5. Historical July 2026 defect register

This table records the original audit, not the current acceptance status.
The P0 command refinements above supersede its earlier syntax and defaults.

| # | Behavior specified above | Class | Fix |
|---|---|---|---|
| 1 | SEARCH/hybrid resolve then **discard** the target (fuzzy work wasted) | defect | P0-T1 |
| 2 | Hybrid pool hardcoded `k=10`, no `K` clause | defect | P0-T2 |
| 3 | `ef_search`/`oversample` unreachable from HQL | exposure gap | P0-T3 |
| 4 | TRAVERSE out-only, single-rel, no retrieval limit | exposure gap | P0-T4 |
| 5 | Silent numeric defaults on overflow | defect | P0-T5 |
| 6 | Per-edge `Utc::now()` in `match_pattern` + `neighbors` | perf debt | P0-T6 |
| 7 | `{id:…}` anchor scans all nodes | perf debt | P0-T7 |
| 8 | Colon ids must be quoted; ADR example broken | ergonomics | P0-T0/T9 |
| 9 | No var-length paths / frontier cap / identity join / alternation | scope (ADR v1 non-goals) | P1 |
| 10 | AND-only WHERE, no aggregation, no label index | scope | P2 |
| 11 | No text query without a caller-supplied vector (path 3) | scope | P3 (ADR first) |

## 6. Historical verification evidence (July 2026)

- **Tests:** `tests/hql.rs`, `hql_collection_tests.rs`, `hql_filter_tests.rs` (27), `hql_cypher_tests.rs` (13), `hql_fuzz_tests.rs` (34/34, ~5k inputs, zero panics). All run under `cargo test --no-default-features` (Linux-CI linkable).
- **Bench:** `benches/hql_query_stress.rs` (`cargo run --release --features bins --bin hql-query-stress`) — covers the node-list commands; **no pattern-match rows yet** (added by P1-T6).
- **Perf context:** HQL adds parse + dispatch + post-transform over the underlying Storage call; the Storage-side baselines are AUDIT--P31 (hop1 p50 22.6 µs, hop3 2,529 µs, hop6 4,902 µs @100k/800k fanout-8, C: SSD).
