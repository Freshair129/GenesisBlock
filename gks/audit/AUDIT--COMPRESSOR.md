---
id: AUDIT--COMPRESSOR
phase: 6
type: audit
status: stable
vault_id: default
title: M7d — context compressor implementation (three-tier shrink-to-fit)
tags:
  - msp
  - compressor
  - compression
  - token-budget
  - m7d
  - audit
crosslinks: {"references":["FEAT--COMPRESSOR","BLUEPRINT--COMPRESSOR","ADR--COMPRESSOR-THREE-TIER","CONCEPT--CONTEXT-COMPRESSION","FRAME--MSP-ARCHITECTURE-V2"]}
linked_symbols:
  - {"file":"src/orchestrator/compressor/index.ts"}
  - {"file":"src/orchestrator/compressor/types.ts"}
  - {"file":"src/orchestrator/compressor/tokens.ts"}
  - {"file":"src/orchestrator/compressor/text.ts"}
  - {"file":"src/orchestrator/compressor/trim.ts"}
  - {"file":"src/orchestrator/compressor/resummarise.ts"}
  - {"file":"test/orchestrator/compressor/tokens.test.ts"}
  - {"file":"test/orchestrator/compressor/trim.test.ts"}
  - {"file":"test/orchestrator/compressor/resummarise.test.ts"}
  - {"file":"test/orchestrator/compressor/index.test.ts"}
created_at: 2026-05-05T09:32:00.000Z
---

# M7d — context compressor implementation (three-tier shrink-to-fit)

## Scope

M7d deliverable: `compress(opts)` orchestrator implementing the three-tier (keep / trim / resummarise) shrink-to-fit pipeline per `FEAT--COMPRESSOR`, `BLUEPRINT--COMPRESSOR`, and `ADR--COMPRESSOR-THREE-TIER`. Consumes importance-scored episodes (with their turns), iterates importance-descending, and emits `CompressedEpisode[]` whose total token cost is bounded by `budgetTokens`. Pure read + transform — no persistence, no mutation.

## What shipped

| File | Purpose |
|---|---|
| `src/orchestrator/compressor/types.ts` | `Tokeniser`, `EpisodeRef`, `CompressorEpisode`, `CompressionTier`, `CompressedEpisode`, `CompressOptions`, `TierCounts`, `CompressResult`, defaults (`DEFAULT_LLM_TIMEOUT_MS`, `TRIM_THRESHOLD`, `TRIM_DROP_FRACTION`, `RESUMMARISE_RATIO`) |
| `src/orchestrator/compressor/tokens.ts` | `DEFAULT_TOKENISER` (char/3.5 heuristic) + `estimateText` injection helper |
| `src/orchestrator/compressor/text.ts` | `joinTurns` shared turn-rendering helper (mirrors `[speakerId] content` layout from M7b tier-2) |
| `src/orchestrator/compressor/trim.ts` | `tier1ScoreSingleTurn` (re-uses M7b `scoreChunk` with a 1-turn chunk) + `trimEpisode` (lowest-score-first drop until fits) |
| `src/orchestrator/compressor/resummarise.ts` | `buildResummarisePrompt`, `cleanLlmText` (strips fences + Summary: prefixes), `resummariseLLM` (timeout + parse + fit-check), `truncate` (deterministic fallback that keeps RECENT turns) |
| `src/orchestrator/compressor/index.ts` | `compress()` orchestrator — importance-desc iteration, tier choice, optional `preserveOrder` chronological re-sort, complete public re-exports |
| `test/orchestrator/compressor/tokens.test.ts` | 6 tests — char/3.5 default, empty string, mixed-language conservativeness, integer non-negative, custom tokeniser injection, fall-through to default |
| `test/orchestrator/compressor/trim.test.ts` | 7 tests — no-op when fits, drop-fillers, never-drop-high-signal, fits=false, ascending dropped indices, chronological output text, idempotence + no-mutation |
| `test/orchestrator/compressor/resummarise.test.ts` | 12 tests — prompt build, text cleanup, success path, no-llm null, LLM throw, LLM timeout, empty response, over-budget rejection, truncate keeps recent, truncate budget exact, truncate empty when too tight, idempotence + no-mutation |
| `test/orchestrator/compressor/index.test.ts` | 13 end-to-end — all-keep, mixed tiers (no LLM), resummarise tier with LLM, LLM timeout fallback, importance-desc iteration, preserveOrder=true reorders, preserveOrder=false retains importance, totalTokensUsed sum invariant, drops when summary won't fit, budget=0 → all dropped, no-mutation, episodeRef provenance, idempotence |

## Boundaries respected

- **No mutation of input `Episode[]`** — sorting is done on a `[{ e, originalIdx }]` projection; turns are read-only throughout.
- **No MCP wrapping** — `msp_compress` is M7f scope.
- **No real tokeniser library** — char/3.5 is the only built-in default; `opts.tokeniser` is the caller's injection point. Zero new runtime deps.
- **No new LLM bundle** — `resummariseLLM` accepts the existing `LlmClient` (= `SlmClient`) interface from M7b/codegen. Tests use `createSlmClient({ provider: 'mock' })`-style hand-rolled stubs.
- **No modifications to `src/orchestrator/consolidator/`, `src/orchestrator/retrieval/`, or `src/memory/episodic/`** — verified by `git diff main --stat`.
- **No cross-episode dedup** — out of scope per ADR.
- **No hierarchical summary-of-summaries** — out of scope per ADR.
- **No persistence** — orchestrator is pure read + transform; episodic store untouched.

## Atoms landed

| Atom | Phase | Type |
|---|---|---|
| `CONCEPT--CONTEXT-COMPRESSION` | 1 | concept (existed) |
| `ADR--COMPRESSOR-THREE-TIER` | 2 | adr (existed) |
| `FEAT--COMPRESSOR` | 2 | feat (existed) |
| `BLUEPRINT--COMPRESSOR` | 3 | blueprint (existed) |
| `AUDIT--COMPRESSOR` | 6 | audit (this atom) |

## Verification

```
npm ci                              → clean install (worktree caveat per CLAUDE.md)
npm test                            → 435 passed (59 files)
npm run typecheck                   → clean
npx tsx src/validator/cli.ts --all  → 130 atoms pass
npm run msp:check-links             → OK
```

Test count delta: 397 → 435 (+38; target was +35, exceeded). Per-file breakdown:

  - `tokens.test.ts`: 6 tests (target 5 — exceeded)
  - `trim.test.ts`: 7 tests (target 7)
  - `resummarise.test.ts`: 12 tests (target 10 — exceeded; truncate path needed extra coverage)
  - `index.test.ts`: 13 tests (target 13)

## Acceptance criteria from `FEAT--COMPRESSOR`

- [x] `compress(opts)` returns `CompressResult` with `compressed[]`, `totalTokensUsed`, `tierCounts`
- [x] Total tokens ≤ `budgetTokens` — enforced; selection drops episodes when even the summary won't fit
- [x] Importance-descending iteration — high-importance gets keep/trim, low gets resummarise/truncate/drop (verified by `index.test.ts` "iterates in importance-descending order")
- [x] Tier choice per `ADR--COMPRESSOR-THREE-TIER`:
  - whole-fits → keep
  - ≥ 30% droppable + trimmed-fits → trim
  - has llm → resummarise (target 0.6 × original, capped at remaining)
  - no llm OR resummarise fails → truncate (drop earliest turns, keep most-recent)
- [x] `opts.preserveOrder` reorders OUTPUT to chronological by `turnRange[0]` after selection
- [x] `opts.tokeniser` injection — defaults to `Math.ceil(s.length / 3.5)`
- [x] No LLM = headless OK — truncate fallback always available; LLM-timeout test verifies the fallthrough path
- [x] No mutation — input episodes unchanged (deep-equal before/after in `index.test.ts`)
- [x] Provenance preserved — every output has `episodeRef` (sessionId+turnRange always; +atomId when input had one)
- [x] Idempotent with deterministic mock LLM — verified end-to-end
- [x] Test target ~395 → ~430 (delivered 397 → 435)

## Decisions during impl

These choices were not pre-specified by the BLUEPRINT and are recorded for future tuning:

1. **`CompressorEpisode` is a new compressor-internal shape**, not the M7b `Episode` directly. M7b's `Episode` carries `turnRange` but not the actual `turns[]`, while the compressor needs both. Rather than mutate M7b's contract, the compressor declares `CompressorEpisode { sessionId, turnRange, summary, score, turns, atomId? }` and re-exports the type from `index.ts`. Callers stitch turns from `readSessionTurns(...)` (M7b) or hydrate from a persisted `Episode` plus its source turns. This keeps M7b's persisted-shape decisions independent of compressor needs.

2. **`tier1ScoreSingleTurn` lives in `trim.ts`, not in M7b's `score.ts`** — exporting a one-turn helper from M7b would have meant either a public re-export change or moving file. Keeping the wrapper in `trim.ts` preserves M7b's surface; the wrapper is a 3-line `scoreChunk([turn], computeSessionStats(episodeTurns), {}, null)` call. Stats are computed per-call from the episode's surrounding turns so length-normalisation is sensible.

3. **`buildEpisodeRef` always carries `sessionId` + `turnRange`** even when `atomId` is provided. The `EpisodeRef` discriminated union still types as "atomId-bearing" (because `atomId` is present), but consumers also get the session context for free. Cheap; preserves more provenance than the strict ADR shape.

4. **Tier-3 truncate produces "" + drops-all when even one turn exceeds `target`** — and `compress()` then bumps the dropped counter and skips the episode entirely (rather than emitting a zero-text record). This avoids polluting the output with empty strings; callers can check `tierCounts.dropped` for diagnostics.

5. **Output array tie-break on equal score is original input order** (using `originalIdx`). Combined with importance-desc, this means callers passing `[ep1, ep2, ep3]` with all-equal scores get `[ep1, ep2, ep3]` back — predictable for tests.

6. **`cleanLlmText` is conservative** — only strips a single full-body fence, leading "Summary:" / "Re-summarised:" prefixes, and surrounding whitespace. Does NOT strip embedded fences in the middle of the body; LLMs that emit code samples in their summary keep them intact.

7. **`resummariseLLM` rejects responses that exceed `target`** rather than truncating them. Truncating LLM output mid-sentence is worse than falling through to the deterministic `truncate()` (which cuts at turn boundaries). Caller controls the fallthrough by checking the returned `null`.

8. **`preserveOrder` ordering uses `turnRange[0]` keyed by `sessionId`** — multi-session inputs sort first by sessionId lexicographic, then by start-of-range within the session. Single-session callers (the common case) get pure chronological order.

9. **Budget guard rounds DOWN** (`Math.floor`) and clamps non-negative — defensive against fractional or negative budgets. `budgetTokens: 0` → all-dropped is tested directly.

10. **`joinTurns` lives in its own `text.ts` module** — both `trim.ts` and `resummarise.ts` need it, plus `index.ts` for tier-1 keep, plus tests. Pulling it into a shared file avoids duplication and circular imports between `trim.ts` and `resummarise.ts`.

## Public API surface

```ts
import { compress } from '@/orchestrator/compressor'
import { createSlmClient } from '@/codegen/slm/factory'

const result = await compress({
  episodes: episodicHits,        // CompressorEpisode[]
  budgetTokens: 4000,
  llm: createSlmClient({ provider: 'auto' }),  // optional
  preserveOrder: false,
  tokeniser: undefined,          // default char/3.5
  llmTimeoutMs: 8000,
})

result.compressed                 // CompressedEpisode[] — fits budget
result.totalTokensUsed            // ≤ budgetTokens
result.tierCounts                 // { keep, trim, resummarise, truncated, dropped }
```

Lower-level entry points re-exported from `index.ts`: `DEFAULT_TOKENISER`, `estimateText`, `trimEpisode`, `tier1ScoreSingleTurn`, `truncate`, `resummariseLLM`, `buildResummarisePrompt`, `cleanLlmText`, `joinTurns`, plus the constants (`DEFAULT_LLM_TIMEOUT_MS`, `TRIM_THRESHOLD`, `TRIM_DROP_FRACTION`, `RESUMMARISE_RATIO`).

## Out of scope (deferred)

- M7f — MCP tool wrapper (`msp_compress`)
- M9+ — Cross-episode set-cover dedup
- M9+ — Hierarchical summary-of-summaries for very long projects
- M9+ — Online learning of per-tier compression ratios
- Vector-similarity-based dedup before compression (depends on M7c embedder availability)
- Snippet-level extractive summarisation (whole-turn drop only, per ADR)

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 38 new tests + typecheck clean + 130 atoms validate + linkcheck OK
- Branch: `claude/msp-m7d-compressor-impl`
