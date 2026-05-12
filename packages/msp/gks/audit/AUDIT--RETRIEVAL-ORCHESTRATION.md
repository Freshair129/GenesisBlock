---
id: AUDIT--RETRIEVAL-ORCHESTRATION
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M7c — retrieval orchestration implementation (RRF fusion across 4 sources)
tags:
  - msp
  - retrieval
  - rrf
  - msp-recall
  - m7c
  - audit
crosslinks: {"references":["FEAT--RETRIEVAL-ORCHESTRATION","BLUEPRINT--RETRIEVAL-ORCHESTRATION","ADR--RETRIEVAL-RRF-FUSION","CONCEPT--RETRIEVAL-ORCHESTRATION","FRAME--MSP-ARCHITECTURE-V2"]}
linked_symbols:
  - {"file":"src/orchestrator/retrieval/index.ts"}
  - {"file":"src/orchestrator/retrieval/types.ts"}
  - {"file":"src/orchestrator/retrieval/fusion.ts"}
  - {"file":"src/orchestrator/retrieval/sources/vector.ts"}
  - {"file":"src/orchestrator/retrieval/sources/obsidian.ts"}
  - {"file":"src/orchestrator/retrieval/sources/episodic.ts"}
  - {"file":"src/orchestrator/retrieval/sources/backlinks.ts"}
  - {"file":"test/orchestrator/retrieval/fusion.test.ts"}
  - {"file":"test/orchestrator/retrieval/sources/vector.test.ts"}
  - {"file":"test/orchestrator/retrieval/sources/obsidian.test.ts"}
  - {"file":"test/orchestrator/retrieval/sources/episodic.test.ts"}
  - {"file":"test/orchestrator/retrieval/sources/backlinks.test.ts"}
  - {"file":"test/orchestrator/retrieval/index.test.ts"}
created_at: 2026-05-05T17:18:00.000+07:00
---

# M7c — retrieval orchestration implementation (RRF fusion across 4 sources)

## Scope

M7c deliverable: `recall(opts)` orchestrator implementing the read-side fan-out + Reciprocal Rank Fusion pipeline per `FEAT--RETRIEVAL-ORCHESTRATION`, `BLUEPRINT--RETRIEVAL-ORCHESTRATION`, and `ADR--RETRIEVAL-RRF-FUSION`. Fans out across four sources (GKS vector / Obsidian text / episodic / backlinks), races each against per-source budgets, fuses surviving results via RRF with per-source weights, and returns ranked hits with provenance + timings + fallback reasons.

## What shipped

| File | Purpose |
|---|---|
| `src/orchestrator/retrieval/types.ts` | `SourceName`, `SourceHit`, `SourceResult`, `RetrievalHit`, `RetrievalResult`, `RecallOptions`, `RetrievalEmbedder`, `RetrievalVectorBackend`, `DEFAULT_WEIGHTS`, `DEFAULT_PER_SOURCE_TIMEOUTS`, `DEFAULT_TOTAL_TIMEOUT_MS`, `DEFAULT_TOP_K`, `DEFAULT_RRF_K` |
| `src/orchestrator/retrieval/fusion.ts` | Pure `rrfFuse(perSource, opts)` — RRF aggregation, sort tie-break (score → sourceCount → minRank → atomId), top-K slice, final rank assignment |
| `src/orchestrator/retrieval/sources/vector.ts` | `vectorSource()` — embeds query → searches injected `RetrievalVectorBackend`, raceTimeout, error-on-failure |
| `src/orchestrator/retrieval/sources/obsidian.ts` | `obsidianSource()` — wraps caller-provided `ObsidianClient` (M7a); source label is `obsidian-text` for REST mode, `grep` for filesystem mode; skipped when client is absent |
| `src/orchestrator/retrieval/sources/episodic.ts` | `episodicSource()` + exported `scoreEpisode()` — Jaccard token overlap on `content.summary` plus +0.2 bonus per matching tag; ENOENT → empty |
| `src/orchestrator/retrieval/sources/backlinks.ts` | `backlinksSource()` — 1-hop expansion (both directions) from phase-A candidates; score = vote count; ENOENT → empty |
| `src/orchestrator/retrieval/index.ts` | `recall(opts)` orchestrator — phase-A (3 sources `Promise.allSettled`) → phase-B (backlinks from candidates) → phase-C (RRF fusion) |
| `test/orchestrator/retrieval/fusion.test.ts` | 14 tests — empty input, single hit, sum-on-overlap, distinct atoms, three-level tie-breaks, weight overrides, rrfK effect, topK, rank assignment, perSourceRanks, snippet preservation |
| `test/orchestrator/retrieval/sources/vector.test.ts` | 5 tests — happy path, no-embedder, empty query, timeout, latencyMs |
| `test/orchestrator/retrieval/sources/obsidian.test.ts` | 6 tests — REST mode, filesystem mode, no-client skipped, timeout, rank ordering, latencyMs |
| `test/orchestrator/retrieval/sources/episodic.test.ts` | 6 tests — missing file, tag-only match, summary-overlap match, tag bonus stacks, sort desc, topK |
| `test/orchestrator/retrieval/sources/backlinks.test.ts` | 6 tests — missing file, outbound expansion, inbound expansion, dedup, candidate-vote scoring, topK |
| `test/orchestrator/retrieval/index.test.ts` | 10 end-to-end tests — 4-source fusion, partial failure with reasons, all-empty, weight override, rrfK respect, obsidian_available flag, semantic_available flag, timings populated, idempotence, topK |

## Boundaries respected

- **No `createObsidianClient` instantiation inside the source** — caller provides the client (`opts.obsidian`), keeping M7c orthogonal to M7a's lifecycle. Tests pass mock clients in both modes.
- **No cross-call caching** — every `recall()` call re-runs all four sources fresh. Cache layer is M9+ work, only if perf demands it.
- **No cross-encoder re-rank** — RRF is the only fusion algorithm. M10c may add re-rank later.
- **No mutation of any source** — strictly read-only on `episodic_memory.json`, `backlinks.jsonl`, vector backend, obsidian client.
- **No MCP tool wrapping** — `msp_recall` is M7f scope.
- **No default-weight tuning beyond ADR** — defaults match the ADR table 1:1; `opts.weights` is the per-call override surface.
- **No new runtime deps** — `node:fs/promises`, `node:path`, `node:perf_hooks`, `node:readline` are all built-in.
- **No imports of aspirational `createNomicEmbedder`** — caller injects an `Embedder` (or its `RetrievalEmbedder` subset). Production callers wire `createEmbedder({ provider: 'auto' })` from GKS 3.5.6; tests pass `provider: 'mock'` via the same factory or a hand-rolled stub.

## Atoms landed

| Atom | Phase | Type |
|---|---|---|
| `CONCEPT--RETRIEVAL-ORCHESTRATION` | 1 | concept (existed) |
| `ADR--RETRIEVAL-RRF-FUSION` | 2 | adr (existed) |
| `FEAT--RETRIEVAL-ORCHESTRATION` | 2 | feat (existed) |
| `BLUEPRINT--RETRIEVAL-ORCHESTRATION` | 3 | blueprint (existed) |
| `AUDIT--RETRIEVAL-ORCHESTRATION` | 6 | audit (this atom) |

## Verification

```
npm ci                              → 163 packages installed (worktree caveat per CLAUDE.md)
npm test                            → 397 passed (55 files)
npm run typecheck                   → clean
npx tsx src/validator/cli.ts --all  → all atoms validate
npm run msp:check-links             → OK
```

Test count delta: 350 → 397 (+47; target was +45, exceeded). Per-file breakdown:
  - `fusion.test.ts`: 14 tests (target 12 — exceeded)
  - `sources/vector.test.ts`: 5 tests (target 5)
  - `sources/obsidian.test.ts`: 6 tests (target 6)
  - `sources/episodic.test.ts`: 6 tests (target 6)
  - `sources/backlinks.test.ts`: 6 tests (target 6)
  - `index.test.ts`: 10 tests (target 10)

## Acceptance criteria from `FEAT--RETRIEVAL-ORCHESTRATION`

- [x] `recall(opts)` returns `RetrievalResult` with `hits`, `semantic_available`, `obsidian_available`, `fallback_reasons`, `timings`
- [x] Parallel fan-out — phase-A's 3 sources run concurrently via `Promise.allSettled`
- [x] Per-source try/catch — one failing source does not break the call
- [x] Per-source timeouts — each source has its own budget (defaults from ADR); timeout omits the source and appends to `fallback_reasons`
- [x] Total budget enforced — `opts.timeoutMs` overrides; per-source budgets scale proportionally when the total is tighter than reference
- [x] RRF fusion per ADR — `weight_s / (k + rank)`, summed across sources, sorted desc
- [x] Tie-breaking — sourceCount DESC → minRank ASC → atomId lexicographic
- [x] No mutation — all four sources are read-only
- [x] Headless-safe — works without Obsidian (skipped) and without an embedder (vector source returns `error: 'no-embedder'`); other sources still fuse
- [x] Snippet preserved from source (no re-extraction); first-seen non-empty wins on collision
- [x] Provenance complete — per-hit `source` plus `perSourceRanks` map
- [x] Idempotent — same query + same on-disk fixtures + same mocks → same hit ordering + identical scores (verified end-to-end)
- [x] Test target 350 → ~395 (delivered 397)

## Decisions during impl

These choices were not pre-specified by the BLUEPRINT and are recorded for future tuning:

1. **`raceTimeout` returns `{ value, timedOut }` at the orchestrator level**, but per-source adapters use a leaner reject-on-timeout race because they each construct their own fallback `SourceResult` shape (with `error: 'timeout'`). Two slightly different patterns kept code readable rather than forcing every source to know how to render its own empty result via the orchestrator's helper.

2. **Vector source accepts a `RetrievalVectorBackend` subset**, not the full GKS `VectorBackend` interface. The narrow shape (just `search(query, opts)`) is structurally compatible with GKS 3.5.6's `VectorBackend` so callers can pass either an actual GKS backend or a thin test stub. Avoids a hard dep on GKS types at the public boundary.

3. **`semantic_available` requires both embedder + backend AND no error**. A vector source that runs but returns 0 hits (legitimate empty-store case) is still "available". Only an actual error path (no-embedder, timeout, search throw) sets it false.

4. **`obsidian_available` reflects only `client.mode === 'rest'`** — the deep-link rendering capability is what callers actually need this flag for, and only REST mode supports `smartViewDeepLink`. A filesystem-mode client still contributes hits (under source name `grep`) but cannot render deep links.

5. **Backlinks neighbours exclude the candidate set itself** — preventing a candidate atom from earning extra RRF points just because it's in its own 1-hop closure. Tested directly via the dedup/score tests.

6. **Obsidian source `atomIdFromPath` accepts atom-id basenames AND non-atom basenames.** Real Obsidian vaults contain free-form notes; we don't want to drop them just because they don't match `TYPE--SLUG`. The basename-without-extension serves as a stable identifier that other sources will rarely collide with (atom IDs match `[A-Z][A-Z0-9-]+--[A-Z0-9-]+`).

7. **Episodic source ties on score break by recency (timestamp DESC).** ADR weights episodic at 1.2 with rationale "recent context is high-signal"; the secondary tie-break inside the episodic source itself (before fusion) honours that intent. atomId lex is the tertiary tie-break for determinism.

8. **`scaleBudgets` only scales DOWN** — when caller's `opts.timeoutMs` is tighter than the sum of per-source defaults, each source's budget is scaled proportionally. When the total is generous (≥ reference sum), each source keeps its default cap. Avoids one source eating the whole budget on long total budgets.

9. **`raceTimeout` adds a +50ms grace at the orchestrator level** vs the per-source `timeoutMs`. The per-source adapter has its own internal timeout that should fire first (and produce a structured error result); the orchestrator's outer timer is a safety net for adapters that don't honour their own budget. Both paths return a `SourceResult` with empty hits, so fusion is unaffected.

10. **Uses `performance.now()` from `node:perf_hooks`** — same pattern as the consolidator. `latencyMs` is rounded to integer to keep timings JSON-friendly.

## Public API surface

```ts
import { recall } from '@/orchestrator/retrieval'
import { createObsidianClient } from '@/obsidian/client'
import { createEmbedder } from '@freshair129/gks'

const obsidian = await createObsidianClient({ root: process.cwd() })
const embedder = await createEmbedder({ provider: 'auto' })
// caller wires up a vector backend separately (or via MemoryStore)

const result = await recall({
  query: 'how did we decide rate limiting?',
  root: process.cwd(),
  namespace: 'evaAI',
  obsidian,
  embedder,
  vectorBackend: myBackend,
  topK: 10,
  timeoutMs: 1500,
  weights: { episodic: 1.5 },     // optional override
  rrfK: 60,
})

result.hits[0]                   // { atomId, source, score, rank, snippet, perSourceRanks }
result.semantic_available        // true if vector path operational
result.obsidian_available        // true if Obsidian REST reachable
result.fallback_reasons          // [] or e.g. ['gks-vector: timeout']
result.timings                   // { vector, obsidian, episodic, backlinks, fusion }
```

Lower-level entry points are also re-exported from `index.ts` for advanced use: `rrfFuse`, defaults (`DEFAULT_WEIGHTS`, `DEFAULT_PER_SOURCE_TIMEOUTS`, `DEFAULT_TOTAL_TIMEOUT_MS`, `DEFAULT_TOP_K`, `DEFAULT_RRF_K`).

## Out of scope (deferred)

- M7f — MCP tool wrapper (`msp_recall`)
- M9 — Cross-namespace recall + tenant auth
- M9 — `PARAM--RETRIEVAL-WEIGHTS` atom for traceable default-weight tuning
- M10c — Cross-encoder re-rank top-50
- Cache layer — only if real-traffic measurements show retrieval > 500ms
- Query rewriting / HyDE — out of M7

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 47 new tests + typecheck clean + npm ci clean
- Branch: `claude/msp-m7c-retrieval-impl`
