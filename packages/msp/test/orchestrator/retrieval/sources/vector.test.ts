import { describe, expect, it } from 'vitest'

import { vectorSource } from '../../../../src/orchestrator/retrieval/sources/vector.js'
import type {
  RetrievalEmbedder,
  RetrievalVectorBackend,
} from '../../../../src/orchestrator/retrieval/types.js'

function mockEmbedder(delayMs = 0): RetrievalEmbedder {
  return {
    async embed(_text: string): Promise<number[]> {
      if (delayMs > 0) await new Promise((r) => setTimeout(r, delayMs))
      return [0.1, 0.2, 0.3]
    },
  }
}

function mockBackend(
  hits: Array<{ id: string; text?: string; score: number }>,
): RetrievalVectorBackend {
  return {
    async search(_query, _opts) {
      return hits.map((h) => ({
        doc: { id: h.id, text: h.text },
        score: h.score,
      }))
    },
  }
}

describe('vectorSource', () => {
  it('returns mapped hits when embedder + backend succeed', async () => {
    const res = await vectorSource({
      query: 'rate limiting',
      topK: 3,
      timeoutMs: 1000,
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([
        { id: 'ADR--A', text: 'rate limiting decision', score: 0.92 },
        { id: 'CONCEPT--B', text: 'context body', score: 0.81 },
      ]),
    })
    expect(res.source).toBe('gks-vector')
    expect(res.hits).toHaveLength(2)
    expect(res.hits[0]).toMatchObject({
      atomId: 'ADR--A',
      rank: 1,
      source: 'gks-vector',
    })
    expect(res.hits[0]!.snippet).toContain('rate limiting decision')
    expect(res.hits[1]).toMatchObject({ atomId: 'CONCEPT--B', rank: 2 })
    expect(res.error).toBeUndefined()
  })

  it('returns empty + error: no-embedder when embedder is missing', async () => {
    const res = await vectorSource({
      query: 'q',
      topK: 5,
      timeoutMs: 100,
    })
    expect(res.hits).toEqual([])
    expect(res.error).toBe('no-embedder')
  })

  it('returns empty (no error) for empty query', async () => {
    const res = await vectorSource({
      query: '   ',
      topK: 5,
      timeoutMs: 100,
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([]),
    })
    expect(res.hits).toEqual([])
    expect(res.error).toBeUndefined()
  })

  it('returns empty + error: timeout when embedder is too slow', async () => {
    const res = await vectorSource({
      query: 'slow',
      topK: 5,
      timeoutMs: 20,
      embedder: mockEmbedder(200),
      vectorBackend: mockBackend([]),
    })
    expect(res.hits).toEqual([])
    expect(res.error).toBe('timeout')
  })

  it('populates latencyMs', async () => {
    const res = await vectorSource({
      query: 'q',
      topK: 1,
      timeoutMs: 1000,
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([{ id: 'A', score: 0.5 }]),
    })
    expect(res.latencyMs).toBeGreaterThanOrEqual(0)
    expect(typeof res.latencyMs).toBe('number')
  })
})
