import { resummariseLLM, truncate } from './resummarise.js'
import { joinTurns } from './text.js'
import { trimEpisode } from './trim.js'
import { DEFAULT_TOKENISER, estimateText } from './tokens.js'
import {
  DEFAULT_LLM_TIMEOUT_MS,
  RESUMMARISE_RATIO,
  TRIM_DROP_FRACTION,
  type CompressOptions,
  type CompressResult,
  type CompressedEpisode,
  type CompressorEpisode,
  type EpisodeRef,
  type TierCounts,
  type Tokeniser,
} from './types.js'

export type {
  CompressOptions,
  CompressResult,
  CompressedEpisode,
  CompressorEpisode,
  CompressionTier,
  EpisodeRef,
  LlmClient,
  TierCounts,
  Tokeniser,
  Turn,
} from './types.js'

export {
  DEFAULT_LLM_TIMEOUT_MS,
  RESUMMARISE_RATIO,
  TRIM_DROP_FRACTION,
  TRIM_THRESHOLD,
} from './types.js'

export { DEFAULT_TOKENISER, estimateText } from './tokens.js'
export { trimEpisode, tier1ScoreSingleTurn } from './trim.js'
export {
  buildResummarisePrompt,
  cleanLlmText,
  resummariseLLM,
  truncate,
} from './resummarise.js'
export { joinTurns } from './text.js'

function buildEpisodeRef(episode: CompressorEpisode): EpisodeRef {
  if (episode.atomId) {
    return {
      atomId: episode.atomId,
      sessionId: episode.sessionId,
      turnRange: episode.turnRange,
    }
  }
  return {
    sessionId: episode.sessionId,
    turnRange: episode.turnRange,
  }
}

function emptyTierCounts(): TierCounts {
  return { keep: 0, trim: 0, resummarise: 0, truncated: 0, dropped: 0 }
}

function refSortKey(ref: EpisodeRef): [string, number] {
  if (ref.turnRange) return [ref.sessionId ?? '', ref.turnRange[0]]
  return [ref.sessionId ?? '', Number.MAX_SAFE_INTEGER]
}

/**
 * Three-tier shrink-to-fit context compressor.
 *
 * Iterates `episodes` in importance-descending order and, per episode,
 * applies the cheapest tier that fits the remaining budget:
 *
 *   1. **keep**         — passthrough verbatim
 *   2. **trim**         — drop low-tier-1-score turns
 *   3. **resummarise**  — LLM-based recompression (target 60%)
 *      ↳ falls back to **truncated** (keep most-recent turns) when no LLM
 *        is supplied or the LLM call fails/timeouts/parses-empty
 *   4. **dropped**      — even the summary won't fit; episode is omitted
 *
 * Total compressed-token cost is guaranteed ≤ `budgetTokens`.
 *
 * Pure / read-only: input episodes are NOT mutated. Output array order is
 * importance-descending unless `preserveOrder: true` (then chronological
 * by `turnRange[0]` after selection).
 *
 * See `FEAT--COMPRESSOR`, `ADR--COMPRESSOR-THREE-TIER`,
 * `BLUEPRINT--COMPRESSOR`.
 */
export async function compress(
  opts: CompressOptions,
): Promise<CompressResult> {
  const tokeniser: Tokeniser = opts.tokeniser ?? DEFAULT_TOKENISER
  const budget = Math.max(0, Math.floor(opts.budgetTokens))
  const llmTimeoutMs = opts.llmTimeoutMs ?? DEFAULT_LLM_TIMEOUT_MS
  const llmModel = opts.llmModel ?? 'compressor'

  const tierCounts = emptyTierCounts()

  // Importance-descending iteration without mutating input.
  const sorted = opts.episodes
    .map((e, originalIdx) => ({ e, originalIdx }))
    .sort((a, b) => {
      if (b.e.score !== a.e.score) return b.e.score - a.e.score
      // Tie-break on original order for determinism.
      return a.originalIdx - b.originalIdx
    })

  const compressed: CompressedEpisode[] = []
  let tokensUsed = 0

  for (const { e: episode } of sorted) {
    const remaining = budget - tokensUsed
    const summaryTokens = estimateText(episode.summary ?? '', tokeniser)

    if (remaining <= 0 || remaining < summaryTokens) {
      tierCounts.dropped += 1
      continue
    }

    const fullText = joinTurns(episode.turns)
    const fullTokens = estimateText(fullText, tokeniser)

    // Tier 1: keep (whole episode fits).
    if (fullTokens <= remaining) {
      compressed.push({
        episodeRef: buildEpisodeRef(episode),
        text: fullText,
        originalTokens: fullTokens,
        compressedTokens: fullTokens,
        compressedBy: 'keep',
        droppedTurnIndices: [],
        score: episode.score,
      })
      tokensUsed += fullTokens
      tierCounts.keep += 1
      continue
    }

    // Tier 2: trim (drop low-score turns; only fires if ≥ 30% droppable
    // AND the trimmed result actually fits).
    const trim = trimEpisode(episode, remaining, tokeniser)
    const droppedFraction =
      episode.turns.length > 0
        ? trim.droppedIndices.length / episode.turns.length
        : 0
    if (trim.fits && droppedFraction >= TRIM_DROP_FRACTION) {
      const trimTokens = estimateText(trim.text, tokeniser)
      compressed.push({
        episodeRef: buildEpisodeRef(episode),
        text: trim.text,
        originalTokens: fullTokens,
        compressedTokens: trimTokens,
        compressedBy: 'trim',
        droppedTurnIndices: trim.droppedIndices,
        score: episode.score,
      })
      tokensUsed += trimTokens
      tierCounts.trim += 1
      continue
    }

    // Tier 3: resummarise via LLM, with truncate fallback.
    const target = Math.max(
      1,
      Math.min(Math.floor(fullTokens * RESUMMARISE_RATIO), remaining),
    )

    if (opts.llm) {
      try {
        const r = await resummariseLLM(episode, target, tokeniser, {
          llm: opts.llm,
          timeoutMs: llmTimeoutMs,
          model: llmModel,
        })
        if (r) {
          compressed.push({
            episodeRef: buildEpisodeRef(episode),
            text: r.text,
            originalTokens: fullTokens,
            compressedTokens: r.tokens,
            compressedBy: 'resummarise',
            droppedTurnIndices: [],
            score: episode.score,
          })
          tokensUsed += r.tokens
          tierCounts.resummarise += 1
          continue
        }
      } catch {
        /* fall through to truncate */
      }
    }

    // Tier 3 fallback: truncate (keeps RECENT turns).
    const tr = truncate(episode, remaining, tokeniser)
    if (tr.text.length === 0) {
      // Couldn't fit even one turn. Drop this episode entirely so we
      // don't push a zero-text record into the output.
      tierCounts.dropped += 1
      continue
    }
    compressed.push({
      episodeRef: buildEpisodeRef(episode),
      text: tr.text,
      originalTokens: fullTokens,
      compressedTokens: tr.tokens,
      compressedBy: 'truncated',
      droppedTurnIndices: tr.droppedIndices,
      score: episode.score,
    })
    tokensUsed += tr.tokens
    tierCounts.truncated += 1
  }

  if (opts.preserveOrder) {
    compressed.sort((a, b) => {
      const [as, ai] = refSortKey(a.episodeRef)
      const [bs, bi] = refSortKey(b.episodeRef)
      if (as !== bs) return as < bs ? -1 : 1
      return ai - bi
    })
  }

  return {
    compressed,
    totalTokensUsed: tokensUsed,
    tierCounts,
  }
}
