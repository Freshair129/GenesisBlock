import { performance } from 'node:perf_hooks'

import { rrfFuse } from './fusion.js'
import { backlinksSource } from './sources/backlinks.js'
import { episodicSource } from './sources/episodic.js'
import { obsidianSource } from './sources/obsidian.js'
import { vectorSource } from './sources/vector.js'
import {
  DEFAULT_NAMESPACE,
  DEFAULT_PER_SOURCE_TIMEOUTS,
  DEFAULT_RRF_K,
  DEFAULT_TOP_K,
  DEFAULT_TOTAL_TIMEOUT_MS,
  type RecallOptions,
  type RetrievalResult,
  type SourceName,
  type SourceResult,
} from './types.js'
import { makeContext, makeSubject } from '../../policy/types.js'

export type {
  RecallOptions,
  RetrievalEmbedder,
  RetrievalHit,
  RetrievalResult,
  RetrievalTimings,
  RetrievalVectorBackend,
  SourceHit,
  SourceName,
  SourceResult,
} from './types.js'
export {
  DEFAULT_PER_SOURCE_TIMEOUTS,
  DEFAULT_RRF_K,
  DEFAULT_TOP_K,
  DEFAULT_TOTAL_TIMEOUT_MS,
  DEFAULT_WEIGHTS,
} from './types.js'
export { rrfFuse } from './fusion.js'

/**
 * Race a promise against a timer. Used for the total-budget enforcement at
 * the orchestrator level. Per-source timeouts are scaled from the total.
 */
function raceTimeout<T>(
  p: Promise<T>,
  timeoutMs: number,
  fallback: T,
): Promise<{ value: T; timedOut: boolean }> {
  return new Promise((resolve) => {
    let done = false
    const t = setTimeout(() => {
      if (done) return
      done = true
      resolve({ value: fallback, timedOut: true })
    }, timeoutMs)
    p.then(
      (value) => {
        if (done) return
        done = true
        clearTimeout(t)
        resolve({ value, timedOut: false })
      },
      () => {
        if (done) return
        done = true
        clearTimeout(t)
        resolve({ value: fallback, timedOut: false })
      },
    )
  })
}

/**
 * Scale per-source budgets when a tighter total `timeoutMs` is supplied.
 * Default per-source timeouts sum to a reference total; we scale down
 * proportionally to fit the caller's budget. We never scale UP — caller
 * may set a larger total but per-source defaults still cap each source.
 */
function scaleBudgets(
  totalBudget: number,
  overrides: Partial<Record<SourceName, number>> | undefined,
): Record<SourceName, number> {
  const baseline = { ...DEFAULT_PER_SOURCE_TIMEOUTS, ...(overrides ?? {}) }
  const referenceSum = Object.values(DEFAULT_PER_SOURCE_TIMEOUTS).reduce((a, b) => a + b, 0)
  if (totalBudget >= referenceSum) {
    return baseline as Record<SourceName, number>
  }
  // Scale down by ratio.
  const ratio = totalBudget / referenceSum
  const out = {} as Record<SourceName, number>
  for (const k of Object.keys(baseline) as SourceName[]) {
    out[k] = Math.max(20, Math.floor(baseline[k]! * ratio))
  }
  return out
}

function emptySettled(source: SourceName): SourceResult {
  return { source, hits: [], latencyMs: 0, error: 'timeout' }
}

function settledValue<T>(result: PromiseSettledResult<T>, fallback: T): T {
  if (result.status === 'fulfilled') return result.value
  return fallback
}

function uniqueAtomIds(results: SourceResult[]): string[] {
  const seen = new Set<string>()
  const out: string[] = []
  for (const r of results) {
    for (const hit of r.hits) {
      if (!seen.has(hit.atomId)) {
        seen.add(hit.atomId)
        out.push(hit.atomId)
      }
    }
  }
  return out
}

function collectFallbackReasons(results: SourceResult[]): string[] {
  const out: string[] = []
  for (const r of results) {
    if (r.error) out.push(`${r.source}: ${r.error}`)
    else if (r.skipped) out.push(`${r.source}: ${r.skipped}`)
  }
  return out
}

/**
 * MSP recall — fan out across 4 sources (GKS vector, Obsidian text,
 * episodic, backlinks), fuse via Reciprocal Rank Fusion. Read-only; no
 * caching across calls; idempotent given fixed inputs.
 *
 * See `FEAT--RETRIEVAL-ORCHESTRATION` and `ADR--RETRIEVAL-RRF-FUSION`.
 */
export async function recall(opts: RecallOptions): Promise<RetrievalResult> {
  const overallStart = performance.now()
  const root = opts.root ?? process.cwd()
  const namespace = opts.namespace ?? DEFAULT_NAMESPACE
  const topK = opts.topK ?? DEFAULT_TOP_K
  const totalBudget = opts.timeoutMs ?? DEFAULT_TOTAL_TIMEOUT_MS
  const rrfK = opts.rrfK ?? DEFAULT_RRF_K
  const weights = opts.weights ?? {}

  // UCF 4-tuple defaults and logging
  const subject = opts.subject ?? makeSubject('user', 'anonymous')
  const action = 'recall'
  const context = opts.context ?? makeContext('internal', 'orchestrator-recall')

  console.debug(
    `[ucf] 4-tuple: orchestrator.recall | sub:${subject.id} | act:${action} | trace:${context.trace_id}`,
  )

  const budgets = scaleBudgets(totalBudget, opts.perSourceTimeouts)

  // Phase A: 3 query-driven sources in parallel.
  const phaseAStart = performance.now()
  const settled = await Promise.allSettled([
    raceTimeout(
      vectorSource({
        query: opts.query,
        topK,
        timeoutMs: budgets['gks-vector'],
        embedder: opts.embedder,
        vectorBackend: opts.vectorBackend,
      }),
      budgets['gks-vector'] + 50,
      emptySettled('gks-vector'),
    ),
    raceTimeout(
      obsidianSource({
        obsidian: opts.obsidian,
        query: opts.query,
        topK,
        timeoutMs: budgets['obsidian-text'],
      }),
      budgets['obsidian-text'] + 50,
      emptySettled('obsidian-text'),
    ),
    raceTimeout(
      episodicSource({ root, namespace, query: opts.query, topK }),
      budgets.episodic + 50,
      emptySettled('episodic'),
    ),
  ])

  const vectorRes = settledValue(settled[0], {
    value: emptySettled('gks-vector'),
    timedOut: false,
  }).value
  const obsidianRes = settledValue(settled[1], {
    value: emptySettled('obsidian-text'),
    timedOut: false,
  }).value
  const episodicRes = settledValue(settled[2], {
    value: emptySettled('episodic'),
    timedOut: false,
  }).value

  const phaseAElapsed = performance.now() - phaseAStart
  const remaining = Math.max(50, totalBudget - phaseAElapsed)

  // Phase B: backlinks expansion from phase-A candidates.
  const candidates = uniqueAtomIds([vectorRes, obsidianRes, episodicRes])
  const backlinksRaced = await raceTimeout(
    backlinksSource({
      root,
      namespace,
      candidateAtomIds: candidates,
      topK,
    }),
    Math.min(budgets.backlinks + 50, remaining),
    emptySettled('backlinks'),
  )
  const backlinksRes = backlinksRaced.value

  // Phase C: fuse.
  const fuseStart = performance.now()
  const allResults: SourceResult[] = [vectorRes, obsidianRes, episodicRes, backlinksRes]
  const fusedHits = rrfFuse(allResults, { k: rrfK, weights, topK })
  const fusionMs = performance.now() - fuseStart

  // Compute output flags + diagnostics.
  const semanticAvailable = !!opts.embedder && !!opts.vectorBackend && !vectorRes.error
  const obsidianAvailable = opts.obsidian?.mode === 'rest'

  const result: RetrievalResult = {
    hits: fusedHits,
    semantic_available: semanticAvailable,
    obsidian_available: obsidianAvailable,
    fallback_reasons: collectFallbackReasons(allResults),
    timings: {
      vector: vectorRes.latencyMs,
      obsidian: obsidianRes.latencyMs,
      episodic: episodicRes.latencyMs,
      backlinks: backlinksRes.latencyMs,
      fusion: Math.round(fusionMs),
    },
  }

  // Total-elapsed sanity log (debug only — not exposed unless callers need it).
  void (performance.now() - overallStart)

  return result
}
