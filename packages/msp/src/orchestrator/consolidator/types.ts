import type { SessionTurn } from '../../memory/sessions/types.js'
import type { SlmClient } from '../../codegen/types.js'

/**
 * A single turn from session.jsonl. Re-exports the existing memory shape so
 * the consolidator can be wired to the sessions reader without remapping.
 */
export type Turn = SessionTurn

/**
 * Re-export the SLM client interface used by tier-2 calls. We deliberately
 * use the existing `SlmClient` factory contract — no new bundle / interface.
 */
export type LlmClient = SlmClient

/**
 * A contiguous group of turns (one possible episode candidate).
 */
export interface Chunk {
  /** Inclusive start index into the session turn array. */
  start: number
  /** Inclusive end index into the session turn array. */
  end: number
  /** The turns themselves (slice of session). */
  turns: Turn[]
}

/**
 * Verdict returned from tier-1 (deterministic) scoring.
 *
 * - `keep`: clearly important — promote to episode without LLM call
 * - `drop`: clearly not — skip
 * - `borderline`: ambiguous — needs tier-2 LLM, or default-keep if budget gone
 */
export type Verdict = 'keep' | 'drop' | 'borderline'

/**
 * Per-feature contribution map for debugging / introspection.
 */
export type ScoreBreakdown = Record<string, number>

/**
 * Tier-1 scoring result.
 */
export interface Tier1Result {
  score: number
  verdict: Verdict
  breakdown: ScoreBreakdown
}

/**
 * Tier-2 LLM result. `tier2-default` is used when the LLM was unavailable,
 * timed out, returned malformed JSON, or budget was exhausted — in all those
 * cases we default to keep with a deterministic summary (see ADR failure-modes).
 */
export type ScoreSource = 'tier1' | 'tier2' | 'tier2-default'

export interface Tier2Result {
  score: number
  summary: string
  tags: string[]
  source: 'tier2' | 'tier2-default'
}

/**
 * Per-session statistics used by length-normalised feature.
 */
export interface SessionStats {
  /** Total number of turns in the session. */
  turnCount: number
  /** Mean per-turn content length in bytes. */
  meanTurnBytes: number
  /** Standard deviation of per-turn content length (in bytes). */
  stddevTurnBytes: number
}

/**
 * Tunable scoring thresholds. Defaults from ADR--CONSOLIDATOR-HYBRID-SCORING.
 */
export interface Thresholds {
  /** Below this → tier-1 verdict 'drop'. Default 0.30. */
  low?: number
  /** Above this → tier-1 verdict 'keep'. Default 0.65. */
  high?: number
  /** Topic-continuity below this triggers an episode boundary. Default 0.25. */
  boundary?: number
}

export interface ConsolidateOptions {
  /** Required: the session id (file basename, without `.jsonl`). */
  sessionId: string
  /** Project root (defaults to `process.cwd()`). */
  root?: string
  /** Namespace under `.brain/msp/projects/<ns>/sessions/`. Default 'evaAI'. */
  namespace?: string
  /** Pluggable LLM client (tier-2). Absent → all borderline default-keep. */
  llm?: LlmClient
  /** Threshold overrides (see ADR for defaults). */
  thresholds?: Thresholds
  /** Max LLM calls per consolidation pass. Default 20. */
  maxLlmCallsPerSession?: number
  /** Per-call LLM timeout in ms. Default 8000. */
  llmCallTimeoutMs?: number
  /** Optional injected `now()` for testable timestamps. */
  now?: () => Date
}

export interface Episode {
  sessionId: string
  /** Inclusive [start, end] index range over the source session.jsonl. */
  turnRange: [number, number]
  /** 1–3 sentence summary. */
  summary: string
  /** 3–5 keywords. */
  tags: string[]
  /** Final importance score, 0..1. */
  score: number
  /** Where the score originated. */
  scoreSource: ScoreSource
  /** ISO 8601 creation timestamp. */
  createdAt: string
}

/**
 * Default threshold values from ADR--CONSOLIDATOR-HYBRID-SCORING.
 */
export const DEFAULT_THRESHOLDS: Required<Thresholds> = {
  low: 0.3,
  high: 0.65,
  boundary: 0.25,
}

export const DEFAULT_MAX_LLM_CALLS = 20
export const DEFAULT_LLM_TIMEOUT_MS = 8000
