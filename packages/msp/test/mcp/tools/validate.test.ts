import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

import { handler, name } from '../../../src/mcp/tools/validate.js'

const repoRoot = fileURLToPath(new URL('../../..', import.meta.url))

describe('msp_validate tool', () => {
  it('has the right name', () => {
    expect(name).toBe('msp_validate')
  })

  it('errors when neither files nor all is set', async () => {
    const result = await handler({ root: repoRoot })({})
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/must supply/)
  })

  it('--all returns ok=true on the real repo', async () => {
    const result = await handler({ root: repoRoot })({ all: true, root: repoRoot })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.results.length).toBeGreaterThan(0)
  }, 30_000)

  it('files=[<known-bad fixture>] returns ok=false with errors', async () => {
    const result = await handler({ root: repoRoot })({
      files: [`${repoRoot}/test/fixtures/CONCEPT--TEST-FORBIDDEN.md`],
      root: repoRoot,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(false)
    expect(parsed.results[0].errors.length).toBeGreaterThan(0)
  }, 30_000)
})
