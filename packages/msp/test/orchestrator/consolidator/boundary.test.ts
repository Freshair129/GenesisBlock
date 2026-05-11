import { describe, expect, it } from 'vitest'

import {
  bagCosine,
  detectBoundaries,
  tokenise,
} from '../../../src/orchestrator/consolidator/boundary.js'
import type { Turn } from '../../../src/orchestrator/consolidator/types.js'

function turn(content: string, turnId: number, speakerId = 'user'): Turn {
  return {
    sessionId: 's1',
    episodicId: 'e1',
    turnId,
    msgId: `m${turnId}`,
    speakerId,
    content,
  }
}

describe('tokenise + bagCosine', () => {
  it('strips stopwords and short tokens', () => {
    const toks = tokenise('the quick brown fox is over there')
    expect(toks).toContain('quick')
    expect(toks).toContain('brown')
    expect(toks).toContain('fox')
    expect(toks).not.toContain('the')
    expect(toks).not.toContain('is')
  })

  it('bagCosine: identical bags = 1', () => {
    const a = ['pgvector', 'index', 'memory']
    expect(bagCosine(a, a)).toBeCloseTo(1)
  })

  it('bagCosine: disjoint bags = 0', () => {
    expect(bagCosine(['cat', 'dog'], ['piano', 'flute'])).toBe(0)
  })
})

describe('detectBoundaries', () => {
  it('returns [] for empty input, [[0,0]] for single turn', () => {
    expect(detectBoundaries([])).toEqual([])
    expect(detectBoundaries([turn('hello world test', 0)])).toEqual([[0, 0]])
  })

  it('keeps a single-topic conversation in one chunk', () => {
    const turns = [
      turn('we will use pgvector for the vector index here', 0),
      turn('pgvector supports the cosine distance and ivfflat indexing', 1),
      turn('the pgvector extension stores embeddings well in postgres', 2),
      turn('we ran benchmarks and pgvector beat qdrant on our workload', 3),
      turn('pgvector index build was fast on the sample dataset', 4),
    ]
    const ranges = detectBoundaries(turns, { window: 2 })
    expect(ranges).toHaveLength(1)
    expect(ranges[0]).toEqual([0, 4])
  })

  it('splits at a topic shift', () => {
    const turns = [
      turn('we will use pgvector for the vector index here', 0),
      turn('pgvector supports cosine distance and ivfflat indexing', 1),
      turn('pgvector beats qdrant in our benchmarks for our workload', 2),
      turn('switching topic — about lunch tomorrow at the cafeteria', 3),
      turn('I prefer pizza for lunch over a salad option', 4),
      turn('pizza is great with cheese and pepperoni and basil leaves', 5),
    ]
    const ranges = detectBoundaries(turns, { window: 2, thresholds: { boundary: 0.5 } })
    expect(ranges.length).toBeGreaterThanOrEqual(2)
    // Chunks should partition without gaps/overlaps and cover full session.
    expect(ranges[0]![0]).toBe(0)
    expect(ranges[ranges.length - 1]![1]).toBe(5)
    for (let i = 1; i < ranges.length; i++) {
      expect(ranges[i]![0]).toBe(ranges[i - 1]![1] + 1)
    }
  })

  it('partitions cover the full turn array contiguously', () => {
    const turns = [
      turn('one alpha beta gamma', 0),
      turn('two alpha delta', 1),
      turn('three alpha epsilon', 2),
      turn('four totally different banana mango pineapple lychee', 3),
      turn('five mango banana fruit basket order', 4),
      turn('six fruit salad recipe needs banana and pineapple', 5),
    ]
    const ranges = detectBoundaries(turns, { window: 2, thresholds: { boundary: 0.4 } })
    // Verify partition coverage
    let last = -1
    for (const [s, e] of ranges) {
      expect(s).toBe(last + 1)
      expect(e).toBeGreaterThanOrEqual(s)
      last = e
    }
    expect(last).toBe(turns.length - 1)
  })
})
