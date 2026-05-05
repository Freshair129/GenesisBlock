import { describe, expect, it } from 'vitest'

import {
  DEFAULT_TOKENISER,
  estimateText,
} from '../../../src/orchestrator/compressor/tokens.js'

describe('DEFAULT_TOKENISER (char/3.5)', () => {
  it('returns ceil(length / 3.5) for plain ASCII', () => {
    // 7 chars / 3.5 = 2 exactly
    expect(DEFAULT_TOKENISER('1234567')).toBe(2)
    // 10 chars / 3.5 = 2.857… → ceil = 3
    expect(DEFAULT_TOKENISER('abcdefghij')).toBe(3)
  })

  it('returns 0 for empty / falsy strings', () => {
    expect(DEFAULT_TOKENISER('')).toBe(0)
  })

  it('handles mixed-language text conservatively', () => {
    // 5-char Thai word + space + 5 ASCII = 11 chars → ceil(11/3.5) = 4.
    // Result must be a positive integer; the heuristic shouldn't blow up.
    const text = 'สวัสดี hello'
    const n = DEFAULT_TOKENISER(text)
    expect(n).toBeGreaterThan(0)
    expect(Number.isInteger(n)).toBe(true)
    // Conservative: never under-estimates char count / 4 (English baseline).
    expect(n).toBeGreaterThanOrEqual(Math.ceil(text.length / 4))
  })

  it('always returns a non-negative integer', () => {
    for (const s of ['', 'a', 'ab', 'abc', '   ', 'mixed CONTENT 123']) {
      const n = DEFAULT_TOKENISER(s)
      expect(Number.isInteger(n)).toBe(true)
      expect(n).toBeGreaterThanOrEqual(0)
    }
  })
})

describe('estimateText', () => {
  it('falls back to DEFAULT_TOKENISER when none is supplied', () => {
    expect(estimateText('1234567')).toBe(DEFAULT_TOKENISER('1234567'))
  })

  it('uses an injected tokeniser verbatim (e.g. tiktoken stand-in)', () => {
    // Hypothetical tiktoken-ish: every word ≈ 1 token.
    const wordCount = (s: string) =>
      s.trim().length === 0 ? 0 : s.trim().split(/\s+/).length
    expect(estimateText('one two three', wordCount)).toBe(3)
    expect(estimateText('', wordCount)).toBe(0)
  })
})
