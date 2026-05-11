import { computeSessionStats, scoreChunk } from '../consolidator/score.js'
import type { Turn } from '../consolidator/types.js'

import { joinTurns } from './text.js'
import { DEFAULT_TOKENISER, estimateText } from './tokens.js'
import {
  TRIM_THRESHOLD,
  type CompressorEpisode,
  type Tokeniser,
} from './types.js'

/**
 * Score a single turn for drop-ability. Re-uses the M7b tier-1 scorer with
 * a 1-turn chunk. Stats are derived from the surrounding episode so the
 * length-normalised feature behaves sensibly (a one-line turn inside an
 * episode of long turns is correctly scored as "short → low").
 */
export function tier1ScoreSingleTurn(turn: Turn, episodeTurns: Turn[]): number {
  const stats = computeSessionStats(episodeTurns)
  const result = scoreChunk([turn], stats, {}, null)
  return result.score
}

export interface TrimResult {
  /** Joined turn text after dropping low-score candidates. */
  text: string
  /** Indices into the input `episode.turns` of dropped turns. */
  droppedIndices: number[]
  /** True when the trimmed text fits within `target`. */
  fits: boolean
}

/**
 * Drop low-score turns from an episode until the joined text fits the
 * `target` token budget. Greedy: drop the lowest-scoring candidate first,
 * stop as soon as the remainder fits.
 *
 * - Only turns with `tier1Score < TRIM_THRESHOLD` are eligible (default 0.3)
 * - High-score turns (decisions, code refs) are never dropped
 * - Output preserves chronological order
 * - If droppable candidates exhausted but text still > target, returns
 *   `fits: false` so the caller can fall through to tier-3
 *
 * Pure: does NOT mutate `episode`.
 */
export function trimEpisode(
  episode: CompressorEpisode,
  target: number,
  tokeniser: Tokeniser = DEFAULT_TOKENISER,
): TrimResult {
  const turns = episode.turns
  if (turns.length === 0) {
    return { text: '', droppedIndices: [], fits: 0 <= target }
  }

  // Score every turn; track its original index.
  const scored = turns.map((t, idx) => ({
    turn: t,
    idx,
    score: tier1ScoreSingleTurn(t, turns),
  }))

  // Candidates = turns below the trim threshold, sorted lowest-first.
  const candidates = scored
    .filter((s) => s.score < TRIM_THRESHOLD)
    .sort((a, b) => a.score - b.score)

  // Start with all turns; drop candidates one at a time until fits.
  const keptIndices = new Set(turns.map((_, i) => i))
  const droppedIndices: number[] = []

  let currentText = joinTurns(turns)
  let currentTokens = estimateText(currentText, tokeniser)

  for (const c of candidates) {
    if (currentTokens <= target) break
    keptIndices.delete(c.idx)
    droppedIndices.push(c.idx)
    const remaining = turns.filter((_, i) => keptIndices.has(i))
    currentText = joinTurns(remaining)
    currentTokens = estimateText(currentText, tokeniser)
  }

  return {
    text: currentText,
    // Sort for determinism (insertion order is score-asc; chronological
    // order is more useful to consumers reasoning about the original
    // session).
    droppedIndices: droppedIndices.slice().sort((a, b) => a - b),
    fits: currentTokens <= target,
  }
}
