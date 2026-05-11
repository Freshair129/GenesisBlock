import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import predicate from '../../../src/validator/proto/authority-enforcement.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-proto-authority-'))
}

async function writeAuthority(root: string, body: string): Promise<void> {
  const dir = join(root, '.brain/msp')
  await mkdir(dir, { recursive: true })
  await writeFile(join(dir, 'authority.yaml'), body)
}

const stubAtomicIndex: AtomicIndexEntry[] = []

describe('PROTO--AUTHORITY-ENFORCEMENT predicate', () => {
  it('passes vacuously when authority.yaml is missing', async () => {
    const root = await freshRoot()
    const result = await predicate({
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('passes for a valid authority.yaml', async () => {
    const root = await freshRoot()
    await writeAuthority(
      root,
      [
        'tiers:',
        '  T1: ["agent-junior"]',
        '  T2: ["alice"]',
        '  T3: ["boss"]',
        'allowed_paths:',
        '  T1: [".brain/msp/projects/*/candidates/**"]',
        '  T2: [".brain/msp/projects/*/candidates/**", "gks/concept/**", "gks/feat/**"]',
        '  T3: [".brain/msp/projects/*/candidates/**", "**"]',
      ].join('\n'),
    )
    const result = await predicate({
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('emits an error violation for invalid YAML', async () => {
    const root = await freshRoot()
    await writeAuthority(root, 'tiers: [unclosed: bracket\n')
    const result = await predicate({
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(result.ok).toBe(false)
    expect(result.violations[0]!.severity).toBe('error')
    expect(result.violations[0]!.message).toMatch(/invalid YAML/)
  })

  it('flags a user appearing in multiple tiers', async () => {
    const root = await freshRoot()
    await writeAuthority(
      root,
      [
        'tiers:',
        '  T1: ["alice"]',
        '  T2: ["alice"]',
        '  T3: ["boss"]',
        'allowed_paths:',
        '  T1: [".brain/msp/projects/*/candidates/**"]',
        '  T2: [".brain/msp/projects/*/candidates/**"]',
        '  T3: [".brain/msp/projects/*/candidates/**", "**"]',
      ].join('\n'),
    )
    const result = await predicate({
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(result.ok).toBe(false)
    const msgs = result.violations.map((v) => v.message)
    expect(msgs.some((m) => /alice.*tiers\.T1.*tiers\.T2|tiers must be disjoint/.test(m))).toBe(true)
  })

  it('warns when a tier has empty allowed_paths', async () => {
    const root = await freshRoot()
    await writeAuthority(
      root,
      [
        'tiers:',
        '  T1: ["agent-junior"]',
        '  T2: ["alice"]',
        '  T3: ["boss"]',
        'allowed_paths:',
        '  T1: []',
        '  T2: [".brain/msp/projects/*/candidates/**", "gks/concept/**"]',
        '  T3: [".brain/msp/projects/*/candidates/**", "**"]',
      ].join('\n'),
    )
    const result = await predicate({
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    // Empty paths is a warning, not an error → ok stays true.
    expect(result.ok).toBe(true)
    const empty = result.violations.find((v) => /T1 is empty/.test(v.message))
    expect(empty).toBeDefined()
    expect(empty!.severity).toBe('warning')
  })

  it('flags a tier whose allowed_paths lacks an candidates entry', async () => {
    const root = await freshRoot()
    await writeAuthority(
      root,
      [
        'tiers:',
        '  T1: ["agent-junior"]',
        '  T2: ["alice"]',
        '  T3: ["boss"]',
        'allowed_paths:',
        '  T1: ["gks/concept/**"]',
        '  T2: [".brain/msp/projects/*/candidates/**", "gks/concept/**"]',
        '  T3: [".brain/msp/projects/*/candidates/**", "**"]',
      ].join('\n'),
    )
    const result = await predicate({
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(result.ok).toBe(false)
    expect(
      result.violations.some((v) => /allowed_paths\.T1.*candidates/.test(v.message)),
    ).toBe(true)
  })

  it('flags missing top-level mappings', async () => {
    const root = await freshRoot()
    // Just tiers, no allowed_paths
    await writeAuthority(
      root,
      'tiers:\n  T1: ["a"]\n  T2: ["b"]\n  T3: ["c"]\n',
    )
    const result = await predicate({
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(result.ok).toBe(false)
    expect(
      result.violations.some((v) => /allowed_paths/.test(v.message)),
    ).toBe(true)
  })

  it('flags non-string entries in tier members', async () => {
    const root = await freshRoot()
    await writeAuthority(
      root,
      [
        'tiers:',
        '  T1: ["agent-junior", 42]',
        '  T2: ["alice"]',
        '  T3: ["boss"]',
        'allowed_paths:',
        '  T1: [".brain/msp/projects/*/candidates/**"]',
        '  T2: [".brain/msp/projects/*/candidates/**"]',
        '  T3: [".brain/msp/projects/*/candidates/**", "**"]',
      ].join('\n'),
    )
    const result = await predicate({
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(result.ok).toBe(false)
    expect(
      result.violations.some((v) => /tiers\.T1\[1\].*non-empty string/.test(v.message)),
    ).toBe(true)
  })
})
