import { performance } from 'node:perf_hooks'

import { rrfFuse } from './fusion.js'
import { backlinksSource } from './sources/backlinks.js'
import { graphSource } from './sources/graph.js'
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
  type RetrievalHit,
  type RetrievalResult,
  type SourceName,
  type SourceResult,
} from './types.js'
import { makeContext, makeResource, makeSubject, type Action } from '../../policy/types.js'
import { enforcePolicy } from '../../policy/pep.js'
import { resolveVault, vaultReadNamespaces } from '../../vault/registry.js'
import type { Namespace } from '@freshair129/gks'

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
  const topK = opts.topK ?? DEFAULT_TOP_K
  const totalBudget = opts.timeoutMs ?? DEFAULT_TOTAL_TIMEOUT_MS
  const rrfK = opts.rrfK ?? DEFAULT_RRF_K
  const weights = opts.weights ?? {}

  // UCF 4-tuple defaults and logging
  const subject = opts.subject ?? makeSubject('user', 'anonymous')
  const action: Action = 'recall'
  const context = opts.context ?? makeContext('internal', 'orchestrator-recall')

  console.debug(
    `[ucf] 4-tuple: orchestrator.recall | sub:${subject.id} | act:${action} | trace:${context.trace_id}`,
  )

  // Resolve Namespaces from Vault (Layer 1)
  let readNamespaces: Namespace[] = []
  if (opts.vaultId) {
    const vault = resolveVault(opts.vaultId)
    if (!vault) {
      console.warn(`[ucf] vault not found: ${opts.vaultId}, falling back to default namespace`)
      readNamespaces = [{ tenant_id: opts.namespace ?? DEFAULT_NAMESPACE }]
    } else {
      readNamespaces = vaultReadNamespaces(vault)
    }
  } else {
    readNamespaces = [{ tenant_id: opts.namespace ?? DEFAULT_NAMESPACE }]
  }

  const budgets = scaleBudgets(totalBudget, opts.perSourceTimeouts)

  // Phase A: 3 query-driven sources in parallel across ALL namespaces.
  // For MVP: we run one vectorSource call per namespace and fuse them.
  // Future: GKS should support multi-namespace filter natively.
  const phaseAStart = performance.now()
  const sourceTasks: Array<Promise<{ value: SourceResult; timedOut: boolean }>> = []

  for (const ns of readNamespaces) {
    const nsId = ns.tenant_id ?? DEFAULT_NAMESPACE
    sourceTasks.push(
      raceTimeout(
        vectorSource({
          query: opts.query,
          topK,
          timeoutMs: budgets['gks-vector'],
          embedder: opts.embedder,
          vectorBackend: opts.vectorBackend,
          // namespace: nsId, // TODO: Update vectorSource to support namespace filtering
        }),
        budgets['gks-vector'] + 50,
        emptySettled('gks-vector'),
      ),
    )
  }

  // Obsidian and Episodic (still single-namespace for now in MVP)
  sourceTasks.push(
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
  )

  sourceTasks.push(
    raceTimeout(
      episodicSource({ root, namespace: readNamespaces[0]?.tenant_id ?? DEFAULT_NAMESPACE, query: opts.query, topK }),
      budgets.episodic + 50,
      emptySettled('episodic'),
    ),
  )

  const settled = await Promise.allSettled(sourceTasks)
  
  const results: SourceResult[] = []
  for (const s of settled) {
    if (s.status === 'fulfilled') results.push(s.value.value)
  }

  // Separate results by source for latency tracking (simplified for MVP)
  const vectorRes = results.find(r => r.source === 'gks-vector') ?? emptySettled('gks-vector')
  const obsidianRes = results.find(r => r.source === 'obsidian-text') ?? emptySettled('obsidian-text')
  const episodicRes = results.find(r => r.source === 'episodic') ?? emptySettled('episodic')

  const phaseAElapsed = performance.now() - phaseAStart
  const remaining = Math.max(50, totalBudget - phaseAElapsed)

  // Phase B: multi-hop graph expansion from phase-A candidates.
  const candidates = uniqueAtomIds(results)
  const graphRaced = await raceTimeout(
    graphSource({
      root,
      namespace: readNamespaces[0]?.tenant_id ?? DEFAULT_NAMESPACE,
      candidateAtomIds: candidates,
      topK,
      timeoutMs: budgets.graph,
    }),
    Math.min(budgets.graph + 50, remaining),
    emptySettled('graph'),
  )
  const graphRes = graphRaced.value

  // Phase C: fuse.
  const fuseStart = performance.now()
  const allResults: SourceResult[] = [...results, graphRes]
  const fusedHits = rrfFuse(allResults, { k: rrfK, weights, topK })

  // Phase D: Enforce Policy (PEP)
  const filteredHits: RetrievalHit[] = []
  const pepOpts = { root, subject, action, context }

  for (const hit of fusedHits) {
    const attributes = {
      ...(hit.attributes ?? {}),
      body: hit.snippet, // Inject snippet as body for regex matching (UCF Phase 4 PII pack)
    }
    const resource = makeResource('atom', hit.atomId, {}, attributes)
    const { permitted } = await enforcePolicy(resource, pepOpts)
    if (permitted) {
      filteredHits.push(hit)
    }
  }

  const fusionMs = performance.now() - fuseStart

  // Compute output flags + diagnostics.
  const semanticAvailable = !!opts.embedder && !!opts.vectorBackend && !vectorRes.error
  const obsidianAvailable = opts.obsidian?.mode === 'rest'

  const result: RetrievalResult = {
    hits: filteredHits,
    semantic_available: semanticAvailable,
    obsidian_available: obsidianAvailable,
    fallback_reasons: collectFallbackReasons(allResults),
    timings: {
      vector: vectorRes.latencyMs,
      obsidian: obsidianRes.latencyMs,
      episodic: episodicRes.latencyMs,
      graph: graphRes.latencyMs,
      fusion: Math.round(fusionMs),
    },
  }

  // Total-elapsed sanity log (debug only — not exposed unless callers need it).
  void (performance.now() - overallStart)

  return result
}
