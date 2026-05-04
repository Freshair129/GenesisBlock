import { describe, expect, it } from 'vitest'

import {
  isExpired,
  makeEntry,
  PreferenceEntrySchema,
} from '../../src/identity/preferences.js'

describe('makeEntry', () => {
  it('produces a non-expiring entry when ttl is omitted', () => {
    const e = makeEntry(5)
    expect(e.value).toBe(5)
    expect(e.expires_at).toBeNull()
  })

  it('produces a non-expiring entry when ttl is null', () => {
    expect(makeEntry('hi', null).expires_at).toBeNull()
  })

  it('encodes ttl as ISO timestamp now+ttl', () => {
    const now = new Date('2026-01-01T00:00:00.000Z')
    const e = makeEntry('x', 60_000, now)
    expect(e.expires_at).toBe('2026-01-01T00:01:00.000Z')
  })

  it('validates against the entry schema', () => {
    expect(() => PreferenceEntrySchema.parse(makeEntry({ nested: [1, 2] }))).not.toThrow()
  })
})

describe('isExpired', () => {
  it('returns false for null expires_at', () => {
    expect(isExpired({ value: 0, expires_at: null })).toBe(false)
  })

  it('returns true once the timestamp is in the past', () => {
    const past = new Date(Date.now() - 1000).toISOString()
    expect(isExpired({ value: 0, expires_at: past })).toBe(true)
  })

  it('returns false while the timestamp is in the future', () => {
    const future = new Date(Date.now() + 60_000).toISOString()
    expect(isExpired({ value: 0, expires_at: future })).toBe(false)
  })
})

describe('PreferenceEntrySchema', () => {
  it('accepts primitives, arrays, and nested objects as values', () => {
    expect(() =>
      PreferenceEntrySchema.parse({ value: 'a', expires_at: null }),
    ).not.toThrow()
    expect(() =>
      PreferenceEntrySchema.parse({ value: [1, 2, 3], expires_at: null }),
    ).not.toThrow()
    expect(() =>
      PreferenceEntrySchema.parse({
        value: { a: { b: [true, null] } },
        expires_at: null,
      }),
    ).not.toThrow()
  })

  it('rejects unknown top-level keys', () => {
    expect(() =>
      PreferenceEntrySchema.parse({ value: 1, expires_at: null, extra: 'no' }),
    ).toThrow()
  })

  it('rejects non-ISO expires_at strings', () => {
    expect(() =>
      PreferenceEntrySchema.parse({ value: 1, expires_at: 'soon' }),
    ).toThrow()
  })
})
