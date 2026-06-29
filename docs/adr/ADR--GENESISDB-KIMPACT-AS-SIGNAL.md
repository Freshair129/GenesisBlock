---
proposed_id: ADR--GENESISDB-KIMPACT-AS-SIGNAL
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
amends: ADR--GENESISDB-KIMPACT-ALGORITHM
---

# ADR--GENESISDB-KIMPACT-AS-SIGNAL

## Context

[[ADR--GENESISDB-KIMPACT-ALGORITHM]] adopted K-Impact as the **primary ranking
engine**, and `hybrid_search` blends it into every vector query:

```
score = similarity × (1 - alpha) + impact × alpha
```

where `impact = DD×0.5 + AS×0.3 + SC×0.2` (degree-depth, axiomatic-strictness by
governance tier, stability-confidence). This is correct *for the Knowledge Graph
(GKS) domain it was designed for* — MASTER/SPEC/ADR tiers encode real authority
there.

The problem surfaced when the engine was embedded in a **non-governance domain**.
The NotiKeeper app (chat/notification archive) mirrors messages/users/threads as
nodes and runs semantic search over them. Two failures emerged:

1. **The tier dimension is inert outside GKS.** `Tier::from_labels` defaults every
   node without a MASTER/SPEC/ADR label to `USER` (`as_score = 0.3`). On chat data
   *every* node is USER, so the 30% authority weight becomes a constant — it adds
   nothing to ranking. K-Impact silently degenerates to plain degree-centrality.

2. **Coupling K-Impact into the default ranking is a footgun.** NotiKeeper called
   `hybridSearch(..., alpha: 1.0)` intending "vector only" — but `alpha=1.0` means
   *K-Impact only*, so search ranked by near-constant authority and the vector
   similarity was discarded entirely. The relevance signal the caller actually
   wanted was silently thrown away.

More broadly: baking a *ranking policy* into the storage core fixes a single
opinionated fusion (weighted-sum of similarity + one graph metric) for all callers.
The modern standard for combining heterogeneous retrievers is **RRF / learned
rerankers at the application layer**, where the app owns domain-specific signals
(recency, personalization, cross-encoder relevance) that the engine cannot know.

## Decision

**Demote K-Impact from "default ranking policy" to "opt-in signal."** Keep the
*computation* in the engine; move the *ranking decision* to the caller.

Concretely, for the current engine version:

1. **`hybrid_search` default `alpha` becomes `0.0`** (pure vector similarity).
   K-Impact blending is opt-in via an explicit `alpha > 0`. Vector-only is the
   least-surprising default and prevents the NotiKeeper-class footgun.

2. **Expose `impact` in search results** (it already lives on `NodeOutput`) so the
   application can fuse it as *one feature among many* — RRF, weighted rerank, or
   a learned model — rather than accepting the engine's hard-coded 0.5/0.3/0.2.

3. **Keep `compute_impact` / `refresh_impacts` in the engine.** Graph centrality is
   a graph-native metric; computing it next to the adjacency indices is far cheaper
   than dragging the graph into the app (compute-near-data). The engine's job is to
   *offer the signal*, not to *impose the ranking*.

4. **(Longer term) make the tier→authority mapping pluggable.** Hard-coded
   MASTER/SPEC/ADR is a GKS opinion; a general-purpose engine should let an embedder
   supply its own authority function (or none).

### What this is NOT
This does **not** remove K-Impact or revert
[[ADR--GENESISDB-KIMPACT-ALGORITHM]]. For the GKS/governance use case K-Impact
remains the recommended ranking — the caller simply opts in with `alpha > 0`. This
ADR narrows *where the ranking decision is made*, not *whether K-Impact exists*.

## Consequences

* **Positive:** Safe default (vector-only); the engine becomes domain-agnostic;
  applications get a fast graph-authority signal without inheriting a governance
  ranking policy; aligns with the standard retrieve→fuse→rerank pipeline.
* **Positive (evidence):** NotiKeeper now runs standard **RRF (dense bge-m3 +
  sparse SQLite FTS5/BM25)** at the app layer with `alpha=0`, getting correct
  semantic recall that the previous `alpha=1.0` path silently broke.
* **Negative:** Changing the default `alpha` is a behavior change for any caller
  that relied on the implicit blend. GKS callers must pass `alpha` explicitly.
* **Migration:** No on-disk format change (`SCHEMA_VERSION` unchanged); `impact`
  is already persisted on nodes. Only query-time defaults shift.

---
### Related Links
- **Amends:** [[ADR--GENESISDB-KIMPACT-ALGORITHM]]
- **Impact Algorithm:** [[ALGO--KIMPACT-CALCULATION]]
- **Orchestrator:** [[GENESIS--BACKEND-ENGINE]]
