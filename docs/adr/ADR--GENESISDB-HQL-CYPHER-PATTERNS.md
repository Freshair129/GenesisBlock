---
proposed_id: ADR--GENESISDB-HQL-CYPHER-PATTERNS
type: adr
status: current
aliases:
  - ADR
phase: 1
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-07-03T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-HQL-CYPHER-PATTERNS

> Status note (2026-07-20): shipped in the current codebase as the planner-free
> linear-path `MATCH (<pattern>)` command family.

## Context

This is **path 1** of the HQL roadmap — the piece that
[ADR--GENESISDB-HQL-FILTER-PROJECTION](ADR--GENESISDB-HQL-FILTER-PROJECTION.md)
(path 2 + path 4) explicitly deferred: *Cypher-style graph pattern matching*
(`(a)-[r]->(b)`). The filter/projection ADR chose post-dispatch clauses over
pattern matching because "the latter would require reintroducing a full
execution/planning layer."

HQL today is four fixed-shape commands (`SEARCH`, `TRAVERSE`, `MATCH`/hybrid,
`CONTEXT`) that dispatch directly to one Storage method. `TRAVERSE` is the only
graph command and it is single-anchor, single-rel, single-direction, fixed
`DEPTH` BFS (`Storage::neighbors`). There is no way to express a **multi-hop,
multi-variable pattern** with per-position label/property/direction constraints
and per-variable projection — e.g. "people `a` who SENT a message `m` that
MENTIONS topic `t`".

## Decision

Add a fifth command, **`MATCH <graph_pattern>`**, that matches a **linear path
pattern** by deterministic left-to-right expansion over the existing graph
indices (`out_idx`/`in_idx`/`edges`/`nodes`). **No cost-based planner is
introduced** — expansion order is exactly the written order of the pattern, so
the engine keeps its "no query planner" property. This is the pattern-matching
analogue of the filter/projection ADR's "planner-free" stance.

### Keyword disambiguation (no breaking change)

`MATCH` is already the hybrid command (`MATCH <target> SIMILAR TO [..] ALPHA n`).
A Cypher pattern always begins with `(`; a hybrid target is an identifier/string
and never starts with `(`. The grammar orders `match_pattern` **before** `hybrid`
in the top-level choice, so `MATCH (` → pattern and `MATCH foo SIMILAR` →
hybrid. PEG backtracking makes this unambiguous and existing hybrid queries parse
byte-identically.

### Grammar (v1)

```
MATCH <node> ( <edge> <node> )*
      [ WHERE <var.field> <op> <value> (AND ...)* ]
      [ ORDER BY <var.field> (ASC|DESC)? ]
      [ LIMIT <n> ]
      [ RETURN <var|var.field> ("," ...)* ]
      [ AS OF "<ts>" ]
```

- **node** := `( var? (:Label)? ({k:v, ...})? )` — every part optional; `()` is an
  anonymous wildcard node, `(:Msg)` a label-only node, `(a {side:"them"})` a
  property-constrained node.
- **edge** := direction-wrapped optional detail:
  - `-[var? :Type?]->` outgoing, `<-[..]-` incoming, `-[..]-` either direction.
  - `-->`/`<--`/`--` are the detail-free forms.
- **var.field** (clauses) := `a` (whole bound node/edge) | `a.id` | `a.label` |
  `a.prop.<key>`. For an **edge** variable, `.label` resolves to its `rel` type
  and `.prop.<key>` into edge props.
- **op / value**: identical to the filter/projection ADR
  (`= != < <= > >= CONTAINS STARTSWITH`; `string_lit | number`).

### Semantics

- **Anchor.** The first node pattern is matched against **all live nodes**
  filtered by its label/prop constraints (an `()` anchor scans every node — see
  "post-retrieval" note). Each surviving node seeds one binding row.
- **Expansion.** For each `<edge> <node>` hop, every partial row's current
  endpoint is expanded through the chosen direction's index, filtered by edge
  `rel` type and the next node pattern's label/props; each surviving neighbour
  extends the row (Cartesian). Direction picks the far endpoint exactly as
  `neighbors()` does (the endpoint that is not the current node).
- **Node/edge constraint** — label: `labels` contains it; prop `{k:v}`:
  `props[k] == v` (numeric if both parse numeric, else string). A `{k:v}` inline
  constraint is exact-equality sugar; richer comparisons go in `WHERE`.
- **Bindings → rows.** A completed row binds each **named** variable to its
  node/edge. `WHERE` (AND-only), `ORDER BY`, `LIMIT`, `RETURN` are applied over
  these rows, reusing the filter/projection ADR's operator + null semantics
  (missing/null/type-mismatch ⇒ predicate false for every op incl. `!=`;
  ORDER BY nulls-last; LIMIT after filter+order).
- **RETURN shape.** Omitted / `RETURN *` ⇒ one object per row mapping each named
  variable to its full node/edge JSON. `RETURN a, b.prop.text` ⇒ a flat object
  per row keyed `a`, `b.text`.
- **Temporal.** A trailing `AS OF "<ts>"` applies the same bitemporal validity
  check (`is_valid_as_of`) to every node and edge in the pattern; retracted edges
  are hidden in the current view exactly as in `neighbors()`.

### Scope (v1 non-goals)

- **Linear paths only.** No branching / comma-separated patterns
  (`(a)-->(b), (a)-->(c)`), no cycles that reuse a bound variable as a join
  constraint (a repeated variable name in v1 binds independently per position;
  it does not enforce identity).
- **No variable-length paths** `-[*2..5]->` (that is a v2 add — the expansion
  loop is already the natural host for it).
- **No relationship-type alternation** `-[:R1|R2]->`, no `OR` in `WHERE`, no
  aggregation / `count` / `group by`.
- Node **props** inline constraints are equality-only.

### Why this stays planner-free

Expansion is a fold over the pattern in written order:
`frontier = anchor(start); for hop { frontier = expand(frontier, hop) }`. No cost
model, no join reordering, no intermediate plan nodes — the same discipline as
the filter/projection ADR. The one honest cost is the **anchor scan** (O(N) over
live nodes for the first pattern), mirroring the engine's existing post-retrieval
philosophy: constrain the anchor (`:Label` / `{prop}`) to shrink it.

## Consequences

- **Positive:** Real multi-hop, multi-variable graph queries in HQL — the
  headline gap vs. Kuzu/Neo4j — without a planner. Automatic NAPI + REST parity
  (both call `execute_hql`; `executeHql`/`/v1/query/hql` gain patterns with no
  signature change; return type is already `any`/JSON).
- **Negative (anchor scan):** An unconstrained `()` anchor is O(N) over nodes.
  Documented; the fix (label/prop index-assisted anchor) is a future
  optimisation, not v1 — same posture as post-retrieval WHERE.
- **Negative (fan-out):** Cartesian expansion on high-degree hubs can be large;
  `LIMIT` caps the **output**, not intermediate frontier. A frontier cap is a v2
  guardrail.
- **Migration:** Purely additive grammar + one new `HqlCommand` variant. Existing
  queries (incl. all `MATCH ... SIMILAR` hybrids) parse and behave identically.
  No on-disk format change.

### Example queries

```
MATCH (a:User)-[:SENT]->(m:Message)
  WHERE m.prop.side = "them"
  RETURN a.id, m.prop.text
  LIMIT 10

MATCH (u {id:"user:5"})-[:SENT]->(m)-[:MENTIONS]->(t:Topic)
  WHERE t.label = "Topic"
  RETURN u, m, t

MATCH (a)<-[:REPLY_TO]-(b)          // b replies to a (incoming edge)
  RETURN a.id, b.id
```

---
### Related Links
- **Parent / prior path:** [ADR--GENESISDB-HQL-FILTER-PROJECTION](ADR--GENESISDB-HQL-FILTER-PROJECTION.md) (path 2 + 4; deferred path 1 to here)
- **Query pipeline:** `src/query/hql.pest`, `src/query/ast.rs`, `Storage::execute_hql` + `Storage::match_pattern`
- **Graph primitives reused:** `Storage::neighbors` (expansion model), `out_idx`/`in_idx`/`edges`/`nodes`, `get_u32`, `is_valid_as_of`
- **Tests:** `tests/hql_cypher_tests.rs`
