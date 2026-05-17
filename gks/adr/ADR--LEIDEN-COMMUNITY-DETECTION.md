---
id: ADR--LEIDEN-COMMUNITY-DETECTION
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Leiden community detection — graphology-communities-leiden v1, Louvain fallback
tags:
  - msp
  - symbol-graph
  - leiden
  - louvain
  - graphology
  - community-detection
  - decision
crosslinks: {"references":["FRAMEWORK--SYMBOL-GRAPH","CONCEPT--SYMBOL-GRAPH","ADR--SYMBOL-GRAPH-PERSISTENCE"]}
created_at: 2026-05-09T16:51:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: "Architecture decision record"
---

# ADR — Leiden community detection

## Context

`[[CONCEPT--SYMBOL-GRAPH]]` requires assigning each symbol a `community_id` that approximates "logical module" — clusters of densely-connected symbols. Two well-known modularity-optimization algorithms fit:

- **Louvain** (2008) — fast, widely-implemented, but produces poorly-connected communities under certain edge distributions
- **Leiden** (2019) — improves Louvain by guaranteeing connected communities, faster convergence, and better modularity scores. User asked for Leiden specifically.

For deterministic Git-friendly builds, the algorithm must be **seedable**.

## Decision

Use **`@aflsolutions/graphology-communities-leiden`** (v1.1.1, npm-published 2026-04-26) as the primary community detection.

Add **`graphology-communities-louvain`** (v2.0.2, npm-published 2024-12-17, official `yomguithereal` maintainer) as `optionalDependencies` for fallback.

Both run through the same internal `detectCommunities(graph, options)` adapter:

```typescript
interface CommunityDetector {
  run(graph: Graph, opts: { resolution: number; seed: number }):
    { partition: Map<NodeId, CommunityId>; modularity: number }
}
```

Selected at runtime via `meta.algorithm = "leiden" | "louvain"` so the choice is visible in build artifacts. Default = leiden; falls back to louvain only if the leiden package fails to load (with a warning logged to stderr).

Parameters:
- `resolution = 1.0` default; CLI flag `--resolution=<n>` (lower = bigger / fewer communities)
- `seed = 42` fixed; CLI flag `--seed=<n>` for experimentation (changing seed = git diff churn, so default is locked)
- `levels = 1` (top-level only); hierarchical Leiden deferred to Phase 2

## Consequences

**Positive**
- Honors user's stated preference for Leiden
- Determinism via fixed seed = git-stable JSONL exports
- Fallback path keeps the build working if the (single-maintainer) Leiden package regresses
- Resolution flag gives users a tunable knob without changing the algorithm

**Negative**
- **`@aflsolutions/...` is a single-maintainer fork.** If it deprecates, we either (a) vendor the algorithm into `src/symbols/communities/leiden-vendored.ts`, (b) fall back to Louvain permanently, or (c) contribute the fork back to the official `graphology` org. Decision deferred — flag in upstream GKS proposal so the GKS maintainers can weigh in.
- Top-level only in v1 — power users wanting hierarchical clusters need to wait for Phase 2. The SQLite schema reserves a nullable `parent_community_id` so adding levels is non-breaking.
- Modularity is a heuristic; community labels (auto-generated as "top-dir / top-symbol-name") are best-effort — a 50-symbol community labeled "src/orchestrator" is approximately right but won't match the human mental model perfectly.

## Alternatives considered

1. **Louvain only.** Rejected. User asked for Leiden; Louvain's known weakness (disconnected communities) shows up in our domain (a function may bridge two otherwise-separate clusters via a single shared util). Leiden's guarantee of connected communities matters here.
2. **Manual / heuristic clustering** (e.g. group by directory). Rejected. Directories are a weak proxy for logical coupling — `src/validator/` has 13 files but they implement 5 different rule families. Algorithmic clustering reflects actual usage.
3. **Spectral clustering / hierarchical agglomerative.** Rejected for v1. Slower, less standard for "find communities" framing. Could be added as a parallel detector if Leiden+Louvain prove inadequate.
4. **No community detection at all.** Rejected. Communities are the primary user-facing feature for "what cluster does this belong to?" — without them, the graph is just a queryable ctags.

## What this ADR does NOT decide

- The exact label-derivation heuristic for `communities.label` (deferred to BLUEPRINT — proposed: `<top-dir>/<top-symbol-name>` derived from majority membership)
- Whether to expose `--levels=N` for hierarchical detection in v1 (no — Phase 2)
- Whether community membership migrates across builds (no — full re-detection per build; rationale: Leiden is fast, full builds keep everything consistent)

## Source

- User design decision 2026-05-09 (Leiden specifically)
- npm registry inspection 2026-05-09:
  - `@aflsolutions/graphology-communities-leiden@1.1.1` (2026-04-26 publish)
  - `graphology-communities-louvain@2.0.2` (2024-12-17 publish, `yomguithereal`)
- Traag, Waltman, van Eck (2019) — "From Louvain to Leiden: guaranteeing well-connected communities" (Sci. Rep.)
- `[[ADR--SYMBOL-GRAPH-PERSISTENCE]]` — `meta.algorithm` field added to schema

## Connections
- [[FRAMEWORK--SYMBOL-GRAPH]]

