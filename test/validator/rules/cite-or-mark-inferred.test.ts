import { describe, expect, it } from 'vitest'

import { citeOrMarkInferred } from '../../../src/validator/rules/cite-or-mark-inferred.js'
import type { ParsedAtom, ValidationContext } from '../../../src/validator/types.js'

const ctx: ValidationContext = { atomicIndex: new Map() }

function atom(body: string, fm: Record<string, unknown> = {}): ParsedAtom {
  return { body, fm, source: '', filepath: 'x.md' }
}

describe('citeOrMarkInferred', () => {
  it('no warning when body has no path-shaped claims', () => {
    expect(citeOrMarkInferred(atom('Just prose, no paths.'), ctx)).toEqual([])
  })

  it('warns on a body claim without linked_symbols', () => {
    const errs = citeOrMarkInferred(
      atom('We changed src/foo/bar.ts to do X.'),
      ctx,
    )
    expect(errs).toHaveLength(1)
    expect(errs[0]!.severity).toBe('warning')
    expect(errs[0]!.rule).toBe('cite-or-mark-inferred')
    expect(errs[0]!.offending).toMatch(/src\/foo\/bar\.ts/)
  })

  it('no warning when linked_symbols cites the path', () => {
    expect(
      citeOrMarkInferred(
        atom('We changed src/foo.ts.', {
          linked_symbols: [{ file: 'src/foo.ts' }],
        }),
        ctx,
      ),
    ).toEqual([])
  })

  it('no warning when epistemic.source_type=inferred + confidence<1', () => {
    expect(
      citeOrMarkInferred(
        atom('Probably src/foo.ts handles this.', {
          epistemic: { source_type: 'inferred', confidence: 0.5 },
        }),
        ctx,
      ),
    ).toEqual([])
  })

  it('still warns when inferred but confidence is exactly 1.0', () => {
    expect(
      citeOrMarkInferred(
        atom('Probably src/foo.ts handles this.', {
          epistemic: { source_type: 'inferred', confidence: 1.0 },
        }),
        ctx,
      ).length,
    ).toBeGreaterThan(0)
  })

  it('skips matches inside fenced code blocks', () => {
    const body = '```ts\nimport x from "src/foo.ts"\n```\n\nProse only.'
    expect(citeOrMarkInferred(atom(body), ctx)).toEqual([])
  })

  it('skips matches inside inline backtick spans', () => {
    expect(citeOrMarkInferred(atom('Refer to `src/foo.ts` only as example.'), ctx)).toEqual([])
  })

  it('caps warnings at 5 to avoid spam', () => {
    const body = Array.from({ length: 20 }, (_, i) => `Edit src/file${i}.ts please.`).join('\n')
    const errs = citeOrMarkInferred(atom(body), ctx)
    expect(errs.length).toBeLessThanOrEqual(5)
  })

  it('matches src/foo.ts:functionName form', () => {
    const errs = citeOrMarkInferred(atom('See src/api/route.ts:handleRequest.'), ctx)
    expect(errs).toHaveLength(1)
    expect(errs[0]!.offending).toMatch(/handleRequest/)
  })
})
