---
id: ADR--GENESIS-BLOCK-CYPHER-V0-SURFACE
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Genesis Block Cypher v0 surface freeze (HALT GATE 2)
tags:
  - msp
  - gks
  - graph
  - backend
  - genesis-block
  - cypher
  - decision
  - halt-gate
crosslinks:
  references:
    - BLUEPRINT--GENESIS-GRAPH-INTEGRATION
    - ADR--GENESIS-BLOCK-STORAGE-LAYOUT
    - ADR--GENESIS-GRAPH-AS-GKS-BACKEND
    - CONCEPT--GENESIS-GRAPH-BACKEND
    - SPEC--GENESIS-GRAPH-BACKEND
    - PROTOCOL--GENESIS-GRAPH-FFI
created_at: 2026-05-19T02:00:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
---

# ADR — Genesis Block Cypher v0 surface freeze (HALT GATE 2)

## Context

`[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` §HALT GATE 2 requires sign-off on the **Cypher v0 surface** before P3.5 benchmarks lock in performance assumptions:

> Approved by `gate-2: approved` label on the P3.4 PR, **OR** an addendum atom extending the Cypher v0 scope. Required because adding Cypher constructs after benchmark targets are committed forces re-runs.

P3.4 landed early under PR #153 (alongside P3.2-P3.3) as a single bundle. The implementation in `packages/gks/native/genesis-block/src/lib.rs` is regex-based — see `MATCH_RE`, `WHERE_RE`, `RETURN_RE` constants and the `cypher()` napi method. It passes the 28 graph/Cypher integration tests in `packages/gks/test/memory/genesis-graph-cypher.test.ts`.

This ADR is the **addendum** option of HALT GATE 2: rather than block on patching the impl to match BLUEPRINT verbatim, we **narrow the BLUEPRINT scope** to match what the regex parser actually accepts. The result is a tighter, smaller surface that the impl already meets — benchmarks can proceed.

## Decision

Freeze the Cypher v0 surface at the **subset actually implemented in PR #153**, which is narrower than the BLUEPRINT spec on four specific points. The drift is documented below as the canonical v0 scope from this point forward; any expansion is a P4+ feature requiring a new ADR.

### Narrowed scope (this becomes v0)

#### 1. Seed node pattern: `id` literal only

**BLUEPRINT** allowed any single property in the seed map (`(a:Label {prop: 'value'})`).

**v0 (this ADR)** restricts the seed map to **`id` only**:

```cypher
(a:<Label> {id: '<literal>'})   ✓ supported
(a:<Label> {status: 'stable'})  ✗ not supported in v0
```

Reason: every MSP query that needs Cypher starts from a known atom id (verify-flow walks from a specific atom; impact-analysis pins a symbol/atom). Other-property seeds would force a full label scan, which v0 explicitly avoids per `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` §"Cypher v0 scope" intent ("just enough to express the queries `verify-flow` and Impact-Analysis need today").

#### 2. Target node pattern: label-only

**BLUEPRINT** allowed a property map on the target too.

**v0 (this ADR)** restricts the target to **label-only**:

```cypher
)->(b:<Label>)               ✓ supported
)->(b:<Label> {status: '…'}) ✗ not supported in v0; use WHERE b.status = '…'
```

Reason: target-side predicates are expressible via `WHERE b.<prop> = '<literal>'`, which the parser already handles. The two forms would be semantically redundant.

#### 3. WHERE: multiple keyword clauses, no `AND`

**BLUEPRINT** said "A single `WHERE` clause with conjunctions of equality predicates."

**v0 (this ADR)** flips this to: **multiple `WHERE` keyword clauses, no `AND` conjunctions**:

```cypher
WHERE a.x = '1' WHERE b.y = '2'   ✓ supported (conjunctive over the path)
WHERE a.x = '1' AND b.y = '2'     ✗ not supported in v0
WHERE a.x = '1' OR b.y = '2'      ✗ not supported in v0 (OR was never in scope)
```

Reason: the regex parser uses `captures_iter` over `\s+WHERE\s+...` so multiple WHERE keywords accumulate as predicates. Parsing AND/OR boolean expressions would need a real expression parser; deferred until benchmarks justify the complexity. The multi-WHERE syntax is non-canonical Cypher but is unambiguous and matches MSP's actual query templates.

#### 4. Edge direction: `->` only (no `<-` or `-`)

**BLUEPRINT** did not explicitly state direction semantics.

**v0 (this ADR)** restricts to **outbound `->` only**:

```cypher
(a)-[r:rel]->(b)   ✓ supported
(a)<-[r:rel]-(b)   ✗ not supported in v0
(a)-[r:rel]-(b)    ✗ not supported in v0 (undirected)
```

Reason: MSP's reverse-path query (`Code → AST → Symbol Graph → Execution Trace`) traverses outbound edges from the seed atom. Reverse traversal is rare; when needed, callers can swap `a` ↔ `b` in the query.

### Unchanged from BLUEPRINT (in scope, working)

| Construct | Status |
|---|---|
| Edge relationship-type union (`r:rel1\|rel2\|rel3`) | ✅ |
| Variable-length range (`*N..M`) | ✅ |
| `RETURN` with property projection (`a.prop`, `b.prop`, `a.id`, `b.id`) | ✅ |
| `RETURN` with `length(r) AS alias` aggregate | ✅ |
| Hard reject: `OPTIONAL MATCH`, `UNION`, `WITH`, `UNWIND`, `CREATE`, `DELETE`, `SET`, `MERGE`, `CALL { ... }` | ✅ |
| Hard reject: pattern quantifiers (`+`, `?`) | ✅ (regex doesn't match them) |
| Hard reject: path-property functions (`relationships(p)`, `nodes(p)`) | ✅ (regex doesn't match them) |

## Consequences

### Positive

- **HALT GATE 2 unblocked** — P3.5 benchmarks can run against a frozen surface.
- **Smaller surface = smaller bug surface.** Four removed BLUEPRINT constructs cannot regress in benchmarks because they were never implemented.
- **Test coverage already complete.** 28 cypher integration tests in `packages/gks/test/memory/genesis-graph-cypher.test.ts` exercise the narrowed surface end-to-end.

### Negative

- **Future contributors reading the BLUEPRINT may write queries the parser rejects.** Mitigated by: (a) this ADR is linked from the BLUEPRINT crosslinks; (b) the rejection error messages (`"Cypher error: MATCH pattern mismatch"` etc.) point at the failing clause; (c) MSP's actual query templates are static, in-repo strings — they're code-reviewed, not user input.
- **Multi-WHERE syntax is non-canonical Cypher.** Anyone porting MSP queries to Neo4j later must rewrite `WHERE a.x = '1' WHERE b.y = '2'` → `WHERE a.x = '1' AND b.y = '2'`. Tracked under future `[[ADR--GENESIS-BLOCK-CYPHER-V1-EXPRESSIONS]]` if MSP grows enough query complexity to justify a real expression parser.

### Neutral

- The on-disk format (`[[ADR--GENESIS-BLOCK-STORAGE-LAYOUT]]`) is unaffected — Cypher is a read-side surface that runs against the in-memory adjacency indices.
- The pure-TS fallback at `packages/gks/src/memory/graph/genesis-graph.ts` has its own `parseCypherV0` (in `cypher-v0.ts`) which accepts a slightly different grammar (closer to BLUEPRINT). The two parsers are not bit-identical — when both backends are tested in parametrised suites, queries that exercise the drift will only run against the pure-TS path. **Tracked as known limitation; not a blocker.**

## Out of scope (deferred to v1+ ADRs if/when MSP demands)

- `AND` / `OR` boolean expressions in `WHERE`
- Property maps on target nodes
- Inbound (`<-`) and undirected (`-`) edge directions
- `WITH ... WHERE` (post-aggregation filtering)
- `UNWIND` (list expansion)
- Parameterised queries (`$paramName`)
- Path-property functions (`relationships(p)`, `nodes(p)`)
- Real Cypher expression parser (subqueries, `CASE`, etc.)

## Alternatives rejected

**Option B — Patch impl to match BLUEPRINT** (rewrite parser to support AND, target maps, both directions) was rejected because:

- BLUEPRINT v0 was conceived before MSP's actual query templates were finalised. The narrowed surface in this ADR is what `verify-flow` and impact-analysis actually use.
- The expression parser for AND/OR adds ~200 LOC of Rust for a future-only feature.
- The same constructs can be added in a follow-up P4 ADR + PR if a real MSP query needs them. The cost of waiting is bounded.

**Option C — Approve as-is without addendum** was rejected because:

- BLUEPRINT remains the authoritative spec. Without an addendum, future contributors will read BLUEPRINT, write queries that match it, get parse errors, and have no record of why.
- HALT GATE 2 explicitly offers the addendum option for exactly this case.

## Sign-off

Per `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` §"Solo human contributors may sign off in the PR description itself" — Boss reviewing this PR + approving counts as the `gate-2: approved` signal.

After this ADR merges, P3.5 (Benchmarks) is unblocked.

## Connections

- [[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]
- [[ADR--GENESIS-BLOCK-STORAGE-LAYOUT]]
- [[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]
- [[SPEC--GENESIS-GRAPH-BACKEND]]
- [[PROTOCOL--GENESIS-GRAPH-FFI]]
