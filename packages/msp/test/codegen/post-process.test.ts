import { describe, expect, it } from 'vitest'

import {
  normaliseLineEndings,
  postProcess,
  stripLeadingCommentary,
  stripMarkdownFences,
  stripTrailingCommentary,
} from '../../src/codegen/post-process.js'

describe('stripMarkdownFences', () => {
  it('removes ```ts ... ```', () => {
    const out = stripMarkdownFences('```ts\nexport const x = 1\n```')
    expect(out).toBe('export const x = 1')
  })

  it('removes ``` (no language)', () => {
    const out = stripMarkdownFences('```\nfoo\n```')
    expect(out).toBe('foo')
  })

  it('handles tilde fences', () => {
    const out = stripMarkdownFences('~~~js\nbar\n~~~')
    expect(out).toBe('bar')
  })

  it('leaves code untouched if no opening fence', () => {
    const out = stripMarkdownFences('export const x = 1')
    expect(out).toBe('export const x = 1')
  })
})

describe('stripLeadingCommentary', () => {
  it('drops "Sure, here is the code:" preamble', () => {
    const out = stripLeadingCommentary('Sure, here is the code:\n\nexport const x = 1')
    expect(out.trim()).toBe('export const x = 1')
  })

  it('preserves leading comments (// ...)', () => {
    const out = stripLeadingCommentary('// header comment\nexport const x = 1')
    expect(out.startsWith('// header comment')).toBe(true)
  })
})

describe('stripTrailingCommentary', () => {
  it('drops trailing "Hope this helps" line', () => {
    const out = stripTrailingCommentary('export function foo() { return 1 }\n\nHope this helps!')
    expect(out.trim().endsWith('}')).toBe(true)
  })

  it('preserves code ending in semicolon', () => {
    const code = 'const x = 1;'
    expect(stripTrailingCommentary(code).trim()).toBe(code)
  })
})

describe('normaliseLineEndings', () => {
  it('converts CRLF to LF', () => {
    expect(normaliseLineEndings('a\r\nb\r\nc')).toBe('a\nb\nc')
  })
})

describe('postProcess (full pipeline)', () => {
  it('strips fence + preamble + trailing comment', () => {
    const raw = `Sure, here you go:

\`\`\`ts
export const handler = async () => 42
\`\`\`

Let me know if you need anything else.`
    const out = postProcess(raw)
    expect(out.trim()).toBe('export const handler = async () => 42')
  })

  it('produces output ending with a single newline', () => {
    const out = postProcess('export const x = 1')
    expect(out.endsWith('\n')).toBe(true)
    expect(out.endsWith('\n\n')).toBe(false)
  })
})
