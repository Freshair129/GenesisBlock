import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { episodicSource } from '../../../../src/orchestrator/retrieval/sources/episodic.js'
import type { Episode } from '../../../../src/memory/episodic/types.js'

async function setupFixture(
  episodes: Episode[],
  namespace = 'evaAI',
): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-retrieval-epi-'))
  const dir = join(root, '.brain/msp/projects', namespace, 'memory')
  await mkdir(dir, { recursive: true })
  await writeFile(
    join(dir, 'episodic_memory.json'),
    JSON.stringify(episodes),
    'utf8',
  )
  return root
}

function makeEpisode(
  episodicId: string,
  summary: string,
  tags: string[] = [],
  timestamp = '2026-05-04T12:00:00.000Z',
): Episode {
  return {
    episodicId,
    sessionId: 'sess-1',
    projectId: 'evaAI',
    timestamp,
    importance_score: 0.7,
    range: ['m1', 'm2'],
    content: { summary },
    tags,
  }
}

describe('episodicSource', () => {
  it('returns empty when episodic_memory.json is missing', async () => {
    const root = await mkdtemp(join(tmpdir(), 'msp-retrieval-epi-empty-'))
    const res = await episodicSource({
      root,
      namespace: 'evaAI',
      query: 'anything',
      topK: 5,
    })
    expect(res.source).toBe('episodic')
    expect(res.hits).toEqual([])
    expect(res.error).toBeUndefined()
  })

  it('matches via tag-only signal', async () => {
    const root = await setupFixture([
      makeEpisode('ep-1', 'unrelated body text', ['rate-limiting', 'msp']),
    ])
    const res = await episodicSource({
      root,
      namespace: 'evaAI',
      query: 'rate-limiting decision',
      topK: 5,
    })
    expect(res.hits).toHaveLength(1)
    expect(res.hits[0]!.atomId).toBe('ep-1')
  })

  it('matches via summary token overlap', async () => {
    const root = await setupFixture([
      makeEpisode('ep-1', 'we discussed rate limiting strategy'),
      makeEpisode('ep-2', 'unrelated topic about apple pie'),
    ])
    const res = await episodicSource({
      root,
      namespace: 'evaAI',
      query: 'rate limiting',
      topK: 5,
    })
    expect(res.hits[0]!.atomId).toBe('ep-1')
    expect(res.hits.find((h) => h.atomId === 'ep-2')).toBeUndefined()
  })

  it('tag bonus stacks with summary overlap', async () => {
    const root = await setupFixture([
      // ep-A: only summary overlap (modest score)
      makeEpisode('ep-A', 'rate limiting'),
      // ep-B: same summary overlap PLUS matching tag → higher score
      makeEpisode('ep-B', 'rate limiting', ['rate']),
    ])
    const res = await episodicSource({
      root,
      namespace: 'evaAI',
      query: 'rate limiting',
      topK: 5,
    })
    expect(res.hits[0]!.atomId).toBe('ep-B')
    expect(res.hits[1]!.atomId).toBe('ep-A')
  })

  it('sorts results descending by score', async () => {
    const root = await setupFixture([
      makeEpisode('ep-low', 'one matching word: alpha'),
      makeEpisode('ep-high', 'alpha beta gamma delta', ['alpha']),
    ])
    const res = await episodicSource({
      root,
      namespace: 'evaAI',
      query: 'alpha beta gamma delta',
      topK: 5,
    })
    expect(res.hits[0]!.atomId).toBe('ep-high')
    expect(res.hits[1]!.atomId).toBe('ep-low')
  })

  it('slices to topK', async () => {
    const root = await setupFixture([
      makeEpisode('ep-1', 'alpha'),
      makeEpisode('ep-2', 'alpha'),
      makeEpisode('ep-3', 'alpha'),
      makeEpisode('ep-4', 'alpha'),
    ])
    const res = await episodicSource({
      root,
      namespace: 'evaAI',
      query: 'alpha',
      topK: 2,
    })
    expect(res.hits).toHaveLength(2)
    expect(res.hits.map((h) => h.rank)).toEqual([1, 2])
  })
})
