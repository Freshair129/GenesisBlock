import { describe, expect, it } from 'vitest'

import { obsidianSource } from '../../../../src/orchestrator/retrieval/sources/obsidian.js'
import type {
  ObsidianClient,
  SearchHit,
} from '../../../../src/obsidian/types.js'

function mockClient(
  mode: 'rest' | 'filesystem',
  hits: SearchHit[],
  delayMs = 0,
): ObsidianClient {
  return {
    mode,
    async search(_query, _opts) {
      if (delayMs > 0) await new Promise((r) => setTimeout(r, delayMs))
      return hits
    },
    async readFile(_p) {
      return ''
    },
  }
}

describe('obsidianSource', () => {
  it('emits source=obsidian-text for rest-mode client', async () => {
    const client = mockClient('rest', [
      {
        path: 'gks/adr/ADR--RATE-LIMIT.md',
        title: 'rate limit',
        snippet: 'rate limit decision',
        score: 1.4,
      },
    ])
    const res = await obsidianSource({
      obsidian: client,
      query: 'rate limit',
      topK: 5,
      timeoutMs: 1000,
    })
    expect(res.source).toBe('obsidian-text')
    expect(res.hits).toHaveLength(1)
    expect(res.hits[0]!.atomId).toBe('ADR--RATE-LIMIT')
    expect(res.hits[0]!.rank).toBe(1)
    expect(res.hits[0]!.source).toBe('obsidian-text')
    expect(res.hits[0]!.snippet).toBe('rate limit decision')
  })

  it('emits source=grep for filesystem-mode client', async () => {
    const client = mockClient('filesystem', [
      {
        path: 'gks/concept/CONCEPT--FOO.md',
        title: 'foo',
        snippet: 'foo body',
        score: 0.5,
      },
    ])
    const res = await obsidianSource({
      obsidian: client,
      query: 'foo',
      topK: 5,
      timeoutMs: 1000,
    })
    expect(res.source).toBe('grep')
    expect(res.hits[0]!.source).toBe('grep')
  })

  it('returns empty + skipped when no client provided', async () => {
    const res = await obsidianSource({
      query: 'q',
      topK: 5,
      timeoutMs: 1000,
    })
    expect(res.hits).toEqual([])
    expect(res.skipped).toBe('no-client')
  })

  it('returns empty + error on timeout', async () => {
    const client = mockClient('rest', [], 100)
    const res = await obsidianSource({
      obsidian: client,
      query: 'q',
      topK: 5,
      timeoutMs: 20,
    })
    expect(res.hits).toEqual([])
    expect(res.error).toBe('timeout')
  })

  it('preserves rank ordering from search results 1..N', async () => {
    const client = mockClient('rest', [
      { path: 'a/A.md', title: 'a', snippet: '', score: 0.9 },
      { path: 'b/B.md', title: 'b', snippet: '', score: 0.5 },
      { path: 'c/C.md', title: 'c', snippet: '', score: 0.2 },
    ])
    const res = await obsidianSource({
      obsidian: client,
      query: 'q',
      topK: 5,
      timeoutMs: 1000,
    })
    expect(res.hits.map((h) => h.rank)).toEqual([1, 2, 3])
  })

  it('populates latencyMs', async () => {
    const client = mockClient('rest', [])
    const res = await obsidianSource({
      obsidian: client,
      query: 'q',
      topK: 1,
      timeoutMs: 1000,
    })
    expect(res.latencyMs).toBeGreaterThanOrEqual(0)
  })
})
