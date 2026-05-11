import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { consolidate } from '../../../src/orchestrator/consolidator/index.js'
import type {
  LlmClient,
  Turn,
} from '../../../src/orchestrator/consolidator/types.js'

interface PartialTurn {
  speakerId?: string
  content: string
}

async function fixtureSession(
  turns: PartialTurn[],
  sessionId = 'sess-test-001',
  namespace = 'evaAI',
): Promise<{ root: string; sessionId: string }> {
  const root = await mkdtemp(join(tmpdir(), 'msp-consolidator-'))
  const dir = join(root, '.brain/msp/projects', namespace, 'sessions')
  await mkdir(dir, { recursive: true })
  const lines: string[] = []
  for (let i = 0; i < turns.length; i++) {
    const t: Turn = {
      sessionId,
      episodicId: 'e1',
      turnId: i,
      msgId: `m${i}`,
      speakerId: turns[i]!.speakerId ?? (i % 2 === 0 ? 'user' : 'assistant'),
      content: turns[i]!.content,
    }
    lines.push(JSON.stringify(t))
  }
  await writeFile(join(dir, `${sessionId}.jsonl`), lines.join('\n') + '\n', 'utf8')
  return { root, sessionId }
}

const fixedNow = () => new Date('2026-05-04T12:00:00.000Z')

describe('consolidate — end-to-end', () => {
  it('returns [] when session file is missing', async () => {
    const root = await mkdtemp(join(tmpdir(), 'msp-consolidator-empty-'))
    const eps = await consolidate({ sessionId: 'nope', root, now: fixedNow })
    expect(eps).toEqual([])
  })

  it('drops greeting-only sessions', async () => {
    const { root, sessionId } = await fixtureSession([
      { content: 'hi' },
      { content: 'thanks' },
      { content: 'got it' },
    ])
    const eps = await consolidate({ sessionId, root, now: fixedNow })
    expect(eps).toEqual([])
  })

  it('keeps decision-rich tier-1 chunks without invoking LLM', async () => {
    let llmCalls = 0
    const llm: LlmClient = async () => {
      llmCalls += 1
      return '{"score":0.9}'
    }
    const { root, sessionId } = await fixtureSession([
      {
        speakerId: 'user',
        content: 'should we use pgvector or qdrant for the index?',
      },
      {
        speakerId: 'assistant',
        content:
          "we will use pgvector — see src/memory/episodic/writer.ts. Decided per ADR--MEMORY-EPISODIC-WRITER on 2026-05-04. Bumped to v0.2.0.",
      },
    ])
    const eps = await consolidate({ sessionId, root, llm, now: fixedNow })
    expect(eps.length).toBeGreaterThan(0)
    expect(eps[0]!.scoreSource).toBe('tier1')
    expect(eps[0]!.sessionId).toBe(sessionId)
    expect(eps[0]!.createdAt).toBe('2026-05-04T12:00:00.000Z')
    expect(llmCalls).toBe(0)
  })

  it('borderline chunks invoke LLM; LLM keep → tier2 episode', async () => {
    const llm: LlmClient = async () =>
      JSON.stringify({
        score: 0.75,
        summary: 'considered pgvector',
        tags: ['pgvector'],
      })
    const { root, sessionId } = await fixtureSession([
      // Crafted so that tier-1 score lands in the borderline band:
      // - has "will" decision marker (+0.35) but nothing else strong
      { speakerId: 'user', content: 'we will think about it tomorrow' },
      { speakerId: 'assistant', content: 'we will consider that next week' },
    ])
    const eps = await consolidate({
      sessionId,
      root,
      llm,
      thresholds: { low: 0.05, high: 0.8 }, // force borderline path
      now: fixedNow,
    })
    expect(eps.length).toBeGreaterThan(0)
    const sources = eps.map((e) => e.scoreSource)
    // At least one episode should come from tier2 in this configuration
    expect(sources).toContain('tier2')
  })

  it('budget cap: borderline beyond max → tier2-default', async () => {
    let calls = 0
    const llm: LlmClient = async () => {
      calls += 1
      return '{"score":0.9}'
    }
    // Force every chunk into the borderline band by setting low=0/high=1 so
    // any score lands inside; force a topic split per turn via boundary=1.0.
    // 9 turns → enough for the window=3 boundary detector to cut multiple times.
    const { root, sessionId } = await fixtureSession([
      { content: 'alpha alpha alpha will think about it tomorrow morning' },
      { content: 'alpha bravo will think about plan tomorrow morning' },
      { content: 'alpha charlie will think about plan tomorrow morning' },
      { content: 'beta beta beta will consider this option later' },
      { content: 'beta delta will consider option later afternoon' },
      { content: 'beta echo will consider option later afternoon' },
      { content: 'gamma gamma gamma will examine maybe next week' },
      { content: 'gamma foxtrot will examine maybe next week' },
      { content: 'gamma golf will examine maybe next week' },
    ])
    const eps = await consolidate({
      sessionId,
      root,
      llm,
      thresholds: { low: 0, high: 1, boundary: 1.0 }, // force borderline + splits
      maxLlmCallsPerSession: 1,
      now: fixedNow,
    })
    expect(calls).toBeLessThanOrEqual(1)
    const defaults = eps.filter((e) => e.scoreSource === 'tier2-default')
    expect(defaults.length).toBeGreaterThan(0)
    const tier2 = eps.filter((e) => e.scoreSource === 'tier2')
    expect(tier2.length).toBe(calls)
  })

  it('idempotent: same input twice → identical Episode[]', async () => {
    const llm: LlmClient = async () =>
      JSON.stringify({
        score: 0.7,
        summary: 'mock',
        tags: ['x'],
      })
    const { root, sessionId } = await fixtureSession([
      {
        speakerId: 'assistant',
        content:
          "we'll use pgvector. see src/memory/episodic/writer.ts. ADR--MEMORY-EPISODIC-WRITER decided 2026-05-04.",
      },
      { speakerId: 'user', content: 'thanks' },
    ])
    const opts = { sessionId, root, llm, now: fixedNow }
    const first = await consolidate(opts)
    const second = await consolidate(opts)
    expect(JSON.stringify(first)).toBe(JSON.stringify(second))
  })
})
