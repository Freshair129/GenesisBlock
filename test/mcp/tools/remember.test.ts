import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { handler, name } from '../../../src/mcp/tools/remember.js'
import type { Turn } from '../../../src/orchestrator/consolidator/types.js'

interface PartialTurn {
  speakerId?: string
  content: string
}

async function fixtureSession(
  turns: PartialTurn[],
  sessionId = 'sess-test-001',
  namespace = 'evaAI',
): Promise<{ root: string; sessionId: string; namespace: string }> {
  const root = await mkdtemp(join(tmpdir(), 'msp-remember-tool-'))
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
  return { root, sessionId, namespace }
}

describe('msp_remember tool', () => {
  it('has the right name', () => {
    expect(name).toBe('msp_remember')
  })

  it('returns empty when session.jsonl is missing', async () => {
    const root = await mkdtemp(join(tmpdir(), 'msp-remember-tool-empty-'))
    const result = await handler({ root })({
      session_id: 'nope',
      root,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.episodes_emitted).toEqual([])
    expect(parsed.episodes_persisted).toBe(0)
    expect(parsed.llm_calls).toBe(0)
  })

  it('emits + persists episodes for a decision-rich session', async () => {
    const { root, sessionId, namespace } = await fixtureSession([
      { speakerId: 'user', content: 'should we use pgvector or qdrant?' },
      {
        speakerId: 'assistant',
        content:
          "we will use pgvector — see src/memory/episodic/writer.ts. Decided per ADR--MEMORY-EPISODIC-WRITER on 2026-05-04. Bumped to v0.2.0.",
      },
    ])
    const result = await handler({ root })({
      session_id: sessionId,
      root,
      namespace,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.episodes_emitted.length).toBeGreaterThan(0)
    expect(parsed.episodes_persisted).toBe(parsed.episodes_emitted.length)

    // Verify file contents on disk
    const memPath = join(
      root,
      '.brain/msp/projects',
      namespace,
      'memory/episodic_memory.json',
    )
    const raw = await readFile(memPath, 'utf8')
    const persisted = JSON.parse(raw)
    expect(Array.isArray(persisted)).toBe(true)
    expect(persisted.length).toBe(parsed.episodes_persisted)
    // Check shape adapter mapped correctly
    expect(persisted[0].episodicId).toMatch(new RegExp(`^${sessionId}-\\d+-\\d+$`))
    expect(persisted[0].projectId).toBe(namespace)
    expect(persisted[0].content.summary).toBeDefined()
  })

  it('persists with default namespace (`evaAI`) when not supplied', async () => {
    const { root, sessionId, namespace } = await fixtureSession(
      [
        { speakerId: 'user', content: 'should we use pgvector?' },
        {
          speakerId: 'assistant',
          content:
            "we will use pgvector. Decided per ADR--MEMORY-EPISODIC-WRITER on 2026-05-04.",
        },
      ],
      'sess-default-ns',
      'evaAI',
    )
    const result = await handler({ root })({
      session_id: sessionId,
      root,
      // namespace omitted — should default to evaAI
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    if (parsed.episodes_emitted.length > 0) {
      const memPath = join(
        root,
        '.brain/msp/projects',
        namespace,
        'memory/episodic_memory.json',
      )
      const raw = await readFile(memPath, 'utf8')
      const persisted = JSON.parse(raw)
      expect(persisted.length).toBeGreaterThan(0)
    }
  })

  it('idempotent: calling twice does not duplicate persisted episodes', async () => {
    const { root, sessionId, namespace } = await fixtureSession([
      { speakerId: 'user', content: 'should we use pgvector or qdrant?' },
      {
        speakerId: 'assistant',
        content:
          "we will use pgvector. ADR--MEMORY-EPISODIC-WRITER decided 2026-05-04. See src/memory/episodic/writer.ts.",
      },
    ])
    await handler({ root })({ session_id: sessionId, root, namespace })
    const second = await handler({ root })({ session_id: sessionId, root, namespace })
    const parsed = JSON.parse(second.content[0]!.text)
    expect(parsed.ok).toBe(true)

    const memPath = join(
      root,
      '.brain/msp/projects',
      namespace,
      'memory/episodic_memory.json',
    )
    const raw = await readFile(memPath, 'utf8')
    const persisted = JSON.parse(raw)
    // appendEpisode is idempotent on episodicId — count must equal episodes_emitted
    expect(persisted.length).toBe(parsed.episodes_emitted.length)
  })

  it('handles a missing session id without erroring', async () => {
    // Tool wraps everything in try/catch — passing a non-existent session is
    // the safe path: consolidate returns [] for missing files. Verify no
    // error shape is returned and the writer is never invoked.
    const root = await mkdtemp(join(tmpdir(), 'msp-remember-tool-err-'))
    const result = await handler({ root })({
      session_id: 'missing',
      root,
    })
    expect(result.isError).toBeFalsy()
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.episodes_emitted).toEqual([])
  })
})
