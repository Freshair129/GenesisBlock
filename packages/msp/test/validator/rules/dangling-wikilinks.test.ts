import { describe, expect, it } from 'vitest'

import { danglingWikilinks } from '../../../src/validator/rules/dangling-wikilinks.js'
import type {
  AtomicIndexEntry,
  ParsedAtom,
  ValidationContext,
} from '../../../src/validator/types.js'

function entry(id: string): AtomicIndexEntry {
  return { id, type: 'feat', status: 'stable', path: `feat/${id}.md` }
}

function ctxWith(ids: string[]): ValidationContext {
  return { atomicIndex: new Map(ids.map((id) => [id, entry(id)])) }
}

function atom(body: string, fm: Record<string, unknown> = {}): ParsedAtom {
  return { body, fm, source: '', filepath: 'x.md' }
}

describe('danglingWikilinks', () => {
  it('returns empty when all body links resolve', () => {
    const ctx = ctxWith(['FEAT--FOO', 'CONCEPT--BAR'])
    const errs = danglingWikilinks(
      atom('see [[FEAT--FOO]] and [[CONCEPT--BAR]]'),
      ctx,
    )
    expect(errs).toEqual([])
  })

  it('flags an unresolved body wikilink', () => {
    const ctx = ctxWith([])
    const errs = danglingWikilinks(atom('points to [[FEAT--GHOST]]'), ctx)
    expect(errs).toHaveLength(1)
    expect(errs[0]!.rule).toBe('dangling-wikilink')
    expect(errs[0]!.offending).toBe('FEAT--GHOST')
  })

  it('flags an unresolved crosslinks.references entry', () => {
    const ctx = ctxWith([])
    const errs = danglingWikilinks(
      atom('', { crosslinks: { references: ['GHOST--ATOM'] } }),
      ctx,
    )
    expect(errs).toHaveLength(1)
    expect(errs[0]!.offending).toBe('GHOST--ATOM')
  })

  it('skips wikilinks inside fenced code blocks', () => {
    const ctx = ctxWith([])
    const errs = danglingWikilinks(
      atom('```ts\nlink = `[[FEAT--HIDDEN]]`\n```'),
      ctx,
    )
    expect(errs).toEqual([])
  })

  it('allows self-reference even when not yet in the index', () => {
    const ctx = ctxWith([])
    const errs = danglingWikilinks(
      atom('I am [[FEAT--SELF]]', { proposed_id: 'FEAT--SELF' }),
      ctx,
    )
    expect(errs).toEqual([])
  })
})
