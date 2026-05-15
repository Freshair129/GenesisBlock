import type { RetrievalHit } from '../retrieval/types.js'

/**
 * Resolution tiers for atoms in context.
 * See ADR--RESOLUTION-TIER-COUNT.
 */
export type ResolutionTier = 'FULL' | 'MENTION' | 'SKELETON' | 'SUMMARY'

/**
 * Assign resolution tiers to retrieval hits.
 * MVP Implementation: Top 3 hits get FULL, others get MENTION.
 */
export function assignResolutionTiers(
  hits: RetrievalHit[],
  opts: { fullCount?: number } = {},
): Array<RetrievalHit & { tier: ResolutionTier }> {
  const fullCount = opts.fullCount ?? 3

  return hits.map((hit, i) => {
    const tier: ResolutionTier = i < fullCount ? 'FULL' : 'MENTION'
    return { ...hit, tier }
  })
}
