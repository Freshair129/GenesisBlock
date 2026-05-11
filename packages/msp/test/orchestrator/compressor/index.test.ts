import { describe, expect, it } from 'vitest'

import { compress } from '../../../src/orchestrator/compressor/index.js'
import type { LlmClient, Turn } from '../../../src/orchestrator/consolidator/types.js'
import type { CompressorEpisode } from '../../../src/orchestrator/compressor/types.js'

function turn(content: string, speakerId = 'a', turnId = 0): Turn {
  return {
    sessionId: 's1',
    episodicId: 'e1',
    turnId,
    msgId: `m${turnId}`,
    speakerId,
    content,
  }
}

function makeEpisode(
  id: string,
  contents: string[],
  score: number,
  startTurn = 0,
): CompressorEpisode {
  return {
    sessionId: id,
    turnRange: [startTurn, startTurn + Math.max(0, contents.length - 1)],
    summary: `summary of ${id}`,
    score,
    turns: contents.map((c, i) => turn(c, 'a', startTurn + i)),
  }
}

const FILLER = 'hi thanks ok'
const FILLER_2 = 'sure noted yep'
const HIGH = 'we will use pgvector per ADR--RETRIEVAL'
const HIGH_2 = "let's ship at v1.2.3 in src/foo.ts"

describe('compress — end-to-end', () => {
  it('all-keep: every episode fits the budget verbatim', async () => {
    const eps = [
      makeEpisode('s1', ['short a'], 0.9),
      makeEpisode('s2', ['short b'], 0.5),
    ]
    const r = await compress({ episodes: eps, budgetTokens: 10_000 })
    expect(r.tierCounts.keep).toBe(2)
    expect(r.tierCounts.trim).toBe(0)
    expect(r.tierCounts.resummarise).toBe(0)
    expect(r.tierCounts.truncated).toBe(0)
    expect(r.tierCounts.dropped).toBe(0)
    expect(r.compressed).toHaveLength(2)
    for (const c of r.compressed) {
      expect(c.compressedBy).toBe('keep')
      expect(c.droppedTurnIndices).toEqual([])
    }
    expect(r.totalTokensUsed).toBeLessThanOrEqual(10_000)
  })

  it('mixed tiers — keep + trim + truncate (no LLM, headless)', async () => {
    const eps = [
      // Will keep verbatim.
      makeEpisode('high', ['short high'], 0.95),
      // Has lots of fillers — trim should drop them.
      makeEpisode(
        'mid',
        [HIGH, FILLER, FILLER_2, FILLER, FILLER_2, FILLER],
        0.75,
      ),
      // Long all-signal — only truncate fits it.
      makeEpisode(
        'long',
        [HIGH, HIGH_2, HIGH, HIGH_2, HIGH, HIGH_2, HIGH, HIGH_2],
        0.3,
      ),
    ]
    const r = await compress({ episodes: eps, budgetTokens: 80 })
    // No LLM → resummarise count should be 0.
    expect(r.tierCounts.resummarise).toBe(0)
    expect(r.totalTokensUsed).toBeLessThanOrEqual(80)
    // Total emitted ≤ 3.
    expect(r.compressed.length).toBeLessThanOrEqual(3)
  })

  it('resummarise tier fires when LLM is supplied and trim/keep do not fit', async () => {
    const llmCalls: string[] = []
    const llm: LlmClient = async ({ prompt }) => {
      llmCalls.push(prompt)
      // Short summary that easily fits any reasonable target.
      return 'tiny summary'
    }
    const eps = [
      makeEpisode(
        'long',
        [HIGH, HIGH_2, HIGH, HIGH_2, HIGH, HIGH_2],
        0.6,
      ),
    ]
    const r = await compress({ episodes: eps, budgetTokens: 30, llm })
    expect(r.tierCounts.resummarise).toBeGreaterThanOrEqual(1)
    expect(r.compressed[0]!.compressedBy).toBe('resummarise')
    expect(r.compressed[0]!.text).toBe('tiny summary')
    expect(llmCalls.length).toBe(1)
    expect(r.totalTokensUsed).toBeLessThanOrEqual(30)
  })

  it('LLM timeout falls through to truncate (headless-safe)', async () => {
    const slowLlm: LlmClient = () => new Promise(() => {})
    const eps = [
      makeEpisode(
        'long',
        [HIGH, HIGH_2, HIGH, HIGH_2, HIGH, HIGH_2],
        0.6,
      ),
    ]
    const r = await compress({
      episodes: eps,
      budgetTokens: 30,
      llm: slowLlm,
      llmTimeoutMs: 25,
    })
    // No resummarise; should have truncated instead.
    expect(r.tierCounts.resummarise).toBe(0)
    expect(r.tierCounts.truncated).toBeGreaterThanOrEqual(1)
    expect(r.totalTokensUsed).toBeLessThanOrEqual(30)
  })

  it('iterates in importance-descending order — high-importance gets keep slot', async () => {
    // Two episodes, only one fits the budget. The HIGHER score must win.
    const eps = [
      makeEpisode('low', ['low score content'], 0.2),
      makeEpisode('high', ['high score content'], 0.9),
    ]
    const r = await compress({ episodes: eps, budgetTokens: 8 })
    // Only one fits.
    expect(r.compressed.length).toBeLessThanOrEqual(2)
    // The high-importance one MUST be present (kept verbatim).
    const highEntry = r.compressed.find((c) =>
      'sessionId' in c.episodeRef && c.episodeRef.sessionId === 'high',
    )
    expect(highEntry).toBeDefined()
    expect(highEntry!.compressedBy).toBe('keep')
  })

  it('preserveOrder=true reorders OUTPUT chronologically by turnRange[0]', async () => {
    const eps = [
      makeEpisode('s1', ['x'], 0.3, 100), // chronologically last
      makeEpisode('s1', ['y'], 0.9, 0),   // chronologically first, highest score
      makeEpisode('s1', ['z'], 0.6, 50),  // chronologically middle
    ]
    const r = await compress({
      episodes: eps,
      budgetTokens: 10_000,
      preserveOrder: true,
    })
    expect(r.compressed).toHaveLength(3)
    const starts = r.compressed.map((c) => {
      const ref = c.episodeRef
      return 'turnRange' in ref && ref.turnRange ? ref.turnRange[0] : -1
    })
    expect(starts).toEqual([0, 50, 100])
  })

  it('preserveOrder=false (default) leaves selection in importance-descending order', async () => {
    const eps = [
      makeEpisode('s1', ['x'], 0.3, 100),
      makeEpisode('s1', ['y'], 0.9, 0),
      makeEpisode('s1', ['z'], 0.6, 50),
    ]
    const r = await compress({ episodes: eps, budgetTokens: 10_000 })
    const scores = r.compressed.map((c) => c.score)
    expect(scores).toEqual([0.9, 0.6, 0.3])
  })

  it('totalTokensUsed equals the sum of compressedTokens', async () => {
    const eps = [
      makeEpisode('s1', ['a a a'], 0.9),
      makeEpisode('s2', ['b b b b'], 0.7),
      makeEpisode('s3', ['c c c c c'], 0.5),
    ]
    const r = await compress({ episodes: eps, budgetTokens: 10_000 })
    const sum = r.compressed.reduce((acc, c) => acc + c.compressedTokens, 0)
    expect(r.totalTokensUsed).toBe(sum)
  })

  it('drops episodes that cannot fit even their summary', async () => {
    const eps = [
      makeEpisode('big', ['short content'], 0.9),
      makeEpisode('small', ['x'], 0.5),
    ]
    // Tiny budget that fits the first short content but the SECOND
    // episode's summary won't fit in what's left.
    const r = await compress({ episodes: eps, budgetTokens: 4 })
    // We expect at least one episode to be dropped (no LLM, no truncate
    // possible because even summary won't fit).
    expect(r.tierCounts.dropped).toBeGreaterThanOrEqual(1)
    expect(r.totalTokensUsed).toBeLessThanOrEqual(4)
  })

  it('budgetTokens=0 → all dropped', async () => {
    const eps = [
      makeEpisode('s1', ['anything'], 0.9),
      makeEpisode('s2', ['else'], 0.5),
    ]
    const r = await compress({ episodes: eps, budgetTokens: 0 })
    expect(r.compressed).toHaveLength(0)
    expect(r.tierCounts.dropped).toBe(2)
    expect(r.totalTokensUsed).toBe(0)
  })

  it('does not mutate input episodes array or any episode', async () => {
    const eps = [
      makeEpisode('s1', ['a', 'b', 'c'], 0.5),
      makeEpisode('s2', ['x', 'y'], 0.9),
    ]
    const before = JSON.parse(JSON.stringify(eps))
    await compress({ episodes: eps, budgetTokens: 50 })
    expect(eps).toEqual(before)
  })

  it('every output entry carries an episodeRef with provenance', async () => {
    const eps = [
      { ...makeEpisode('s1', ['x'], 0.9), atomId: 'episodic-1' },
      makeEpisode('s2', ['y'], 0.5),
    ]
    const r = await compress({ episodes: eps, budgetTokens: 10_000 })
    expect(r.compressed).toHaveLength(2)
    for (const c of r.compressed) {
      expect(c.episodeRef).toBeDefined()
      // Either sessionId+turnRange OR atomId must be present.
      const ref = c.episodeRef
      const hasSession =
        'sessionId' in ref && typeof ref.sessionId === 'string'
      const hasAtomId = 'atomId' in ref && typeof ref.atomId === 'string'
      expect(hasSession || hasAtomId).toBe(true)
    }
    // The episode that came in with atomId should preserve it.
    const withAtom = r.compressed.find(
      (c) => 'atomId' in c.episodeRef && c.episodeRef.atomId === 'episodic-1',
    )
    expect(withAtom).toBeDefined()
  })

  it('idempotent under same input + budget with deterministic mock LLM', async () => {
    const llm: LlmClient = async () => 'fixed summary'
    const eps = [
      makeEpisode('s1', [HIGH, HIGH_2, HIGH, HIGH_2, HIGH, HIGH_2], 0.6),
      makeEpisode('s2', ['short'], 0.9),
    ]
    const a = await compress({ episodes: eps, budgetTokens: 25, llm })
    const b = await compress({ episodes: eps, budgetTokens: 25, llm })
    expect(a).toEqual(b)
  })
})
