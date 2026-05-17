import { performance } from 'node:perf_hooks'
import { createGenesisGraphBackend } from '@freshair129/gks'
import { resolve } from 'node:path'

import type { SourceHit, SourceResult } from '../types.js'

export interface GraphSourceOptions {
  root: string
  namespace: string
  /** Atom ids from initial candidates to expand via structural reasoning. */
  candidateAtomIds: string[]
  topK: number
  timeoutMs: number
}

/**
 * Deep Reasoning source — multi-hop graph expansion using Genesis Graph (Cypher).
 *
 * This source takes candidates from vector/text search and finds structurally
 * related atoms (references, implements, supersedes) using graph traversal.
 *
 * Implements T1 of BLUEPRINT--DEEP-REASONING-RECALL.
 */
export async function graphSource(opts: GraphSourceOptions): Promise<SourceResult> {
  const start = performance.now()
  const dbPath = resolve(opts.root, 'gks') // For now, GKS root is the store

  if (opts.candidateAtomIds.length === 0) {
    return {
      source: 'graph',
      hits: [],
      latencyMs: Math.round(performance.now() - start),
    }
  }

  try {
    // 1. Initialize Genesis Graph Backend (TS subset)
    // Future: This should be cached or passed in via context
    const backend = createGenesisGraphBackend({ path: dbPath })
    await backend.load()

    const results = new Map<string, { score: number, hops: number }>()

    // 2. Perform Reasoning for each candidate
    // We use a multi-hop query to find related atoms.
    // Logic: find atoms within 3 hops via any crosslink type.
    for (const seedId of opts.candidateAtomIds) {
      const neighbors = await backend.neighbors(seedId, {
        depth: 3,
        direction: 'out',
        limit: opts.topK * 2
      })

      for (const n of neighbors) {
        const id = n.node.id
        if (opts.candidateAtomIds.includes(id)) continue // Skip self

        const existing = results.get(id)
        if (existing) {
          // Accumulate score: closer is better
          existing.score += (1 / n.depth)
        } else {
          results.set(id, { score: 1 / n.depth, hops: n.depth })
        }
      }
    }

    // 3. Rank and Slice
    const ranked = Array.from(results.entries())
      .map(([atomId, data]) => ({ atomId, ...data }))
      .sort((a, b) => b.score - a.score)
      .slice(0, opts.topK)

    const hits: SourceHit[] = ranked.map((r, i) => ({
      atomId: r.atomId,
      rank: i + 1,
      source: 'graph',
    }))

    return {
      source: 'graph',
      hits,
      latencyMs: Math.round(performance.now() - start),
    }
  } catch (err) {
    return {
      source: 'graph',
      hits: [],
      latencyMs: Math.round(performance.now() - start),
      error: (err as Error).message,
    }
  }
}
