import { describe, expect, it } from 'vitest'

import { validateEpisode } from '../../../src/memory/episodic/schema.js'
import { EpisodicSchemaError } from '../../../src/memory/episodic/types.js'

const VALID = {
  episodicId: 'ep_001',
  sessionId: 'sess_001',
  projectId: 'evaAI',
  importance_score: 0.8,
  range: ['turnIdx-1..turnIdx-5'],
  content: { summary: 'a brief summary' },
}

describe('validateEpisode', () => {
  it('accepts a complete minimal episode', () => {
    expect(validateEpisode(VALID)).toBe(VALID)
  })

  it('rejects when importance_score is missing', () => {
    const { importance_score: _, ...e } = VALID
    expect(() => validateEpisode(e)).toThrow(EpisodicSchemaError)
  })

  it('rejects NaN importance_score', () => {
    expect(() => validateEpisode({ ...VALID, importance_score: NaN })).toThrow(EpisodicSchemaError)
  })

  it('rejects out-of-range importance_score', () => {
    expect(() => validateEpisode({ ...VALID, importance_score: 1.1 })).toThrow(EpisodicSchemaError)
    expect(() => validateEpisode({ ...VALID, importance_score: -0.1 })).toThrow(EpisodicSchemaError)
  })

  it('rejects empty range', () => {
    expect(() => validateEpisode({ ...VALID, range: [] })).toThrow(EpisodicSchemaError)
  })

  it('rejects content without summary', () => {
    expect(() => validateEpisode({ ...VALID, content: {} })).toThrow(EpisodicSchemaError)
  })

  it('rejects non-objects', () => {
    expect(() => validateEpisode(null)).toThrow(EpisodicSchemaError)
    expect(() => validateEpisode([VALID])).toThrow(EpisodicSchemaError)
  })
})
