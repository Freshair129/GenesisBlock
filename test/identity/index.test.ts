import { mkdtemp, readFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import {
  getIdentity,
  getPreference,
  setPreference,
  setProfile,
  setVoice,
} from '../../src/identity/index.js'
import { identityFilePath } from '../../src/identity/profile.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-identity-api-'))
}

describe('getIdentity', () => {
  it('returns an empty profile when no file exists', async () => {
    const root = await freshRoot()
    expect(await getIdentity('evaAI', { root })).toEqual({})
  })

  it('returns the loaded profile when present', async () => {
    const root = await freshRoot()
    await setProfile('evaAI', { name: 'EVA' }, { root })
    expect((await getIdentity('evaAI', { root })).name).toBe('EVA')
  })

  it('defaults namespace to evaAI', async () => {
    const root = await freshRoot()
    await setProfile('evaAI', { name: 'EVA' }, { root })
    expect((await getIdentity(undefined, { root })).name).toBe('EVA')
  })
})

describe('setProfile', () => {
  it('creates the file with merged content on first call', async () => {
    const root = await freshRoot()
    await setProfile('evaAI', { name: 'EVA', origin_story: 'one' }, { root })
    const text = await readFile(identityFilePath(root, 'evaAI'), 'utf8')
    expect(text).toContain('name: EVA')
    expect(text).toContain('origin_story: one')
  })

  it('preserves prior unrelated fields', async () => {
    const root = await freshRoot()
    await setProfile('evaAI', { name: 'EVA' }, { root })
    await setProfile('evaAI', { origin_story: 'born' }, { root })
    const p = await getIdentity('evaAI', { root })
    expect(p.name).toBe('EVA')
    expect(p.origin_story).toBe('born')
  })
})

describe('setVoice', () => {
  it('writes voice without touching other fields', async () => {
    const root = await freshRoot()
    await setProfile('evaAI', { name: 'EVA', origin_story: 'x' }, { root })
    await setVoice('evaAI', { tone: ['warm'], language_preference: 'en' }, { root })
    const p = await getIdentity('evaAI', { root })
    expect(p.name).toBe('EVA')
    expect(p.origin_story).toBe('x')
    expect(p.voice?.tone).toEqual(['warm'])
    expect(p.voice?.language_preference).toBe('en')
  })

  it('merges over a prior voice', async () => {
    const root = await freshRoot()
    await setVoice('evaAI', { tone: ['warm'], language_preference: 'en' }, { root })
    await setVoice('evaAI', { tone: ['concise'] }, { root })
    const p = await getIdentity('evaAI', { root })
    expect(p.voice?.tone).toEqual(['concise'])
    expect(p.voice?.language_preference).toBe('en')
  })
})

describe('getPreference / setPreference', () => {
  it('round-trips a non-expiring entry', async () => {
    const root = await freshRoot()
    await setPreference('evaAI', 'default_top_k', 5, null, { root })
    expect(await getPreference('evaAI', 'default_top_k', { root })).toBe(5)
  })

  it('returns undefined for a missing key', async () => {
    const root = await freshRoot()
    expect(await getPreference('evaAI', 'nope', { root })).toBeUndefined()
  })

  it('returns undefined for an expired entry and prunes it from disk', async () => {
    const root = await freshRoot()
    await setPreference('evaAI', 'transient', 'gone', 1, { root })
    // Advance past the 1ms TTL.
    await new Promise((r) => setTimeout(r, 5))
    expect(await getPreference('evaAI', 'transient', { root })).toBeUndefined()
    const text = await readFile(identityFilePath(root, 'evaAI'), 'utf8')
    expect(text).not.toContain('transient')
  })

  it('overwrites existing entries', async () => {
    const root = await freshRoot()
    await setPreference('evaAI', 'k', 1, null, { root })
    await setPreference('evaAI', 'k', 2, null, { root })
    expect(await getPreference('evaAI', 'k', { root })).toBe(2)
  })

  it('supports complex JSON values', async () => {
    const root = await freshRoot()
    await setPreference('evaAI', 'shape', { tags: ['a', 'b'] }, null, { root })
    expect(await getPreference('evaAI', 'shape', { root })).toEqual({ tags: ['a', 'b'] })
  })

  it('does not disturb other identity fields', async () => {
    const root = await freshRoot()
    await setProfile('evaAI', { name: 'EVA' }, { root })
    await setPreference('evaAI', 'k', 1, null, { root })
    const p = await getIdentity('evaAI', { root })
    expect(p.name).toBe('EVA')
    expect(p.preferences?.k.value).toBe(1)
  })
})
