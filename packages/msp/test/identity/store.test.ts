import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { globalIdentityPath } from '../../src/lib/msp-home.js'
import {
  identityPath,
  projectOverridePath,
  readIdentity,
  resolveOptions,
  writeIdentity,
} from '../../src/identity/store.js'
import { defaultIdentity, type Identity } from '../../src/identity/types.js'

async function freshRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-identity-store-'))
  // Isolate global identity per-test by routing MSP_HOME under the fresh root.
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

describe('identityPath', () => {
  it('joins root + .brain/msp/projects/<ns>/identity.json (legacy/migration path)', () => {
    const p = identityPath('/home/user/work', 'evaAI')
    expect(p).toBe('/home/user/work/.brain/msp/projects/evaAI/identity.json')
  })
})

describe('projectOverridePath', () => {
  it('joins root + .brain/msp/projects/<ns>/identity.override.json', () => {
    const p = projectOverridePath('/home/user/work', 'evaAI')
    expect(p).toBe(
      '/home/user/work/.brain/msp/projects/evaAI/identity.override.json',
    )
  })
})

describe('resolveOptions', () => {
  it('defaults root to cwd, namespace to evaAI, view to merged', () => {
    const { root, namespace, view } = resolveOptions()
    expect(root).toBe(process.cwd())
    expect(namespace).toBe('evaAI')
    expect(view).toBe('merged')
  })

  it('honours explicit overrides', () => {
    const { root, namespace, view } = resolveOptions({
      root: '/r',
      namespace: 'ns2',
      view: 'global',
    })
    expect(root).toBe('/r')
    expect(namespace).toBe('ns2')
    expect(view).toBe('global')
  })
})

describe('readIdentity', () => {
  it('returns default-constructed Identity when no file exists (no file created)', async () => {
    const root = await freshRoot()
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.schemaVersion).toBe(1)
    expect(id.profile.name).toBe('')
    expect(id.profile.tier).toBe('T3')
    expect(id.voice.formality).toBe('neutral')
    expect(id.preferences).toEqual({})
    // Confirm the default-construction did NOT materialise the global file.
    await expect(readFile(globalIdentityPath())).rejects.toThrow()
  })

  it('throws when on-disk global file has schemaVersion > 1', async () => {
    await freshRoot()
    const path = globalIdentityPath()
    await mkdir(dirname(path), { recursive: true })
    await writeFile(
      path,
      JSON.stringify({ schemaVersion: 2, profile: {}, voice: {}, preferences: {} }),
      'utf8',
    )
    await expect(readIdentity({ namespace: 'evaAI' })).rejects.toThrow(
      /schemaVersion=2/,
    )
  })

  it('shallow-merges missing fields with defaults', async () => {
    await freshRoot()
    const path = globalIdentityPath()
    await mkdir(dirname(path), { recursive: true })
    // Partial file: profile.name only, no voice, no preferences.
    await writeFile(
      path,
      JSON.stringify({ schemaVersion: 1, profile: { name: 'EVA' } }),
      'utf8',
    )
    const id = await readIdentity({ namespace: 'evaAI' })
    expect(id.profile.name).toBe('EVA')
    // Missing fields filled from defaults
    expect(id.profile.tier).toBe('T3')
    expect(id.voice.formality).toBe('neutral')
    expect(id.preferences).toEqual({})
  })
})

describe('writeIdentity', () => {
  it('default scope=global writes to ~/.msp/identity.json', async () => {
    await freshRoot()
    const id = defaultIdentity()
    id.profile.name = 'EVA'
    await writeIdentity(undefined, id)
    const text = await readFile(globalIdentityPath(), 'utf8')
    const parsed = JSON.parse(text) as Identity
    expect(parsed.schemaVersion).toBe(1)
    expect(parsed.profile.name).toBe('EVA')
  })

  it('round-trips through readIdentity (merged view)', async () => {
    await freshRoot()
    const written = defaultIdentity()
    written.profile.name = 'EVA'
    written.profile.role = 'research assistant'
    written.voice.tone = ['analytical', 'warm']
    written.voice.languagePreference = 'thai+english'
    written.preferences.top_k = { value: 5, expiresAt: null }
    await writeIdentity({ scope: 'global' }, written)
    const loaded = await readIdentity({ namespace: 'evaAI' })
    expect(loaded).toEqual(written)
  })

  it('scope=project writes a sparse override per workspace+namespace', async () => {
    const root = await freshRoot()
    // Seed a global identity first
    const global = defaultIdentity()
    global.profile.name = 'GlobalEVA'
    global.voice.formality = 'casual'
    await writeIdentity({ scope: 'global' }, global)

    // Override only voice.formality in the workspace
    await writeIdentity(
      { scope: 'project', root, namespace: 'clinic' },
      { voice: { ...global.voice, formality: 'formal' } },
    )

    // Merged read in clinic namespace returns the override
    const clinic = await readIdentity({ root, namespace: 'clinic' })
    expect(clinic.profile.name).toBe('GlobalEVA') // from global
    expect(clinic.voice.formality).toBe('formal') // from override

    // Merged read in a different namespace returns the global value
    const other = await readIdentity({ root, namespace: 'other' })
    expect(other.voice.formality).toBe('casual')
  })

  it('global view ignores project overrides', async () => {
    const root = await freshRoot()
    const global = defaultIdentity()
    global.voice.formality = 'casual'
    await writeIdentity({ scope: 'global' }, global)
    await writeIdentity(
      { scope: 'project', root, namespace: 'ns' },
      { voice: { ...global.voice, formality: 'formal' } },
    )
    const id = await readIdentity({ root, namespace: 'ns', view: 'global' })
    expect(id.voice.formality).toBe('casual')
  })

  it('project view returns only override fields layered on defaults', async () => {
    const root = await freshRoot()
    await writeIdentity(
      { scope: 'project', root, namespace: 'ns' },
      { voice: { ...defaultIdentity().voice, formality: 'formal' } },
    )
    const id = await readIdentity({ root, namespace: 'ns', view: 'project' })
    expect(id.voice.formality).toBe('formal')
    // Profile is default (override did not touch it)
    expect(id.profile.name).toBe('')
  })

  it('forces schemaVersion to 1 on every global write', async () => {
    await freshRoot()
    const bogus = {
      ...defaultIdentity(),
      schemaVersion: 99 as unknown as 1,
    }
    await writeIdentity({ scope: 'global' }, bogus)
    const text = await readFile(globalIdentityPath(), 'utf8')
    const parsed = JSON.parse(text) as Identity
    expect(parsed.schemaVersion).toBe(1)
  })

  it('atomic write — temp file is gone after rename (global)', async () => {
    await freshRoot()
    const id = defaultIdentity()
    id.profile.name = 'EVA'
    await writeIdentity({ scope: 'global' }, id)
    const { readdir } = await import('node:fs/promises')
    const files = await readdir(dirname(globalIdentityPath()))
    expect(files).toContain('identity.json')
    expect(files.filter((f) => f.includes('.tmp.'))).toEqual([])
  })

  it('serialised writes preserve last-writer-wins (in-process)', async () => {
    await freshRoot()
    const a = defaultIdentity()
    a.profile.name = 'first'
    await writeIdentity({ scope: 'global' }, a)
    const b = defaultIdentity()
    b.profile.name = 'second'
    await writeIdentity({ scope: 'global' }, b)
    const id = await readIdentity()
    expect(id.profile.name).toBe('second')
  })

  it('readIdentity uses defaults for missing nested fields (forward-compat)', async () => {
    await freshRoot()
    const path = globalIdentityPath()
    await mkdir(dirname(path), { recursive: true })
    // File with only voice.tone — missing voice.formality, etc.
    await writeFile(
      path,
      JSON.stringify({
        schemaVersion: 1,
        voice: { tone: ['analytical'] },
      }),
      'utf8',
    )
    const id = await readIdentity()
    expect(id.voice.tone).toEqual(['analytical'])
    expect(id.voice.formality).toBe('neutral')
    expect(id.voice.languagePreference).toBe('auto')
    expect(id.voice.responseCadence).toBe('normal')
  })
})
