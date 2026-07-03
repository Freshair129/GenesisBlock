---
proposed_id: DESIGN--HQL-P0-DECISIONS
type: design
status: Accepted
tier: process
cluster: implementation_flow
role: "P0-T0 design-gate decisions for PLAN--HQL-REFINEMENT: search-by-node target semantics, colon-id policy, strict-number policy"
date: 2026-07-03
related:
  - PLAN--HQL-REFINEMENT
  - SPEC--HQL-V2
  - adr/ADR--GENESISDB-HQL-FILTER-PROJECTION
  - adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS
---

# DESIGN — HQL P0 decisions (design gate for P0-T1/T2/T4/T5/T9)

**Status: Accepted.** Three decided designs. Cross-links: [ADR--GENESISDB-HQL-FILTER-PROJECTION](adr/ADR--GENESISDB-HQL-FILTER-PROJECTION.md) (path 2+4, clauses), [ADR--GENESISDB-HQL-CYPHER-PATTERNS](adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS.md) (path 1, pattern grammar). Confirms SPEC--HQL-V2 §2.1/§2.5/§2.6 with one deviation (noted in §2).

---

## 1. Search-by-node target semantics (P0-T1)

### 1a. Grammar — make the vector group optional

```pest
search = { ^"SEARCH" ~ target ~ (^"SIMILAR" ~ ^"TO" ~ "[" ~ vector ~ "]")? ~ ^"K" ~ k ~ collection_spec? ~ lang_spec? ~ as_of? ~ clauses }
hybrid = { ^"MATCH" ~ target ~ (^"SIMILAR" ~ ^"TO" ~ "[" ~ vector ~ "]")? ~ ^"ALPHA" ~ alpha ~ collection_spec? ~ lang_spec? ~ as_of? ~ clauses }
```

**PEG safety.** The optional group is followed by a mandatory keyword: `^"K"` (search) / `^"ALPHA"` (hybrid). pest's `(…)?` is greedy — it enters the group whenever the next tokens are `SIMILAR TO [`. So the disambiguation is purely: does `SIMILAR` follow the target? `target = { fuzzy_prefix? ~ (identifier | string_lit) }`, and `identifier = @{ (ALNUM | "_" | "-")+ }` is atomic and cannot contain a space, so a target token ends before `SIMILAR`. Two exhaustive cases after the target:
1. Next token is `SIMILAR` → group matches (greedy), then `K`/`ALPHA` must follow — this is v1's shape byte-for-byte. No behavior change.
2. Next token is not `SIMILAR` → group is skipped, `^"K"`/`^"ALPHA"` matched next.

**No misparse of existing queries.** Every v1 query has `SIMILAR TO [...]`, so case 1 always fires; the `(…)?` never elides for a valid v1 query, so v1 parses are unchanged. **No backtracking hazard:** because the group is anchored by a required keyword that is disjoint from `SIMILAR`, pest never needs to un-match the group — there is exactly one greedy decision and both branches lead to a required terminal. A pathological target literally named `SIMILAR` is impossible for the bare form (it would enter the group); to search *for* a node whose id is the word "SIMILAR", quote it (`"SIMILAR"` is a `string_lit`, still followed by `K`) — an acceptable and documented corner. `ast.rs`: `vector` becomes `Option<Vec<f64>>` on both `Search` and `Hybrid` (absent group ⇒ `None`).

### 1b. Semantics

- **Vector present** (group matched) → exactly v1 behavior. `vector = Some(v)`; target is documentation-only; results byte-identical (back-compat oracle = unedited `tests/hql.rs`).
- **Vector absent** (`vector = None`) → *search-by-node*. Resolve the target id (fuzzy honored via `find_fuzzy_id`, `src/lib.rs:2336`), fetch that node's **stored embedding**, and use it as the query vector into `hybrid_search` (alpha 0.0 for SEARCH, the query's alpha for hybrid).

**Embedding-fetch mechanism (exact, use what exists).** `reconstruct_embedding(&self, node: &NodeOutput, node_u32: u32) -> Option<Vec<f64>>` (`src/lib.rs:5357`). It reads the node's OWN collection: `resolve_collection(&node.collection)` → `coll.node_to_arena.get(&node_u32)` → `coll.metadata[aid]` → `(embedding_offset, vector_dim)` → `coll.arena.f32_at(start,len)`, mapping `f32→f64`. This is the correct primitive — it already routes by `node.collection`, needs no embedder, and is the same call WAL-compaction uses (`src/lib.rs:5433`). Call sequence in the Search/Hybrid arms:
```
let id  = if fuzzy { self.find_fuzzy_id(&target).ok_or(<unresolvable-err>)? } else { target };
let u32 = self.get_u32(&id).ok_or(<unresolvable-err>)?;                 // src/lib.rs:1709
let node = self.nodes.get(&u32).ok_or(<unresolvable-err>)?.clone();      // NodeOutput
let qvec = self.reconstruct_embedding(&node, u32).ok_or(<no-embedding-err>)?;
```
Non-fuzzy targets that don't intern (`get_u32` = None) are also unresolvable → error (not the current silent pass-through).

**Collection resolution.** The searched collection is the node's OWN collection (`node.collection`), because `reconstruct_embedding` reads from there and the resulting vector's dim matches only that collection. Pass `collection: node.collection.clone()` into `HybridSearchInput` for the vector-absent path (do NOT default to `default`). If the query has an explicit `IN <collection>` that **differs** from `node.collection`, **error** — do not silently search the wrong space:
`"HQL: node '<id>' lives in collection '<node.collection>' but IN '<override>' was given; omit IN or match the node's collection"`. If `IN` equals the node's collection, it is redundant but accepted. (Vector-present path keeps v1's `IN`/collection handling untouched.)

### 1c. Error cases (all errors, never silent empty results)

| Case | Message |
|---|---|
| Target unresolvable after fuzzy, vector absent | `"HQL: target '<t>' does not resolve to a node and no vector was given"` |
| Resolved node has no stored embedding | `"HQL: node '<id>' has no stored embedding and no vector was given"` |
| `IN <c>` differs from node's collection | `"HQL: node '<id>' lives in collection '<coll>' but IN '<c>' was given; omit IN or match the node's collection"` |
| Dim mismatch (defensive; hybrid_search also guards) | `"HQL: query vector dim <n> does not match collection '<coll>' dim <m>"` |

**Decision:** SEARCH/hybrid make `SIMILAR TO [vector]` optional; absent ⇒ the (fuzzy-)resolved node's stored embedding (via `reconstruct_embedding`, `src/lib.rs:5357`) is the query vector, searched in the node's own collection; unresolvable target, missing embedding, and conflicting `IN` are hard errors with the strings above; the dead `_resolved` bindings are deleted.

---

## 2. Colon-id policy (grammar batch)

**Decision: ADD `qualified_id`, scoped to `seed` and `target` only.**

```pest
qualified_id = @{ identifier ~ (":" ~ identifier)+ }
target = { fuzzy_prefix? ~ (qualified_id | identifier | string_lit) }
seed   = { fuzzy_prefix? ~ (qualified_id | identifier | string_lit) }
```
`qualified_id` is atomic (`@`) and tried **before** `identifier` in the ordered choice (PEG longest-viable-first): `user:5` matches `qualified_id` whole; a bare `user` fails the required `(":" ~ identifier)+` and falls to `identifier`. `parse_id_with_fuzzy` in `ast.rs` gains a `Rule::qualified_id => id = inner.as_str().to_string()` arm (whole matched text, colon included).

**(a) Pattern syntax `(a:Label)` is unaffected — proven.** The pattern rules `pat_var`, `pat_label` (`":" ~ identifier`), `node_pattern`, `graph_pattern` are entirely separate productions; none reference `seed`/`target`/`qualified_id`, and this change touches none of them. `MATCH (` routes to `match_pattern` (grammar line 76, ordered before `hybrid`), which consumes `graph_pattern`, never `target`. `qualified_id` is atomic and never crosses a `(`/`)` or whitespace, so it cannot bleed into a pattern. Zero shared rules change → `(a:Label)` parse is byte-identical.

**(b) `TRAVERSE FROM user:5 DEPTH 2 REL X` parses.** `traverse = ^"TRAVERSE" ~ ^"FROM" ~ seed ~ ^"DEPTH" ~ ...`. `seed` now matches `user:5` via `qualified_id`; `DEPTH` follows. `depth`/`k` are atomic digit rules, so the colon-id cannot swallow past the space into `DEPTH`. Confirmed parseable — this fixes the filter-ADR's broken example (P0-T9).

**(c) No existing valid query changes meaning.** `qualified_id` requires `identifier (":" identifier)+` — at least one embedded colon. Any v1 id was a bare `identifier` or `string_lit` (colons had to be quoted); neither contains an unquoted `:`, so no v1 seed/target reaches the new alternative. Quoted colon-ids (`"user:5"`) still match `string_lit` first is not a concern — `string_lit` starts with `"`, disjoint from `qualified_id`'s leading `identifier`. New capability is purely additive.

**No PEG hazard found.** The only place `:` is otherwise meaningful is inside pattern rules, which this change does not touch. Quoting remains valid and equivalent; it is no longer *mandatory* for the seed/target position.

**Decision:** Add `qualified_id = @{ identifier ~ (":" ~ identifier)+ }` as the first alternative in `seed` and `target` only; pattern grammar untouched; `user:5` unquoted now parses in TRAVERSE/SEARCH/MATCH-hybrid/CONTEXT seed/target positions.

---

## 3. Strict-number policy (P0-T5)

Every silent numeric `unwrap_or` in `src/query/ast.rs` becomes a parse **error** except `LIMIT`'s deliberate saturate (SPEC--HQL-V2 §1 rule 2). `TryFrom<&str>` and the `parse_*` helpers become fallible (thread `Result` through; the outer `try_from` already returns `Result<_, String>`).

| # | file:line | field | current fallback | v2 behavior |
|---|---|---|---|---|
| 1 | ast.rs:313 | filter_value number (`parse_predicate`) | `unwrap_or(0.0)` | **parse-error** |
| 2 | ast.rs:373 | LIMIT (`parse_clauses`) | `unwrap_or(usize::MAX)` | **keep** (saturate = documented "no cap") |
| 3 | ast.rs:433 | vector component (`parse_search`) | `unwrap_or(0.0)` | **parse-error** |
| 4 | ast.rs:436 | K (`parse_search`) | `unwrap_or(5)` | **parse-error** |
| 5 | ast.rs:472 | DEPTH (`parse_traverse`) | `unwrap_or(1)` | **parse-error** |
| 6 | ast.rs:529 | ALPHA (`parse_hybrid`) | `unwrap_or(0.5)` | **parse-error** |
| 7 | ast.rs:526 | vector component (`parse_hybrid`) | `unwrap_or(0.0)` | **parse-error** |
| 8 | ast.rs:564 | BUDGET (`parse_context`) | `unwrap_or(32000)` | **parse-error** |
| 9 | ast.rs:598 | filter_value number (`parse_filter_value`, pattern side) | `unwrap_or(0.0)` | **parse-error** |
| 10 | ast.rs:788 | LIMIT (`parse_pat_clauses`) | `unwrap_or(usize::MAX)` | **keep** (same rationale as #2) |

Rows 2 and 10 (both LIMIT) are the only keeps. All other numeric parses are grammar-guaranteed to be digit/number strings, so the only way `.parse()` fails is **value out of range** (u32/f64/usize overflow) — exactly the wrong-answer-generator SPEC §2.5 targets.

**Error message format:** `HQL Parse Error: <FIELD> value out of range: '<token>'`
where `<FIELD>` ∈ {`K`, `DEPTH`, `ALPHA`, `BUDGET`, `vector component`, `filter value`} and `<token>` is the offending source text. Example: `K 99999999999999999999` → `HQL Parse Error: K value out of range: '99999999999999999999'`.

**Fuzz invariant preserved:** errors are allowed, panics are not — the 34/34 no-panic corpus (AUDIT--HQL-FUZZ) stays green; new cases assert *error* (not default) on overflow tokens.

**Decision:** All ten numeric fallback sites become range parse-errors except the two `LIMIT` saturates (#2, #10); error string is `HQL Parse Error: <FIELD> value out of range: '<token>'`.

---

## Deviations from SPEC--HQL-V2

One clarification, not a contradiction: SPEC §2.1 says "the node's embedding lives in the collection it was ingested into; an explicit `IN <collection>` must match that collection's dimension or the query errors." This design tightens *dimension-match* to *collection-identity-match* (error if `IN` names a different collection at all, even if dims coincide), because `reconstruct_embedding` fetches from `node.collection` and searching a *different* same-dim collection with a node's vector would be a silently-wrong result — the exact class §2.1 forbids. SPEC §2.1 should be updated to "must equal the node's collection" in the P0-T1 PR. No other deviation; §2.5 and §2.6 recommendations are adopted as written.
