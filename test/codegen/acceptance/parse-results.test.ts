import { describe, expect, it } from 'vitest'

import { parseVitestJson } from '../../../src/codegen/acceptance/parse-results.js'

const ALL_PASSING = JSON.stringify({
  numFailedTests: 0,
  numPassedTests: 1,
  testResults: [
    {
      name: '/sandbox/test/foo.test.ts',
      assertionResults: [
        { status: 'passed', fullName: 'foo > works', failureMessages: [] },
      ],
    },
  ],
})

const ONE_FAIL = JSON.stringify({
  numFailedTests: 1,
  numPassedTests: 1,
  testResults: [
    {
      name: '/sandbox/test/foo.test.ts',
      assertionResults: [
        { status: 'passed', fullName: 'foo > works', failureMessages: [] },
        { status: 'failed', fullName: 'foo > fails', failureMessages: ['expected 41 to be 42\n  at line ...'] },
      ],
    },
  ],
})

describe('parseVitestJson', () => {
  it('returns empty array for all-passing JSON', () => {
    expect(parseVitestJson(ALL_PASSING, '', 0)).toEqual([])
  })

  it('emits one error per failed assertion', () => {
    const errs = parseVitestJson(ONE_FAIL, '', 1)
    expect(errs).toHaveLength(1)
    expect(errs[0]).toMatch(/foo\.test\.ts/)
    expect(errs[0]).toMatch(/foo > fails/)
    expect(errs[0]).toMatch(/expected 41 to be 42/)
  })

  it('handles JSON wrapped in log lines', () => {
    const wrapped = `vitest version 2.1.9\n${ONE_FAIL}\nDone.`
    const errs = parseVitestJson(wrapped, '', 1)
    expect(errs).toHaveLength(1)
  })

  it('falls back to stderr summary on malformed JSON + non-zero exit', () => {
    const errs = parseVitestJson('not json', 'Error: cannot find module foo\n', 1)
    expect(errs).toHaveLength(1)
    expect(errs[0]).toMatch(/cannot find module foo/)
  })

  it('returns empty array on malformed JSON + zero exit', () => {
    expect(parseVitestJson('', 'noisy stderr', 0)).toEqual([])
  })

  it('truncates first line of failure message to 240 chars', () => {
    const longMsg = 'x'.repeat(500)
    const json = JSON.stringify({
      testResults: [
        {
          name: 'a.test.ts',
          assertionResults: [{ status: 'failed', fullName: 't', failureMessages: [longMsg] }],
        },
      ],
    })
    const errs = parseVitestJson(json, '', 1)
    expect(errs[0]!.length).toBeLessThan(longMsg.length)
  })

  it('produces a default message when failureMessages is empty', () => {
    const json = JSON.stringify({
      testResults: [
        {
          name: 'a.test.ts',
          assertionResults: [{ status: 'failed', fullName: 't', failureMessages: [] }],
        },
      ],
    })
    const errs = parseVitestJson(json, '', 1)
    expect(errs[0]).toMatch(/no message/)
  })
})
