# PLAN — HQL Refinement (Swarm Execution Plan)

**Status:** Proposed (2026-07-03) · **Scope:** `src/query/hql.pest`, `src/query/ast.rs`, `src/lib.rs` (`execute_hql` / `match_pattern` / `apply_hql_clauses` / `neighbors`), tests, docs
**Parents:** ADR--GENESISDB-HQL-FILTER-PROJECTION (path 2+4, shipped), ADR--GENESISDB-HQL-CYPHER-PATTERNS (path 1, shipped PR #60), ADR--GENESISDB-COMPETITIVE-SUPERIORITY §7 (Wave 2.3 lean traversal, Wave 2.5 native BM25+RRF — cross-linked, not duplicated)
**What this corrects:** HQL's three shipped paths left (a) genuine defects — `SEARCH`/`MATCH…SIMILAR` compute a fuzzy-resolved target and **throw it away**, hybrid's candidate pool is **hardcoded K=10**, malformed numbers silently become defaults; (b) capability the engine already has but HQL can't reach — `ef_search`/`oversample` (the P32 recall fix), traversal direction/multi-rel/limit; (c) the v1 non-goals both ADRs put on record — variable-length paths, frontier guardrails, OR-predicates, identity joins, O(N) anchor scans; and (d) the still-deferred **path 3** (text query without a caller-supplied vector). No cost-based planner is introduced anywhere in this plan — every task preserves the "dispatch directly to Storage, transform after" invariant all three HQL ADRs commit to.

> **Track boundary (2026-07-05):** `P0` remains a **native HQL correctness/exposure track** and must stay independently shippable from the SQLite substrate program. The SQLite ADR may supersede `P2/P3` execution strategy, but it does **not** block or reshape `P0` semantics, grammar, or public-surface exposure. See [SPEC--SQLITE-SUBSTRATE-S0-S1](SPEC--SQLITE-SUBSTRATE-S0-S1.md).

**Verified facts this plan is built on (independently code-checked 2026-07-03):**
- `SEARCH`/`Hybrid` resolve the target (incl. the expensive `~` fuzzy path: trigram index + jaro-winkler + neural vector fallback, `find_fuzzy_id` `src/lib.rs:2336`) into `_resolved` — which is **never used**; the query runs on the literal vector alone (`src/lib.rs:3343-3347`, `src/lib.rs:3402-3406`). Only `TRAVERSE` uses its resolved seed. — *confirmed*
- Hybrid `k` is hardcoded to `10` (`src/lib.rs:3409`); the grammar has no `K` clause on the hybrid form (`src/query/hql.pest:45`). `LIMIT` can only shrink that pool, never widen it. — *confirmed*
- HQL passes `ef_search: None, oversample: None` on both search paths (`src/lib.rs:3355-3356`, `3414-3415`): the per-query knobs that fixed recall@500k (AUDIT--P32) and the rerank oversample knob (VQ plan P1b) are unreachable from the query language. — *confirmed*
- `TRAVERSE` hardcodes `direction: "out"`, single `rel`, `limit: None` (`src/lib.rs:3377-3389`) while `neighbors()` itself already supports `in`/`both`, a `rels` set, and a retrieval limit (`src/lib.rs:3646-3655`). Engine capability exists; grammar doesn't expose it. — *confirmed*
- `MATCH (…)` pattern anchor is a full scan of `self.nodes` **even when the anchor is `{id:"…"}`-constrained** (`src/lib.rs:3050-3064`); there is no label index. — *confirmed*
- Pattern expansion has no frontier cap (ADR names this a v2 guardrail), eagerly serializes every bound node/edge to JSON per row (`src/lib.rs:3129-3135`), and recomputes `Utc::now().to_rfc3339()` **per edge** for the retraction check (`src/lib.rs:3098`; `neighbors` has the same per-edge allocation at `src/lib.rs:3696`). P29 established that per-result materialization dominates deep-hop cost. — *confirmed*
- Parser numeric fallbacks are silent: bad `K` → 5, bad `DEPTH` → 1, bad `ALPHA` → 0.5, bad vector component → 0.0 (`src/query/ast.rs:436,472,529,433`). (`LIMIT`'s saturate-to-`usize::MAX` is deliberate and documented — keep.) The fuzz suite (34/34, AUDIT--HQL-FUZZ) proves no-panic, not no-silent-wrong-answer. — *confirmed*
- `identifier` excludes `:` (`src/query/hql.pest:2`), so colon ids — the project's own idiom (`user:5`, NotiKeeper, agent registry) — must be quoted; the filter ADR's example `TRAVERSE FROM user:5 …` cannot parse as written. — *confirmed*
- Both HQL ADRs still carry `status: candidate` though both shipped and merged; CLAUDE.md's HQL bullet lists four command forms (five exist). — *confirmed*
- NAPI/REST/MCP parity is automatic for grammar work: all three surfaces funnel through `execute_hql` (NAPI `executeHql`, REST `/v1/query/hql` untagged body, MCP `query_hql`). Doc surfaces (index.d.ts docstring, MCP tool description) still need text updates when syntax grows. — *confirmed*

---

## (a) Summary table — all tasks

| ID | Title | Model | Depends-on | Gate |
|----|-------|-------|------------|------|
| **P0-T0** | Design gate: target semantics, colon-id policy, strict-number policy | Opus 4.8 | none | Opus 4.8 |
| **P0-T1** | Search-by-node: make the discarded `SEARCH`/hybrid target mean something | Opus 4.8 | P0-T0 | Opus 4.8 |
| **P0-T2** | `K <n>` clause on the hybrid form (kill the hardcoded 10) | Sonnet 4.6 | P0-T0 | Opus 4.8 |
| **P0-T3** | `EF <n>` / `OVERSAMPLE <n>` clauses on SEARCH + hybrid | Sonnet 4.6 | P0-T2 | Opus 4.8 |
| **P0-T4** | `TRAVERSE` `DIRECTION in\|out\|both` + multi-rel `REL a\|b` | Sonnet 4.6 | P0-T0 | Opus 4.8 |
| **P0-T5** | Strict numeric parse errors (end silent defaults) | Sonnet 4.6 | P0-T0 | Opus 4.8 |
| **P0-T6** | Hoist per-edge `Utc::now()` out of `match_pattern` + `neighbors` loops | Sonnet 4.6 | none | Opus 4.8 |
| **P0-T7** | Id-anchored pattern fast-path (skip the O(N) scan for `{id:…}` anchors) | Sonnet 4.6 | none | Opus 4.8 |
| **P0-T8** | Tests: new grammar/semantics + fuzz-corpus extension | Sonnet 4.6 | P0-T1..T7 | Opus 4.8 |
| **P0-T9** | Docs de-stale: ADR statuses, CLAUDE.md 5 forms, broken `user:5` example | Sonnet 4.6 | P0-T0 | Opus 4.8 |
| **P1-T0** | Design gate: var-length semantics, frontier cap policy, identity-join rule | Opus 4.8 | P0 merged | Opus 4.8 |
| **P1-T1** | Variable-length paths `-[:R*1..d]->` | Opus 4.8 | P1-T0 | Opus 4.8 |
| **P1-T2** | Frontier cap guardrail (const + per-query override) | Sonnet 4.6 | P1-T0 | Opus 4.8 |
| **P1-T3** | Lazy bindings: expand over ids, materialize JSON only for referenced vars | Opus 4.8 | P1-T0 | Opus 4.8 |
| **P1-T4** | Repeated-variable identity join (`(a)-->(b)-->(a)` means the same `a`) | Sonnet 4.6 | P1-T0, P1-T1 | Opus 4.8 |
| **P1-T5** | Rel-type alternation `-[:R1\|R2]->` | Sonnet 4.6 | P1-T1 | Opus 4.8 |
| **P1-T6** | Tests + bench: pattern/var-length rows in `hql-query-stress`, no-regression gate | Sonnet 4.6 | P1-T1..T5 | Opus 4.8 |
| **P2-T1** | `OR` + parentheses in WHERE (both clause systems) | Opus 4.8 | P1 merged | Opus 4.8 |
| **P2-T2** | `RETURN count(*)` (count-only aggregation) | Sonnet 4.6 | P1 merged | Opus 4.8 |
| **P2-T3** | Label index (`label_idx`) + index-assisted anchor | Opus 4.8 | P1 merged | Opus 4.8 |
| **P2-T4** | `CONTEXT` clause support (design-gated; may be dropped) | Sonnet 4.6 | P2-T1 | Opus 4.8 |
| **P3-T0** | Path-3 design ADR: text query without a caller vector | Opus 4.8 | P0 merged | Opus 4.8 |

P3 implementation tasks are **deliberately not pre-enumerated** — P3-T0's accepted design defines them (same discipline as the VQ plan's P2a-T0 gate).

---

## (b) Dependency / ordering overview

**Phase sequencing: P0 → P1 → P2 as separate PRs, each merged to `main` before the next phase dispatches.** P0 changes grammar rules and executor plumbing that P1/P2 tasks would otherwise rebase across. P3-T0 (design doc, no code) can run any time after P0 merges and in parallel with P1/P2.

**Within P0:**
```
P0-T0 (design) ──┬──> P0-T1 (target semantics)
                 ├──> P0-T2 (hybrid K) ──> P0-T3 (EF/OVERSAMPLE)
                 ├──> P0-T4 (direction/multi-rel)
                 ├──> P0-T5 (strict numbers)
                 └──> P0-T9 (docs, parallel)
P0-T6, P0-T7 (no design dep, start immediately)
P0-T1..T7 ──> P0-T8 (tests)
```
- **Grammar-file serialization rule:** P0-T1/T2/T3/T4/T5 all edit `hql.pest` + `ast.rs`. Dispatch them **sequentially in that order** (or as one batch to a single executor) — the file is small and parallel edits guarantee integration conflicts. T6/T7 touch only `src/lib.rs` executor internals and parallelize freely against the grammar chain.
- **P1:** T0 gates everything; T1 is the spine; T2/T3 parallelize after T0; T4/T5 need T1's expansion-loop shape; T6 gates the PR.
- **P2:** T1/T2/T3 are mutually independent; T4 only makes sense after T1's predicate grammar settles.

Recommended PR cadence: **PR1 = P0** (correctness + exposure — every item is either a bug fix or a ≤1-day knob). **PR2 = P1** (pattern power). **PR3 = P2**. **P3 = its own ADR, then its own plan/PR(s).**

---

## (c) Orchestration flow

1. Each task dispatches to its executor model with only its self-contained task description.
2. Diff → **Opus 4.8 review gate** with the task's checklist; reviewer runs the named acceptance tests (`cargo test --no-default-features --test <file>` for Rust; `npm test` where NAPI surface text changes).
3. Reviewer blocks on any unmet gate item; executor iterates.
4. Orchestrator integrates, runs the cross-task sweep: full `cargo test --no-default-features`, `npm test`, and — because P0-T6/T7 and all of P1 touch traversal hot paths — the perf harnesses per the `run-bench-audit` skill: `cargo run --release --features bins --bin hql-query-stress` and `--bin graph-bench` (no-regression vs the P31 baseline: hop1 ~22.6 µs class, hop6 ~4.9 ms class).
5. NAPI/REST/MCP parity is structural for HQL (single `execute_hql` funnel) — the parity check per phase reduces to: index.d.ts `executeHql` docstring, MCP `query_hql` tool description, and `docs/` syntax examples updated to the new grammar.
6. All Rust work must pass under `--no-default-features` (core/napi split; Linux CI links without `napi_*` symbols).

---

# P0 — Correctness & exposure (defects + engine capability HQL can't reach)

> **Separation rule:** Everything in `P0` ships against the native executor path (`src/query/*` + `execute_hql` + existing public funnels) with no SQLite dependency. If an implementation choice would make `P0` wait on substrate work, that choice belongs in `S2+`, not here.

### P0-T0 — Design gate: three grammar-semantic forks
- **Scope:** One design note (append a "v2 decisions" section to ADR--GENESISDB-HQL-FILTER-PROJECTION or a new short doc). Settles, with grammar sketches: **(1) target semantics** — what `SEARCH <target> …` / `MATCH <target> SIMILAR …` means now that the target must do something: recommended = *search-by-node*: when the (fuzzy-)resolved target names a live node and the literal vector is omitted, the node's stored embedding becomes the query vector (primitive exists: `reconstruct_embedding`, WAL-compaction work); when a literal vector is present it wins and the target is documentation-only (back-compat). **(2) colon-id policy** — `user:5` unquoted: extend seed/target (NOT the pattern grammar — `(a:Label)` must not change meaning) with a `qualified_id = identifier (":" identifier)+` alternative, or mandate quoting and fix the docs; decide. **(3) strict-number policy** — malformed `K`/`DEPTH`/`ALPHA`/vector components become parse **errors**, not silent defaults; enumerate which `unwrap_or`s stay (only `LIMIT`'s deliberate saturate).
- **Complexity:** S · **Executor:** Opus 4.8 (each decision constrains every P0 grammar task) · **Depends-on:** none
- **Review gate:** (1) search-by-node semantics defined incl. the collection-resolution rule for the node's embedding and behavior when the target resolves to nothing (error, not empty-vector search); (2) colon-id decision proves `(a:Label)` parse is unaffected (PEG ordering argument written down); (3) strict-number list is exhaustive over `ast.rs`'s `unwrap_or` sites.
- **Acceptance:** Decision doc exists; P0-T1/T2/T4/T5 implementable without an open fork. **Risk:** none (doc).

### P0-T1 — Search-by-node target semantics
- **Scope:** `hql.pest` (make `SIMILAR TO [vector]` optional per T0's chosen syntax), `ast.rs` (vector becomes `Option<Vec<f64>>`), `execute_hql` Search/Hybrid arms (`src/lib.rs:3333-3358`, `3392-3417`): resolved target + no literal vector → fetch the node's stored embedding (T0 rule) and search with it; literal vector present → current behavior byte-identical; target unresolvable + no vector → error. Delete the dead `_resolved` bindings (both arms) — after this task the resolution result is always consumed or an error.
- **Why:** The headline defect — today the engine pays full fuzzy resolution (trigram + jaro-winkler + neural fallback) and discards the answer.
- **Complexity:** M · **Executor:** Opus 4.8 (grammar optionality + embedding fetch across collections is the correctness-critical piece) · **Depends-on:** P0-T0
- **Review gate:** (1) all existing queries with literal vectors parse and return byte-identical results (back-compat oracle: existing `tests/hql.rs` unedited); (2) no-vector form uses the node's own embedding — verified against a direct `hybrid_search` call with that embedding; (3) dim mismatch between node's collection and `IN <collection>` override errors cleanly; (4) `~fuzzy` resolution result is consumed, `_resolved` gone.
- **Acceptance:** New cases in `tests/hql.rs`: `SEARCH <id> K 5` returns the id's neighbors-by-similarity; unresolvable target errors; literal-vector queries unchanged. **Risk:** Medium (grammar optionality) · Rollback: keep vector mandatory, revert arms.

### P0-T2 — `K <n>` on the hybrid form
- **Scope:** `hql.pest:45` add optional `k` to `hybrid`; `ast.rs` `Hybrid { k: Option<u32> }`; `execute_hql` uses it, default 10 (back-compat).
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** P0-T0 (sequencing only)
- **Review gate:** (1) omitted K ⇒ 10 (existing tests unedited); (2) `K` composes with `ALPHA`/`IN`/`LANGUAGE`/`AS OF`/clauses in any documented order the grammar defines; (3) digit rule is atomic (`@{…}` — the DEPTH-bug precedent in `hql.pest:14-17`).
- **Acceptance:** `tests/hql.rs` case: `MATCH t SIMILAR TO […] ALPHA 0.5 K 50 LIMIT 5` yields ≤5 rows from a 50-candidate pool (constructed so K=10 would provably miss). **Risk:** low.

### P0-T3 — `EF <n>` / `OVERSAMPLE <n>` clauses
- **Scope:** `hql.pest` optional clauses on `search` + `hybrid`; `ast.rs` fields; `execute_hql` passes them into `HybridSearchInput.ef_search` / `.oversample` (today `None`, `src/lib.rs:3355-3356`, `3414-3415`).
- **Why:** Per-query `ef` is the shipped recall@500k fix (P32); `oversample` is the shipped rerank knob (VQ P1b). HQL is the only surface that can't set them.
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** P0-T2 (same grammar region)
- **Review gate:** (1) omitted ⇒ `None` ⇒ engine defaults (collection → global fallback chain untouched); (2) values flow to `HybridSearchInput` verbatim; (3) atomic digit rules.
- **Acceptance:** `tests/hql.rs`: `EF 512` query on a deliberately low-recall fixture returns the known-missing neighbor that the default `ef` misses (mirror the P32 test pattern); parse round-trip asserts fields. **Risk:** low.

### P0-T4 — `TRAVERSE` direction + multi-rel
- **Scope:** `hql.pest:44`: optional `DIRECTION` (`^"DIRECTION" ~ (^"in"|^"out"|^"both")`) and `REL a|b|c` alternation on the existing `rel` rule; `ast.rs` `Traverse { direction, rels }`; `execute_hql` maps to `NeighborInput.direction` / `.rels` (both already supported by `neighbors`, `src/lib.rs:3646-3655`). `INFER(...)` stays single-rel.
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** P0-T0 (sequencing)
- **Review gate:** (1) omitted DIRECTION ⇒ `"out"` (back-compat); (2) `REL a|b` maps to `rels: Some(vec![a,b])`, single rel still uses `rel:` (or `rels` uniformly — pick one, document); (3) no collision with P1-T5's edge-pattern alternation syntax (different grammar rules).
- **Acceptance:** `tests/hql.rs`: `DIRECTION in` returns the reverse-edge neighbor the `out` form doesn't; `REL a|b` returns union. **Risk:** low.

### P0-T5 — Strict numeric parse errors
- **Scope:** `ast.rs` per T0's policy: `TryFrom` becomes fallible through the numeric parses — malformed `K`/`DEPTH`/`ALPHA`/`BUDGET`/vector component/filter number returns `Err("HQL Parse Error: …")` with the offending token, instead of `unwrap_or(default)`. `LIMIT` saturate stays.
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** P0-T0
- **Review gate:** (1) every changed site listed in T0 is covered, none new invented; (2) fuzz suite still 34/34 **no-panic** (errors fine, panics not); (3) error text names the field.
- **Acceptance:** `tests/hql_fuzz_tests.rs` extended: `K 99999999999999999999` errors (today the u32 overflow silently becomes `K 5` via `unwrap_or`); existing valid-query round-trip tests unedited. **Risk:** low-medium (behavioral change for garbage inputs — that's the point; REST/MCP callers get a 4xx-class error instead of wrong results).

### P0-T6 — Hoist the per-edge retraction timestamp
- **Scope:** `src/lib.rs` only: compute `let now = Utc::now().to_rfc3339();` once per query in `match_pattern` (before the hop loop, replacing `src/lib.rs:3098`) and once in `neighbors` (before the BFS loop, replacing `src/lib.rs:3696`); compare against `&now`. Semantics: a query now uses one consistent "current" instant — strictly better than a timestamp that drifts across the scan.
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** none
- **Review gate:** (1) exactly one `Utc::now()` per query path, string allocated once; (2) retraction visibility tests (`tests/retract_edge_tests.rs` or equivalent) unedited and green; (3) no other behavior change in the diff.
- **Acceptance:** existing traversal + retraction + cypher tests green; `hql-query-stress` and `graph-bench` show no regression (any improvement is a bonus, not a claim — single-run deep-hop variance per P31 §4). **Risk:** trivial.

### P0-T7 — Id-anchored pattern fast-path
- **Scope:** `src/lib.rs` `match_pattern` anchor block (`src/lib.rs:3049-3064`): if `pattern.start.props` contains an `id` constraint (string form), seed the frontier via `get_u32(id)` + `nodes.get` directly (still applying `is_valid_as_of` + the full `node_matches`), skipping the `self.nodes.iter()` scan. Label-only/unconstrained anchors keep the scan (label index is P2-T3).
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** none
- **Review gate:** (1) fast path applies **only** on an exact-id string constraint; numeric id constraint and every other shape falls through to the scan; (2) results identical to the scan path (same row, same bindings) — assert by running both on a fixture; (3) missing id ⇒ empty result (not error), matching scan behavior.
- **Acceptance:** `tests/hql_cypher_tests.rs`: id-anchored pattern on a 10k-node fixture returns identical rows to pre-change; a timing assertion is NOT required (variance) — correctness only, perf claimed via bench in the PR. **Risk:** low.

### P0-T8 — Tests: consolidation + fuzz corpus extension
- **Scope:** Extend `tests/hql.rs` / `hql_filter_tests.rs` / `hql_cypher_tests.rs` / `hql_fuzz_tests.rs` with the new grammar forms (search-by-node, K, EF/OVERSAMPLE, DIRECTION, multi-rel, strict-number errors) including mutation coverage in the fuzz categories (new keywords into category 3/10 generators).
- **Complexity:** M · **Executor:** Sonnet 4.6 · **Depends-on:** P0-T1..T7
- **Review gate:** (1) every new grammar production has ≥1 positive + ≥1 negative parse test; (2) fuzz generators include the new keywords; (3) all pre-existing tests unedited (back-compat oracle).
- **Acceptance:** `cargo test --no-default-features --test hql --test hql_filter_tests --test hql_cypher_tests --test hql_fuzz_tests` green. **Risk:** none.

### P0-T9 — Docs de-stale
- **Scope:** (1) both HQL ADRs: `status: candidate` → shipped/accepted, add "shipped in PR #60 / filter-projection PR" note; (2) CLAUDE.md HQL bullet: five command forms (`SEARCH`, `TRAVERSE`, `MATCH <pattern>`, `MATCH…SIMILAR`/`HYBRID`, `CONTEXT`); (3) fix the filter ADR's unparseable `TRAVERSE FROM user:5` example per T0's colon-id decision; (4) index.d.ts `executeHql` docstring + MCP `query_hql` tool description mention the new clauses.
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** P0-T0 (colon decision)
- **Review gate:** (1) no doc claim contradicts shipped code; (2) every syntax example in the touched docs parses against the post-P0 grammar (reviewer runs them via a scratch test).
- **Acceptance:** doc diff; orchestrator greps for `status: candidate` in the two ADRs → none. **Risk:** none.

---

# P1 — Pattern power (the graph track)

> Together these lift `MATCH (…)` from "linear demo" to "can express the competitor-bench query shape" — `MATCH (a {id:…})-[:LINK*1..6]->(b) RETURN b.id LIMIT 1000` is exactly the P26/P30 Kuzu/LadybugDB harness query, which HQL today **cannot write**. That also unlocks a future apples-to-apples HQL-vs-Cypher head-to-head instead of benching `neighbors()` directly.

### P1-T0 — Design gate: var-length semantics, frontier policy, identity rule
- **Scope:** Extend ADR--GENESISDB-HQL-CYPHER-PATTERNS with a v2 section deciding: **(1) var-length** `-[:R*min..max]->` semantics — expansion = the existing per-hop loop iterated with a visited-set policy (per-row path-visited like Cypher trails, vs global BFS visited like `neighbors`— they give different row counts; pick and justify), result binds the terminal node (no path variable in v2); bounds required (`*` alone = 1..default_cap, define cap). **(2) frontier cap** — const default (e.g. 100k rows), per-query override syntax or none, and the policy at the cap: **error** ("frontier exceeded, add constraints/LIMIT") vs silent truncate — recommended: error (silent truncation is the kind of dishonesty the competitive ADR bans in benches). **(3) identity join** — repeated variable = same entity (Cypher semantics), applied as an expansion-time filter.
- **Complexity:** S/M · **Executor:** Opus 4.8 · **Depends-on:** P0 merged
- **Review gate:** (1) visited-set choice is justified against both a diamond graph and a cycle fixture with worked row counts; (2) cap policy defined incl. error text; (3) identity rule covers node AND edge variables; (4) explicitly reaffirms: no planner, expansion order = written order.
- **Acceptance:** ADR section merged-ready; P1-T1/T2/T4 implementable without a fork. **Risk:** none (doc).

### P1-T1 — Variable-length paths
- **Scope:** `hql.pest` `rel_detail` gains optional `*min..max` (atomic digits); `ast.rs` `EdgePattern { min_hops, max_hops }`; `match_pattern` expansion loop iterates the hop `min..=max` times per T1-T0's visited policy, honoring the frontier cap (T2's constant even before T2's override lands).
- **Complexity:** L · **Executor:** Opus 4.8 (the semantics-bearing loop; a wrong visited policy silently changes row multiplicity) · **Depends-on:** P1-T0
- **Review gate:** (1) `*1..1` ≡ today's single hop (byte-identical rows on the cypher test fixture); (2) diamond + cycle fixtures return exactly the row counts the ADR worked out; (3) depth-6 fanout-8 doesn't blow memory absent constraints — cap fires per policy; (4) direction applies at every step; rel filter at every step.
- **Acceptance:** `tests/hql_cypher_tests.rs`: the P30-shape query on a small fanout graph returns the same node set as `neighbors(depth=d)` modulo the documented visited-policy difference (test encodes which). **Risk:** high (semantics) · Rollback: grammar rejects `*` (additive).

### P1-T2 — Frontier cap guardrail
- **Scope:** `match_pattern`: `const PATTERN_FRONTIER_CAP: usize` + check per expansion round; policy per P1-T0 (error with actionable text, or documented truncate). Optional per-query override only if T0 chose one.
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** P1-T0
- **Review gate:** (1) cap counts **rows**, not nodes visited; (2) error path is a clean `Err`, no partial JSON; (3) cap is documented in the ADR + error message says how to raise/avoid it.
- **Acceptance:** test: hub fixture (one node, 200k fan-out via 2 hops) errors at the cap with the documented message; capped-off query under the limit unaffected. **Risk:** low.

### P1-T3 — Lazy bindings
- **Scope:** `match_pattern`: expansion carries `(u32 node, u128 edge)` refs per variable instead of eagerly serialized JSON (`src/lib.rs:3129-3135`); materialize `serde_json` values **only** for variables actually referenced by WHERE/ORDER BY/RETURN (compute the referenced-var set from the clauses up front; `RETURN *`/no-RETURN materializes all named vars, as today). This is the pattern-side twin of the competitive ADR's Wave 2.3 "lean traversal" item (P29: materialization dominates).
- **Complexity:** M/L · **Executor:** Opus 4.8 (row identity/ordering must survive the refactor; WHERE must see identical values) · **Depends-on:** P1-T0 (runs fine in parallel with T1 if coordinated on the loop shape — orchestrator may serialize T1→T3 to avoid a rebase)
- **Review gate:** (1) output JSON byte-identical for all existing cypher tests (oracle unedited); (2) a superseded/deleted node between expansion and materialization degrades defined-ly (row dropped or stale-read documented — DashMap ref lifetime decides; state it); (3) memory win demonstrated on the hub fixture (rows × unused vars no longer serialized).
- **Acceptance:** existing `hql_cypher_tests.rs` green unedited + new wide-pattern fixture asserting equal output pre/post. Perf via `hql-query-stress` in the PR body. **Risk:** medium.

### P1-T4 — Repeated-variable identity join
- **Scope:** `match_pattern`: per P1-T0's rule — when a hop's node/edge variable name is already bound in the row, the candidate must **be** that entity (compare u32/eid), else the row dies. Enables cycles: `(a)-[:KNOWS]->(b)-[:KNOWS]->(a)`.
- **Complexity:** S/M · **Executor:** Sonnet 4.6 · **Depends-on:** P1-T0, P1-T1 (loop shape)
- **Review gate:** (1) triangle fixture: cycle query returns exactly the triangles, no independent-binding false rows; (2) anonymous/unnamed positions unaffected; (3) edge-variable identity too, not just nodes.
- **Acceptance:** new cases in `tests/hql_cypher_tests.rs` (triangle + false-positive control). **Risk:** low-medium.

### P1-T5 — Rel-type alternation
- **Scope:** `hql.pest` `pat_label` in `rel_detail` context → `:R1|R2` list; `ast.rs` `EdgePattern.rel_type: Vec<String>` (empty = any); executor membership test. Mirrors P0-T4's TRAVERSE syntax choice for consistency.
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** P1-T1
- **Review gate:** (1) node-pattern `:Label` syntax untouched (alternation is edge-only in v2); (2) single-type patterns parse byte-identically; (3) syntax matches P0-T4's `REL a|b` (one alternation idiom across HQL).
- **Acceptance:** cypher test: `-[:SENT|FORWARDED]->` returns the union. **Risk:** low.

### P1-T6 — Tests + bench gate
- **Scope:** Consolidated P1 test pass + extend `benches/hql_query_stress.rs` with pattern-match rows: id-anchored 1-hop, var-length `*1..3` and `*1..6`, and a WHERE-filtered pattern; record p50/p95 next to the existing HQL command numbers. Wire the P26/P30-shape query in as a named case for future head-to-heads.
- **Complexity:** M · **Executor:** Sonnet 4.6 · **Depends-on:** P1-T1..T5
- **Review gate:** (1) bench compiles under `--features bins` and runs on the standard corpus; (2) no-regression on the pre-existing stress rows (variance caveat per P31 §4 — two runs minimum); (3) the new rows' numbers land in the PR body with the exact command.
- **Acceptance:** `cargo run --release --features bins --bin hql-query-stress` output table includes pattern rows; full test suite green. **Risk:** none.

---

# P2 — Clause ergonomics

> **⚠ SUPERSEDED IN EXECUTION (2026-07-03) by [ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE](adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md) phase S2.** The *language contract* below (grammar, semantics, tests) stands unchanged; the *execution strategy* changes: P2-T1's WHERE/OR evaluator and P2-T2's count compile down to SQL over the embedded SQLite projection (gaining pushed-down, indexed filtering), and P2-T3's bespoke `label_idx` is cancelled in favor of an indexed `node_labels` table. `score`/`depth` predicates stay in the small native evaluator. P2-T4 unchanged. Do not implement these tasks as written without reading the ADR first.

### P2-T1 — `OR` + parentheses in WHERE
- **Scope:** `hql.pest`: predicate grammar becomes `expr = term (OR term)*; term = factor (AND factor)*; factor = predicate | "(" expr ")"` for **both** `where_clause` and `pat_where` (shared rules where pest allows); `ast.rs`: predicate tree replaces `Vec<HqlPredicate>` (keep a compat constructor so `apply_hql_clauses`/`pattern_eval_predicate` evaluate the tree); SQL-null semantics unchanged (null ⇒ false at the leaf, standard three-valued collapse documented).
- **Complexity:** M · **Executor:** Opus 4.8 (precedence + PEG backtracking against the existing clause chain is the risk) · **Depends-on:** P1 merged
- **Review gate:** (1) AND-only queries parse into the same effective evaluation (oracle tests unedited); (2) precedence: AND binds tighter than OR, parens override — property-tested against a reference evaluator on random small prop sets; (3) fuzz corpus extended with unbalanced parens (error, no panic).
- **Acceptance:** filter + cypher test files gain OR/paren cases; fuzz green. **Risk:** medium (grammar surface).

### P2-T2 — `RETURN count(*)`
- **Scope:** `hql.pest` `return_clause`/`pat_return` alternative `^"count" ~ "(" ~ "*" ~ ")"`; executor: after WHERE (before ORDER/LIMIT are meaningless — define: count applies post-WHERE, ORDER BY/LIMIT with count is a parse error) return `[{"count": n}]`. Count-only; no group-by, no other aggregates (non-goal below).
- **Complexity:** S · **Executor:** Sonnet 4.6 · **Depends-on:** P1 merged
- **Review gate:** (1) `count(*)` + ORDER BY/LIMIT rejected at parse with a clear message; (2) shape is `[{"count": n}]` on both clause systems (documented in ADR + index.d.ts docstring).
- **Acceptance:** tests on both TRAVERSE-with-count and pattern-with-count. **Risk:** low.

### P2-T3 — Label index + index-assisted anchor
- **Scope:** `src/lib.rs`: `label_idx: DashMap<String, HashSet<u32>>` maintained on `insert_node_lean`/supersede/load (and rebuilt in compaction like the other indices — audit every node-mutation site); `match_pattern` anchor: `(:Label)` seeds from `label_idx` ∩ validity instead of the full scan; `neighbors` untouched (it's seed-anchored already). This is the ADR's named "label/prop index-assisted anchor" future item; prop-value indexing stays OUT (non-goal — that's a real secondary-index feature with its own ADR if ever).
- **Complexity:** L · **Executor:** Opus 4.8 (a missed mutation site silently desyncs the index — the class of bug the edge-interning work fought) · **Depends-on:** P1 merged
- **Review gate:** (1) every write path that touches `labels` updates the index (grep-audit list in the PR: add, bulk_add, supersede, WAL replay, snapshot load, compaction); (2) anchor via index returns identical rows to the scan on a randomized fixture (dual-run assert); (3) RSS cost of the index measured and stated (it's u32 sets — expected small vs the 686 MB baseline, but state it); (4) snapshot format: index derivable ⇒ NOT persisted (rebuild on load), keeping `state.json` unchanged — confirm.
- **Acceptance:** new `tests/label_index_tests.rs` (mutation-site matrix: add→visible, supersede→moves, retracted-era `as_of` still correct via validity filter); cypher anchor tests dual-run green. **Risk:** medium-high (index consistency) · Rollback: anchor falls back to scan (index additive).

### P2-T4 — `CONTEXT` clauses (design-gated, droppable)
- **Scope:** Decide (mini-gate inside the task): does `WHERE`/`LIMIT` over a `ContextPackage` mean filtering atoms pre-budget or post-assembly? If no crisp semantics emerges in a day's design, **drop the task** — the filter ADR already deferred it once and consumers haven't asked (NotiKeeper uses `retrieve_tiered_context` directly).
- **Complexity:** S/M · **Executor:** Sonnet 4.6 · **Depends-on:** P2-T1
- **Review gate:** if implemented: budget interaction defined (filter-then-budget), GRL tests green; if dropped: one paragraph in the ADR recording why.
- **Acceptance:** either shipped-with-tests or documented-dropped. **Risk:** low either way.

---

# P3 — Path 3: text query without a caller vector (design first)

> **⚠ DESIGN FORK RESOLVED (2026-07-03) by [ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE](adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md) phase S3:** option (b) in-engine lexical, implemented on embedded SQLite FTS5 (trigram BM25) + RRF — not hand-rolled Rust BM25 — serving HQL `SEARCH TEXT` and Wave 2.5's native hybrid through one lexical engine. P3-T0 shrinks to specifying ranking/fusion semantics and the honest-failure contract.

### P3-T0 — Design ADR: `SEARCH TEXT "…"`
- **Scope:** New `docs/adr/ADR--GENESISDB-HQL-TEXT-QUERY.md` weighing the three candidate designs with evidence: **(a) node-anchored search** — ships in P0-T1, zero embedder, covers "more like this node"; record it as the shipped baseline, not the answer. **(b) in-engine lexical search** — score over the existing trigram index (`find_fuzzy_id` already walks it, `src/lib.rs:2336-2372`) generalized from id-matching to text scoring, optionally BM25-weighted, fused with the vector score via RRF when a vector/`ALPHA` is also present. This is the **same primitive** the competitive ADR's Wave 2.5 (native dense+sparse hybrid, NotiKeeper-validated RRF+BM25) needs — one implementation should serve both; this ADR must reconcile scope with Wave 2.5 rather than shipping a second lexical path. **(c) client-side embed hook** — NAPI callback / REST sidecar contract so `SEARCH TEXT` embeds via the caller's model (bge-m3 at our consumers); zero engine ML, but per-front-end wiring and it breaks the "one funnel" parity property. Decision + task list for the implementation PR(s).
- **Complexity:** M · **Executor:** Opus 4.8 · **Depends-on:** P0 merged (must build on P0-T1's target semantics)
- **Review gate:** (1) the Wave 2.5 overlap is resolved explicitly (one lexical engine, two surfaces — or a written reason why not); (2) whatever is chosen, `SEARCH TEXT` with no vector and no embedder available has a **defined honest failure** (error naming the missing capability, never a silent empty result); (3) the recall story is stated with the measurement plan (which harness, which corpus — bge-m3 real-data per P33 precedent); (4) implementation tasks enumerated with the same field template as this plan.
- **Acceptance:** ADR file merged-ready with a decision, not a survey; follow-up tasks defined. **Risk:** none (doc). Implementation is a separate PR under its own plan section.

---

## Non-goals (this plan, on record)

- **No cost-based planner, no EXPLAIN.** Three ADRs deep, the engine's contract is dispatch-directly + transform-after; every task above preserves it. An EXPLAIN over a planner-free engine would echo the dispatch table — revisit only if P1's var-length semantics ever grow join reordering (they must not).
- **No branching/comma patterns** (`(a)-->(b), (a)-->(c)`), no OPTIONAL MATCH, no path variables, no shortest-path.
- **No aggregation beyond `count(*)`** — no group-by, sum, collect.
- **No prop-value secondary indexes** (P2-T3 is labels only).
- **No engine-embedded ML model** — path 3 options are lexical or caller-side; an in-engine embedder is out of scope permanently (mobile targets forbid the weight).

## Standing verification per phase

Every phase PR: full `cargo test --no-default-features` + `npm test` (after `npm run build:debug`), the HQL fuzz suite, and for anything touching `match_pattern`/`neighbors`: `hql-query-stress` + `graph-bench` (two runs, C: SSD via `GB_VBENCH`, variance caveat per P31 §4) with numbers in the PR body. Grammar changes additionally re-run the full fuzz categories with new keywords injected into the mutation generators.
