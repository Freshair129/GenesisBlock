import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { globalIdentityPath } from '../../../src/lib/msp-home.js'
import { handler, name } from '../../../src/mcp/tools/identity-get.js'

async function setupRoot(namespace = 'evaAI'): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-identity-get-tool-'))
  await mkdir(join(root, '.brain/msp/projects', namespace), {
    recursive: true,
  })
  process.env['MSP_HOME'] = resolve(root, '.msp')
  return root
}

let savedHome: string | undefined
beforeEach(() => {
  savedHome = process.env['MSP_HOME']
})
afterEach(() => {
  if (savedHome === undefined) delete process.env['MSP_HOME']
  else process.env['MSP_HOME'] = savedHome
})

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

  it('returns persisted identity from the global file', async () => {
    const root = await setupRoot()
    // Pre-create the global file directly.
    const globalPath = globalIdentityPath()
    await mkdir(join(globalPath, '..'), { recursive: true })
    await writeFile(
      globalPath,
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

  it('migrates a legacy workspace identity to global on first read', async () => {
    const root = await setupRoot('foo')
    const legacyPath = join(root, '.brain/msp/projects/foo/identity.json')
    await writeFile(
      legacyPath,
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
    // Migration also wrote the global file
    const globalText = await readFile(globalIdentityPath(), 'utf8')
    expect(JSON.parse(globalText).profile.name).toBe('foo-agent')
  })

  it('prune=true removes expired preferences before returning', async () => {
    const root = await setupRoot()
    const globalPath = globalIdentityPath()
    await mkdir(join(globalPath, '..'), { recursive: true })
    await writeFile(
      globalPath,
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

    // Verify pruning was persisted to disk (global file)
    const raw = JSON.parse(await readFile(globalPath, 'utf8'))
    expect(raw.preferences.stale).toBeUndefined()
  })

  it('refuses files with schemaVersion > 1', async () => {
    const root = await setupRoot()
    const globalPath = globalIdentityPath()
    await mkdir(join(globalPath, '..'), { recursive: true })
    await writeFile(globalPath, JSON.stringify({ schemaVersion: 99 }), 'utf8')
    const result = await handler({ root })({ root })
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/schemaVersion=99/)
  })

  it('view=global returns only the global identity (ignores project override)', async () => {
    const root = await setupRoot()
    // Set up global + project override differing on formality.
    const { writeIdentity } = await import('../../../src/identity/index.js')
    await writeIdentity({ scope: 'global' }, {
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
        formality: 'casual',
        languagePreference: 'auto',
        responseCadence: 'normal',
      },
      preferences: {},
    })
    await writeIdentity(
      { scope: 'project', root, namespace: 'clinic' },
      {
        voice: {
          tone: [],
          formality: 'formal',
          languagePreference: 'auto',
          responseCadence: 'normal',
        },
      },
    )
    const result = await handler({ root })({
      root,
      namespace: 'clinic',
      view: 'global',
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.identity.voice.formality).toBe('casual')
  })

  it('explain=true returns the resolution chain alongside the identity', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({ root, explain: true })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.explain).toBeDefined()
    expect(parsed.explain.global).toBeDefined()
    expect(parsed.explain.project).toBeDefined()
  })
})
