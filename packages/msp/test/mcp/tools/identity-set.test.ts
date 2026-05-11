import { mkdir, mkdtemp, readFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { globalIdentityPath } from '../../../src/lib/msp-home.js'
import { handler, name } from '../../../src/mcp/tools/identity-set.js'

async function setupRoot(namespace = 'evaAI'): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-identity-set-tool-'))
  await mkdir(join(root, '.brain/msp/projects', namespace), { recursive: true })
  process.env['MSP_HOME'] = resolve(root, '.msp')
  return root
}

async function readGlobalIdentityFile(): Promise<{
  schemaVersion: 1
  profile: { name: string }
  voice: unknown
  preferences: Record<string, unknown>
}> {
  return JSON.parse(await readFile(globalIdentityPath(), 'utf8'))
}

let savedHome: string | undefined
beforeEach(() => {
  savedHome = process.env['MSP_HOME']
})
afterEach(() => {
  if (savedHome === undefined) delete process.env['MSP_HOME']
  else process.env['MSP_HOME'] = savedHome
})

describe('msp_identity_set tool', () => {
  it('has the right name', () => {
    expect(name).toBe('msp_identity_set')
  })

  it('kind=profile partial-merges and stamps createdAt on first write', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({
      kind: 'profile',
      partial: { name: 'EVA', role: 'research' },
      root,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.identity.profile.name).toBe('EVA')
    expect(parsed.identity.profile.role).toBe('research')
    expect(parsed.identity.profile.tier).toBe('T3')
    expect(parsed.identity.profile.createdAt).not.toBe('')
    // Persisted to the global path (default scope=global)
    const onDisk = await readGlobalIdentityFile()
    expect(onDisk.profile.name).toBe('EVA')
  })

  it('kind=profile preserves createdAt across writes (set-once)', async () => {
    const root = await setupRoot()
    const first = await handler({ root })({
      kind: 'profile',
      partial: { name: 'first' },
      root,
    })
    const initialCreatedAt = JSON.parse(first.content[0]!.text).identity.profile
      .createdAt
    // small delay to ensure now() differs
    await new Promise((r) => setTimeout(r, 10))
    const second = await handler({ root })({
      kind: 'profile',
      partial: { role: 'updated' },
      root,
    })
    const parsed = JSON.parse(second.content[0]!.text)
    expect(parsed.identity.profile.name).toBe('first')
    expect(parsed.identity.profile.role).toBe('updated')
    expect(parsed.identity.profile.createdAt).toBe(initialCreatedAt)
  })

  it('kind=profile errors when partial is missing', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({ kind: 'profile', root })
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/partial/)
  })

  it('kind=voice replaces the entire voice sub-field', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({
      kind: 'voice',
      voice: {
        tone: ['analytical', 'concise'],
        formality: 'neutral',
        languagePreference: 'en',
        responseCadence: 'terse',
      },
      root,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.identity.voice.tone).toEqual(['analytical', 'concise'])
    expect(parsed.identity.voice.responseCadence).toBe('terse')
  })

  it('kind=voice errors when voice is missing', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({ kind: 'voice', root })
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/voice/)
  })

  it('kind=preference saves a value with no TTL', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({
      kind: 'preference',
      key: 'top_k',
      value: 5,
      root,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.identity.preferences.top_k.value).toBe(5)
    expect(parsed.identity.preferences.top_k.expiresAt).toBe(null)
  })

  it('kind=preference accepts expires_at (absolute ISO)', async () => {
    const root = await setupRoot()
    const expiresAt = '2030-01-01T00:00:00.000Z'
    const result = await handler({ root })({
      kind: 'preference',
      key: 'flag',
      value: 'on',
      expires_at: expiresAt,
      root,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.identity.preferences.flag.expiresAt).toBe(expiresAt)
  })

  it('kind=preference accepts expires_in_ms (relative)', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({
      kind: 'preference',
      key: 'transient',
      value: { x: 1 },
      expires_in_ms: 60_000,
      root,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    const ts = Date.parse(parsed.identity.preferences.transient.expiresAt)
    expect(Number.isFinite(ts)).toBe(true)
    expect(ts - Date.now()).toBeGreaterThan(50_000)
    expect(ts - Date.now()).toBeLessThan(120_000)
  })

  it('kind=preference overrides an existing entry', async () => {
    const root = await setupRoot()
    await handler({ root })({
      kind: 'preference',
      key: 'count',
      value: 1,
      root,
    })
    const result = await handler({ root })({
      kind: 'preference',
      key: 'count',
      value: 99,
      root,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.identity.preferences.count.value).toBe(99)
  })

  it('kind=preference errors when key is missing or empty', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({
      kind: 'preference',
      value: 'x',
      root,
    } as Parameters<ReturnType<typeof handler>>[0])
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/key/)
  })

  it('kind=preference errors when value is missing', async () => {
    const root = await setupRoot()
    const result = await handler({ root })({
      kind: 'preference',
      key: 'x',
      root,
    } as Parameters<ReturnType<typeof handler>>[0])
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/value/)
  })

  it('scope=project writes a per-project override (not the global file)', async () => {
    const root = await setupRoot()
    // Seed the global first so we have something to override
    await handler({ root })({
      kind: 'voice',
      voice: {
        tone: [],
        formality: 'casual',
        languagePreference: 'en',
        responseCadence: 'normal',
      },
      scope: 'global',
      root,
    })
    // Override formality at project scope
    const result = await handler({ root })({
      kind: 'voice',
      voice: {
        tone: [],
        formality: 'formal',
        languagePreference: 'en',
        responseCadence: 'normal',
      },
      scope: 'project',
      namespace: 'clinic',
      root,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    // Merged read in clinic returns the override
    expect(parsed.identity.voice.formality).toBe('formal')
    // Global remains casual
    const global = await readGlobalIdentityFile()
    expect((global.voice as { formality: string }).formality).toBe('casual')
  })
})
