import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setProfile } from '../../src/identity/profile.js'
import { readIdentity } from '../../src/identity/store.js'
import { setVoice } from '../../src/identity/voice.js'

async function freshRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-identity-voice-'))
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

describe('setVoice', () => {
  it('replaces the entire voice object on first write', async () => {
    const root = await freshRoot()
    await setVoice(
      { root, namespace: 'evaAI' },
      {
        tone: ['analytical', 'concise'],
        formality: 'neutral',
        languagePreference: 'thai+english',
        responseCadence: 'terse',
      },
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.voice.tone).toEqual(['analytical', 'concise'])
    expect(id.voice.formality).toBe('neutral')
    expect(id.voice.languagePreference).toBe('thai+english')
    expect(id.voice.responseCadence).toBe('terse')
  })

  it('full-replace: a second setVoice call wipes prior tone array', async () => {
    const root = await freshRoot()
    await setVoice(
      { root, namespace: 'evaAI' },
      {
        tone: ['warm', 'analytical'],
        formality: 'casual',
        languagePreference: 'en',
        responseCadence: 'verbose',
      },
    )
    // Second call with a wholly different voice — prior tone must NOT leak.
    await setVoice(
      { root, namespace: 'evaAI' },
      {
        tone: ['terse'],
        formality: 'formal',
        languagePreference: 'th',
        responseCadence: 'terse',
      },
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.voice.tone).toEqual(['terse'])
    expect(id.voice.formality).toBe('formal')
    expect(id.voice.languagePreference).toBe('th')
    expect(id.voice.responseCadence).toBe('terse')
  })

  it('accepts free-form languagePreference (no enum validation)', async () => {
    const root = await freshRoot()
    // CONCEPT / BLUEPRINT: languagePreference is intentionally free-form.
    await setVoice(
      { root, namespace: 'evaAI' },
      {
        tone: [],
        formality: 'neutral',
        languagePreference: 'klingon-en-pidgin',
        responseCadence: 'normal',
      },
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.voice.languagePreference).toBe('klingon-en-pidgin')
  })

  it('accepts free-form tone strings (no enum validation)', async () => {
    const root = await freshRoot()
    await setVoice(
      { root, namespace: 'evaAI' },
      {
        // Arbitrary tone descriptors — implementation must not gatekeep.
        tone: ['inquisitive', 'pedagogical', 'gentle-skeptic'],
        formality: 'casual',
        languagePreference: 'en',
        responseCadence: 'verbose',
      },
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.voice.tone).toEqual(['inquisitive', 'pedagogical', 'gentle-skeptic'])
  })

  it('does not touch profile or preferences', async () => {
    const root = await freshRoot()
    // Seed a profile first.
    await setProfile(
      { root, namespace: 'evaAI' },
      { name: 'EVA', role: 'research' },
    )
    // Now setVoice — profile must remain.
    await setVoice(
      { root, namespace: 'evaAI' },
      {
        tone: ['concise'],
        formality: 'neutral',
        languagePreference: 'auto',
        responseCadence: 'normal',
      },
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.name).toBe('EVA')
    expect(id.profile.role).toBe('research')
    expect(id.voice.tone).toEqual(['concise'])
    expect(id.preferences).toEqual({})
  })
})
