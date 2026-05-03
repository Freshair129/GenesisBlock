import { mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

import { createVitestAcceptance } from '../../../src/codegen/acceptance/vitest.js'

const repoRoot = fileURLToPath(new URL('../../..', import.meta.url))

const VERIFICATION_TEST = `import { describe, expect, it } from 'vitest'
import { sum } from '../src/sum.ts'

describe('sum', () => {
  it('adds two numbers', () => {
    expect(sum(1, 2)).toBe(3)
  })
})
`

describe('createVitestAcceptance', () => {
  it('returns [] for code that passes the verification test', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'vt-fixture-'))
    const verificationPath = join(dir, 'expected.test.ts')
    await writeFile(verificationPath, VERIFICATION_TEST)

    const accept = createVitestAcceptance({
      repoRoot,
      verificationFiles: [{ src: verificationPath, dest: 'test/sum.test.ts' }],
      timeoutMs: 60_000,
    })

    const code = 'export function sum(a: number, b: number): number { return a + b }\n'
    const errs = await accept(
      { id: 'T', parent_blueprint: 'B', prompt: 'p', acceptance: ['ok'], geography: ['src/sum.ts'] },
      code,
    )
    expect(errs).toEqual([])
  }, 60_000)

  it('returns at least one error string for code that fails the verification', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'vt-fixture-'))
    const verificationPath = join(dir, 'expected.test.ts')
    await writeFile(verificationPath, VERIFICATION_TEST)

    const accept = createVitestAcceptance({
      repoRoot,
      verificationFiles: [{ src: verificationPath, dest: 'test/sum.test.ts' }],
      timeoutMs: 60_000,
    })

    // Buggy candidate — adds 1 instead of summing
    const badCode = 'export function sum(a: number, _b: number): number { return a + 1 }\n'
    const errs = await accept(
      { id: 'T', parent_blueprint: 'B', prompt: 'p', acceptance: ['ok'], geography: ['src/sum.ts'] },
      badCode,
    )
    expect(errs.length).toBeGreaterThan(0)
    // Either a per-assertion failure or a stderr-fallback summary
    expect(errs.join('\n')).toMatch(/sum|adds two numbers|expected/i)
  }, 60_000)
})
