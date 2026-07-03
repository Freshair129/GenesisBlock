---
proposed_id: ADR--GENESISDB-HQL-FILTER-PROJECTION
type: adr
status: candidate
aliases:
  - ADR
phase: 2
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-06-29T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-HQL-FILTER-PROJECTION

## Context

HQL today is four rigid, fixed-shape commands (`SEARCH`, `TRAVERSE`,
`MATCH`/hybrid, `CONTEXT`) that **dispatch directly to Storage methods** — there
is no logical plan or executor (the old `LogicalPlanner` was removed in MARK XIV).
The result of `SEARCH`/`MATCH`/`TRAVERSE` is a `Vec<NeighborOutput>`; `CONTEXT`
returns a `ContextPackage`.

There is no way to:
- filter results by props / labels / time / side,
- choose which fields come back (projection),
- order or cap the result set beyond the retrieval `K`.

This is the "path 2 + path 4" improvement: **filter + projection + ergonomics**,
chosen over Cypher-style pattern matching because the latter would require
reintroducing a full execution/planning layer (a separate, larger ADR).

## Decision

Add four **optional trailing clauses** to the node-list commands (`SEARCH`,
`MATCH`, `TRAVERSE`), applied as a single **post-dispatch transform** over the
`Vec<NeighborOutput>` the command already produces. **No planner is introduced** —
the engine still dispatches to one Storage method, then filters/orders/projects
the returned list in memory.

### Grammar (appended after each existing command body)

```
<command> [ WHERE <pred> (AND <pred>)* ] [ ORDER BY <field> (ASC|DESC)? ] [ LIMIT <n> ] [ RETURN <field> ("," <field>)* ]
```

- **pred** := `<field> <op> <value>`
- **field** := `id` | `label` | `score` | `depth` | `prop.<key>` (dotted access into `props`)
- **op** := `=` | `!=` | `<` | `<=` | `>` | `>=` | `CONTAINS` | `STARTSWITH`
- **value** := `string_lit` | `number`

### Semantics

- **WHERE** — conjunction of predicates (AND-only in v1). Evaluated per
  `NeighborOutput`:
  - `label = "Message"` → true if `node.labels` contains `"Message"`
    (`!=` → does not contain).
  - `prop.side = "them"` → compares `node.props["side"]`.
  - `score`, `depth`, `id` read the obvious fields.
  - `CONTAINS` / `STARTSWITH` are string-only; comparison ops coerce to number
    when both sides parse numerically, else compare as strings.
  - **SQL-style null handling:** a missing field, JSON `null`, or a type
    mismatch makes the predicate `false` for **every** operator — *including
    `!=`* (NULL comparison is UNKNOWN, so the row is excluded). Never errors.
    e.g. `WHERE prop.side != "me"` excludes rows that have no `side` at all.
- **ORDER BY** — sort by `<field>`; default `ASC` (use `DESC` to reverse).
  Applied after WHERE.
- **LIMIT** — truncate after WHERE + ORDER BY. Distinct from `K`/`DEPTH`, which
  bound *retrieval*; `LIMIT` bounds the *final* set.
- **RETURN** — projection. When present, each hit becomes a flat object of the
  selected fields (e.g. `RETURN id, prop.text, score`). When omitted, the full
  `NeighborOutput` is returned as today (**backward compatible**).

### Scope (v1 non-goals)

- No `OR` / parentheses in `WHERE` (AND-only). 
- No aggregation (`count`, `group by`), no joins, no multi-command chaining.
- `CONTEXT` is **not** filtered in v1 (it returns a package, not a list); revisit
  separately.
- Pattern matching (`(a)-[r]->(b)`) is explicitly out — that is the path-1 ADR.

### Why this stays planner-free

All four clauses are pure transforms on the already-materialised result list:
`results = dispatch(...); apply_where; sort; truncate; project`. No cost model, no
join planning, no intermediate plan nodes. This preserves the engine's
"dispatch-directly-to-Storage" design while delivering the most-requested
expressiveness.

## Consequences

- **Positive:** Real filtering/projection/ordering for the primary query
  language with minimal risk and no architectural commitment. Automatic
  NAPI/REST parity — both front-ends call `execute_hql`, so the clauses land on
  both surfaces at once.
- **Positive (consumer):** Unblocks NotiKeeper-class queries over raw HQL, e.g.
  "latest 10 inbound messages about X" or "messages in this thread, text only".
- **Negative:** Filtering is **post-retrieval** — `WHERE` runs *after* `K`/`DEPTH`,
  so a restrictive predicate may return fewer than `K` rows. Callers over-fetch
  with a larger `K` when filtering hard. (A pushed-down filter is a future
  optimisation, not v1.)
- **Negative:** Grows the grammar/AST surface and adds an evaluator to maintain.
- **Migration:** Purely additive grammar; existing queries parse and behave
  identically. No on-disk format change.
- **Bonus fix:** the digit rules (`k`, `depth`, `budget`, `limit_n`) were
  non-atomic `{ ASCII_DIGIT+ }`, which let pest's implicit WHITESPACE creep into
  the matched span (e.g. `"2 "`); the subsequent `.parse()` failed and silently
  fell back to the default — this was the long-standing **"DEPTH always parses to
  1"** bug. Making them atomic (`@{ ASCII_DIGIT+ }`) fixes it, so `DEPTH`, `K`,
  and `BUDGET` are now honored as written. (Required anyway for `LIMIT` to work.)

### Post-retrieval filtering — usage warning

> **WHERE is post-retrieval in v1.** For `SEARCH`/`MATCH`, `K` controls the
> candidate pool that `WHERE` then filters; for `TRAVERSE`, `DEPTH` does. If a
> filter is selective, **increase `K`/`DEPTH`** so enough candidates survive —
> e.g. `SEARCH ... K 100 WHERE prop.side = "them" LIMIT 10`. A pushed-down,
> index-aware filter is deferred to a future phase.

### Example queries (NotiKeeper)

```sql
SEARCH ~weather SIMILAR TO [..] K 50
  WHERE prop.side = "them" AND prop.time > 1782000000000
  ORDER BY prop.time DESC LIMIT 10

TRAVERSE FROM user:5 DEPTH 2 REL SENT_BY
  WHERE label = "Message"
  RETURN id, prop.text

MATCH topic SIMILAR TO [..] ALPHA 0
  WHERE label = "Notification" AND prop.text CONTAINS "โอน"
  LIMIT 5
```

---
### Related Links
- **Query pipeline:** `src/query/hql.pest`, `src/query/ast.rs`, `Storage::execute_hql`
- **Path 1 (shipped):** Cypher-style pattern matching landed *without* a planner —
  linear path expansion over the graph indices. See
  [ADR--GENESISDB-HQL-CYPHER-PATTERNS](ADR--GENESISDB-HQL-CYPHER-PATTERNS.md).
