import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { handler, name } from '../../../src/mcp/tools/identity-get.js'

async function setupRoot(namespace = 'evaAI'): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-identity-get-tool-'))
  await mkdir(join(root, '.brain/msp/projects', namespace), {
    recursive: true,
  })
  return root
}

describe('msp_identity_get tool', () => {
  it('has the right name', () => {
    expect(name).toBe('msp_identity_get')
  })

  it('returns default identity when file is missing', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({ root })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.identity.schemaVersion).toBe(1)
    expect(parsed.identity.profile.name).toBe('')
    expect(parsed.identity.profile.tier).toBe('T3')
    expect(parsed.identity.voice.formality).toBe('neutral')
    expect(parsed.identity.preferences).toEqual({})
  })

  it('returns persisted identity when file exists', async () => {
    const root = await setupRoot()
    const path = join(root, '.brain/msp/projects/evaAI/identity.json')
    await writeFile(
      path,
      JSON.stringify({
        schemaVersion: 1,
        profile: {
          name: 'EVA',
          role: 'research',
          tier: 'T2',
          originStory: 'born in lab',
          createdAt: '2026-01-01T00:00:00.000Z',
          guardrails: ['never invent atom IDs'],
          extensions: {},
        },
        voice: {
          tone: ['analytical'],
          formality: 'neutral',
          languagePreference: 'en',
          responseCadence: 'normal',
        },
        preferences: { topK: { value: 5, expiresAt: null } },
      }),
      'utf8',
    )
    const result = await handler({ root })({ root })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.identity.profile.name).toBe('EVA')
    expect(parsed.identity.voice.tone).toEqual(['analytical'])
    expect(parsed.identity.preferences.topK.value).toBe(5)
  })

  it('honours namespace argument', async () => {
    const root = await setupRoot('foo')
    const path = join(root, '.brain/msp/projects/foo/identity.json')
    await writeFile(
      path,
      JSON.stringify({
        schemaVersion: 1,
        profile: {
          name: 'foo-agent',
          role: '',
          tier: 'T3',
          originStory: '',
          createdAt: '2026-01-01T00:00:00.000Z',
          guardrails: [],
          extensions: {},
        },
        voice: {
          tone: [],
          formality: 'neutral',
          languagePreference: 'auto',
          responseCadence: 'normal',
        },
        preferences: {},
      }),
      'utf8',
    )
    const result = await handler({ root })({ root, namespace: 'foo' })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.identity.profile.name).toBe('foo-agent')
  })

  it('prune=true removes expired preferences before returning', async () => {
    const root = await setupRoot()
    const path = join(root, '.brain/msp/projects/evaAI/identity.json')
    await writeFile(
      path,
      JSON.stringify({
        schemaVersion: 1,
        profile: {
          name: '',
          role: '',
          tier: 'T3',
          originStory: '',
          createdAt: '',
          guardrails: [],
          extensions: {},
        },
        voice: {
          tone: [],
          formality: 'neutral',
          languagePreference: 'auto',
          responseCadence: 'normal',
        },
        preferences: {
          stale: {
            value: 'gone',
            expiresAt: '2020-01-01T00:00:00.000Z',
          },
          fresh: { value: 'kept', expiresAt: null },
        },
      }),
      'utf8',
    )
    const result = await handler({ root })({ root, prune: true })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.identity.preferences.stale).toBeUndefined()
    expect(parsed.identity.preferences.fresh).toBeDefined()

    // Verify pruning was persisted to disk
    const raw = JSON.parse(await readFile(path, 'utf8'))
    expect(raw.preferences.stale).toBeUndefined()
  })

  it('refuses files with schemaVersion > 1', async () => {
    const root = await setupRoot()
    const path = join(root, '.brain/msp/projects/evaAI/identity.json')
    await writeFile(path, JSON.stringify({ schemaVersion: 99 }), 'utf8')
    const result = await handler({ root })({ root })
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/schemaVersion=99/)
  })
})
