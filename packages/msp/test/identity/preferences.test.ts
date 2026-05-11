import { mkdtemp, readFile, stat } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { globalIdentityPath } from '../../src/lib/msp-home.js'
import {
  getPreference,
  prunePreferences,
  setPreference,
} from '../../src/identity/preferences.js'
import { readIdentity } from '../../src/identity/store.js'

async function freshRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-identity-prefs-'))
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

describe('setPreference / getPreference (no TTL)', () => {
  it('sets a value and reads it back', async () => {
    const root = await freshRoot()
    await setPreference({ root, namespace: 'evaAI' }, 'top_k', 5)
    expect(await getPreference({ root, namespace: 'evaAI' }, 'top_k')).toBe(5)
    // expiresAt is null when no TTL given.
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.preferences.top_k).toEqual({ value: 5, expiresAt: null })
  })

  it('returns null for a missing key', async () => {
    const root = await freshRoot()
    const v = await getPreference({ root, namespace: 'evaAI' }, 'nope')
    expect(v).toBeNull()
  })
})

describe('setPreference with TTL', () => {
  it('expiresInMs computes correct expiresAt relative to now', async () => {
    const root = await freshRoot()
    const fixed = new Date('2026-05-04T12:00:00.000Z')
    await setPreference(
      { root, namespace: 'evaAI' },
      'session_verbose',
      true,
      { expiresInMs: 60 * 60 * 1000 }, // 1h
      () => fixed,
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.preferences.session_verbose.value).toBe(true)
    expect(id.preferences.session_verbose.expiresAt).toBe(
      '2026-05-04T13:00:00.000Z',
    )
  })

  it('expiresAt (ISO) wins over expiresInMs when both given', async () => {
    const root = await freshRoot()
    const fixed = new Date('2026-05-04T12:00:00.000Z')
    await setPreference(
      { root, namespace: 'evaAI' },
      'k',
      'v',
      {
        expiresAt: '2030-01-01T00:00:00.000Z',
        expiresInMs: 1000,
      },
      () => fixed,
    )
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(id.preferences.k.expiresAt).toBe('2030-01-01T00:00:00.000Z')
  })
})

describe('lazy expiry', () => {
  it('getPreference returns null for an expired entry but does NOT mutate the file', async () => {
    const root = await freshRoot()
    const setAt = new Date('2026-05-04T12:00:00.000Z')
    await setPreference(
      { root, namespace: 'evaAI' },
      'short',
      'value',
      { expiresInMs: 1000 },
      () => setAt,
    )
    const path = globalIdentityPath()
    const mtimeBefore = (await stat(path)).mtimeMs

    // Read 1h after expiry — must return null.
    const readAt = new Date('2026-05-04T13:00:00.000Z')
    const v = await getPreference(
      { root, namespace: 'evaAI' },
      'short',
      () => readAt,
    )
    expect(v).toBeNull()

    // File must not have been re-written (lazy expiry).
    const mtimeAfter = (await stat(path)).mtimeMs
    expect(mtimeAfter).toBe(mtimeBefore)
    // The entry is still on disk.
    const text = await readFile(path, 'utf8')
    expect(text).toContain('"short"')
  })

  it('non-expired entry returns its value', async () => {
    const root = await freshRoot()
    const setAt = new Date('2026-05-04T12:00:00.000Z')
    await setPreference(
      { root, namespace: 'evaAI' },
      'k',
      42,
      { expiresInMs: 60 * 60 * 1000 }, // 1h
      () => setAt,
    )
    const readAt = new Date('2026-05-04T12:30:00.000Z')
    const v = await getPreference(
      { root, namespace: 'evaAI' },
      'k',
      () => readAt,
    )
    expect(v).toBe(42)
  })
})

describe('value shapes', () => {
  it('accepts arbitrary JSON-serialisable values (number, string, bool, object, array, null)', async () => {
    const root = await freshRoot()
    await setPreference({ root, namespace: 'evaAI' }, 'num', 42)
    await setPreference({ root, namespace: 'evaAI' }, 'str', 'hello')
    await setPreference({ root, namespace: 'evaAI' }, 'bool', true)
    await setPreference({ root, namespace: 'evaAI' }, 'obj', { a: 1, b: 'x' })
    await setPreference({ root, namespace: 'evaAI' }, 'arr', [1, 2, 3])
    await setPreference({ root, namespace: 'evaAI' }, 'null', null)
    expect(await getPreference({ root, namespace: 'evaAI' }, 'num')).toBe(42)
    expect(await getPreference({ root, namespace: 'evaAI' }, 'str')).toBe('hello')
    expect(await getPreference({ root, namespace: 'evaAI' }, 'bool')).toBe(true)
    expect(await getPreference({ root, namespace: 'evaAI' }, 'obj')).toEqual({ a: 1, b: 'x' })
    expect(await getPreference({ root, namespace: 'evaAI' }, 'arr')).toEqual([1, 2, 3])
    expect(await getPreference({ root, namespace: 'evaAI' }, 'null')).toBeNull()
  })

  it('setPreference replaces an existing entry (last-writer-wins per key)', async () => {
    const root = await freshRoot()
    await setPreference({ root, namespace: 'evaAI' }, 'k', 'v1')
    await setPreference({ root, namespace: 'evaAI' }, 'k', 'v2')
    expect(await getPreference({ root, namespace: 'evaAI' }, 'k')).toBe('v2')
  })
})

describe('prunePreferences', () => {
  it('removes expired entries, keeps non-expired, returns count', async () => {
    const root = await freshRoot()
    const setAt = new Date('2026-05-04T12:00:00.000Z')
    await setPreference(
      { root, namespace: 'evaAI' },
      'expired1',
      'a',
      { expiresInMs: 1000 },
      () => setAt,
    )
    await setPreference(
      { root, namespace: 'evaAI' },
      'expired2',
      'b',
      { expiresInMs: 2000 },
      () => setAt,
    )
    await setPreference(
      { root, namespace: 'evaAI' },
      'alive',
      'c',
      { expiresInMs: 60 * 60 * 1000 }, // 1h
      () => setAt,
    )
    await setPreference(
      { root, namespace: 'evaAI' },
      'never',
      'd',
      // no TTL
      undefined,
      () => setAt,
    )

    const pruneAt = new Date('2026-05-04T12:30:00.000Z')
    const count = await prunePreferences(
      { root, namespace: 'evaAI' },
      () => pruneAt,
    )
    expect(count).toBe(2)
    const id = await readIdentity({ root, namespace: 'evaAI' })
    expect(Object.keys(id.preferences).sort()).toEqual(['alive', 'never'])
  })

  it('returns 0 and does not rewrite when nothing is expired', async () => {
    const root = await freshRoot()
    const setAt = new Date('2026-05-04T12:00:00.000Z')
    await setPreference(
      { root, namespace: 'evaAI' },
      'forever',
      'x',
      undefined,
      () => setAt,
    )
    const path = globalIdentityPath()
    const mtimeBefore = (await stat(path)).mtimeMs
    const count = await prunePreferences({ root, namespace: 'evaAI' })
    expect(count).toBe(0)
    const mtimeAfter = (await stat(path)).mtimeMs
    expect(mtimeAfter).toBe(mtimeBefore)
  })
})
