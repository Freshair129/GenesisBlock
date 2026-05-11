import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { handler, name } from '../../../src/mcp/tools/compress.js'
import type { CompressorEpisode } from '../../../src/orchestrator/compressor/types.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-compress-tool-'))
}

function makeEpisode(opts: {
  sessionId?: string
  start?: number
  end?: number
  summary?: string
  score?: number
  turnCount?: number
} = {}): CompressorEpisode {
  const sessionId = opts.sessionId ?? 'sess-test'
  const start = opts.start ?? 0
  const end = opts.end ?? (opts.turnCount ? start + opts.turnCount - 1 : start + 2)
  const turnCount = opts.turnCount ?? end - start + 1
  const turns = Array.from({ length: turnCount }, (_, i) => ({
    sessionId,
    episodicId: `${sessionId}-${start + i}`,
    turnId: start + i,
    msgId: `msg-${start + i}`,
    speakerId: i % 2 === 0 ? 'user' : 'agent',
    content: `Turn ${start + i} content with some details about rate limiting decisions and pgvector configuration.`,
  }))
  return {
    sessionId,
    turnRange: [start, end],
    summary: opts.summary ?? 'Decision about rate limiting using pgvector.',
    score: opts.score ?? 0.7,
    turns,
  }
}

describe('msp_compress tool', () => {
  it('has the right name', () => {
    expect(name).toBe('msp_compress')
  })

  it('compresses episodes within budget and returns tier counts', async () => {
    const root = await freshRoot()
    const episodes = [
      makeEpisode({ start: 0, end: 0, score: 0.9, turnCount: 1 }),
      makeEpisode({ start: 1, end: 1, score: 0.5, turnCount: 1 }),
    ]
    const result = await handler({ root })({
      episodes,
      budget_tokens: 10000, // generous budget so all keep
      provider: 'mock',
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.compressed).toHaveLength(2)
    expect(parsed.total_tokens_used).toBeLessThanOrEqual(10000)
    expect(parsed.tier_counts).toEqual({
      keep: 2,
      trim: 0,
      resummarise: 0,
      truncated: 0,
      dropped: 0,
    })
  })

  it('respects token budget under tight constraints', async () => {
    const root = await freshRoot()
    const episodes = [
      makeEpisode({ start: 0, end: 4, score: 0.9, turnCount: 5 }),
      makeEpisode({ start: 10, end: 14, score: 0.2, turnCount: 5 }),
    ]
    const result = await handler({ root })({
      episodes,
      budget_tokens: 50, // very tight — forces compression decisions
      provider: 'mock',
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.total_tokens_used).toBeLessThanOrEqual(50)
    // At least some compression happened (not all `keep`).
    const nonKeep =
      parsed.tier_counts.trim +
      parsed.tier_counts.resummarise +
      parsed.tier_counts.truncated +
      parsed.tier_counts.dropped
    expect(nonKeep).toBeGreaterThan(0)
  })

  it('preserves provenance (sessionId + turnRange) on each compressed entry', async () => {
    const root = await freshRoot()
    const episodes = [makeEpisode({ start: 5, end: 7, turnCount: 3 })]
    const result = await handler({ root })({
      episodes,
      budget_tokens: 1000,
      provider: 'mock',
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.compressed[0].episodeRef.sessionId).toBe('sess-test')
    expect(parsed.compressed[0].episodeRef.turnRange).toEqual([5, 7])
  })

  it('does not mutate input episodes', async () => {
    const root = await freshRoot()
    const ep = makeEpisode({ turnCount: 3 })
    const before = JSON.stringify(ep)
    await handler({ root })({
      episodes: [ep],
      budget_tokens: 1000,
      provider: 'mock',
    })
    expect(JSON.stringify(ep)).toBe(before)
  })

  it('returns error result on invalid budget', async () => {
    const root = await freshRoot()
    const result = await handler({ root })({
      episodes: [makeEpisode()],
      budget_tokens: -1, // invalid; compress() may handle or this surfaces an error
      provider: 'mock',
    })
    const parsed = JSON.parse(result.content[0]!.text)
    // Either ok:true with everything dropped, or ok:false with an error.
    if (parsed.ok) {
      expect(parsed.compressed).toEqual([])
    } else {
      expect(parsed.error).toBeDefined()
    }
  })
})
