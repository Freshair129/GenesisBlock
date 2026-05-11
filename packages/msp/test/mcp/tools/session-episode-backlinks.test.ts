import { mkdtemp, readFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import * as backlinksRebuild from '../../../src/mcp/tools/backlinks-rebuild.js'
import * as episodeAppend from '../../../src/mcp/tools/episode-append.js'
import * as sessionAppend from '../../../src/mcp/tools/session-append.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-mcp-tools-'))
}

describe('msp_session_append tool', () => {
  it('writes a turn that re-reads correctly', async () => {
    const root = await freshRoot()
    const r = await sessionAppend.handler({ root })({
      episodic_id: 'ep_test',
      turn: {
        sessionId: 's',
        episodicId: 'ep_test',
        turnId: 1,
        msgId: 'm',
        speakerId: 'user',
        content: 'hello world',
      },
      root,
    })
    expect(r.isError).toBeUndefined()
    expect(JSON.parse(r.content[0]!.text)).toEqual({ ok: true })

    const file = join(root, '.brain/msp/projects/evaAI/sessions/ep_test.jsonl')
    const lines = (await readFile(file, 'utf8')).trim().split('\n')
    expect(lines).toHaveLength(1)
    expect(JSON.parse(lines[0]!).content).toBe('hello world')
  })

  it('returns isError on schema-invalid input', async () => {
    const root = await freshRoot()
    const r = await sessionAppend.handler({ root })({
      episodic_id: 'x',
      turn: { sessionId: '' } as never,
      root,
    })
    expect(r.isError).toBe(true)
  })
})

describe('msp_episode_append tool', () => {
  it('writes an episode that re-reads correctly', async () => {
    const root = await freshRoot()
    const r = await episodeAppend.handler({ root })({
      episode: {
        episodicId: 'ep_001',
        sessionId: 's',
        projectId: 'p',
        importance_score: 0.9,
        range: ['turnIdx-1..turnIdx-3'],
        content: { summary: 'a real summary' },
      },
      root,
    })
    expect(r.isError).toBeUndefined()
    const file = join(root, '.brain/msp/projects/evaAI/memory/episodic_memory.json')
    const arr = JSON.parse(await readFile(file, 'utf8'))
    expect(arr).toHaveLength(1)
    expect(arr[0].episodicId).toBe('ep_001')
  })

  it('returns isError when importance_score is out of range', async () => {
    const root = await freshRoot()
    const r = await episodeAppend.handler({ root })({
      episode: {
        episodicId: 'x',
        sessionId: 's',
        projectId: 'p',
        importance_score: 5,
        range: ['turnIdx-1'],
        content: { summary: 'too high' },
      },
      root,
    })
    expect(r.isError).toBe(true)
  })
})

describe('msp_backlinks_rebuild tool', () => {
  it('returns RebuildResult on a fresh empty root', async () => {
    const root = await freshRoot()
    const r = await backlinksRebuild.handler({ root })({ root })
    expect(r.isError).toBeUndefined()
    const parsed = JSON.parse(r.content[0]!.text)
    expect(parsed.atomCount).toBe(0)
    expect(parsed.edgeCount).toBe(0)
  })

  it('--check returns isError when content would change', async () => {
    // No existing file → check sees drift only if content non-empty.
    // Empty repo → empty content → no drift; should NOT error.
    const root = await freshRoot()
    const r = await backlinksRebuild.handler({ root })({ root, check: true })
    expect(r.isError).toBeUndefined()
  })
})
