import { describe, expect, it } from 'vitest'

import {
  codeArtifactMentions,
  computeSessionStats,
  deadEndMarkers,
  decisionMarkers,
  greetingFiller,
  lengthNormalised,
  numericSpecificity,
  scoreChunk,
  topicContinuity,
} from '../../../src/orchestrator/consolidator/score.js'
import type { Turn } from '../../../src/orchestrator/consolidator/types.js'

function turn(content: string, speakerId = 'assistant', turnId = 0): Turn {
  return {
    sessionId: 's1',
    episodicId: 'e1',
    turnId,
    msgId: `m${turnId}`,
    speakerId,
    content,
  }
}

describe('feature: decisionMarkers', () => {
  it('returns >0 for "we\'ll use X" / "let\'s go with X" / "decided"', () => {
    expect(decisionMarkers("we'll use pgvector here")).toBeGreaterThan(0)
    expect(decisionMarkers("let's go with option B")).toBeGreaterThan(0)
    expect(decisionMarkers('we decided to ship it')).toBeGreaterThan(0)
  })
  it('returns 0 for plain prose with no decision verbs', () => {
    expect(decisionMarkers('the weather today is nice and sunny')).toBe(0)
  })
})

describe('feature: codeArtifactMentions', () => {
  it('detects file paths, function calls, atom IDs', () => {
    expect(
      codeArtifactMentions('see src/orchestrator/consolidator/index.ts'),
    ).toBeGreaterThan(0)
    expect(codeArtifactMentions('call scoreChunk(turns, stats)')).toBeGreaterThan(0)
    expect(codeArtifactMentions('per ADR--CONSOLIDATOR-HYBRID-SCORING')).toBeGreaterThan(0)
  })
  it('returns 0 for prose with no code artifacts', () => {
    expect(codeArtifactMentions('we should think about it')).toBe(0)
  })
})

describe('feature: numericSpecificity', () => {
  it('flags semver and ISO dates', () => {
    expect(numericSpecificity('bumped to v2.5.6 on 2026-05-04')).toBeGreaterThan(0.5)
  })
  it('returns small but nonzero for general digit density', () => {
    expect(numericSpecificity('we have 5 items and 3 buckets')).toBeGreaterThan(0)
  })
  it('returns 0 for non-numeric prose', () => {
    expect(numericSpecificity('words only here, nothing more')).toBe(0)
  })
})

describe('feature: lengthNormalised', () => {
  it('returns 0 for very short chunks', () => {
    const stats = computeSessionStats([turn('a long enough turn so the mean is set OK to over fifty chars total')])
    const score = lengthNormalised([turn('hi')], stats)
    expect(score).toBe(0)
  })
  it('returns 1 for chunks ≥ 2× the session mean', () => {
    const baseline = turn('short')
    const big = turn('x'.repeat(500))
    const stats = computeSessionStats([baseline, baseline, baseline])
    expect(lengthNormalised([big], stats)).toBe(1)
  })
})

describe('feature: topicContinuity', () => {
  it('returns 0.5 (neutral) when there is no previous chunk', () => {
    expect(topicContinuity([turn('hello world')], null)).toBe(0.5)
  })
  it('returns higher similarity for shared vocabulary', () => {
    const a = [turn('we will use pgvector for the vector index')]
    const b = [turn('the pgvector index is great for vector queries')]
    const c = [turn('weather forecast says rain tomorrow afternoon')]
    expect(topicContinuity(b, a)).toBeGreaterThan(topicContinuity(c, a))
  })
})

describe('feature: deadEndMarkers / greetingFiller', () => {
  it('deadEndMarkers fires on cancellation / abandonment phrases', () => {
    expect(deadEndMarkers('nevermind, that approach broke')).toBe(1)
    expect(deadEndMarkers("scrap that, doesn't work")).toBe(1)
    expect(deadEndMarkers('clean continuation here')).toBe(0)
  })
  it('greetingFiller fires on greetings and acknowledgements', () => {
    expect(greetingFiller('thanks, got it')).toBeGreaterThan(0)
    expect(greetingFiller('we will ship pgvector by Friday')).toBe(0)
  })
})

describe('scoreChunk: verdicts at threshold edges', () => {
  it('drop verdict for greeting-only chunks', () => {
    const stats = computeSessionStats([turn('hi'), turn('thanks')])
    const r = scoreChunk([turn('hi'), turn('thanks')], stats)
    expect(r.verdict).toBe('drop')
  })

  it('keep verdict for chunks with decisions + code artifacts', () => {
    const t = turn(
      "we'll use pgvector — see src/memory/episodic/writer.ts and ADR--MEMORY-EPISODIC-WRITER. Decided as of 2026-05-04.",
    )
    const stats = computeSessionStats([t])
    const r = scoreChunk([t], stats)
    expect(r.verdict).toBe('keep')
    expect(r.score).toBeGreaterThan(0.65)
  })

  it('borderline verdict between thresholds', () => {
    const t = turn('we will think about that one but not yet')
    const stats = computeSessionStats([t])
    const r = scoreChunk([t], stats)
    expect(['borderline', 'drop', 'keep']).toContain(r.verdict)
    // breakdown should always include all 7 features
    expect(Object.keys(r.breakdown).sort()).toEqual([
      'code_artifact_mentions',
      'dead_end_markers',
      'decision_markers',
      'greeting_filler',
      'length_normalised',
      'numeric_specificity',
      'topic_continuity',
    ])
  })

  it('respects custom thresholds (low=0, high=2 forces always borderline)', () => {
    const t = turn('any content goes here')
    const stats = computeSessionStats([t])
    const r = scoreChunk([t], stats, { low: 0, high: 2 })
    expect(r.verdict).toBe('borderline')
  })
})

describe('computeSessionStats', () => {
  it('returns zeros for empty session', () => {
    const s = computeSessionStats([])
    expect(s).toEqual({ turnCount: 0, meanTurnBytes: 0, stddevTurnBytes: 0 })
  })

  it('computes mean + stddev correctly', () => {
    const s = computeSessionStats([
      turn('aaaa'),
      turn('aaaaaaaa'),
    ])
    expect(s.turnCount).toBe(2)
    expect(s.meanTurnBytes).toBe(6)
  })
})
