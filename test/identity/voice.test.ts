import { describe, expect, it } from 'vitest'

import { emptyVoice, mergeVoice, VoiceSchema } from '../../src/identity/voice.js'

describe('VoiceSchema', () => {
  it('defaults tone to []', () => {
    expect(VoiceSchema.parse({}).tone).toEqual([])
  })

  it('accepts the documented shape', () => {
    expect(() =>
      VoiceSchema.parse({
        tone: ['analytical', 'warm', 'concise'],
        language_preference: 'thai+english',
      }),
    ).not.toThrow()
  })

  it('rejects unknown keys (strict)', () => {
    expect(() => VoiceSchema.parse({ tone: [], unknown: 'x' })).toThrow()
  })

  it('rejects empty tone strings', () => {
    expect(() => VoiceSchema.parse({ tone: [''] })).toThrow()
  })
})

describe('mergeVoice', () => {
  it('takes prior fields when patch is empty', () => {
    const m = mergeVoice({ tone: ['warm'], language_preference: 'en' }, {})
    expect(m.tone).toEqual(['warm'])
    expect(m.language_preference).toBe('en')
  })

  it('overwrites only the fields in patch', () => {
    const m = mergeVoice(
      { tone: ['warm'], language_preference: 'en' },
      { tone: ['concise'] },
    )
    expect(m.tone).toEqual(['concise'])
    expect(m.language_preference).toBe('en')
  })

  it('drops undefined patch fields rather than nulling prior', () => {
    const m = mergeVoice(
      { tone: ['warm'], language_preference: 'en' },
      { language_preference: undefined },
    )
    expect(m.language_preference).toBe('en')
  })

  it('starts from empty when prior is undefined', () => {
    expect(mergeVoice(undefined, { tone: ['warm'] })).toEqual(emptyVoice() && { tone: ['warm'] })
  })
})
