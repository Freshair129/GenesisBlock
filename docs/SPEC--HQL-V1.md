---
proposed_id: SPEC--HQL-V1
type: spec
status: current
tier: process
cluster: implementation_flow
role: "HQL v1 language specification — the query language exactly as shipped on main (2026-07-03), defects documented as-is"
date: 2026-07-03
related:
  - adr/ADR--GENESISDB-HQL-FILTER-PROJECTION
  - adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS
  - PLAN--HQL-REFINEMENT
  - SPEC--HQL-V2
  - AUDIT--HQL-FUZZ
---

# SPEC — HQL v1 (current, as-shipped)

**This document specifies HQL exactly as it behaves on `main` as of 2026-07-03** (post PR #60). It is descriptive, not aspirational: known defects are specified as current behavior and cross-referenced to the refinement plan that fixes them. The target spec after refinement is [SPEC--HQL-V2](SPEC--HQL-V2.md).

**Source of truth:** grammar `src/query/hql.pest` (loaded by `pest_derive` via `src/query/ast.rs`), executor `Storage::execute_hql` (`src/lib.rs:3326`), clause transforms `apply_hql_clauses` (`src/lib.rs:2927`) and `match_pattern` (`src/lib.rs:3037`). Where this document and the code disagree, the code wins and this document has a bug.

---

## 1. Overview

HQL (Hybrid Query Language) is GenesisBlockDB's text query language: **five fixed-shape commands that dispatch directly to one Storage method each, then optionally post-process the result list**. There is no logical plan, no cost-based planner, no join reordering — this is a deliberate, ADR-committed invariant (the old `LogicalPlanner` was removed in MARK XIV).

| Command | Dispatches to | Returns |
|---|---|---|
| `SEARCH` | `hybrid_search` (α = 0.0) | node list |
| `TRAVERSE` | `neighbors` | node list |
| `MATCH <target> SIMILAR TO …` (hybrid) | `hybrid_search` (α = user) | node list |
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
- `identifier = (ASCII_ALPHANUMERIC | "_" | "-")+`. **Colons are not identifier characters**: ids like `user:5` MUST be written as string literals (`"user:5"`). *(Known ergonomics gap — the filter ADR's own `TRAVERSE FROM user:5` example does not parse. Refinement: PLAN--HQL-REFINEMENT P0-T0/T9.)*
- `string_lit` = double-quoted, no escape sequences (`(!"\"" ~ ANY)*` — a literal `"` cannot appear inside).
- `number` = optional sign, digits, optional decimal part. Digit-only rules (`k`, `depth`, `budget`, `limit_n`) are **atomic** — the fix for the historical "DEPTH always parses to 1" whitespace bug (see ADR--GENESISDB-HQL-FILTER-PROJECTION).
- Whitespace: space/tab/CR/LF between tokens.

---

## 2. Commands

### 2.1 SEARCH — pure vector search

```
SEARCH [~]<target> SIMILAR TO [ <n>(,<n>)* ] K <k>
       [IN <collection>] [LANGUAGE "<lang>"] [AS OF "<ts>"] [<clauses>]
```

- Executes `hybrid_search` with `alpha = 0.0` (pure vector similarity; no K-Impact blend).
- `K <k>` is **required** and sets the retrieval pool size.
- **The target is decorative.** It is resolved — including the expensive `~` fuzzy path (§4.1) — into a variable that is **never used**; the search runs on the literal vector alone (`src/lib.rs:3343-3347`). *(Defect. Refinement: P0-T1 makes the target meaningful — search-by-node.)*
- `IN <collection>` scopes the search to a named vector collection and validates query dimension against it; default = `default` collection.
- `LANGUAGE "<lang>"` adds the named language centroid to the query vector before search (if that centroid exists).
- `AS OF "<ts>"` applies bitemporal validity filtering (§4.2).
- Knobs **not** expressible: `ef_search`, `oversample` — both are hardwired to `None` (`src/lib.rs:3355-3356`), so HQL cannot use the per-query recall levers that exist on `HybridSearchInput`. *(Gap. Refinement: P0-T3.)*

### 2.2 TRAVERSE — k-hop graph traversal

```
TRAVERSE FROM [~]<seed> DEPTH <d> REL <rel | INFER(<rel>)>
         [AS OF "<ts>"] [<clauses>]
```

- Executes `neighbors` (BFS over adjacency indices), **outgoing direction only** — `direction` is hardcoded `"out"` (`src/lib.rs:3383`) although the engine supports `in`/`both`. *(Gap. Refinement: P0-T4.)*
- Exactly **one** rel type; `INFER(<rel>)` requests inferred-relationship traversal. The engine's multi-rel filter (`rels`) is not reachable. *(Gap. Refinement: P0-T4.)*
- No retrieval limit is passed (`limit: None`) — `LIMIT` in the clauses truncates only **after** the full BFS materializes. `DEPTH` is the only retrieval bound.
- `~` fuzzy seed **is** honored (unlike SEARCH): the resolved id seeds the traversal.
- BFS semantics: node-deduplicated (`visited` set — each node is reported once, via its first-discovered path), results carry the full path.

### 2.3 MATCH … SIMILAR TO — hybrid search

```
MATCH [~]<target> SIMILAR TO [ <vector> ] ALPHA <a>
      [IN <collection>] [LANGUAGE "<lang>"] [AS OF "<ts>"] [<clauses>]
```

- Executes `hybrid_search` with the user's `ALPHA` = blend weight for the K-Impact graph signal (engine default α = 0.0 per ADR--GENESISDB-KIMPACT-AS-SIGNAL; the signal is opt-in).
- **`k` is hardcoded to 10** (`src/lib.rs:3409`) — there is no `K` clause on this form, so the candidate pool is always 10 and `LIMIT` can only shrink it. *(Defect. Refinement: P0-T2.)*
- Target handling has the same discard defect as SEARCH.
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

### 4.5 Error model & numeric fallbacks

- Unparseable queries return `Err("HQL Parse Error: <pest error with position>")` — a string error over every surface (REST maps to an error response; NAPI rejects the promise). **Zero panics** across a 5,000+ input fuzz corpus (AUDIT--HQL-FUZZ, 34/34).
- **Silent numeric fallbacks (specified as current behavior, scheduled for removal):** an in-grammar numeric token that fails Rust-side parsing (overflow) silently becomes a default — `K` → 5, `DEPTH` → 1, `ALPHA` → 0.5, `BUDGET` → 32000, vector component → 0.0. Only `LIMIT`'s saturate is intended. *(Defect class. Refinement: P0-T5 turns these into parse errors.)*

---

## 5. Known-defect register (v1, all scheduled)

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

## 6. Verification status (v1)

- **Tests:** `tests/hql.rs`, `hql_collection_tests.rs`, `hql_filter_tests.rs` (27), `hql_cypher_tests.rs` (13), `hql_fuzz_tests.rs` (34/34, ~5k inputs, zero panics). All run under `cargo test --no-default-features` (Linux-CI linkable).
- **Bench:** `benches/hql_query_stress.rs` (`cargo run --release --features bins --bin hql-query-stress`) — covers the node-list commands; **no pattern-match rows yet** (added by P1-T6).
- **Perf context:** HQL adds parse + dispatch + post-transform over the underlying Storage call; the Storage-side baselines are AUDIT--P31 (hop1 p50 22.6 µs, hop3 2,529 µs, hop6 4,902 µs @100k/800k fanout-8, C: SSD).
