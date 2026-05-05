import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import predicate from '../../../src/validator/proto/phase-gates.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-proto-phase-gates-'))
}

/**
 * Write an atom file under `<root>/gks/<relPath>`.
 * Returns the relPath for putting back into AtomicIndexEntry.path.
 */
async function writeAtom(
  root: string,
  relPath: string,
  frontmatter: Record<string, unknown>,
  body = '# body',
): Promise<string> {
  const abs = join(root, 'gks', relPath)
  await mkdir(join(abs, '..'), { recursive: true })
  // serialize frontmatter as YAML-like JSON (yaml accepts JSON)
  const fmYaml = Object.entries(frontmatter)
    .map(([k, v]) => `${k}: ${JSON.stringify(v)}`)
    .join('\n')
  await writeFile(abs, `---\n${fmYaml}\n---\n\n${body}\n`)
  return relPath
}

function entry(partial: Partial<AtomicIndexEntry>): AtomicIndexEntry {
  return {
    id: partial.id ?? 'X--Y',
    type: partial.type ?? 'feat',
    status: partial.status ?? 'stable',
    path: partial.path ?? 'feat/X--Y.md',
    phase: partial.phase,
    vault_id: partial.vault_id,
    title: partial.title,
    tags: partial.tags,
    crosslinks: partial.crosslinks,
    linked_symbols: partial.linked_symbols,
    geography: partial.geography,
  } as AtomicIndexEntry
}

describe('PROTO--PHASE-GATES predicate', () => {
  it('passes vacuously on an empty atomicIndex', async () => {
    const result = await predicate({ atomicIndex: [], repoRoot: '/tmp' })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('passes vacuously when there are no phase-5/6 code-writing atoms', async () => {
    const idx: AtomicIndexEntry[] = [
      entry({ id: 'CONCEPT--FOO', type: 'concept', phase: 1, path: 'concept/CONCEPT--FOO.md' }),
      entry({ id: 'ADR--FOO', type: 'adr', phase: 2, path: 'adr/ADR--FOO.md', crosslinks: { references: ['CONCEPT--FOO'] } }),
      entry({ id: 'FEAT--FOO', type: 'feat', phase: 2, path: 'feat/FEAT--FOO.md' }),
    ]
    const result = await predicate({ atomicIndex: idx, repoRoot: '/tmp' })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('passes when every phase-5/6 code-writer has a backing phase-3 BLUEPRINT covering ≥1 file', async () => {
    const idx: AtomicIndexEntry[] = [
      entry({
        id: 'BLUEPRINT--FOO',
        type: 'blueprint',
        phase: 3,
        path: 'blueprint/BLUEPRINT--FOO.md',
        linked_symbols: [{ file: 'src/foo/index.ts' }, { file: 'src/foo/types.ts' }],
      }),
      entry({
        id: 'AUDIT--FOO',
        type: 'audit',
        phase: 6,
        path: 'audit/AUDIT--FOO.md',
        linked_symbols: [{ file: 'src/foo/index.ts' }, { file: 'test/foo/index.test.ts' }],
      }),
      entry({
        id: 'CONCEPT--FOO',
        type: 'concept',
        phase: 1,
        path: 'concept/CONCEPT--FOO.md',
      }),
      entry({
        id: 'ADR--FOO',
        type: 'adr',
        phase: 2,
        path: 'adr/ADR--FOO.md',
        crosslinks: { references: ['CONCEPT--FOO'] },
      }),
    ]
    const result = await predicate({ atomicIndex: idx, repoRoot: '/tmp' })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('emits a hard error when a phase-5 FEAT writes code but no BLUEPRINT covers any file', async () => {
    const root = await freshRoot()
    const path = await writeAtom(root, 'feat/FEAT--ORPHAN.md', {
      id: 'FEAT--ORPHAN',
      phase: 5,
      type: 'feat',
      status: 'stable',
    })
    const idx: AtomicIndexEntry[] = [
      entry({
        id: 'FEAT--ORPHAN',
        type: 'feat',
        phase: 5,
        path,
        linked_symbols: [{ file: 'src/orphan/index.ts' }],
      }),
    ]
    const result = await predicate({ atomicIndex: idx, repoRoot: root })
    expect(result.ok).toBe(false)
    const errors = result.violations.filter((v) => v.severity === 'error')
    expect(errors).toHaveLength(1)
    expect(errors[0]!.atomId).toBe('FEAT--ORPHAN')
    expect(errors[0]!.message).toMatch(/no phase-3 BLUEPRINT/)
  })

  it('honours phase_override.skip_blueprint: true (no error emitted)', async () => {
    const root = await freshRoot()
    const path = await writeAtom(root, 'audit/AUDIT--HOOK.md', {
      id: 'AUDIT--HOOK',
      phase: 6,
      type: 'audit',
      status: 'stable',
      phase_override: { skip_blueprint: true, reason: 'examples/hooks/ scripts' },
    })
    const idx: AtomicIndexEntry[] = [
      entry({
        id: 'AUDIT--HOOK',
        type: 'audit',
        phase: 6,
        path,
        linked_symbols: [{ file: 'examples/hooks/pre-commit-validator.sh' }],
      }),
    ]
    const result = await predicate({ atomicIndex: idx, repoRoot: root })
    const errors = result.violations.filter((v) => v.severity === 'error')
    expect(errors).toEqual([])
  })

  it('emits a soft warning when an ADR has no CONCEPT-- referenced', async () => {
    const idx: AtomicIndexEntry[] = [
      entry({
        id: 'ADR--LONELY',
        type: 'adr',
        phase: 2,
        path: 'adr/ADR--LONELY.md',
        crosslinks: { references: ['ADR--SOMETHING-ELSE'] },
      }),
    ]
    const result = await predicate({ atomicIndex: idx, repoRoot: '/tmp' })
    const warnings = result.violations.filter((v) => v.severity === 'warning')
    expect(warnings).toHaveLength(1)
    expect(warnings[0]!.atomId).toBe('ADR--LONELY')
    expect(warnings[0]!.message).toMatch(/no CONCEPT-- referenced/)
  })

  it('does not emit a soft warning when ADR references a CONCEPT', async () => {
    const idx: AtomicIndexEntry[] = [
      entry({
        id: 'ADR--GOOD',
        type: 'adr',
        phase: 2,
        path: 'adr/ADR--GOOD.md',
        crosslinks: { references: ['CONCEPT--BAR', 'ADR--OTHER'] },
      }),
    ]
    const result = await predicate({ atomicIndex: idx, repoRoot: '/tmp' })
    const warnings = result.violations.filter((v) => v.severity === 'warning')
    expect(warnings).toEqual([])
  })

  it('ignores phase-5/6 atoms that have no linked_symbols (nothing to gate)', async () => {
    const idx: AtomicIndexEntry[] = [
      entry({
        id: 'AUDIT--PROSE-ONLY',
        type: 'audit',
        phase: 6,
        path: 'audit/AUDIT--PROSE-ONLY.md',
        // no linked_symbols
      }),
    ]
    const result = await predicate({ atomicIndex: idx, repoRoot: '/tmp' })
    const errors = result.violations.filter((v) => v.severity === 'error')
    expect(errors).toEqual([])
  })

  it('flags multiple violators independently', async () => {
    const root = await freshRoot()
    const p1 = await writeAtom(root, 'feat/FEAT--A.md', {
      id: 'FEAT--A',
      phase: 5,
      type: 'feat',
      status: 'stable',
    })
    const p2 = await writeAtom(root, 'audit/AUDIT--B.md', {
      id: 'AUDIT--B',
      phase: 6,
      type: 'audit',
      status: 'stable',
    })
    const idx: AtomicIndexEntry[] = [
      entry({
        id: 'FEAT--A',
        type: 'feat',
        phase: 5,
        path: p1,
        linked_symbols: [{ file: 'src/a.ts' }],
      }),
      entry({
        id: 'AUDIT--B',
        type: 'audit',
        phase: 6,
        path: p2,
        linked_symbols: [{ file: 'src/b.ts' }],
      }),
    ]
    const result = await predicate({ atomicIndex: idx, repoRoot: root })
    const errors = result.violations.filter((v) => v.severity === 'error')
    expect(errors).toHaveLength(2)
    const ids = new Set(errors.map((e) => e.atomId))
    expect(ids).toEqual(new Set(['FEAT--A', 'AUDIT--B']))
  })
})
