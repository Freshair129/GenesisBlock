import { describe, expect, it } from 'vitest'

import { heuristicSummariser } from '../../../src/memory/episodic/summarisers/heuristic.js'
import type { SessionTurn } from '../../../src/memory/sessions/types.js'

function turn(over: Partial<SessionTurn>): SessionTurn {
  return {
    sessionId: 'sess',
    episodicId: 'ep',
    turnId: 1,
    msgId: 'm',
    speakerId: 'user',
    content: '',
    ...over,
  }
}

describe('heuristicSummariser', () => {
  it('handles empty input', async () => {
    const r = await heuristicSummariser([])
    expect(r.summary).toMatch(/no turns/)
    expect(r.key_decisions).toBeUndefined()
  })

  it('uses first non-trivial assistant turn as summary anchor', async () => {
    const r = await heuristicSummariser([
      turn({ turnId: 1, speakerId: 'user', content: 'hi' }),
      turn({ turnId: 2, speakerId: 'MSP-AGT-CLAUDE', content: 'I think we should adopt Postgres for storage.' }),
      turn({ turnId: 3, speakerId: 'user', content: 'sounds good' }),
    ])
    expect(r.summary).toMatch(/Postgres/)
  })

  it('extracts decision-shaped sentences', async () => {
    const r = await heuristicSummariser([
      turn({ turnId: 1, speakerId: 'MSP-AGT', content: 'We decided to use bcrypt. Let us also pick Argon2id for new accounts.' }),
    ])
    expect(r.key_decisions).toBeDefined()
    expect(r.key_decisions!.length).toBeGreaterThanOrEqual(1)
    expect(r.key_decisions!.some((s) => /decided/i.test(s))).toBe(true)
  })

  it('caps key_decisions to 5 entries', async () => {
    const lines = Array.from({ length: 10 }, (_, i) => `We decided to do thing ${i}.`).join(' ')
    const r = await heuristicSummariser([turn({ content: lines })])
    expect(r.key_decisions!.length).toBeLessThanOrEqual(5)
  })

  it('returns empty summary fallback for content shorter than threshold', async () => {
    const r = await heuristicSummariser([turn({ content: 'short' })])
    expect(r.summary).toBe('short')
  })
})
