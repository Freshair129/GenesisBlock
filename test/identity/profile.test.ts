import { mkdtemp, readFile, writeFile, mkdir } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import {
  identityFilePath,
  IdentityParseError,
  loadProfile,
  mergeProfile,
  saveProfile,
} from '../../src/identity/profile.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-identity-'))
}

describe('loadProfile', () => {
  it('returns null when identity.yaml is absent', async () => {
    const root = await freshRoot()
    expect(await loadProfile(root, 'evaAI')).toBeNull()
  })

  it('parses a complete profile', async () => {
    const root = await freshRoot()
    const path = identityFilePath(root, 'evaAI')
    await mkdir(join(root, '.brain/msp/projects/evaAI'), { recursive: true })
    await writeFile(
      path,
      [
        'name: EVA',
        'voice:',
        '  tone: [analytical, warm]',
        '  language_preference: thai+english',
        'preferences:',
        '  default_top_k:',
        '    value: 5',
        '    expires_at: null',
        'origin_story: born in May',
      ].join('\n'),
      'utf8',
    )
    const p = await loadProfile(root, 'evaAI')
    expect(p?.name).toBe('EVA')
    expect(p?.voice?.tone).toEqual(['analytical', 'warm'])
    expect(p?.preferences?.default_top_k.value).toBe(5)
    expect(p?.preferences?.default_top_k.expires_at).toBeNull()
    expect(p?.origin_story).toBe('born in May')
  })

  it('treats an empty file as an empty profile', async () => {
    const root = await freshRoot()
    const path = identityFilePath(root, 'evaAI')
    await mkdir(join(root, '.brain/msp/projects/evaAI'), { recursive: true })
    await writeFile(path, '', 'utf8')
    expect(await loadProfile(root, 'evaAI')).toEqual({})
  })

  it('throws IdentityParseError on bad YAML', async () => {
    const root = await freshRoot()
    const path = identityFilePath(root, 'evaAI')
    await mkdir(join(root, '.brain/msp/projects/evaAI'), { recursive: true })
    await writeFile(path, 'voice:\n  tone: [unterminated', 'utf8')
    await expect(loadProfile(root, 'evaAI')).rejects.toBeInstanceOf(IdentityParseError)
  })

  it('throws IdentityParseError on schema-invalid content', async () => {
    const root = await freshRoot()
    const path = identityFilePath(root, 'evaAI')
    await mkdir(join(root, '.brain/msp/projects/evaAI'), { recursive: true })
    // `name` must be a non-empty string when present
    await writeFile(path, 'name: ""', 'utf8')
    await expect(loadProfile(root, 'evaAI')).rejects.toBeInstanceOf(IdentityParseError)
  })
})

describe('saveProfile', () => {
  it('creates the namespace directory and writes valid YAML', async () => {
    const root = await freshRoot()
    await saveProfile(root, 'evaAI', {
      name: 'EVA',
      voice: { tone: ['warm'] },
    })
    const text = await readFile(identityFilePath(root, 'evaAI'), 'utf8')
    expect(text).toContain('name: EVA')
    expect(text).toContain('tone:')
  })

  it('round-trips through loadProfile', async () => {
    const root = await freshRoot()
    const profile = {
      name: 'EVA',
      voice: {
        tone: ['analytical', 'concise'],
        language_preference: 'en',
      },
      preferences: {
        x: { value: 42, expires_at: null },
      },
      origin_story: 'one line',
    }
    await saveProfile(root, 'evaAI', profile)
    expect(await loadProfile(root, 'evaAI')).toEqual(profile)
  })

  it('rejects an invalid profile before writing', async () => {
    const root = await freshRoot()
    await expect(
      saveProfile(root, 'evaAI', { name: '' as unknown as string }),
    ).rejects.toThrow()
    // file must not have been created
    await expect(readFile(identityFilePath(root, 'evaAI'))).rejects.toThrow()
  })
})

describe('mergeProfile', () => {
  it('overwrites top-level scalars', () => {
    const next = mergeProfile({ name: 'old' }, { name: 'new' })
    expect(next.name).toBe('new')
  })

  it('merges voice fields without losing prior fields', () => {
    const next = mergeProfile(
      { voice: { tone: ['warm'], language_preference: 'en' } },
      { voice: { tone: ['concise'] } as never },
    )
    expect(next.voice?.tone).toEqual(['concise'])
    expect(next.voice?.language_preference).toBe('en')
  })

  it('merges preferences key-by-key', () => {
    const next = mergeProfile(
      { preferences: { a: { value: 1, expires_at: null } } },
      { preferences: { b: { value: 2, expires_at: null } } },
    )
    expect(Object.keys(next.preferences ?? {}).sort()).toEqual(['a', 'b'])
  })
})
