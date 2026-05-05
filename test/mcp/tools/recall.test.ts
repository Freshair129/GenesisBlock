import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { handler, name } from '../../../src/mcp/tools/recall.js'

async function setupRoot(opts: { episodes?: unknown[] } = {}): Promise<string> {
  const namespace = 'evaAI'
  const root = await mkdtemp(join(tmpdir(), 'msp-recall-tool-'))
  const memoryDir = join(root, '.brain/msp/projects', namespace, 'memory')
  await mkdir(memoryDir, { recursive: true })
  if (opts.episodes) {
    await writeFile(
      join(memoryDir, 'episodic_memory.json'),
      JSON.stringify(opts.episodes),
      'utf8',
    )
  }
  return root
}

describe('msp_recall tool', () => {
  it('has the right name', () => {
    expect(name).toBe('msp_recall')
  })

  it('returns ok=true on an empty repo with no obsidian', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({ query: 'anything', root })
    expect(result.isError).toBeFalsy()
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(Array.isArray(parsed.hits)).toBe(true)
    expect(parsed.semantic_available).toBe(false)
    expect(parsed.timings).toBeDefined()
    expect(parsed.fallback_reasons.length).toBeGreaterThan(0)
  }, 10_000)

  it('finds episodic hits when episodes match the query', async () => {
    const root = await setupRoot({
      episodes: [
        {
          episodicId: 'ep-rate',
          sessionId: 'sess-1',
          projectId: 'evaAI',
          timestamp: '2026-05-04T12:00:00.000Z',
          importance_score: 0.7,
          range: ['m1'],
          content: { summary: 'rate limiting decision' },
          tags: ['rate-limit'],
        },
      ],
    })
    const result = await handler({ root })({
      query: 'rate limiting',
      root,
      top_k: 5,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    const ep = parsed.hits.find((h: { atomId: string }) => h.atomId === 'ep-rate')
    expect(ep).toBeDefined()
  }, 10_000)

  it('honours top_k cap on the fused hits list', async () => {
    const episodes = []
    for (let i = 0; i < 6; i++) {
      episodes.push({
        episodicId: `ep-${i}`,
        sessionId: 'sess-1',
        projectId: 'evaAI',
        timestamp: '2026-05-04T12:00:00.000Z',
        importance_score: 0.5,
        range: ['m1'],
        content: { summary: 'rate limiting note ' + i },
        tags: ['rate-limit'],
      })
    }
    const root = await setupRoot({ episodes })
    const result = await handler({ root })({
      query: 'rate limiting',
      root,
      top_k: 3,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.hits.length).toBeLessThanOrEqual(3)
  }, 10_000)

  it('passes weights through to RRF fusion (smoke test — does not crash)', async () => {
    const root = await setupRoot({
      episodes: [
        {
          episodicId: 'ep-a',
          sessionId: 'sess-1',
          projectId: 'evaAI',
          timestamp: '2026-05-04T12:00:00.000Z',
          importance_score: 0.7,
          range: ['m1'],
          content: { summary: 'matching content here' },
          tags: ['x'],
        },
      ],
    })
    const result = await handler({ root })({
      query: 'matching',
      root,
      weights: { episodic: 5.0, 'gks-vector': 0 },
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
  }, 10_000)

  it('respects timeout_ms (does not exceed total budget significantly)', async () => {
    const root = await setupRoot()
    const start = Date.now()
    const result = await handler({ root })({
      query: 'fast',
      root,
      timeout_ms: 200,
    })
    const elapsed = Date.now() - start
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    // budget + slack — never block the test for >2s on a 200ms budget
    expect(elapsed).toBeLessThan(2000)
  }, 10_000)

  it('uses namespace argument when supplied', async () => {
    const root = await mkdtemp(join(tmpdir(), 'msp-recall-tool-ns-'))
    const namespace = 'custom-ns'
    const memoryDir = join(root, '.brain/msp/projects', namespace, 'memory')
    await mkdir(memoryDir, { recursive: true })
    await writeFile(
      join(memoryDir, 'episodic_memory.json'),
      JSON.stringify([
        {
          episodicId: 'ep-custom',
          sessionId: 'sess-1',
          projectId: namespace,
          timestamp: '2026-05-04T12:00:00.000Z',
          importance_score: 0.7,
          range: ['m1'],
          content: { summary: 'custom namespace hit' },
          tags: ['custom'],
        },
      ]),
      'utf8',
    )
    const result = await handler({ root })({
      query: 'custom',
      namespace,
      root,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    const hit = parsed.hits.find((h: { atomId: string }) => h.atomId === 'ep-custom')
    expect(hit).toBeDefined()
  }, 10_000)
})
