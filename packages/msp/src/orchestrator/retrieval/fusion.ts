import {
  DEFAULT_RRF_K,
  DEFAULT_TOP_K,
  DEFAULT_WEIGHTS,
  type RetrievalHit,
  type SourceName,
  type SourceResult,
} from './types.js'

/**
 * Internal accumulator for RRF aggregation.
 */
interface AtomScore {
  atomId: string
  score: number
  contributions: Array<{ source: SourceName; rank: number }>
  /** First non-empty snippet seen (deterministic by source iteration order). */
  snippet?: string
  /** First source seen (used for the per-hit `source` field on the fused hit). */
  primarySource: SourceName
  perSourceRanks: Partial<Record<SourceName, number>>
}

export interface RrfFuseOptions {
  k?: number
  weights?: Partial<Record<SourceName, number>>
  topK?: number
}

/**
 * Pure Reciprocal Rank Fusion over per-source ranked hit lists.
 *
 * `score_s(hit) = weight_s / (k + rank_s)`; per-atom score = sum over sources
 * where the atom appears. Sort:
 *   1. score DESC
 *   2. sourceCount DESC (tie-break: more sources)
 *   3. min source rank ASC (best single-source rank)
 *   4. atomId lexicographic ASC
 *
 * Slice top-K, assign final ranks 1..topK.
 */
export function rrfFuse(
  perSource: SourceResult[],
  opts: RrfFuseOptions = {},
): RetrievalHit[] {
  const k = opts.k ?? DEFAULT_RRF_K
  const topK = opts.topK ?? DEFAULT_TOP_K
  const weights = opts.weights ?? {}

  const acc = new Map<string, AtomScore>()

  for (const sourceResult of perSource) {
    const sourceName = sourceResult.source
    const weight =
      weights[sourceName] ?? DEFAULT_WEIGHTS[sourceName] ?? 1.0

    for (const hit of sourceResult.hits) {
      const rank = hit.rank
      if (!Number.isFinite(rank) || rank < 1) continue

      const contribution = weight / (k + rank)
      const existing = acc.get(hit.atomId)

      if (existing) {
        existing.score += contribution
        existing.contributions.push({ source: sourceName, rank })
        existing.perSourceRanks[sourceName] = rank
        if (!existing.snippet && hit.snippet) {
          existing.snippet = hit.snippet
        }
      } else {
        const entry: AtomScore = {
          atomId: hit.atomId,
          score: contribution,
          contributions: [{ source: sourceName, rank }],
          snippet: hit.snippet,
          primarySource: sourceName,
          perSourceRanks: { [sourceName]: rank } as Partial<
            Record<SourceName, number>
          >,
        }
        acc.set(hit.atomId, entry)
      }
    }
  }

  const sorted = Array.from(acc.values()).sort((a, b) => {
    // 1. score DESC
    if (b.score !== a.score) return b.score - a.score
    // 2. sourceCount DESC
    if (a.contributions.length !== b.contributions.length) {
      return b.contributions.length - a.contributions.length
    }
    // 3. min source rank ASC
    const aMin = Math.min(...a.contributions.map((c) => c.rank))
    const bMin = Math.min(...b.contributions.map((c) => c.rank))
    if (aMin !== bMin) return aMin - bMin
    // 4. lexicographic atomId
    if (a.atomId < b.atomId) return -1
    if (a.atomId > b.atomId) return 1
    return 0
  })

  const sliced = sorted.slice(0, Math.max(0, topK))

  return sliced.map((entry, i) => ({
    atomId: entry.atomId,
    source: entry.primarySource,
    score: entry.score,
    rank: i + 1,
    snippet: entry.snippet,
    perSourceRanks: entry.perSourceRanks,
  }))
}
