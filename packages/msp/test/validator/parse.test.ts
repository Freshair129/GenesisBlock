import { describe, expect, it } from 'vitest'

import { extractWikilinks, parseSource } from '../../src/validator/parse.js'
import { ValidatorIOError } from '../../src/validator/types.js'

describe('parseSource', () => {
  it('extracts frontmatter object and body', () => {
    const src = '---\nid: CONCEPT--FOO\nphase: 1\n---\n# Body\n\ntext\n'
    const atom = parseSource(src, 'fixture.md')
    expect(atom.fm.id).toBe('CONCEPT--FOO')
    expect(atom.fm.phase).toBe(1)
    expect(atom.body).toBe('# Body\n\ntext\n')
  })

  it('throws on missing leading delimiter', () => {
    expect(() => parseSource('# no frontmatter\n', 'x.md')).toThrow(ValidatorIOError)
  })

  it('throws on missing closing delimiter', () => {
    expect(() => parseSource('---\nid: X\nno close', 'x.md')).toThrow(ValidatorIOError)
  })

  it('throws on malformed YAML', () => {
    expect(() => parseSource('---\nid: : :\n---\nbody', 'x.md')).toThrow(ValidatorIOError)
  })
})

describe('extractWikilinks', () => {
  it('finds simple wikilinks', () => {
    const links = extractWikilinks('see [[FEAT--FOO]] and [[CONCEPT--BAR]]\n')
    expect(links).toHaveLength(2)
    expect(links[0]!.id).toBe('FEAT--FOO')
    expect(links[0]!.line).toBe(1)
    expect(links[1]!.id).toBe('CONCEPT--BAR')
  })

  it('skips wikilinks inside fenced code blocks', () => {
    const body = '```ts\n[[FEAT--HIDDEN]]\n```\n[[FEAT--VISIBLE]]\n'
    const links = extractWikilinks(body)
    expect(links).toHaveLength(1)
    expect(links[0]!.id).toBe('FEAT--VISIBLE')
  })

  it('skips wikilinks inside inline code spans', () => {
    const body = 'use `[[FEAT--EXAMPLE]]` to reference real [[FEAT--FOO]]\n'
    const links = extractWikilinks(body)
    expect(links).toHaveLength(1)
    expect(links[0]!.id).toBe('FEAT--FOO')
  })

  it('handles multiple wikilinks per line with correct columns', () => {
    const body = '[[A--ONE]] then [[B--TWO]]\n'
    const links = extractWikilinks(body)
    expect(links).toHaveLength(2)
    expect(links[0]!.column).toBe(1)
    expect(links[1]!.column).toBeGreaterThan(10)
  })
})
