import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { recall } from '../../../src/orchestrator/retrieval/index.js'
import type {
  RetrievalEmbedder,
  RetrievalVectorBackend,
} from '../../../src/orchestrator/retrieval/types.js'
import type {
  ObsidianClient,
  SearchHit,
} from '../../../src/obsidian/types.js'
import type { Episode } from '../../../src/memory/episodic/types.js'

interface FixtureOpts {
  episodes?: Episode[]
  edges?: Array<{ from: string; to: string; type: string }>
  namespace?: string
}

async function setupRoot(opts: FixtureOpts = {}): Promise<string> {
  const namespace = opts.namespace ?? 'evaAI'
  const root = await mkdtemp(join(tmpdir(), 'msp-retrieval-'))
  const memoryDir = join(root, '.brain/msp/projects', namespace, 'memory')
  const vectorDir = join(root, '.brain/msp/projects', namespace, 'vector')
  await mkdir(memoryDir, { recursive: true })
  await mkdir(vectorDir, { recursive: true })
  if (opts.episodes) {
    await writeFile(
      join(memoryDir, 'episodic_memory.json'),
      JSON.stringify(opts.episodes),
      'utf8',
    )
  }
  if (opts.edges) {
    const lines =
      opts.edges.map((e) => JSON.stringify(e)).join('\n') +
      (opts.edges.length > 0 ? '\n' : '')
    await writeFile(join(vectorDir, 'backlinks.jsonl'), lines, 'utf8')
  }
  return root
}

function mockEmbedder(): RetrievalEmbedder {
  return {
    async embed() {
      return [0.1, 0.2, 0.3]
    },
  }
}

function mockBackend(
  hits: Array<{ id: string; text?: string; score: number }>,
): RetrievalVectorBackend {
  return {
    async search() {
      return hits.map((h) => ({
        doc: { id: h.id, text: h.text },
        score: h.score,
      }))
    },
  }
}

function mockObsidian(
  mode: 'rest' | 'filesystem',
  hits: SearchHit[],
): ObsidianClient {
  return {
    mode,
    async search() {
      return hits
    },
    async readFile() {
      return ''
    },
  }
}

function makeEpisode(id: string, summary: string, tags: string[] = []): Episode {
  return {
    episodicId: id,
    sessionId: 'sess-1',
    projectId: 'evaAI',
    timestamp: '2026-05-04T12:00:00.000Z',
    importance_score: 0.7,
    range: ['m1'],
    content: { summary },
    tags,
  }
}

describe('recall — end-to-end', () => {
  it('fuses hits across all 4 sources with provenance', async () => {
    const root = await setupRoot({
      episodes: [makeEpisode('ep-rate-limit', 'rate limiting decision')],
      edges: [
        { from: 'ADR--RATE-LIMIT', to: 'CONCEPT--FAIRNESS', type: 'references' },
      ],
    })
    const result = await recall({
      query: 'rate limiting',
      root,
      namespace: 'evaAI',
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([
        { id: 'ADR--RATE-LIMIT', text: 'rate limit', score: 0.9 },
      ]),
      obsidian: mockObsidian('rest', [
        {
          path: 'gks/adr/ADR--RATE-LIMIT.md',
          title: 'rate limit',
          snippet: 'rate limit',
          score: 1.0,
        },
      ]),
      timeoutMs: 1500,
    })
    expect(result.hits.length).toBeGreaterThan(0)
    // ADR--RATE-LIMIT appears in both vector and obsidian sources → top hit
    expect(result.hits[0]!.atomId).toBe('ADR--RATE-LIMIT')
    expect(result.hits[0]!.perSourceRanks['gks-vector']).toBe(1)
    expect(result.hits[0]!.perSourceRanks['obsidian-text']).toBe(1)
    // Backlinks should add CONCEPT--FAIRNESS via expansion from ADR--RATE-LIMIT
    const fairness = result.hits.find((h) => h.atomId === 'CONCEPT--FAIRNESS')
    expect(fairness).toBeDefined()
    expect(fairness!.perSourceRanks.backlinks).toBe(1)
    // Episodic should add ep-rate-limit
    const ep = result.hits.find((h) => h.atomId === 'ep-rate-limit')
    expect(ep).toBeDefined()
    expect(ep!.perSourceRanks.episodic).toBe(1)
  })

  it('reports fallback_reasons when one source fails', async () => {
    const root = await setupRoot({})
    // No embedder → vector source error: no-embedder.
    const result = await recall({
      query: 'foo',
      root,
      namespace: 'evaAI',
      obsidian: mockObsidian('rest', []),
      timeoutMs: 1500,
    })
    expect(result.fallback_reasons).toContain('gks-vector: no-embedder')
    expect(result.semantic_available).toBe(false)
  })

  it('returns empty hits + reasons when everything is empty', async () => {
    const root = await setupRoot({})
    const result = await recall({
      query: 'nothing-matches-this',
      root,
      namespace: 'evaAI',
      timeoutMs: 1500,
    })
    expect(result.hits).toEqual([])
    expect(result.fallback_reasons.length).toBeGreaterThan(0)
  })

  it('respects opts.weights for re-ordering', async () => {
    const root = await setupRoot({
      episodes: [makeEpisode('ep-1', 'unique-summary-token-zzz')],
    })
    // Vector has X, episodic has ep-1. Default weights: vector=1.0, episodic=1.2.
    // With heavy weight on vector, X wins; heavy weight on episodic, ep-1 wins.
    const heavyVec = await recall({
      query: 'unique-summary-token-zzz',
      root,
      namespace: 'evaAI',
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([{ id: 'X', score: 0.9 }]),
      weights: { 'gks-vector': 100, episodic: 0.001 },
      timeoutMs: 1500,
    })
    expect(heavyVec.hits[0]!.atomId).toBe('X')

    const heavyEpi = await recall({
      query: 'unique-summary-token-zzz',
      root,
      namespace: 'evaAI',
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([{ id: 'X', score: 0.9 }]),
      weights: { 'gks-vector': 0.001, episodic: 100 },
      timeoutMs: 1500,
    })
    expect(heavyEpi.hits[0]!.atomId).toBe('ep-1')
  })

  it('respects opts.rrfK', async () => {
    const root = await setupRoot({})
    const k60 = await recall({
      query: 'q',
      root,
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([{ id: 'A', score: 0.9 }]),
      rrfK: 60,
      timeoutMs: 1500,
    })
    const k10 = await recall({
      query: 'q',
      root,
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([{ id: 'A', score: 0.9 }]),
      rrfK: 10,
      timeoutMs: 1500,
    })
    // Smaller k → larger 1/(k+1). Hit at rank 1 should reflect.
    expect(k10.hits[0]!.score).toBeGreaterThan(k60.hits[0]!.score)
  })

  it('obsidian_available reflects rest mode', async () => {
    const root = await setupRoot({})
    const rest = await recall({
      query: 'q',
      root,
      obsidian: mockObsidian('rest', []),
      timeoutMs: 1500,
    })
    expect(rest.obsidian_available).toBe(true)

    const fs = await recall({
      query: 'q',
      root,
      obsidian: mockObsidian('filesystem', []),
      timeoutMs: 1500,
    })
    expect(fs.obsidian_available).toBe(false)
  })

  it('semantic_available reflects vector source health', async () => {
    const root = await setupRoot({})
    const ok = await recall({
      query: 'q',
      root,
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([{ id: 'A', score: 0.5 }]),
      timeoutMs: 1500,
    })
    expect(ok.semantic_available).toBe(true)

    const missing = await recall({ query: 'q', root, timeoutMs: 1500 })
    expect(missing.semantic_available).toBe(false)
  })

  it('populates timings for each source plus fusion', async () => {
    const root = await setupRoot({
      episodes: [makeEpisode('ep-1', 'foo')],
    })
    const res = await recall({
      query: 'foo',
      root,
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([{ id: 'A', score: 0.5 }]),
      obsidian: mockObsidian('rest', []),
      timeoutMs: 1500,
    })
    expect(res.timings.vector).toBeGreaterThanOrEqual(0)
    expect(res.timings.obsidian).toBeGreaterThanOrEqual(0)
    expect(res.timings.episodic).toBeGreaterThanOrEqual(0)
    expect(res.timings.backlinks).toBeGreaterThanOrEqual(0)
    expect(res.timings.fusion).toBeGreaterThanOrEqual(0)
  })

  it('is idempotent with mocked sources (same input → same hits)', async () => {
    const root = await setupRoot({
      episodes: [makeEpisode('ep-1', 'rate limiting')],
      edges: [{ from: 'ADR--A', to: 'CONCEPT--B', type: 'references' }],
    })
    const opts = {
      query: 'rate limiting',
      root,
      embedder: mockEmbedder(),
      vectorBackend: mockBackend([{ id: 'ADR--A', score: 0.7 }]),
      obsidian: mockObsidian('rest', []),
      timeoutMs: 1500,
    }
    const a = await recall(opts)
    const b = await recall(opts)
    expect(a.hits.map((h) => h.atomId)).toEqual(b.hits.map((h) => h.atomId))
    expect(a.hits.map((h) => h.score)).toEqual(b.hits.map((h) => h.score))
  })

  it('limits results to topK', async () => {
    const root = await setupRoot({
      episodes: [
        makeEpisode('ep-1', 'foo bar baz'),
        makeEpisode('ep-2', 'foo bar baz'),
        makeEpisode('ep-3', 'foo bar baz'),
        makeEpisode('ep-4', 'foo bar baz'),
        makeEpisode('ep-5', 'foo bar baz'),
      ],
    })
    const res = await recall({
      query: 'foo bar baz',
      root,
      topK: 3,
      timeoutMs: 1500,
    })
    expect(res.hits.length).toBeLessThanOrEqual(3)
  })
})
