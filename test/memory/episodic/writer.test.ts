import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { appendEpisode } from '../../../src/memory/episodic/writer.js'
import { EpisodicSchemaError } from '../../../src/memory/episodic/types.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-episodic-'))
}

const VALID_EP = {
  episodicId: 'ep_001',
  sessionId: 'sess_001',
  projectId: 'evaAI',
  importance_score: 0.8,
  range: ['turnIdx-1..turnIdx-5'],
  content: { summary: 'a brief summary' },
}

function memoryPath(root: string): string {
  return join(root, '.brain/msp/projects/evaAI/memory/episodic_memory.json')
}

describe('appendEpisode', () => {
  it('creates the memory file and writes one episode', async () => {
    const root = await freshRoot()
    await appendEpisode(VALID_EP, { root })
    const text = await readFile(memoryPath(root), 'utf8')
    const parsed = JSON.parse(text)
    expect(parsed).toHaveLength(1)
    expect(parsed[0].episodicId).toBe('ep_001')
    expect(parsed[0].timestamp).toBeDefined()
  })

  it('overwrites existing entry by episodicId (idempotent)', async () => {
    const root = await freshRoot()
    await appendEpisode(VALID_EP, { root })
    await appendEpisode({ ...VALID_EP, importance_score: 0.5 }, { root })
    const parsed = JSON.parse(await readFile(memoryPath(root), 'utf8'))
    expect(parsed).toHaveLength(1)
    expect(parsed[0].importance_score).toBe(0.5)
  })

  it('appends new episodes with distinct ids', async () => {
    const root = await freshRoot()
    await appendEpisode(VALID_EP, { root })
    await appendEpisode({ ...VALID_EP, episodicId: 'ep_002' }, { root })
    const parsed = JSON.parse(await readFile(memoryPath(root), 'utf8'))
    expect(parsed.map((e: { episodicId: string }) => e.episodicId).sort()).toEqual(['ep_001', 'ep_002'])
  })

  it('rejects invalid episode without writing', async () => {
    const root = await freshRoot()
    const { importance_score: _, ...bad } = VALID_EP
    await expect(appendEpisode(bad as never, { root })).rejects.toBeInstanceOf(EpisodicSchemaError)
    // No file should have been created.
    await expect(readFile(memoryPath(root))).rejects.toThrow()
  })

  it('atomic write — does not leave a tmp file on success', async () => {
    const root = await freshRoot()
    await appendEpisode(VALID_EP, { root })
    await expect(readFile(memoryPath(root) + '.tmp')).rejects.toThrow()
  })
})

describe('appendEpisode.fromTurns', () => {
  it('reads turns in range, summarises, and appends', async () => {
    const root = await freshRoot()
    // Plant a session JSONL.
    const sessionsDir = join(root, '.brain/msp/projects/evaAI/sessions')
    await mkdir(sessionsDir, { recursive: true })
    const lines = [
      JSON.stringify({ sessionId: 'sess_001', episodicId: 'ep_007', turnId: 1, msgId: 'm1', speakerId: 'user', content: 'hello' }),
      JSON.stringify({ sessionId: 'sess_001', episodicId: 'ep_007', turnId: 2, msgId: 'm2', speakerId: 'MSP-AGT', content: 'I decided to use Postgres for storage today.' }),
      JSON.stringify({ sessionId: 'sess_001', episodicId: 'ep_007', turnId: 3, msgId: 'm3', speakerId: 'user', content: 'ok' }),
      JSON.stringify({ sessionId: 'sess_001', episodicId: 'ep_007', turnId: 99, msgId: 'm99', speakerId: 'user', content: 'out of range' }),
    ].join('\n') + '\n'
    await writeFile(join(sessionsDir, 'ep_007.jsonl'), lines)

    await appendEpisode.fromTurns({
      root,
      episodicId: 'ep_007',
      sessionId: 'sess_001',
      projectId: 'evaAI',
      range: ['turnIdx-1..turnIdx-3'],
      importance_score: 0.7,
    })

    const parsed = JSON.parse(await readFile(memoryPath(root), 'utf8'))
    expect(parsed).toHaveLength(1)
    expect(parsed[0].content.summary).toMatch(/Postgres/)
  })

  it('honours a custom summariser plugin', async () => {
    const root = await freshRoot()
    const sessionsDir = join(root, '.brain/msp/projects/evaAI/sessions')
    await mkdir(sessionsDir, { recursive: true })
    await writeFile(
      join(sessionsDir, 'ep_008.jsonl'),
      JSON.stringify({ sessionId: 's', episodicId: 'ep_008', turnId: 1, msgId: 'm', speakerId: 'u', content: 'x' }) + '\n',
    )

    await appendEpisode.fromTurns({
      root,
      episodicId: 'ep_008',
      sessionId: 's',
      projectId: 'evaAI',
      range: ['turnIdx-1'],
      importance_score: 0.5,
      summariser: async () => ({ summary: 'CUSTOM SUMMARY' }),
    })

    const parsed = JSON.parse(await readFile(memoryPath(root), 'utf8'))
    expect(parsed[0].content.summary).toBe('CUSTOM SUMMARY')
  })
})
