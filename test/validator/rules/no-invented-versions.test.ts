import { describe, expect, it } from 'vitest'

import { noInventedVersions } from '../../../src/validator/rules/no-invented-versions.js'
import type {
  AtomicIndexEntry,
  ParsedAtom,
  ValidationContext,
} from '../../../src/validator/types.js'

function ctx(idsInIndex: string[] = []): ValidationContext {
  const m = new Map<string, AtomicIndexEntry>()
  for (const id of idsInIndex) {
    m.set(id, { id, type: 'feat', status: 'stable', path: '' })
  }
  return { atomicIndex: m }
}

function atom(fm: Record<string, unknown>): ParsedAtom {
  return { fm, body: '', source: '', filepath: 'x.md' }
}

describe('noInventedVersions', () => {
  it('no-op for types that do not require version (e.g. concept)', () => {
    expect(noInventedVersions(atom({ type: 'concept', version: '99.99.99' }), ctx())).toEqual([])
  })

  it('no-op for module/feat/protocol when version is missing (handled by required-fields rule)', () => {
    expect(noInventedVersions(atom({ type: 'module' }), ctx())).toEqual([])
  })

  it('rejects non-semver strings', () => {
    const errs = noInventedVersions(atom({ type: 'module', version: '1.5.2-beta' }), ctx())
    expect(errs).toHaveLength(1)
    expect(errs[0]!.rule).toBe('no-invented-versions')
  })

  it('rejects "latest" or "v1.0"', () => {
    expect(noInventedVersions(atom({ type: 'module', version: 'latest' }), ctx())).toHaveLength(1)
    expect(noInventedVersions(atom({ type: 'module', version: 'v1.0' }), ctx())).toHaveLength(1)
  })

  it('rejects non-string version', () => {
    const errs = noInventedVersions(atom({ type: 'module', version: 1 }), ctx())
    expect(errs).toHaveLength(1)
  })

  it('passes 0.1.0 for a new atom', () => {
    expect(
      noInventedVersions(atom({ type: 'module', id: 'MOD--NEW', version: '0.1.0' }), ctx()),
    ).toEqual([])
  })

  it('rejects a new atom starting at 1.0.0 or higher', () => {
    const errs = noInventedVersions(
      atom({ type: 'module', id: 'MOD--NEW', version: '1.0.0' }),
      ctx(),
    )
    expect(errs).toHaveLength(1)
    expect(errs[0]!.message).toMatch(/0\.1\.0/)
  })

  it('passes 2.5.3 for an existing atom (already in index)', () => {
    expect(
      noInventedVersions(
        atom({ type: 'module', id: 'MOD--EXISTING', version: '2.5.3' }),
        ctx(['MOD--EXISTING']),
      ),
    ).toEqual([])
  })
})
