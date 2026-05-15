import type { ObsidianClient } from '../../obsidian/client.js'
import type { RequestContext, Subject } from '../../policy/types.js'

/**
 * Logical retrieval source name. Each source produces ranked hits which
 * RRF fuses into a single result list.
 */
export type SourceName = 'gks-vector' | 'obsidian-text' | 'grep' | 'episodic' | 'backlinks'

/**
 * Minimal embedder shape accepted by the vector source. Compatible with the
 * GKS `Embedder` interface (only `embed(text)` is required); declared
 * locally so the orchestrator does not depend on `@freshair129/gks` types
 * at the public boundary.
 */
export interface RetrievalEmbedder {
  embed(text: string): Promise<number[]>
}

/**
 * Minimal vector backend shape accepted by the vector source. Mirrors the
 * GKS `VectorBackend.search()` surface (queryVec OR text → ranked hits).
 */
export interface RetrievalVectorBackend {
  search(
    query: string | number[],
    opts?: { topK?: number; scoreThreshold?: number },
  ): Promise<
    Array<{
      doc: { id: string; text?: string; metadata?: Record<string, unknown> }
      score: number
    }>
  >
}

/**
 * One hit emitted by a single source, prior to fusion. Rank is 1-based
 * within the source's own ranked output.
 */
export interface SourceHit {
  atomId: string
  rank: number
  snippet?: string
  source: SourceName
  /** §4 — Domain-specific attributes for policy checks (UCF). */
  attributes?: Record<string, any>
}

/**
 * Full result of one source adapter. `hits` may be empty on success
 * (source had nothing to say). `error` populates on failure (timeout,
 * exception). `skipped` populates when the source was deliberately not
 * invoked (e.g. no obsidian client).
 */
export interface SourceResult {
  source: SourceName
  hits: SourceHit[]
  latencyMs: number
  error?: string
  skipped?: string
}

/**
 * One fused hit returned by `recall()`. `score` is the RRF-summed score
 * (higher = better); `rank` is 1-based within the merged top-K list.
 */
export interface RetrievalHit {
  atomId: string
  source: SourceName
  score: number
  rank: number
  snippet?: string
  perSourceRanks: Partial<Record<SourceName, number>>
  /** §4 — Domain-specific attributes for policy checks (UCF). */
  attributes?: Record<string, any>
}

/**
 * Per-source latency breakdown (for debug / telemetry).
 */
export interface RetrievalTimings {
  vector?: number
  obsidian?: number
  episodic?: number
  backlinks?: number
  fusion: number
}

export interface RetrievalResult {
  hits: RetrievalHit[]
  /** True when the GKS vector path was operational (no embedder/backend error). */
  semantic_available: boolean
  /** True when an Obsidian REST client is reachable (for deep-link rendering). */
  obsidian_available: boolean
  /** Human-readable reasons for any source that did not contribute fully. */
  fallback_reasons: string[]
  timings: RetrievalTimings
}

export interface RecallOptions {
  query: string
  root?: string
  namespace?: string
  /**
   * Optional Obsidian client (M7a). If absent, the obsidian source is
   * skipped (recorded in `fallback_reasons`). Caller owns the lifecycle
   * — the source MUST NOT instantiate its own client.
   */
  obsidian?: ObsidianClient
  /**
   * Optional embedder for the vector source. If absent, vector source
   * returns no hits with `error: 'no-embedder'` and `semantic_available`
   * is false.
   */
  embedder?: RetrievalEmbedder
  /**
   * Optional vector backend for semantic search. If absent (and no
   * embedder either), vector source is a no-op.
   */
  vectorBackend?: RetrievalVectorBackend
  /** Default 10. */
  topK?: number
  /** Total budget across all sources. Default 1500ms. */
  timeoutMs?: number
  /**
   * Per-source timeout overrides. Each defaults to the ADR-specified value.
   * If `timeoutMs` is set, per-source budgets are scaled proportionally.
   */
  perSourceTimeouts?: Partial<Record<SourceName, number>>
  /** RRF per-source weight overrides. Defaults from ADR. */
  weights?: Partial<Record<SourceName, number>>
  /** RRF k constant. Default 60. */
  rrfK?: number

  /** §3 — UCF Subject (identity / clearance). */
  subject?: Subject
  /** §3 — UCF Request Context (trace id / time). */
  context?: RequestContext
}

/**
 * Default per-source RRF weights from `ADR--RETRIEVAL-RRF-FUSION`.
 * Tuning is M9 work via a `PARAM--` atom.
 */
export const DEFAULT_WEIGHTS: Record<SourceName, number> = {
  'gks-vector': 1.0,
  'obsidian-text': 0.8,
  grep: 0.6,
  episodic: 1.2,
  backlinks: 0.5,
}

/**
 * Default per-source timeouts (ms) from `ADR--RETRIEVAL-RRF-FUSION`.
 */
export const DEFAULT_PER_SOURCE_TIMEOUTS: Record<SourceName, number> = {
  'gks-vector': 800,
  'obsidian-text': 400,
  grep: 600,
  episodic: 100,
  backlinks: 100,
}

export const DEFAULT_TOTAL_TIMEOUT_MS = 1500
export const DEFAULT_TOP_K = 10
export const DEFAULT_RRF_K = 60
export const DEFAULT_NAMESPACE = 'evaAI'
