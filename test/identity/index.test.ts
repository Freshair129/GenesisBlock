import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import {
  getIdentity,
  getPreference,
  prunePreferences,
  setPreference,
  setProfile,
  setVoice,
} from '../../src/identity/index.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-identity-index-'))
}

describe('getIdentity (default values)', () => {
  it('returns default-constructed identity when no file exists', async () => {
    const root = await freshRoot()
    const id = await getIdentity({ root, namespace: 'evaAI' })
    expect(id.schemaVersion).toBe(1)
    expect(id.profile).toEqual({
      name: '',
      role: '',
      tier: 'T3',
      originStory: '',
      createdAt: '',
      guardrails: [],
      extensions: {},
    })
    expect(id.voice).toEqual({
      tone: [],
      formality: 'neutral',
      languagePreference: 'auto',
      responseCadence: 'normal',
    })
    expect(id.preferences).toEqual({})
  })
})

describe('round-trip — profile + voice + preferences', () => {
  it('stores and retrieves an identity built incrementally', async () => {
    const root = await freshRoot()
    const fixed = new Date('2026-05-04T12:00:00.000Z')

    await setProfile(
      { root, namespace: 'evaAI' },
      {
        name: 'EVA',
        role: 'research assistant',
        tier: 'T3',
        originStory: 'Created during M7e bootstrap.',
      },
      () => fixed,
    )
    await setVoice(
      { root, namespace: 'evaAI' },
      {
        tone: ['analytical', 'warm', 'concise'],
        formality: 'neutral',
        languagePreference: 'thai+english',
        responseCadence: 'terse',
      },
    )
    await setPreference(
      { root, namespace: 'evaAI' },
      'default_top_k',
      5,
    )
    await setPreference(
      { root, namespace: 'evaAI' },
      'session_verbose',
      true,
      { expiresInMs: 60 * 60 * 1000 },
      () => fixed,
    )

    const id = await getIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.name).toBe('EVA')
    expect(id.profile.createdAt).toBe('2026-05-04T12:00:00.000Z')
    expect(id.voice.tone).toEqual(['analytical', 'warm', 'concise'])
    expect(id.voice.languagePreference).toBe('thai+english')
    expect(id.preferences.default_top_k.value).toBe(5)
    expect(id.preferences.default_top_k.expiresAt).toBeNull()
    expect(id.preferences.session_verbose.value).toBe(true)
    expect(id.preferences.session_verbose.expiresAt).toBe(
      '2026-05-04T13:00:00.000Z',
    )

    // getPreference (post-set, pre-expiry) returns the value
    const k = await getPreference({ root, namespace: 'evaAI' }, 'default_top_k')
    expect(k).toBe(5)
  })
})

describe('default-state-not-null guarantee', () => {
  it('every API path on a brand new namespace returns a usable default shape', async () => {
    const root = await freshRoot()
    // No setProfile / setVoice / setPreference yet.
    const id = await getIdentity({ root, namespace: 'fresh' })
    // Profile is a complete Profile (no null fields, all 5 keys present)
    expect(typeof id.profile.name).toBe('string')
    expect(typeof id.profile.role).toBe('string')
    expect(typeof id.profile.tier).toBe('string')
    expect(typeof id.profile.originStory).toBe('string')
    expect(typeof id.profile.createdAt).toBe('string')
    // Voice is complete
    expect(Array.isArray(id.voice.tone)).toBe(true)
    expect(typeof id.voice.formality).toBe('string')
    expect(typeof id.voice.languagePreference).toBe('string')
    expect(typeof id.voice.responseCadence).toBe('string')
    // Preferences is an object (possibly empty)
    expect(typeof id.preferences).toBe('object')
    expect(id.preferences).not.toBeNull()
  })

  it('getPreference on fresh namespace returns null (not undefined / not throw)', async () => {
    const root = await freshRoot()
    const v = await getPreference({ root, namespace: 'fresh' }, 'anything')
    expect(v).toBeNull()
  })
})

describe('multi-namespace isolation', () => {
  it('operations on namespace A never touch namespace B', async () => {
    const root = await freshRoot()
    await setProfile({ root, namespace: 'nsA' }, { name: 'AgentA' })
    await setProfile({ root, namespace: 'nsB' }, { name: 'AgentB' })
    await setPreference({ root, namespace: 'nsA' }, 'k', 'A-value')
    await setPreference({ root, namespace: 'nsB' }, 'k', 'B-value')

    const a = await getIdentity({ root, namespace: 'nsA' })
    const b = await getIdentity({ root, namespace: 'nsB' })
    expect(a.profile.name).toBe('AgentA')
    expect(b.profile.name).toBe('AgentB')
    expect(a.preferences.k.value).toBe('A-value')
    expect(b.preferences.k.value).toBe('B-value')

    // Pruning on nsA must not touch nsB.
    await setPreference(
      { root, namespace: 'nsA' },
      'temp',
      'x',
      { expiresInMs: -1 }, // already expired
    )
    const pruned = await prunePreferences({ root, namespace: 'nsA' })
    expect(pruned).toBe(1)
    const b2 = await getIdentity({ root, namespace: 'nsB' })
    expect(b2.preferences.k.value).toBe('B-value')
  })
})
