import type { Turn, LlmClient } from '../consolidator/types.js'

/**
 * Pluggable token estimator. The default is a conservative char-count
 * heuristic (`Math.ceil(s.length / 3.5)`) — see `tokens.ts`. Callers
 * can inject a real tokeniser (e.g. tiktoken) per-call.
 */
export type Tokeniser = (s: string) => number

/**
 * Re-export for convenience so the compressor's public surface does not
 * force callers to dig into `consolidator/types`.
 */
export type { LlmClient } from '../consolidator/types.js'
export type { Turn } from '../consolidator/types.js'

/**
 * Provenance reference for a compressed episode. Either a session-relative
 * turn range (in-memory pipeline from M7b consolidate → compress) OR a
 * persisted episodic atom id (M7c retrieval → compress) — at least one
 * must be present.
 */
export type EpisodeRef =
  | { sessionId: string; turnRange: [number, number]; atomId?: string }
  | { atomId: string; sessionId?: string; turnRange?: [number, number] }

/**
 * Input shape for the compressor: an importance-scored episode plus its
 * source turns. Re-uses the M7b `Episode` fields the compressor needs
 * (`turnRange`, `summary`, `score`) and adds `turns` so the compressor
 * can compute token counts and trim/resummarise without re-reading the
 * session jsonl.
 *
 * Compressor input is read-only — `compress()` MUST NOT mutate.
 */
export interface CompressorEpisode {
  /** Source session id (`turnRange` is relative to this session). */
  sessionId: string
  /** Inclusive [start, end] turn-index range over the source session. */
  turnRange: [number, number]
  /** Pre-computed summary (M7b output or persisted episodic). */
  summary: string
  /** Importance score 0..1 used to prioritise compression. */
  score: number
  /** The actual turn objects (length === turnRange[1] - turnRange[0] + 1). */
  turns: Turn[]
  /**
   * Optional persisted atom id, populated when episodes come from
   * `episodic_memory.json` (M7c retrieval) rather than freshly
   * consolidated turns.
   */
  atomId?: string
}

export type CompressionTier = 'keep' | 'trim' | 'resummarise' | 'truncated'

/**
 * One compressed episode in the output. `text` is the joined turn text
 * post-compression; `compressedTokens` reflects that text's token cost
 * under the active tokeniser.
 */
export interface CompressedEpisode {
  episodeRef: EpisodeRef
  text: string
  originalTokens: number
  compressedTokens: number
  compressedBy: CompressionTier
  /**
   * Indices (relative to the input episode's `turns` array) of turns
   * dropped during trim/truncate. Empty for `keep` / `resummarise`.
   */
  droppedTurnIndices: number[]
  score: number
}

export interface CompressOptions {
  /** Importance-scored episodes with source turns. Read-only. */
  episodes: CompressorEpisode[]
  /** Token budget — total compressed-token cost MUST be ≤ this. */
  budgetTokens: number
  /**
   * Optional LLM client for tier-3 resummarise. Without it the
   * compressor falls back to deterministic truncation (still
   * headless-safe).
   */
  llm?: LlmClient
  /** Per-LLM-call timeout in ms. Default 8000. */
  llmTimeoutMs?: number
  /**
   * If true, the OUTPUT array is sorted chronologically by start-of-range
   * after selection. Selection is always importance-driven. Default false
   * (output stays in importance-descending order).
   */
  preserveOrder?: boolean
  /** Pluggable tokeniser. Defaults to `Math.ceil(s.length / 3.5)`. */
  tokeniser?: Tokeniser
  /** Optional model name forwarded to the SLM client. Default 'compressor'. */
  llmModel?: string
}

/**
 * Per-tier counters for diagnostics. `dropped` counts episodes whose
 * even-the-summary-doesn't-fit case caused them to be omitted entirely.
 */
export type TierCounts = Record<CompressionTier | 'dropped', number>

export interface CompressResult {
  compressed: CompressedEpisode[]
  totalTokensUsed: number
  tierCounts: TierCounts
}

/** Default LLM call timeout, mirrors M7b `DEFAULT_LLM_TIMEOUT_MS`. */
export const DEFAULT_LLM_TIMEOUT_MS = 8000

/**
 * Tier-2 trim threshold: turns whose tier-1 score sits below this are
 * candidates for dropping. 0.30 mirrors `DEFAULT_THRESHOLDS.low` from
 * M7b — a turn that would tier-1-drop on its own is fair game.
 */
export const TRIM_THRESHOLD = 0.3

/**
 * Minimum droppable fraction for the trim tier to fire. Per
 * ADR--COMPRESSOR-THREE-TIER: only trim when ≥ 30% of the turns are
 * candidates AND the trimmed result actually fits.
 */
export const TRIM_DROP_FRACTION = 0.3

/**
 * Resummarise target ratio: aim for 60% of original token count (or
 * remaining budget, whichever is smaller).
 */
export const RESUMMARISE_RATIO = 0.6
