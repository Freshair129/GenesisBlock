# Change Request: GRL Coverage Report and Context Fitness Diagnostics

## CR ID

`CR--GRL-COVERAGE-AND-CONTEXT-FITNESS`

## Owner

GenesisBlockDB / GRL / Retrieval Engine

## Scale

L2-L3

## Summary

Add explicit coverage reporting and context fitness diagnostics to the Graph Retrieval Layer (GRL). The system must never return partial, compressed, truncated, or incomplete context as if it were complete.

## Background

GRL currently defines context retrieval through H0-H6 tiers, hybrid semantic expansion, graph traversal, ranking, token budget estimation, and SuperNode compression. However, it needs a formal output contract for incomplete retrieval and over-budget conditions.

## Scope

In scope:

* Add `CoverageReport` to `ContextPackage`
* Detect whether retrieval is complete, compressed, partial, or over budget
* Diagnose H0/H1/H2 token explosion
* Identify dense target nodes and dense neighbor nodes
* Return recommended follow-up queries
* Return context fitness warnings

Out of scope:

* Agent branching
* Runtime rollback
* Memory distillation
* Belief revision
* KV cache handling

## Proposed Changes

Extend `ContextPackage` with:

```json
{
  "coverage": {
    "status": "complete | compressed | partial_budget | partial_frontier | partial_missing_edges | needs_decomposition",
    "reason": "string",
    "requested_tier": "H2",
    "requested_hops": 2,
    "covered_hops": 2,
    "budget": 12000,
    "token_estimate_full": 38400,
    "token_estimate_returned": 11900,
    "nodes_found": 184,
    "nodes_returned": 61,
    "edges_found": 420,
    "edges_returned": 139,
    "supernodes_used": 7,
    "dropped_nodes": [],
    "missing_frontiers": [],
    "recommended_followups": []
  }
}
```

Add over-budget diagnostic categories:

* `target_too_large`
* `neighbor_too_large`
* `too_many_neighbors`
* `edge_explosion`
* `budget_too_small`
* `frontier_cap_exceeded`
* `missing_or_weak_graph_structure`

Add context fitness output:

```yaml
context_fitness:
  target: FEAT--TAX-DEDUCT
  default_budget: 12000

  h0:
    token_estimate: 9200
    status: warning

  h1:
    token_estimate: 38400
    status: fail
    reason: direct_neighbors_exceed_task_budget

  recommendation:
    - split dense feature node into feature summary, SRS, API, TEST, ADR
    - use delegated exploration for current execution
```

## Acceptance Criteria

1. Every GRL `ContextPackage` includes coverage status.
2. Partial context is never returned as complete.
3. H1 over-budget retrieval returns diagnostics.
4. Dense nodes are identified.
5. Recommended follow-up queries are returned.
6. SuperNode compression is reported explicitly.
7. Existing HQL v2 behavior remains compatible unless an HQL amendment is separately approved.

## Related Specs

* `SPEC--GRAPH-RETRIEVAL-LAYER`
* `SPEC--HQL-V2`
* `C4--GENESISDB-ARCHITECTURE`
