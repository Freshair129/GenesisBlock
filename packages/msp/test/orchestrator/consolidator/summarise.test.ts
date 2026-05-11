import { describe, expect, it } from 'vitest'

import {
  deterministicSummary,
  extractDeterministicTags,
} from '../../../src/orchestrator/consolidator/summarise.js'
import type { Turn } from '../../../src/orchestrator/consolidator/types.js'

function turn(content: string, speakerId = 'user', turnId = 0): Turn {
  return {
    sessionId: 's1',
    episodicId: 'e1',
    turnId,
    msgId: `m${turnId}`,
    speakerId,
    content,
  }
}

describe('deterministicSummary', () => {
  it('prefers the first non-user (assistant) sentence', () => {
    const chunk = [
      turn('quick question', 'user', 0),
      turn(
        'We will use pgvector here. Decided after benchmark.',
        'assistant',
        1,
      ),
    ]
    expect(deterministicSummary(chunk)).toMatch(/pgvector/)
  })

  it('falls back to first user sentence when there is no assistant turn', () => {
    const chunk = [turn('I want to ship pgvector by Friday.', 'user', 0)]
    expect(deterministicSummary(chunk)).toMatch(/pgvector/)
  })

  it('returns "(no summary available)" for empty chunk', () => {
    expect(deterministicSummary([])).toBe('(no summary available)')
  })

  it('truncates at word boundary near the cap', () => {
    const long = 'a '.repeat(300) + 'pgvector'
    const out = deterministicSummary([turn(long, 'assistant', 0)])
    expect(out.length).toBeLessThanOrEqual(241)
  })
})

describe('extractDeterministicTags', () => {
  it('pulls wikilinks, atom IDs, and hashtags; caps at 5', () => {
    const chunk = [
      turn('see [[pgvector-bench]] and ADR--FOO', 'assistant', 0),
      turn('also FEAT--BAR plus #memory and #consolidator and #m7b', 'assistant', 1),
    ]
    const tags = extractDeterministicTags(chunk)
    expect(tags).toContain('pgvector-bench')
    expect(tags).toContain('ADR--FOO')
    expect(tags.length).toBeLessThanOrEqual(5)
  })

  it('returns [] for chunks with no taggable artifacts', () => {
    expect(extractDeterministicTags([turn('plain prose', 'user', 0)])).toEqual([])
  })

  it('de-duplicates repeated tags', () => {
    const chunk = [
      turn('ADR--FOO and ADR--FOO again with [[same]] and [[same]]', 'user', 0),
    ]
    const tags = extractDeterministicTags(chunk)
    expect(new Set(tags).size).toBe(tags.length)
  })
})
