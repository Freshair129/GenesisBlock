import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { backlinksSource } from '../../../../src/orchestrator/retrieval/sources/backlinks.js'

interface Edge {
  from: string
  to: string
  type: string
}

async function setupFixture(
  edges: Edge[],
  namespace = 'evaAI',
): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-retrieval-bl-'))
  const dir = join(root, '.brain/msp/projects', namespace, 'vector')
  await mkdir(dir, { recursive: true })
  const lines = edges.map((e) => JSON.stringify(e)).join('\n') + (edges.length > 0 ? '\n' : '')
  await writeFile(join(dir, 'backlinks.jsonl'), lines, 'utf8')
  return root
}

describe('backlinksSource', () => {
  it('returns empty when backlinks.jsonl is missing', async () => {
    const root = await mkdtemp(join(tmpdir(), 'msp-retrieval-bl-empty-'))
    const res = await backlinksSource({
      root,
      namespace: 'evaAI',
      candidateAtomIds: ['ADR--FOO'],
      topK: 5,
    })
    expect(res.hits).toEqual([])
    expect(res.error).toBeUndefined()
  })

  it('expands 1-hop outbound (candidate.from → neighbour)', async () => {
    const root = await setupFixture([
      { from: 'ADR--A', to: 'CONCEPT--X', type: 'references' },
    ])
    const res = await backlinksSource({
      root,
      namespace: 'evaAI',
      candidateAtomIds: ['ADR--A'],
      topK: 5,
    })
    expect(res.hits).toHaveLength(1)
    expect(res.hits[0]!.atomId).toBe('CONCEPT--X')
  })

  it('expands 1-hop inbound (neighbour → candidate.to)', async () => {
    const root = await setupFixture([
      { from: 'FEAT--Z', to: 'ADR--A', type: 'implements' },
    ])
    const res = await backlinksSource({
      root,
      namespace: 'evaAI',
      candidateAtomIds: ['ADR--A'],
      topK: 5,
    })
    expect(res.hits).toHaveLength(1)
    expect(res.hits[0]!.atomId).toBe('FEAT--Z')
  })

  it('dedupes neighbours that appear multiple times for the same candidate', async () => {
    const root = await setupFixture([
      { from: 'ADR--A', to: 'CONCEPT--X', type: 'references' },
      { from: 'ADR--A', to: 'CONCEPT--X', type: 'implements' }, // duplicate edge type
    ])
    const res = await backlinksSource({
      root,
      namespace: 'evaAI',
      candidateAtomIds: ['ADR--A'],
      topK: 5,
    })
    // The neighbour appears once, not twice; voter set has size 1.
    const x = res.hits.find((h) => h.atomId === 'CONCEPT--X')
    expect(x).toBeDefined()
    expect(res.hits.filter((h) => h.atomId === 'CONCEPT--X')).toHaveLength(1)
  })

  it('scores by candidate-vote count (more candidates pointing → higher rank)', async () => {
    const root = await setupFixture([
      { from: 'ADR--A', to: 'CONCEPT--X', type: 'references' },
      { from: 'ADR--B', to: 'CONCEPT--X', type: 'references' }, // 2 candidates → X
      { from: 'ADR--A', to: 'CONCEPT--Y', type: 'references' }, // 1 candidate → Y
    ])
    const res = await backlinksSource({
      root,
      namespace: 'evaAI',
      candidateAtomIds: ['ADR--A', 'ADR--B'],
      topK: 5,
    })
    expect(res.hits[0]!.atomId).toBe('CONCEPT--X')
    expect(res.hits[1]!.atomId).toBe('CONCEPT--Y')
  })

  it('slices to topK', async () => {
    const root = await setupFixture([
      { from: 'ADR--A', to: 'CONCEPT--X', type: 'references' },
      { from: 'ADR--A', to: 'CONCEPT--Y', type: 'references' },
      { from: 'ADR--A', to: 'CONCEPT--Z', type: 'references' },
    ])
    const res = await backlinksSource({
      root,
      namespace: 'evaAI',
      candidateAtomIds: ['ADR--A'],
      topK: 2,
    })
    expect(res.hits).toHaveLength(2)
    expect(res.hits.map((h) => h.rank)).toEqual([1, 2])
  })
})
