import { describe, expect, it } from 'vitest'

import { checkForbiddenPatterns, checkImports } from '../../src/codegen/forbidden-patterns.js'

describe('checkForbiddenPatterns', () => {
  it('returns empty errors for clean code', () => {
    const r = checkForbiddenPatterns('export const x = 1')
    expect(r.errors).toEqual([])
    expect(r.warnings).toEqual([])
  })

  it('flags export default as error', () => {
    const r = checkForbiddenPatterns('export default function foo() {}')
    expect(r.errors.some((e) => e.includes('[export-default]'))).toBe(true)
  })

  it('flags req.body as error', () => {
    const r = checkForbiddenPatterns('const x = req.body')
    expect(r.errors.some((e) => e.includes('[req-body]'))).toBe(true)
  })

  it('flags TODO as error', () => {
    const r = checkForbiddenPatterns('// TODO: implement later')
    expect(r.errors.some((e) => e.includes('[todo-comment]'))).toBe(true)
  })

  it('flags console.log as warning (not error)', () => {
    const r = checkForbiddenPatterns('console.log("hi")')
    expect(r.errors).toEqual([])
    expect(r.warnings.some((w) => w.includes('[console-log]'))).toBe(true)
  })

  it('flags process.env as warning', () => {
    const r = checkForbiddenPatterns('const x = process.env.FOO')
    expect(r.warnings.some((w) => w.includes('[process-env]'))).toBe(true)
  })
})

describe('checkImports', () => {
  it('allows zod when in deps', () => {
    const r = checkImports('import { z } from "zod"', new Set(['zod']))
    expect(r).toEqual([])
  })

  it('rejects zod when not in deps', () => {
    const r = checkImports('import { z } from "zod"', new Set())
    expect(r.length).toBe(1)
    expect(r[0]).toMatch(/zod/)
  })

  it('rejects fs always', () => {
    const r = checkImports('import { readFile } from "fs"', new Set(['fs']))
    expect(r.length).toBe(1)
    expect(r[0]).toMatch(/route layer/)
  })

  it('rejects ../ relative imports', () => {
    const r = checkImports('import { foo } from "../foo"', new Set())
    expect(r.length).toBe(1)
    expect(r[0]).toMatch(/relative parent/)
  })

  it('allows @/ alias imports', () => {
    const r = checkImports('import { foo } from "@/lib/foo"', new Set())
    expect(r).toEqual([])
  })

  it('handles multiple imports per code block', () => {
    const code = `
      import { foo } from "@/lib/foo"
      import axios from "axios"
      import { z } from "zod"
    `
    const r = checkImports(code, new Set())
    expect(r.length).toBe(2) // axios + zod, both forbidden when not in deps
  })
})
