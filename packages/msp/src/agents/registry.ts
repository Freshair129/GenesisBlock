import { claudeAdapter } from './tiers/claude.js'
import { geminiAdapter } from './tiers/gemini.js'
import { qwenAdapter } from './tiers/qwen.js'
import type { TierAdapter } from './tiers/types.js'
import type { Tier } from './types.js'

/**
 * Lookup table from Tier → TierAdapter implementation (P3).
 *
 * Kept as a function (not a constant) so that test-time `vi.mock()` of any
 * adapter module is observed: each call resolves the current bound import.
 */
export function getAdapter(tier: Tier): TierAdapter {
  switch (tier) {
    case 'T1':
      return qwenAdapter
    case 'T2':
      return geminiAdapter
    case 'T3':
      return claudeAdapter
    default: {
      // Exhaustiveness check — unreachable if Tier stays narrow.
      const exhaustive: never = tier
      throw new Error(`Unknown tier: ${String(exhaustive)}`)
    }
  }
}
