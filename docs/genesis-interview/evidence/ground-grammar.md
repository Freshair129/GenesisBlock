# HQL Grounding Report — WORKING TREE state (2026-07-06)

## 0. Working-tree status (uncommitted changes present)

`git diff --stat` shows uncommitted modifications on `main`:
- `src/lib.rs` (+133/-…), `src/query/ast.rs` (245 lines changed), `src/query/hql.pest` (+16), `tests/hql.rs` (+89), `tests/hql_filter_tests.rs` (+77), `tests/hql_fuzz_tests.rs` (2), plus doc tweaks (`docs/PLAN--HQL-REFINEMENT.md`, `docs/SPEC--HQL-V2.md`).

The working tree contains the **P0 fixes already applied** (verified below): SEARCH target no longer discarded, hybrid K no longer hardcoded, EF/OVERSAMPLE now reachable, plus **new** TRAVERSE `DIRECTION` and multi-rel `REL a|b` union, and SEARCH/MATCH `SIMILAR TO` made optional.

Grammar diff vs HEAD (`git diff src/query/hql.pest`):
- `rel_type` = single identifier → `rel_name ~ ("|" ~ rel_name)*` (rel union)
- new rules: `ef`, `oversample`, `ef_spec`, `oversample_spec`, `direction`, `direction_spec`, `similar_clause`
- `search`: `SIMILAR TO [vec]` mandatory → optional `similar_clause?`; adds `ef_spec? oversample_spec?`
- `traverse`: adds `direction_spec?`
- `hybrid`: `SIMILAR TO` optional; adds optional `(K k)?`, `ef_spec?`, `oversample_spec?`

---

## 1. Grammar — every rule and expressible surface (`src/query/hql.pest`, 94 lines)

### Lexical / shared rules
| Rule | Line | Definition |
|---|---|---|
| `WHITESPACE` | hql.pest:1 | space/tab/CR/LF (implicit between tokens of non-atomic rules) |
| `identifier` | :2 | `(ALNUM | "_" | "-")+` |
| `string_lit` | :3 | `"..."` — **no escape sequences**; any char except `"` |
| `fuzzy_prefix` | :4 | `~` (fuzzy id resolution) |
| `target` / `seed` | :5-6 | `~?` + identifier or string literal |
| `rel_name` / `rel_type` | :8-9 | **NEW**: `rel_name ("|" rel_name)*` — pipe-union of rel types |
| `infer_rel` | :10 | `INFER(<ident>)` — inferred/transitive relation |
| `rel` | :11 | `infer_rel | rel_type` |
| `number` | :13 | signed decimal |
| `vector` | :14 | comma-separated numbers |
| `k`, `depth`, `ef`, `oversample`, `budget`, `limit_n` | :19-23, :86, :43 | atomic digit runs (atomicity documented as the fix for the old "DEPTH always 1" whitespace bug, :15-18) |
| `alpha` | :21 | number |
| `lang_spec` | :24 | `LANGUAGE "<s>"` |
| `as_of` | :25 | `AS OF "<timestamp-string>"` |
| `collection_spec` | :26 | `IN <ident|"str">` |
| `ef_spec` / `oversample_spec` | :27-28 | **NEW** `EF <n>` / `OVERSAMPLE <n>` |
| `direction` / `direction_spec` | :29-30 | **NEW** `DIRECTION in|out|both` |

### Post-process clauses (shared by SEARCH/TRAVERSE/MATCH-hybrid) — hql.pest:32-48
- `prop_field` (:35) = `prop.<key>`; `field` (:36) = `prop.<k> | id | label | score | depth`
- `op` (:37) = `<= >= != = < > CONTAINS STARTSWITH`
- `predicate` (:39) = `field op (string|number)`
- `where_clause` (:40) = `WHERE pred (AND pred)*` — **AND only, no OR, no NOT, no parens**
- `order_clause` (:42) = `ORDER BY field [ASC|DESC]` — single key
- `limit_clause` (:44) = `LIMIT n`
- `return_clause` (:46) = `RETURN * | field,...`
- `clauses` (:48) = all four optional, **fixed order** WHERE → ORDER BY → LIMIT → RETURN

### Commands (hql.pest:50-93)
1. **SEARCH** (:51): `SEARCH target [SIMILAR TO [vec]] K k [EF n] [OVERSAMPLE n] [IN coll] [LANGUAGE "x"] [AS OF "t"] clauses` — `similar_clause` now **optional** (:50-51).
2. **TRAVERSE** (:52): `TRAVERSE FROM seed DEPTH d REL rel[|rel...] [DIRECTION in|out|both] [AS OF "t"] clauses`.
3. **MATCH/HYBRID** (:53): `MATCH target [SIMILAR TO [vec]] ALPHA a [K k] [EF n] [OVERSAMPLE n] [IN coll] [LANGUAGE] [AS OF] clauses` — `K` now grammatically expressible (optional).
4. **MATCH pattern** (Cypher subset, PR #60) (:55-84):
   - `node_pattern` (:63) = `( var? :Label? {k:v,...}? )` — one label max, inline props are exact-equality; `{id:"..."}` addresses top-level id.
   - `rel_detail` (:67) = `[ var? :Type? ]` — one type max, **no `|` union in patterns**, no props inside `[...]`.
   - directions: `<-[..]-` / `-[..]->` / `-[..]-` (:68-71); `hop` = edge+node (:72); `graph_pattern` = anchor + `hop*` (:73) — **linear path only, fixed length, no `*min..max` var-length**.
   - `pat_clauses` (:83) = `pat_where? pat_order? limit_clause? pat_return?` with variable-qualified fields `a`, `a.id`, `a.label`, `a.prop.<k>` (:77-82). Note `qual_tail` (:77) has **no score/depth**.
   - `match_pattern` (:84) supports `AS OF`. Grammar-ordering note: `match_pattern` must precede `hybrid` in `query` (:90-93).
5. **CONTEXT** (:88): `CONTEXT FOR target TIER H0..H5 [BUDGET n]` — **no clauses, no AS OF, no collection** (test `context_rejects_trailing_clauses`, tests/hql_filter_tests.rs:542).

`query` (:93) = `SOI (search | traverse | match_pattern | hybrid | context) EOI` — exactly one command per query; **no composition, no chaining, no subqueries**.

---

## 2. Full AST (`src/query/ast.rs`)

`HqlCommand` enum (ast.rs:161-209), 5 variants:

- **`Search`** (ast.rs:163-174): `target: String`, `vector: Option<Vec<f64>>` (**now Option** — was mandatory), `k: u32` (default 5, ast.rs:448), `ef_search: Option<u32>`, `oversample: Option<u32>`, `fuzzy: bool`, `lang: Option<String>`, `as_of: Option<String>`, `collection: Option<String>`, `clauses: HqlClauses`.
- **`Traverse`** (ast.rs:175-184): `seed`, `depth: u32` (default 1, ast.rs:508), `rel: HqlRel` (Physical/Inferred, ast.rs:10-13; default `Physical("ANY")` ast.rs:509), `rels: Option<Vec<String>>` (**new** — set when `|` union has >1 name, ast.rs:536-538), `direction: Option<String>` (**new**, ast.rs:551-557), `fuzzy`, `as_of`, `clauses`.
- **`Hybrid`** (ast.rs:185-197): `target`, `vector: Option<Vec<f64>>`, `alpha: f64` (default 0.5, ast.rs:579), `k: u32` (**default 10 at parse layer**, ast.rs:580 — now overridable via grammar), `ef_search`, `oversample`, `fuzzy`, `lang`, `as_of`, `collection`, `clauses`.
- **`Context`** (ast.rs:198-203): `target`, `tier: String` (default "H1"), `budget: Option<u32>`, `fuzzy`.
- **`MatchPattern`** (ast.rs:204-208): `pattern: GraphPattern`, `as_of: Option<String>`, `clauses: PatternClauses`.

Support types:
- `HqlField` = Id | Label | Score | Depth | Prop(String) (ast.rs:18-24); `HqlOp` = Eq/Ne/Lt/Le/Gt/Ge/Contains/StartsWith (ast.rs:41-50); `HqlValue` = Str | Num(f64) (ast.rs:53-56).
- `HqlClauses` (ast.rs:77-82): `where_preds: Vec<HqlPredicate>`, `order_by: Option<(HqlField, bool)>`, `limit: Option<usize>` (overflow saturates to usize::MAX, ast.rs:399), `ret: Option<HqlReturn{All|Fields}>`.
- Pattern types: `PatternDirection` Out/In/Both (ast.rs:88-92); `NodePattern{var,label,props}` (ast.rs:97-101); `EdgePattern{var, rel_type: Option<String>, direction}` (ast.rs:105-109); `GraphPattern{start, hops: Vec<(EdgePattern,NodePattern)>}` (ast.rs:113-116); `QualField{var, field: Option<HqlField>}` (ast.rs:121-124); `PatternClauses` (ast.rs:153-159) mirrors HqlClauses with QualField.

Parse entry: `TryFrom<&str> for HqlCommand` (ast.rs:211-241). Numeric parsing is strict (`parse_f64` rejects non-finite, ast.rs:244-254; `parse_u32` errors on overflow, ast.rs:256-260). Unknown `field` text falls back to `HqlField::Prop(text)` (ast.rs:366) — the grammar makes this unreachable in practice but the code path exists.

---

## 3. Dispatch — `Storage::execute_hql` (src/lib.rs:3354-3500)

Entry: parse (lib.rs:3398), match on `HqlCommand`. Async wrapper: `GenesisDatabase::execute_hql` at lib.rs:5827 (spawn_blocking over the same core fn).

Helper closures (lib.rs:3359-3397):
- `resolved_target_id` — fuzzy targets resolve via `find_fuzzy_id` (trigram index, lib.rs:2336) or error.
- `hql_query_vector` — **new**: if a literal `SIMILAR TO` vector exists, return it (target is then ignored); otherwise resolve `target` to a node and use `reconstruct_embedding` of that node as the query vector (lib.rs:3370-3397). Errors if target has no node / no embedding.

### SEARCH (lib.rs:3400-3424)
→ `self.hybrid_search(HybridSearchInput{...})` with:
- `query_vector` = literal vector OR target's stored embedding — **target-discarded defect is FIXED** (target used as vector source when no literal vector; still ignored when a literal vector is given).
- `k` = parsed K (passed through).
- **`alpha: Some(0.0)` hardcoded** (lib.rs:3416) — SEARCH is pure vector similarity; the K-Impact blend is forced off (consistent with ADR--GENESISDB-KIMPACT-AS-SIGNAL).
- `ef_search`, `oversample` passed through — **the "parsed-but-unreachable" defect is FIXED** (lib.rs:3420-3421); consumed in `hybrid_search` at lib.rs:3549-3561 (per-query → per-collection → engine-global fallback for ef; oversample → RERANK_OVERFETCH default, only effective when an f32 sidecar exists).
- `lang`, `as_of`, `collection` passed through. Then `apply_hql_clauses` (lib.rs:3423).
- Note: **`lang` adds a language centroid to the query vector** (lib.rs:3529-3537) — it is a vector-space shift, not a filter.

### TRAVERSE (lib.rs:3425-3458)
→ `self.neighbors(seed, NeighborInput{...}, is_inferred)`:
- fuzzy seed resolved (falls back to the raw seed on miss, lib.rs:3435-3439 — silent, unlike SEARCH which errors).
- `depth`, `rel` (first name), `rels` (full union — overrides `rel` when non-empty, lib.rs:3694-3704), `as_of` passed.
- `direction`: parsed value or default `"out"` (lib.rs:3450) — **DIRECTION defect fixed/new**.
- **`include_invalid: Some(false)` hardcoded** (lib.rs:3452) — HQL cannot request retracted edges; only REST/NAPI `neighbors` can.
- **`limit: None` hardcoded** (lib.rs:3453) — HQL `LIMIT` is post-process truncation in `apply_hql_clauses`, not pushed into the BFS early-exit that `NeighborInput.limit` provides (lib.rs:3805-3809). Full traversal materializes before LIMIT.
- `INFER(rel)` sets `is_inferred=true`, which makes the BFS ignore the depth bound (`curr_depth >= depth && !is_inferred`, lib.rs:3732) — effectively transitive closure.

### MATCH/HYBRID (lib.rs:3459-3484)
→ `self.hybrid_search(...)` with `alpha: Some(alpha)` (caller-set blend of vector similarity vs node `impact`), `k` = parsed (grammar `K` optional, AST default 10) — **the K=10 hardcode is FIXED at dispatch** (old `k: 10` literal removed per `git diff`; now `k` field passes through). `ef_search`/`oversample` wired same as SEARCH. Then `apply_hql_clauses`.

### CONTEXT (lib.rs:3485-3493)
→ `self.retrieve_context(&target, &tier, budget, fuzzy)` (lib.rs:4257) — GRL tiered BFS (tier→hops) returning a `ContextPackage`. No clauses applied.

### MATCH pattern (lib.rs:3494-3498)
→ `self.match_pattern(&pattern, &as_of, &clauses)` (lib.rs:3037-3222):
- Anchor: `{id:"..."}` on the anchor node = O(1) interned-id fast path (lib.rs:3054-3076); **otherwise a full scan of `self.nodes`** (lib.rs:3078-3091) — no label index, `:Label` is a per-node check inside the scan (`node_matches`, lib.rs:2994-3024).
- Hop expansion: left-to-right frontier expansion over `out_idx`/`in_idx` (lib.rs:3095-3168); per-hop checks: edge as-of validity + retraction hiding in current view (lib.rs:3120-3130), `rel_type` exact match (lib.rs:3131-3135), far-node as-of + `node_matches` (lib.rs:3151-3156). Bound vars (node and edge) serialize the full entity into the row.
- **No cycle guard / visited set** in pattern expansion (unlike `neighbors`) — a `Both` hop can walk back to the previous node.
- Post-process: WHERE (conjunction, lib.rs:3173-3180) → ORDER BY nulls-last (3181-3199) → LIMIT (3200-3202) → RETURN projection (3204-3221). Predicate eval: `pattern_eval_predicate` (3256-3281) — node `.label` uses membership semantics over the labels array (Eq/Ne only); edge `.label` maps to `rel` string (`pattern_resolve`, 3237-3246); `.prop.<k>` works on **both nodes and edges** (both entities have `props`; EdgeOutput at lib.rs:156-169).

### Clause post-processor (`apply_hql_clauses`, lib.rs:2927-2989)
Order: WHERE (all-AND, `hql_eval_predicate` lib.rs:2877-2911; SQL-style: missing/null/type-mismatch = false for every op including `!=`) → ORDER BY (nulls last both directions, lib.rs:2946-2965) → LIMIT truncate → RETURN projection (`hql_field_value`, lib.rs:2827-2846: `label` projects the whole labels array; `score` is None on TRAVERSE rows). Pure post-processing on the already-materialized `Vec<NeighborOutput>` — WHERE is **not pushed down** into HNSW search or BFS (a `SEARCH ... K 10 WHERE ...` filters *after* top-K, so filters reduce, never widen, the result set — classic post-filter recall loss vs Qdrant-style pre-filter).

### `hybrid_search` internals relevant to HQL (lib.rs:3516-3673)
Dim validated against collection (3520-3527); lang centroid add (3529); cosine normalize (3539); ef resolution chain (3549-3553); oversample only matters with f32 sidecar (3554-3567); exact-brute-force fallback when overfetch ≥ collection size (3576-3596); sidecar rerank (3603-3618); score = `similarity*(1-alpha) + impact*alpha` (3641-3643); NaN-safe sort, dedupe by node id, truncate to k (3657-3672). `as_of` filters nodes by valid-time (3633-3639).

---

## 4. What is NOT expressible today

- **No OR / NOT / parenthesized boolean expressions** — WHERE is a flat AND-conjunction (hql.pest:40, :80). No BETWEEN, no IN-list, no regex, no ENDSWITH, no null-test (`IS NULL`).
- **No var-length paths** — `graph_pattern` hops are fixed count (hql.pest:72-73); no `-[:R*1..3]->`. TRAVERSE DEPTH n is the only multi-hop tool, and it returns nodes-with-paths, not pattern bindings.
- **No branching/tree patterns, no multiple MATCH clauses, no OPTIONAL MATCH** — a single linear chain only.
- **No property predicates inside edge patterns** — `rel_detail` (hql.pest:67) takes only var + one `:Type`; edge props are reachable only post-hoc via a bound var in `pat_where` (`e.prop.k`).
- **No rel-type union in patterns** — `[:A|B]` not parseable (union exists only in TRAVERSE `REL a|b`).
- **No fusion / RANK BY** — no RRF, no multi-signal weights. The only fusion is the fixed linear `alpha` blend of vector-similarity + node impact inside `hybrid_search` (lib.rs:3643); nothing caller-parameterized beyond that scalar. No `recency`, `hops`, `epistemic` signals.
- **No text/lexical query** — SEARCH's `target` is an id (used to fetch an embedding), never a full-text/BM25/trigram query; the fuzzy `~` prefix only fuzzy-resolves an id via `find_fuzzy_id`. (This is the P3 text-query ADR gate in PLAN--HQL-REFINEMENT.)
- **No vector search inside pattern MATCH, no graph hops inside SEARCH** — the cross-dimension combination (G3 target) does not exist as one query; you cannot chain SEARCH → TRAVERSE. Each HQL query = exactly one command (hql.pest:93).
- **No LIMIT pushdown / no pre-filtering** — WHERE/LIMIT run after top-K / full BFS materialization (lib.rs:2927 doc: "Pure post-processing — no planner").
- **No `include_invalid` in HQL** — hardcoded false for TRAVERSE (lib.rs:3452); patterns hide retracted edges in the current view unconditionally (lib.rs:3124-3130). Bitemporal access from HQL = `AS OF` only (valid-time; there is no tx-time `recorded_at` query surface in HQL).
- **No AS OF on CONTEXT**; no clauses on CONTEXT (hql.pest:88).
- **No collection spec on TRAVERSE / pattern MATCH / CONTEXT** (vector collections are irrelevant to pure graph ops, but noteworthy for a future hybrid op).
- **No ORDER BY multiple keys**; no aggregation (count/collect), no DISTINCT, no SKIP/OFFSET.
- **No escape sequences in string literals** (hql.pest:3) — an id/prop value containing `"` is unqueryable.
- **No parameterized queries** — values are inline literals only.

## 5. Dead / unwired grammar & AST

- **`HqlField::Score` / `Depth` in pattern context**: `QualField::output_key` handles Score/Depth (ast.rs:133-134) and `pattern_resolve` explicitly nulls them (lib.rs:3252), but the pattern grammar's `qual_tail` (hql.pest:77) only admits `prop_field | id | label` — Score/Depth are grammatically unreachable there (defensive dead arms).
- **`Context.fuzzy`**: wired (target `~` prefix parsed ast.rs:648-652, passed lib.rs:3491) — not dead, but budget semantics live entirely in `retrieve_context`.
- **`NeighborInput.limit` early-exit** (lib.rs:3721, 3805-3809) exists in the engine but HQL never uses it (`limit: None`, lib.rs:3453) — an available pushdown left on the table.
- **`ast.rs:366` fallback** (`other => HqlField::Prop(other)`) — unreachable given the grammar's closed `field` rule; same for `op` fallbacks to `Eq` (ast.rs:325, :679).
- **The former dead items are now live**: `ef_spec`/`oversample_spec` and hybrid `K` are parsed AND consumed (verified above) — the three known P0 suspects are all fixed in the working tree.
- `get_ranked_context` (lib.rs:3675-3679) hardcodes `alpha=0.4` — not reachable from HQL (NAPI/REST-only path).

## 6. Behavior pinned by tests (headers/names)

- `tests/hql.rs` (header lines 1-4): TRAVERSE/CONTEXT via execute_hql — quoting, Unicode/Thai labels, depth 0/1/bounds, parse errors; **new working-tree tests**: `hql_search_without_literal_vector_uses_target_embedding` (:313), `hql_search_without_vector_errors_for_missing_embedding` (:363), `hql_traverse_direction_and_multi_rel_are_exposed` (:373).
- `tests/hql_filter_tests.rs` (header): ADR--GENESISDB-HQL-FILTER-PROJECTION semantics — SQL-null WHERE, nulls-last ORDER BY, RETURN * equivalence, label membership, LIMIT overflow saturation, `context_rejects_trailing_clauses` (:542), **new**: `hybrid_and_search_parse_new_exposed_knobs` (:625), `traverse_parse_direction_and_rel_union` (:656).
- `tests/hql_fuzz_tests.rs`: parser never-panics over random bytes/unicode/mutations/boundary values; round-trip parse of every command variant. Working-tree diff only adapts `vector` to `Option` (`Some(vec![...])`, diff at :381-384).

## 7. Summary table — command → engine method

| HQL command | Storage method | Hardcoded at dispatch | Passed through |
|---|---|---|---|
| SEARCH | `hybrid_search` (lib.rs:3413) | `alpha=Some(0.0)` (:3416) | vector-or-target-embedding, k, ef, oversample, lang, as_of, collection |
| TRAVERSE | `neighbors` (:3444) | `include_invalid=Some(false)` (:3452), `limit=None` (:3453), direction default "out" (:3450) | seed(fuzzy), depth, rel/rels, direction, as_of |
| MATCH…ALPHA (hybrid) | `hybrid_search` (:3473) | — (k default 10 comes from AST, ast.rs:580, overridable) | vector-or-embedding, alpha, k, ef, oversample, lang, as_of, collection |
| MATCH (pattern) | `match_pattern` (:3498) | retracted-edge hiding always on (:3124) | pattern, as_of, pat_clauses |
| CONTEXT | `retrieve_context` (:3491) | — | target(fuzzy), tier, budget |

All commands are single-shot; clauses are post-process only; the only cross-signal ranking primitive in the engine today is the scalar `alpha` impact blend inside `hybrid_search` (lib.rs:3641-3643).