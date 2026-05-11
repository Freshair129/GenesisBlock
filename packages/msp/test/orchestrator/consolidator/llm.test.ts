import { describe, expect, it } from 'vitest'

import {
  buildTier2Prompt,
  callTier2,
  defaultKeepResult,
  extractJsonObject,
  parseTier2Response,
} from '../../../src/orchestrator/consolidator/llm.js'
import type { LlmClient, Turn } from '../../../src/orchestrator/consolidator/types.js'

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

function fixedLlm(response: string): LlmClient {
  return async () => response
}

describe('buildTier2Prompt', () => {
  it('contains the rubric and the chunk speaker tags', () => {
    const prompt = buildTier2Prompt([
      turn('we will ship pgvector', 'assistant', 0),
      turn('great', 'user', 1),
    ])
    expect(prompt).toContain('Scoring rubric')
    expect(prompt).toContain('[assistant]')
    expect(prompt).toContain('[user]')
    expect(prompt).toContain('we will ship pgvector')
  })
})

describe('extractJsonObject', () => {
  it('parses bare JSON', () => {
    expect(extractJsonObject('{"score":0.9,"summary":"x","tags":["a"]}')).toEqual({
      score: 0.9,
      summary: 'x',
      tags: ['a'],
    })
  })

  it('strips markdown fences', () => {
    const text = '```json\n{"score":0.5}\n```'
    expect(extractJsonObject(text)).toEqual({ score: 0.5 })
  })

  it('finds an embedded { ... } block in noisy prose', () => {
    const text = 'Here is your answer: {"score":0.7} thanks!'
    expect(extractJsonObject(text)).toEqual({ score: 0.7 })
  })

  it('returns null when no JSON object is present', () => {
    expect(extractJsonObject('plain prose with no braces')).toBeNull()
    expect(extractJsonObject('')).toBeNull()
  })
})

describe('parseTier2Response', () => {
  it('clamps score to 0..1 and slices tags to 5', () => {
    const r = parseTier2Response({
      score: 1.5,
      summary: ' got it ',
      tags: ['a', 'b', 'c', 'd', 'e', 'f', 'g'],
    })
    expect(r).not.toBeNull()
    expect(r!.score).toBe(1)
    expect(r!.summary).toBe('got it')
    expect(r!.tags).toHaveLength(5)
  })

  it('rejects non-objects and missing score', () => {
    expect(parseTier2Response(null)).toBeNull()
    expect(parseTier2Response([])).toBeNull()
    expect(parseTier2Response({ summary: 'no score' })).toBeNull()
    expect(parseTier2Response({ score: 'not a number' })).toBeNull()
  })
})

describe('callTier2', () => {
  it('returns default-keep when no llm is supplied', async () => {
    const r = await callTier2([turn('x')])
    expect(r.source).toBe('tier2-default')
    expect(r.score).toBe(0.6)
  })

  it('parses a valid LLM response into a tier-2 result', async () => {
    const llm = fixedLlm(
      '{"score":0.82,"summary":"chose pgvector","tags":["pgvector","memory"]}',
    )
    const r = await callTier2([turn('x')], { llm })
    expect(r.source).toBe('tier2')
    expect(r.score).toBeCloseTo(0.82)
    expect(r.summary).toBe('chose pgvector')
    expect(r.tags).toEqual(['pgvector', 'memory'])
  })

  it('falls back to default-keep on parse error', async () => {
    const llm = fixedLlm('completely not JSON')
    const r = await callTier2([turn('x')], { llm })
    expect(r.source).toBe('tier2-default')
  })

  it('falls back to default-keep when LLM throws', async () => {
    const llm: LlmClient = async () => {
      throw new Error('boom')
    }
    const r = await callTier2([turn('x')], { llm })
    expect(r.source).toBe('tier2-default')
  })

  it('falls back to default-keep on timeout', async () => {
    const llm: LlmClient = () => new Promise(() => {}) // never resolves
    const r = await callTier2([turn('x')], { llm, timeoutMs: 25 })
    expect(r.source).toBe('tier2-default')
  })

  it('parses a fenced JSON response', async () => {
    const llm = fixedLlm(
      'Here you go:\n```json\n{"score":0.4,"summary":"meh","tags":[]}\n```\n',
    )
    const r = await callTier2([turn('x')], { llm })
    expect(r.source).toBe('tier2')
    expect(r.score).toBeCloseTo(0.4)
  })

  it('defaultKeepResult shape matches contract', () => {
    expect(defaultKeepResult()).toEqual({
      score: 0.6,
      summary: '',
      tags: [],
      source: 'tier2-default',
    })
  })
})
