---
id: AUDIT--CONSOLIDATOR
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M7b — consolidator implementation (hybrid scoring + boundary detection + summariser)
tags:
  - msp
  - consolidator
  - m7b
  - audit
crosslinks: {"references":["FEAT--CONSOLIDATOR","BLUEPRINT--CONSOLIDATOR","ADR--CONSOLIDATOR-HYBRID-SCORING","CONCEPT--CONSOLIDATOR","FRAMEWORK--MSP-ARCHITECTURE-V2"]}
linked_symbols:
  - {"file":"packages/msp/src/orchestrator/consolidator/index.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/types.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/score.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/boundary.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/summarise.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/llm.ts"}
  - {"file":"packages/msp/test/orchestrator/consolidator/score.test.ts"}
  - {"file":"packages/msp/test/orchestrator/consolidator/boundary.test.ts"}
  - {"file":"packages/msp/test/orchestrator/consolidator/summarise.test.ts"}
  - {"file":"packages/msp/test/orchestrator/consolidator/llm.test.ts"}
  - {"file":"packages/msp/test/orchestrator/consolidator/index.test.ts"}
created_at: 2026-05-04T23:29:00.000+07:00
---

# M7b — consolidator implementation (hybrid scoring + boundary detection + summariser)

## Scope

M7b deliverable: `consolidate(opts)` orchestrator implementing the hybrid (deterministic + LLM borderline) scoring pipeline per `[[FEAT--CONSOLIDATOR]]`, `[[BLUEPRINT--CONSOLIDATOR]]`, and `[[ADR--CONSOLIDATOR-HYBRID-SCORING]]`. Consumes session.jsonl turns, partitions into topic-coherent chunks, scores each via tier-1 deterministic features, escalates borderline cases to tier-2 LLM, and emits `Episode[]` in memory. Caller decides persistence.

## What shipped

| File | Purpose |
|---|---|
| `src/orchestrator/consolidator/types.ts` | `Turn`, `Chunk`, `Episode`, `ConsolidateOptions`, `Verdict`, `Tier1Result`, `Tier2Result`, `SessionStats`, `Thresholds`, defaults |
| `src/orchestrator/consolidator/boundary.ts` | `tokenise`, `bagCosine`, `detectBoundaries` — windowed topic-continuity partitioner |
| `src/orchestrator/consolidator/score.ts` | 7 tier-1 features + `scoreChunk()` + `computeSessionStats()` |
| `src/orchestrator/consolidator/summarise.ts` | `deterministicSummary()` + `extractDeterministicTags()` (tier-1-keep fallback) |
| `src/orchestrator/consolidator/llm.ts` | `buildTier2Prompt`, `extractJsonObject`, `parseTier2Response`, `callTier2`, `defaultKeepResult` |
| `src/orchestrator/consolidator/index.ts` | `consolidate()` orchestrator + `readSessionTurns()` |
| `test/orchestrator/consolidator/score.test.ts` | 19 tests — 7 features + threshold edges + session stats |
| `test/orchestrator/consolidator/boundary.test.ts` | 7 tests — tokenisation, cosine, single-topic, topic-shift, partition coverage |
| `test/orchestrator/consolidator/summarise.test.ts` | 7 tests — sentence selection, tag extraction, fallback paths |
| `test/orchestrator/consolidator/llm.test.ts` | 14 tests — prompt build, JSON parse (bare/fenced/embedded), success/timeout/parse-error/no-provider |
| `test/orchestrator/consolidator/index.test.ts` | 6 fixture-driven end-to-end (mock LLM) — drop, tier1-keep, tier2-keep, budget cap, idempotence, missing file |

## Boundaries respected

- **No persistence inside `consolidate()`** — returns `Episode[]`; caller pipes to `EpisodicWriter`.
- **No mutation of session.jsonl** — read-only via `createReadStream` + `readline`.
- **No new LLM bundle** — re-uses `SlmClient` interface from `src/codegen/types.ts` via the alias `LlmClient`. Tests use bare async functions; production callers pass `createSlmClient({ provider: 'auto' })`.
- **No MCP tool wrapping** — `msp_remember` is M7f scope.
- **No threshold tuning beyond ADR defaults** — `low: 0.30`, `high: 0.65`, `boundary: 0.25` from `[[ADR--CONSOLIDATOR-HYBRID-SCORING]]`. Tunable via `ConsolidateOptions.thresholds`. PARAM atom is M9.
- **No edits to `src/memory/episodic/` or `src/memory/sessions/`** — consumed as-is.

## Atoms landed

| Atom | Phase | Type |
|---|---|---|
| `[[CONCEPT--CONSOLIDATOR]]` | 1 | concept (existed) |
| `[[ADR--CONSOLIDATOR-HYBRID-SCORING]]` | 2 | adr (existed) |
| `[[FEAT--CONSOLIDATOR]]` | 2 | feat (existed) |
| `[[BLUEPRINT--CONSOLIDATOR]]` | 3 | blueprint (existed) |
| `[[AUDIT--CONSOLIDATOR]]` | 6 | audit (this atom) |

## Verification

```
npm test                            → 347 passed, 1 pre-existing failure (test/hooks/pre-push.test.ts unrelated)
npm run typecheck                   → clean
npx tsx src/validator/cli.ts --all  → 107 passed, 0 failed
npm run msp:check-links             → OK (107 atoms scanned)
```

Test count delta: 295 → 348 (+53; target was +35, exceeded).
Consolidator test files contribute 53 tests:
  - score.test.ts: 19
  - boundary.test.ts: 7
  - summarise.test.ts: 7
  - llm.test.ts: 14
  - index.test.ts: 6

## Acceptance criteria from `[[FEAT--CONSOLIDATOR]]`

- [x] `consolidate(opts)` reads session turns from `.brain/msp/projects/<ns>/sessions/<sessionId>.jsonl`
- [x] Returns `Episode[]` — does NOT write to episodic store
- [x] Tier-1 deterministic scoring: 7 features per ADR
- [x] Tier-1 verdicts: `< low_thresh` → drop; `> high_thresh` → keep; otherwise → tier-2
- [x] Tier-2 LLM: only invoked for borderline; bounded by `maxLlmCallsPerSession` + `llmCallTimeoutMs`; timeout / parse-error / no-provider → default to keep
- [x] Boundary detection: chunks turns into contiguous episodes via topic-continuity threshold
- [x] Summariser: tier-2 LLM returns summary + tags in single call; tier-1-keep chunks get a deterministic summary + tags
- [x] Idempotent: same input twice → same `Episode[]` (verified with mock LLM)
- [x] No mutation of source `session.jsonl`
- [x] All 4 sub-modules unit-testable in isolation

## Decisions during impl

These choices were not pre-specified and are recorded for future tuning:

1. **Symmetric forward-window for boundary detection.** The naive "windows of size N vs whatever-remains" approach produced spurious tail-cuts on short single-topic sessions. The implementation only emits a boundary when both the trailing and leading windows are full size — i.e. boundaries are only detected in the interior `[window, n-window]` range. This preserves the "single-topic stays one chunk" invariant.

2. **`numericSpecificity` uses density + bonus.** Pure digit-density was too weak for single-mention semver / dates. Added a fixed +0.4 bonus for each of `vX.Y.Z` and ISO `YYYY-MM-DD` patterns. Caps at 1.0.

3. **`topicContinuity` returns 0.5 for the first chunk.** No prior chunk → "neutral" rather than 0 (which would push the chunk toward `drop` via the +0.10 weight). Documented in the function.

4. **Tier-2-default keeps the tier-1 score** rather than synthesising a 0.6 number when the LLM call itself fails. This preserves more signal in the persisted episode and matches the spirit of "default to keep".

5. **`callTier2` race-with-timeout pattern.** The shared `SlmClient` interface lacks an `AbortSignal`, so we rely on `Promise.race` against a timer. Real Ollama clients have their own internal timeout, which acts as a backstop.

6. **JSON extraction is robust to fences and noise.** `extractJsonObject` tries direct parse → `\`\`\`json` fence stripping → first balanced `{...}` block — handles common LLM output styles.

7. **Default `now` injection for testable timestamps.** `ConsolidateOptions.now` lets fixture tests assert exact `createdAt` values.

## Public API surface

```ts
import { consolidate } from '@/orchestrator/consolidator'

const episodes = await consolidate({
  sessionId: 'sess-2026-05-04-abc',
  root: process.cwd(),
  llm: createSlmClient({ provider: 'ollama' }),
  thresholds: { low: 0.30, high: 0.65, boundary: 0.25 },
  maxLlmCallsPerSession: 20,
  llmCallTimeoutMs: 8000,
})
// episodes is Episode[]; pass to EpisodicWriter to persist.
```

Sub-module entry points are also re-exported from `index.ts` for advanced use:
`scoreChunk`, `computeSessionStats`, `detectBoundaries`, `callTier2`, `deterministicSummary`, `extractDeterministicTags`.

## Out of scope (deferred)

- M7c — Retrieval orchestration (cross-session episode dedup)
- M7d — Context compression (hierarchical summary-of-summaries)
- M7f — MCP tool wrapper (`msp_remember`)
- M9 — Threshold tuning (`[[PARAM--CONSOLIDATOR-THRESHOLDS]]` atom + real-session calibration)
- M9 — Forgetting / decay (`valid_until`)
- Session-end auto-call hook (lives in agent harness, not in this repo)

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 53 new tests + validator (107/107) + check-links OK + typecheck clean
- Branch: `claude/msp-m7b-consolidator-impl`

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

