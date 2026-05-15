import type { RetrievalHit } from '../retrieval/types.js'
import type { ResolutionTier } from './tier.js'

export interface BudgetOptions {
  maxTokens?: number
  onOverflow?: 'drop' | 'downgrade'
}

/**
 * Enforce token budget on retrieval hits by dropping or downgrading FULL hits.
 * MVP: Simple token count sum.
 */
export function enforceResolutionBudget(
  hits: Array<RetrievalHit & { tier: ResolutionTier; body?: string }>,
  opts: BudgetOptions = {},
): Array<RetrievalHit & { tier: ResolutionTier }> {
  const maxTokens = opts.maxTokens ?? 2000 // Default 2k tokens for context
  const onOverflow = opts.onOverflow ?? 'downgrade'

  let currentTokens = 0
  const out: Array<RetrievalHit & { tier: ResolutionTier }> = []

  for (const hit of hits) {
    // Heuristic: estimate tokens from hit snippet or body
    const hitTokens = hit.body ? estimateTokens(hit.body) : hit.snippet ? estimateTokens(hit.snippet) : 50

    if (hit.tier === 'FULL') {
      if (currentTokens + hitTokens > maxTokens) {
        if (onOverflow === 'downgrade') {
          out.push({ ...hit, tier: 'MENTION' })
          currentTokens += 50 // Fixed cost for mention
        }
        // else 'drop' -> skip hit
      } else {
        out.push(hit)
        currentTokens += hitTokens
      }
    } else {
      out.push(hit)
      currentTokens += 50 // MENTION budget
    }
  }

  return out
}

function estimateTokens(text: string): number {
  return Math.ceil(text.split(/\s+/).length * 1.3)
}
