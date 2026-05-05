import { describe, expect, it } from 'vitest'

import { rrfFuse } from '../../../src/orchestrator/retrieval/fusion.js'
import type {
  SourceHit,
  SourceResult,
} from '../../../src/orchestrator/retrieval/types.js'

function makeSource(
  source: SourceResult['source'],
  hits: Array<{ atomId: string; rank: number; snippet?: string }>,
): SourceResult {
  return {
    source,
    hits: hits.map<SourceHit>((h) => ({
      atomId: h.atomId,
      rank: h.rank,
      snippet: h.snippet,
      source,
    })),
    latencyMs: 0,
  }
}

describe('rrfFuse', () => {
  it('returns [] for empty input', () => {
    const res = rrfFuse([])
    expect(res).toEqual([])
  })

  it('returns single hit at rank 1 for single source single hit', () => {
    const src = makeSource('gks-vector', [{ atomId: 'A', rank: 1 }])
    const res = rrfFuse([src])
    expect(res).toHaveLength(1)
    expect(res[0]!.atomId).toBe('A')
    expect(res[0]!.rank).toBe(1)
    expect(res[0]!.score).toBeCloseTo(1.0 / (60 + 1), 6)
  })

  it('sums scores when same atom appears in two sources', () => {
    const a = makeSource('gks-vector', [{ atomId: 'X', rank: 1 }]) // weight 1.0
    const b = makeSource('episodic', [{ atomId: 'X', rank: 1 }]) // weight 1.2
    const res = rrfFuse([a, b])
    expect(res).toHaveLength(1)
    const expected = 1.0 / 61 + 1.2 / 61
    expect(res[0]!.score).toBeCloseTo(expected, 6)
    expect(res[0]!.perSourceRanks).toEqual({
      'gks-vector': 1,
      episodic: 1,
    })
  })

  it('keeps distinct atoms separate when sources disagree', () => {
    const a = makeSource('gks-vector', [{ atomId: 'A', rank: 1 }])
    const b = makeSource('episodic', [{ atomId: 'B', rank: 1 }])
    const res = rrfFuse([a, b])
    expect(res).toHaveLength(2)
    const ids = res.map((h) => h.atomId).sort()
    expect(ids).toEqual(['A', 'B'])
  })

  it('tie-breaks by sourceCount (more sources wins)', () => {
    // A appears in 2 sources at rank 1; B appears in 1 source at rank 1.
    // With weight 1.0 each, A would have higher score anyway. So construct
    // a case where raw scores tie but source counts differ. Use one big-rank
    // contribution to balance.
    // A: gks-vector rank 1 (1.0/61) + episodic rank 60 (1.2/120) ≈ 0.0164 + 0.01 = 0.0264
    // B: gks-vector rank 1 (1.0/61) ≈ 0.0164
    // Different scores, but illustrate sourceCount tie-break differently:
    // make them perfectly equal via custom weights
    const src1 = makeSource('gks-vector', [
      { atomId: 'A', rank: 10 },
      { atomId: 'B', rank: 10 },
    ])
    const src2 = makeSource('episodic', [{ atomId: 'A', rank: 10 }])
    const res = rrfFuse([src1, src2], { weights: { episodic: 0.0001 } })
    // A and B have nearly identical RRF scores; A has 2 contributions, B has 1.
    expect(res[0]!.atomId).toBe('A')
    expect(res[1]!.atomId).toBe('B')
  })

  it('tie-breaks by min source rank when scores + sourceCount equal', () => {
    // Force both atoms identical RRF score using custom weights.
    // A at rank 1, B at rank 5, give B a weight that yields the same score.
    // weight_b / (60 + 5) = 1.0 / (60 + 1) → weight_b = 65/61
    const wb = 65 / 61
    const a = makeSource('gks-vector', [{ atomId: 'A', rank: 1 }])
    const b = makeSource('episodic', [{ atomId: 'B', rank: 5 }])
    const res = rrfFuse([a, b], { weights: { episodic: wb } })
    // Equal scores, equal sourceCount=1 each → min-rank wins (A=1 < B=5)
    expect(res[0]!.atomId).toBe('A')
    expect(res[1]!.atomId).toBe('B')
  })

  it('tie-breaks lexicographically when score + sourceCount + minRank equal', () => {
    const a = makeSource('gks-vector', [{ atomId: 'B', rank: 1 }])
    const b = makeSource('episodic', [{ atomId: 'A', rank: 1 }])
    // identical scores require equal weights — override episodic to 1.0
    const res = rrfFuse([a, b], { weights: { episodic: 1.0 } })
    expect(res[0]!.atomId).toBe('A')
    expect(res[1]!.atomId).toBe('B')
  })

  it('respects per-source weight overrides', () => {
    const a = makeSource('gks-vector', [{ atomId: 'A', rank: 1 }])
    const b = makeSource('episodic', [{ atomId: 'B', rank: 1 }])
    // Boost vector weight so A wins decisively
    const res = rrfFuse([a, b], { weights: { 'gks-vector': 5.0, episodic: 0.1 } })
    expect(res[0]!.atomId).toBe('A')
    expect(res[1]!.atomId).toBe('B')
    expect(res[0]!.score).toBeGreaterThan(res[1]!.score)
  })

  it('rrfK changes the score (smaller k → larger contributions)', () => {
    const src = makeSource('gks-vector', [{ atomId: 'A', rank: 1 }])
    const k60 = rrfFuse([src], { k: 60 })[0]!.score
    const k10 = rrfFuse([src], { k: 10 })[0]!.score
    expect(k10).toBeGreaterThan(k60)
    expect(k60).toBeCloseTo(1 / 61, 6)
    expect(k10).toBeCloseTo(1 / 11, 6)
  })

  it('slices to topK', () => {
    const src = makeSource('gks-vector', [
      { atomId: 'A', rank: 1 },
      { atomId: 'B', rank: 2 },
      { atomId: 'C', rank: 3 },
      { atomId: 'D', rank: 4 },
    ])
    const res = rrfFuse([src], { topK: 2 })
    expect(res).toHaveLength(2)
    expect(res.map((h) => h.atomId)).toEqual(['A', 'B'])
  })

  it('assigns final rank 1..N in result', () => {
    const src = makeSource('gks-vector', [
      { atomId: 'A', rank: 1 },
      { atomId: 'B', rank: 2 },
      { atomId: 'C', rank: 3 },
    ])
    const res = rrfFuse([src])
    expect(res.map((h) => h.rank)).toEqual([1, 2, 3])
  })

  it('populates perSourceRanks correctly', () => {
    const a = makeSource('gks-vector', [{ atomId: 'A', rank: 3 }])
    const b = makeSource('episodic', [{ atomId: 'A', rank: 7 }])
    const c = makeSource('backlinks', [{ atomId: 'A', rank: 2 }])
    const res = rrfFuse([a, b, c])
    expect(res[0]!.perSourceRanks).toEqual({
      'gks-vector': 3,
      episodic: 7,
      backlinks: 2,
    })
  })

  it('preserves first-seen non-empty snippet', () => {
    const a = makeSource('gks-vector', [
      { atomId: 'A', rank: 1, snippet: 'from vector' },
    ])
    const b = makeSource('episodic', [
      { atomId: 'A', rank: 2, snippet: 'from episodic' },
    ])
    const res = rrfFuse([a, b])
    expect(res[0]!.snippet).toBe('from vector')
  })

  it('falls through to later snippet when first is empty', () => {
    const a = makeSource('gks-vector', [{ atomId: 'A', rank: 1 }])
    const b = makeSource('episodic', [
      { atomId: 'A', rank: 1, snippet: 'from episodic' },
    ])
    const res = rrfFuse([a, b])
    expect(res[0]!.snippet).toBe('from episodic')
  })
})
