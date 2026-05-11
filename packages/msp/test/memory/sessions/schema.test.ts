import { describe, expect, it } from 'vitest'

import { serialiseTurn, validateTurn } from '../../../src/memory/sessions/schema.js'
import { SessionSchemaError } from '../../../src/memory/sessions/types.js'

describe('validateTurn', () => {
  it('accepts a complete row', () => {
    const r = validateTurn({
      sessionId: 's1',
      episodicId: 'e1',
      turnId: 1,
      msgId: 'm1',
      speakerId: 'user',
      content: 'hi',
    })
    expect(r.turnId).toBe(1)
  })

  it('accepts optional learnId', () => {
    const r = validateTurn({
      sessionId: 's1',
      episodicId: 'e1',
      turnId: 1,
      msgId: 'm1',
      speakerId: 'user',
      content: 'hi',
      learnId: 'FEAT--FOO',
    })
    expect(r.learnId).toBe('FEAT--FOO')
  })

  it('rejects when sessionId is missing', () => {
    expect(() =>
      validateTurn({
        episodicId: 'e',
        turnId: 1,
        msgId: 'm',
        speakerId: 'u',
        content: 'c',
      }),
    ).toThrow(SessionSchemaError)
  })

  it('rejects when turnId is not a number', () => {
    expect(() =>
      validateTurn({
        sessionId: 's',
        episodicId: 'e',
        turnId: 'one' as unknown as number,
        msgId: 'm',
        speakerId: 'u',
        content: 'c',
      }),
    ).toThrow(SessionSchemaError)
  })

  it('rejects empty string content', () => {
    expect(() =>
      validateTurn({
        sessionId: 's',
        episodicId: 'e',
        turnId: 1,
        msgId: 'm',
        speakerId: 'u',
        content: '',
      }),
    ).toThrow(SessionSchemaError)
  })

  it('rejects non-objects', () => {
    expect(() => validateTurn(null)).toThrow(SessionSchemaError)
    expect(() => validateTurn('not an object')).toThrow(SessionSchemaError)
    expect(() => validateTurn([])).toThrow(SessionSchemaError)
  })
})

describe('serialiseTurn', () => {
  it('escapes embedded newlines via JSON.stringify', () => {
    const line = serialiseTurn({
      sessionId: 's',
      episodicId: 'e',
      turnId: 1,
      msgId: 'm',
      speakerId: 'u',
      content: 'multi\nline',
    })
    expect(line.includes('\n')).toBe(false)
    expect(line.includes('\\n')).toBe(true)
  })
})
