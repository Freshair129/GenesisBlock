import { describe, expect, it } from 'vitest'

import {
  buildResummarisePrompt,
  cleanLlmText,
  resummariseLLM,
  truncate,
} from '../../../src/orchestrator/compressor/resummarise.js'
import type { LlmClient, Turn } from '../../../src/orchestrator/consolidator/types.js'
import type { CompressorEpisode } from '../../../src/orchestrator/compressor/types.js'

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

function makeEpisode(turns: Turn[], score = 0.7): CompressorEpisode {
  return {
    sessionId: 's1',
    turnRange: [0, Math.max(0, turns.length - 1)],
    summary: 's',
    score,
    turns,
  }
}

function fixedLlm(response: string): LlmClient {
  return async () => response
}

describe('buildResummarisePrompt', () => {
  it('includes target token count and the turns body', () => {
    const ep = makeEpisode([
      turn('we will ship pgvector', 'a', 0),
      turn('great', 'u', 1),
    ])
    const prompt = buildResummarisePrompt(ep, 80)
    expect(prompt).toContain('approximately 80 tokens')
    expect(prompt).toContain('[a] we will ship pgvector')
    expect(prompt).toContain('[u] great')
    expect(prompt).toContain('Return ONLY the summary text')
  })
})

describe('cleanLlmText', () => {
  it('strips markdown fences and Summary: prefixes', () => {
    expect(cleanLlmText('```\nhello world\n```')).toBe('hello world')
    expect(cleanLlmText('```text\nfoo bar\n```')).toBe('foo bar')
    expect(cleanLlmText('Summary: chose pgvector')).toBe('chose pgvector')
    expect(cleanLlmText('   trimmed   ')).toBe('trimmed')
  })
})

describe('resummariseLLM', () => {
  it('returns null when no llm is supplied (caller falls through to truncate)', async () => {
    const ep = makeEpisode([turn('x')])
    const r = await resummariseLLM(ep, 100)
    expect(r).toBeNull()
  })

  it('returns the cleaned LLM text on success when it fits target', async () => {
    const ep = makeEpisode([turn('we chose pgvector for retrieval')])
    const llm = fixedLlm('chose pgvector')
    const r = await resummariseLLM(ep, 100, undefined, { llm })
    expect(r).not.toBeNull()
    expect(r!.text).toBe('chose pgvector')
    expect(r!.tokens).toBeGreaterThan(0)
    expect(r!.tokens).toBeLessThanOrEqual(100)
  })

  it('returns null on LLM throw', async () => {
    const ep = makeEpisode([turn('x')])
    const llm: LlmClient = async () => {
      throw new Error('boom')
    }
    const r = await resummariseLLM(ep, 100, undefined, { llm })
    expect(r).toBeNull()
  })

  it('returns null on LLM timeout', async () => {
    const ep = makeEpisode([turn('x')])
    const llm: LlmClient = () => new Promise(() => {}) // never resolves
    const r = await resummariseLLM(ep, 100, undefined, {
      llm,
      timeoutMs: 25,
    })
    expect(r).toBeNull()
  })

  it('returns null when the LLM response is empty', async () => {
    const ep = makeEpisode([turn('x')])
    const llm = fixedLlm('   ')
    const r = await resummariseLLM(ep, 100, undefined, { llm })
    expect(r).toBeNull()
  })

  it('returns null when the LLM response still exceeds target', async () => {
    const ep = makeEpisode([turn('x')])
    const llm = fixedLlm('a'.repeat(1000)) // way too long
    const r = await resummariseLLM(ep, 5, undefined, { llm })
    expect(r).toBeNull()
  })
})

describe('truncate', () => {
  it('keeps RECENT turns first; drops earliest until budget fits', () => {
    const turns = [
      turn('OLDEST and quite long content here aaaa', 'a', 0),
      turn('middle turn ccc', 'a', 1),
      turn('most recent turn bbb', 'a', 2),
    ]
    const ep = makeEpisode(turns)
    // Budget that allows ~1 turn.
    const r = truncate(ep, 10)
    // Most-recent turn should be present; oldest absent.
    expect(r.text).toContain('most recent')
    expect(r.text).not.toContain('OLDEST')
    // droppedIndices should include 0 (oldest), in chronological order.
    expect(r.droppedIndices[0]).toBe(0)
    for (let i = 1; i < r.droppedIndices.length; i++) {
      expect(r.droppedIndices[i]).toBeGreaterThan(r.droppedIndices[i - 1]!)
    }
  })

  it('respects budget exactly: tokens(text) ≤ target', () => {
    const turns = [
      turn('a'.repeat(50), 'a', 0),
      turn('b'.repeat(50), 'a', 1),
      turn('c'.repeat(50), 'a', 2),
    ]
    const ep = makeEpisode(turns)
    for (const target of [5, 20, 50, 100]) {
      const r = truncate(ep, target)
      expect(r.tokens).toBeLessThanOrEqual(target)
    }
  })

  it('returns empty text and drops all when even the last turn does not fit', () => {
    const ep = makeEpisode([turn('a'.repeat(100), 'a', 0)])
    const r = truncate(ep, 1)
    expect(r.text).toBe('')
    expect(r.droppedIndices).toEqual([0])
    expect(r.tokens).toBe(0)
  })

  it('is deterministic / idempotent and does not mutate input', () => {
    const turns = [
      turn('first', 'a', 0),
      turn('second', 'a', 1),
      turn('third', 'a', 2),
    ]
    const ep = makeEpisode(turns)
    const before = JSON.parse(JSON.stringify(ep))
    const r1 = truncate(ep, 15)
    const r2 = truncate(ep, 15)
    expect(r1).toEqual(r2)
    expect(ep).toEqual(before)
  })
})
