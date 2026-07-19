---
proposed_id: SPEC--HQL-V2
type: spec
status: target
tier: process
cluster: implementation_flow
role: "HQL v2 target specification — normative contract for PLAN--HQL-REFINEMENT (P0–P2 normative; design-gated items marked; P3 reserved)"
date: 2026-07-03
related:
  - SPEC--HQL-V1
  - PLAN--HQL-REFINEMENT
  - adr/ADR--GENESISDB-HQL-FILTER-PROJECTION
  - adr/ADR--GENESISDB-HQL-CYPHER-PATTERNS
  - ADR--GENESISDB-COMPETITIVE-SUPERIORITY
---

# SPEC — HQL v2 (refinement target)

**This is the normative target contract for [PLAN--HQL-REFINEMENT](PLAN--HQL-REFINEMENT.md).** Everything here is either (a) fixed by the plan's task definitions, or (b) marked **⚙ design-gated** — the recommended form is written here and P0-T0/P1-T0/P3-T0 may amend it; if a gate amends, this spec is updated in the same PR. Sections not restated (clause null semantics, collections, AS OF, surfaces, return shapes) are **inherited unchanged from [SPEC--HQL-V1](SPEC--HQL-V1.md)**.

**Track boundary (2026-07-05):** `P0` is a native HQL correctness/exposure milestone and remains intentionally separable from the SQLite substrate implementation track. SQLite may change the execution strategy for `P2/P3`, but `P0` semantics in this spec must remain implementable and shippable without waiting for `S0/S1`. See [SPEC--SQLITE-SUBSTRATE-S0-S1](SPEC--SQLITE-SUBSTRATE-S0-S1.md).

**Version identity:** "HQL v2" = the language state after the P0, P1, and P2 PRs are merged. Each PR ships a self-consistent subset (P0-only is a valid intermediate state); this spec describes the union. P3 (`SEARCH TEXT`) is **reserved syntax**, specified only after its ADR (P3-T0).

---

## 1. Compatibility contract

1. **Every semantically valid v1 query parses in v2 and returns the same result**, with exactly one documented exception (rule 2). Existing test files are the enforcement mechanism: they must pass **unedited**.
2. **The only breaking change:** a numeric token that overflows/fails value-parsing is now a **parse error**, not a silent default (v1 §4.5). Rationale: silently converting `K 99999999999999999999` into `K 5` is a wrong-answer generator. `LIMIT`'s saturate-to-max stays (deliberate v1 semantics).
3. All grammar additions are optional clauses or new alternatives — omitting them reproduces v1 behavior (modulo the defect fixes, which change *wrong* behavior only: the discarded target now errors-or-acts, per §2.1).
4. No planner, no EXPLAIN, no on-disk format change, no new required fields on any surface.

---

## 2. P0 — command deltas (correctness & exposure)

`P0` is the last phase in this spec whose implementation contract is fully substrate-independent: it is about fixing wrong-answer behavior and exposing existing native engine knobs/capabilities through HQL, REST, NAPI, and MCP's shared `execute_hql` funnel.

### 2.1 SEARCH / hybrid: search-by-node (target becomes meaningful) — ⚙ design-gated (P0-T0)

```
SEARCH [~]<target> [SIMILAR TO [ <vector> ]] K <k> [EF <n>] [OVERSAMPLE <n>]
       [IN <collection>] [LANGUAGE "…"] [AS OF "…"] [<clauses>]

MATCH  [~]<target> [SIMILAR TO [ <vector> ]] ALPHA <a> [K <k>] [EF <n>] [OVERSAMPLE <n>]
       [IN <collection>] [LANGUAGE "…"] [AS OF "…"] [<clauses>]
```

- **Literal vector present** → exactly v1 behavior (vector wins; target is documentation-only). Byte-identical results.
- **`SIMILAR TO` omitted** → *search-by-node*: the (fuzzy-)resolved target must name a live node; its **stored embedding** becomes the query vector (fetched via the collection resolution rule below). "More like this node" without any client-side embedding.
  - Collection resolution: the node's embedding lives in the collection it was ingested into; an explicit `IN <collection>` must match that collection's dimension or the query errors.
  - Target resolves to nothing (even after fuzzy) → **error** `"HQL: target '<t>' does not resolve to a node and no vector was given"`. Never an empty-result silent success.
  - Resolved node has no stored embedding → same error class, naming the cause.
- The dead `_resolved` binding is gone: resolution output is always consumed or surfaced as an error.

### 2.2 Hybrid `K <k>` (P0-T2)

Optional on the hybrid form; **default 10** preserves v1 output for existing queries. Applies before clauses; `LIMIT` still only shrinks.

### 2.3 `EF <n>` / `OVERSAMPLE <n>` (P0-T3)

Optional on SEARCH and hybrid. Map 1:1 onto `HybridSearchInput.ef_search` / `.oversample`. Omitted ⇒ `None` ⇒ the engine's existing resolution chain (per-query → per-collection → global) is untouched. These expose the P32 recall lever and the rerank oversample knob to the query language.

### 2.4 TRAVERSE direction + multi-rel (P0-T4)

```
TRAVERSE FROM [~]<seed> DEPTH <d> REL <rel>[|<rel>…] | INFER(<rel>)
         [DIRECTION in|out|both] [AS OF "…"] [<clauses>]
```

- `DIRECTION` omitted ⇒ `out` (v1). Maps to `NeighborInput.direction`.
- `REL a|b|c` maps to the engine's `rels` set (union). Single rel unchanged. `INFER(…)` stays single-rel.
- The `a|b` alternation idiom is **the same syntax** as pattern-edge alternation (§3.4) — one idiom across HQL.

### 2.5 Strict numeric errors (P0-T5)

Any numeric token whose value-parse fails (u32/f64 overflow, etc.) is a parse error naming the field: `"HQL Parse Error: K value out of range"`. Exhaustive site list fixed at P0-T0. Fuzz invariant unchanged: errors allowed, panics never.

### 2.6 Colon ids — ⚙ design-gated (P0-T0)

Recommended: `qualified_id = identifier (":" identifier)+` accepted for `seed`/`target` **only** (never inside pattern syntax, where `:` introduces labels), so `TRAVERSE FROM user:5 …` parses. If the gate rejects (PEG risk), the fallback decision is: quoting is mandatory and all docs/examples are corrected (P0-T9). Either way the v1 broken-example state ends.

### 2.7 Executor guarantees (no syntax)

- One `Utc::now()` per query (not per edge) in `match_pattern` and `neighbors` — a query sees a single consistent "current" instant (P0-T6).
- `{id:"…"}`-anchored patterns seed by direct interned-id lookup, O(1) instead of O(N); result-identical to the scan (P0-T7).

---

## 3. P1 — pattern power

### 3.1 Variable-length paths (P1-T1) — semantics ⚙ design-gated (P1-T0)

```
-[r:REL*<min>..<max>]->     e.g.  -[:LINK*1..6]->
```

- Bounds required in v2 (`*` alone = `1..<default cap>` only if P1-T0 sets one; otherwise required).
- Expansion = the existing hop loop iterated `min..=max` times; **visited-set policy** (per-row trail vs global BFS) is fixed by P1-T0 with worked row counts on diamond and cycle fixtures — the spec adopts whichever the gate proves, and `*1..1` MUST be row-identical to a plain single hop.
- Binds the terminal node only; no path variable in v2.
- Direction and rel filter apply at **every** step.
- This makes the P26/P30 competitor-bench query shape expressible: `MATCH (a {id:"g0"})-[:LINK*1..6]->(b) RETURN b.id LIMIT 1000`.

### 3.2 Frontier cap (P1-T2) — policy ⚙ design-gated (P1-T0)

A per-expansion-round row cap (`PATTERN_FRONTIER_CAP`, recommended default 100k rows). Recommended policy: **hard error** with actionable text ("frontier exceeded at hop N — add label/prop constraints or lower *max"), not silent truncation. Rows, not visited nodes, are counted.

### 3.3 Repeated-variable identity join (P1-T4)

A hop whose node/edge variable is already bound requires the candidate to **be** that entity (u32/eid comparison). `(a)-[:KNOWS]->(b)-[:KNOWS]->(a)` now means a real cycle through the same `a`. Applies to node and edge variables. *(Semantics change from v1's independent binding — v1 documented that as a non-goal, and no shipped test depends on independent rebinding; the cypher test suite gains explicit fixtures.)*

### 3.4 Rel-type alternation (P1-T5)

`-[:R1|R2]->` — membership over the listed types; empty/absent = any. Edge-position only; node `:Label` syntax is unchanged (single label).

### 3.5 Lazy bindings (P1-T3, executor guarantee)

Expansion carries entity references; JSON is materialized only for variables referenced by WHERE / ORDER BY / RETURN (`RETURN *`/no-RETURN materializes all named vars). Output is **byte-identical** to v1 for every query — this is a pure cost change, enforced by the unedited cypher test oracle.

---

## 4. P2 — clause ergonomics

### 4.1 `OR` + parentheses in WHERE (P2-T1)

```
WHERE <expr>        expr := term (OR term)* ; term := factor (AND factor)* ; factor := pred | "(" expr ")"
```

- `AND` binds tighter than `OR`; parentheses override. Applies to **both** clause systems (plain + pattern-qualified).
- Null semantics unchanged at the leaf: null/missing/mismatch ⇒ leaf false; the tree combines plain booleans (SQL three-valued logic collapses to this under our leaf rule; documented).
- v1 AND-only queries evaluate identically.

### 4.2 `RETURN count(*)` (P2-T2)

- Both clause systems. Result shape: `[{"count": <n>}]`, counted **after WHERE**.
- Combining `count(*)` with `ORDER BY` or additional RETURN fields is a **parse error** (no group-by in v2).

### 4.3 Label index (P2-T3, executor guarantee)

`(:Label)` anchors seed from a maintained `label_idx` (u32 sets) intersected with validity, instead of the O(N) scan. Result-identical; index is rebuilt on load (not persisted — `state.json` unchanged). Every node-mutation path updates it (audited list in the PR).

### 4.4 CONTEXT clauses (P2-T4 — droppable)

Only if a crisp semantics emerges (filter-atoms-then-budget). Otherwise formally dropped with rationale recorded in the ADR; this spec then carries "CONTEXT takes no clauses" forward as permanent.

---

## 5. P3 — reserved: `SEARCH TEXT` (specified by ADR only)

`TEXT` is a **reserved keyword** adjacent to the SEARCH form. Its semantics (in-engine lexical/BM25+RRF vs client-side embed hook vs node-anchored-only) are decided by ADR--GENESISDB-HQL-TEXT-QUERY (P3-T0), which MUST reconcile with ADR--GENESISDB-COMPETITIVE-SUPERIORITY Wave 2.5 (one lexical engine, not two). Non-negotiable regardless of choice: **a text query with no available backing capability fails with a named error, never a silent empty result.**

---

## 6. Error model (v2 consolidated)

| Condition | v2 behavior |
|---|---|
| Grammar mismatch | `HQL Parse Error: <pest error + position>` |
| Numeric overflow / bad value | parse error naming the field (**new**) |
| Unresolvable target, no vector | execution error naming the target (**new**) |
| Node without embedding, no vector | execution error naming the cause (**new**) |
| Dim mismatch (query/collection) | execution error (unchanged) |
| Frontier cap exceeded | execution error with hop + remedy (**new**) |
| `count(*)` + ORDER BY/fields | parse error (**new**) |
| Anything, any input | **never a panic** (fuzz invariant, corpus extended per phase) |

## 7. Non-goals (carried forward, on record)

No cost-based planner or EXPLAIN; no branching/comma patterns, OPTIONAL MATCH, path variables, shortest-path; no aggregation beyond `count(*)`; no prop-value secondary indexes (labels only); no in-engine embedding model (mobile weight budget). Any of these requires a new ADR, not a plan amendment.

## 8. Conformance

**Every syntax example in this spec must exist as a passing test** by the end of its phase (the spec-example sweep in each phase's DoD — see [REPORT--HQL-V1-VS-V2](REPORT--HQL-V1-VS-V2.md) §2). When all three PRs are merged, SPEC--HQL-V2 is retitled `status: current` and SPEC--HQL-V1 is archived as historical.
