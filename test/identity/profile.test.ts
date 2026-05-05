import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { setProfile } from '../../src/identity/profile.js'
import { readIdentity } from '../../src/identity/store.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-identity-profile-'))
}

describe('setProfile', () => {
  it('writes a profile with set-once createdAt on first write', async () => {
    const root = await freshRoot()
    const fixed = new Date('2026-05-04T12:00:00.000Z')
    await setProfile(
      { root, namespace: 'evaAI' },
      { name: 'EVA', role: 'research assistant', tier: 'T3' },
      () => fixed,
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.name).toBe('EVA')
    expect(id.profile.role).toBe('research assistant')
    expect(id.profile.tier).toBe('T3')
    expect(id.profile.createdAt).toBe('2026-05-04T12:00:00.000Z')
  })

  it('preserves createdAt on subsequent writes', async () => {
    const root = await freshRoot()
    const first = new Date('2026-05-04T12:00:00.000Z')
    const second = new Date('2026-06-15T15:30:00.000Z')
    await setProfile(
      { root, namespace: 'evaAI' },
      { name: 'EVA' },
      () => first,
    )
    // Second call uses a different `now`; createdAt must be the FIRST one.
    await setProfile(
      { root, namespace: 'evaAI' },
      { role: 'updated' },
      () => second,
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.createdAt).toBe('2026-05-04T12:00:00.000Z')
    expect(id.profile.name).toBe('EVA')
    expect(id.profile.role).toBe('updated')
  })

  it('partial merge keeps unspecified fields', async () => {
    const root = await freshRoot()
    await setProfile(
      { root, namespace: 'evaAI' },
      { name: 'EVA', role: 'research', originStory: 'born in May' },
    )
    // Only update role; name + originStory must persist.
    await setProfile({ root, namespace: 'evaAI' }, { role: 'orchestrator' })
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.name).toBe('EVA')
    expect(id.profile.role).toBe('orchestrator')
    expect(id.profile.originStory).toBe('born in May')
  })

  it('refuses caller-supplied createdAt (set-once defends against override)', async () => {
    const root = await freshRoot()
    const real = new Date('2026-05-04T12:00:00.000Z')
    await setProfile({ root, namespace: 'evaAI' }, { name: 'EVA' }, () => real)
    // Caller tries to overwrite createdAt via partial — must be ignored.
    await setProfile(
      { root, namespace: 'evaAI' },
      {
        role: 'r',
        createdAt: '1999-01-01T00:00:00.000Z',
      } as unknown as Partial<import('../../src/identity/types.js').Profile>,
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.createdAt).toBe('2026-05-04T12:00:00.000Z')
  })

  it('does not touch voice or preferences', async () => {
    const root = await freshRoot()
    await setProfile({ root, namespace: 'evaAI' }, { name: 'EVA' })
    const id = await readIdentity({ root, namespace: 'evaAI' })
    // Voice + preferences are still default after setProfile
    expect(id.voice.formality).toBe('neutral')
    expect(id.voice.tone).toEqual([])
    expect(id.preferences).toEqual({})
  })

  it('accepts tier override (T1 / T2 / T3)', async () => {
    const root = await freshRoot()
    await setProfile({ root, namespace: 'evaAI' }, { tier: 'T1' })
    let id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.tier).toBe('T1')
    await setProfile({ root, namespace: 'evaAI' }, { tier: 'T2' })
    id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.tier).toBe('T2')
  })

  // Guardrails + extensions added by ADR--IDENTITY-GUARDRAILS-AND-EXTENSIONS.

  it('default profile has empty guardrails and extensions', async () => {
    const root = await freshRoot()
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.guardrails).toEqual([])
    expect(id.profile.extensions).toEqual({})
  })

  it('persists guardrails as ordered string list', async () => {
    const root = await freshRoot()
    await setProfile(
      { root, namespace: 'evaAI' },
      {
        name: 'EVA',
        guardrails: [
          'Never invent atom IDs.',
          'Never claim a test pass without naming the runner output.',
        ],
      },
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.guardrails).toEqual([
      'Never invent atom IDs.',
      'Never claim a test pass without naming the runner output.',
    ])
  })

  it('persists extensions as opaque key/value bag', async () => {
    const root = await freshRoot()
    await setProfile(
      { root, namespace: 'evaAI' },
      {
        extensions: {
          'claude-code/notes': 'starter profile',
          'cursor/last-seen': 1735689600000,
          'eva/affect': { ri: 'L3', valence: 0.6 },
        },
      },
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.extensions['claude-code/notes']).toBe('starter profile')
    expect(id.profile.extensions['cursor/last-seen']).toBe(1735689600000)
    expect(id.profile.extensions['eva/affect']).toEqual({
      ri: 'L3',
      valence: 0.6,
    })
  })

  it('partial-merges guardrails (replace, not append)', async () => {
    const root = await freshRoot()
    await setProfile(
      { root, namespace: 'evaAI' },
      { guardrails: ['rule-1', 'rule-2'] },
    )
    await setProfile({ root, namespace: 'evaAI' }, { guardrails: ['rule-3'] })
    const id = await readIdentity({ root, namespace: 'evaAI' })
    // setProfile replaces fields it touches (consistent with rest of partial merge).
    expect(id.profile.guardrails).toEqual(['rule-3'])
  })

  it('preserves guardrails / extensions when other fields are updated', async () => {
    const root = await freshRoot()
    await setProfile(
      { root, namespace: 'evaAI' },
      {
        name: 'EVA',
        guardrails: ['rule-1'],
        extensions: { 'k/x': 1 },
      },
    )
    await setProfile({ root, namespace: 'evaAI' }, { role: 'researcher' })
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.profile.role).toBe('researcher')
    expect(id.profile.guardrails).toEqual(['rule-1'])
    expect(id.profile.extensions).toEqual({ 'k/x': 1 })
  })
})
