import { describe, expect, it } from 'vitest'

import { parseRootFromArgv } from '../../src/mcp/argv.js'

describe('parseRootFromArgv', () => {
  it('returns undefined when --root is absent', () => {
    expect(parseRootFromArgv([])).toBeUndefined()
    expect(parseRootFromArgv(['--something-else=foo'])).toBeUndefined()
    expect(parseRootFromArgv(['some', 'positional', 'args'])).toBeUndefined()
  })

  it('parses --root=<path> form', () => {
    expect(parseRootFromArgv(['--root=G:\\msp'])).toBe('G:\\msp')
    expect(parseRootFromArgv(['--root=/home/user/repo'])).toBe('/home/user/repo')
  })

  it('parses --root <path> (space-separated) form', () => {
    expect(parseRootFromArgv(['--root', 'G:\\msp'])).toBe('G:\\msp')
    expect(parseRootFromArgv(['--root', '/home/user/repo'])).toBe('/home/user/repo')
  })

  it('treats empty value as missing (not empty string)', () => {
    // --root= with nothing after → undefined
    expect(parseRootFromArgv(['--root='])).toBeUndefined()
    // --root with empty next arg → undefined
    expect(parseRootFromArgv(['--root', ''])).toBeUndefined()
  })

  it('returns the first occurrence (does not let later flags override)', () => {
    // CLI norm: first one wins. Anyone passing --root twice probably has a bug.
    expect(parseRootFromArgv(['--root=/first', '--root=/second'])).toBe('/first')
  })

  it('handles --root mixed with other args', () => {
    expect(parseRootFromArgv(['--verbose', '--root=/x', '--quiet'])).toBe('/x')
    expect(parseRootFromArgv(['--verbose', '--root', '/x'])).toBe('/x')
  })

  it('does not match --rootless or other --root prefixes', () => {
    expect(parseRootFromArgv(['--rootless'])).toBeUndefined()
    expect(parseRootFromArgv(['--root-fake=x'])).toBeUndefined()
  })

  it('handles trailing --root with no value gracefully', () => {
    // --root at end of argv with no value — should not crash, returns undefined.
    expect(parseRootFromArgv(['--root'])).toBeUndefined()
  })
})
