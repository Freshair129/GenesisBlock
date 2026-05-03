import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { rebuildBacklinks } from '../../../src/memory/backlinks/indexer.js'

async function makeRoot(): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), 'msp-backlinks-'))
  await mkdir(join(dir, 'gks/concept'), { recursive: true })
  await mkdir(join(dir, 'gks/adr'), { recursive: true })
  await mkdir(join(dir, 'gks/feat'), { recursive: true })
  return dir
}

const ATOM_C = `---
id: CONCEPT--FOO
type: concept
status: stable
phase: 1
vault_id: TEST
---
body
`

const ATOM_A = `---
id: ADR--FOO
type: adr
status: stable
phase: 2
vault_id: TEST
crosslinks:
  references: [CONCEPT--FOO]
---
body
`

const ATOM_F = `---
id: FEAT--FOO
type: feat
status: stable
phase: 2
vault_id: TEST
crosslinks:
  implements: [ADR--FOO]
  references: [CONCEPT--FOO]
---
body
`

describe('rebuildBacklinks', () => {
  it('walks gks/ and emits one edge per crosslinks entry', async () => {
    const root = await makeRoot()
    await writeFile(join(root, 'gks/concept/CONCEPT--FOO.md'), ATOM_C)
    await writeFile(join(root, 'gks/adr/ADR--FOO.md'), ATOM_A)
    await writeFile(join(root, 'gks/feat/FEAT--FOO.md'), ATOM_F)

    const r = await rebuildBacklinks({ root })
    expect(r.atomCount).toBe(3)
    expect(r.edgeCount).toBe(3)
    expect(r.changed).toBe(true)

    const out = await readFile(r.outputPath, 'utf8')
    const lines = out.trim().split('\n').map((l) => JSON.parse(l))
    expect(lines).toHaveLength(3)
    // Sorted by (from, to, type)
    expect(lines[0]).toEqual({ from: 'ADR--FOO', to: 'CONCEPT--FOO', type: 'references' })
    expect(lines[1]).toEqual({ from: 'FEAT--FOO', to: 'ADR--FOO', type: 'implements' })
    expect(lines[2]).toEqual({ from: 'FEAT--FOO', to: 'CONCEPT--FOO', type: 'references' })
  })

  it('produces byte-identical output across runs (idempotent)', async () => {
    const root = await makeRoot()
    await writeFile(join(root, 'gks/feat/FEAT--FOO.md'), ATOM_F)
    await writeFile(join(root, 'gks/adr/ADR--FOO.md'), ATOM_A)

    const a = await rebuildBacklinks({ root })
    const first = await readFile(a.outputPath, 'utf8')
    const b = await rebuildBacklinks({ root })
    expect(b.changed).toBe(false)
    const second = await readFile(b.outputPath, 'utf8')
    expect(second).toBe(first)
  })

  it('--check returns changed=true when file would differ', async () => {
    const root = await makeRoot()
    await writeFile(join(root, 'gks/feat/FEAT--FOO.md'), ATOM_F)

    // First build to populate the file.
    const built = await rebuildBacklinks({ root })

    // Then add another atom and run with --check; should report changed.
    await writeFile(join(root, 'gks/adr/ADR--FOO.md'), ATOM_A)
    const checked = await rebuildBacklinks({ root, check: true })
    expect(checked.changed).toBe(true)
    // --check must NOT write
    const onDisk = await readFile(built.outputPath, 'utf8')
    expect(onDisk.split('\n').filter(Boolean)).toHaveLength(2) // unchanged from first build
  })

  it('--dry-run does not write', async () => {
    const root = await makeRoot()
    await writeFile(join(root, 'gks/feat/FEAT--FOO.md'), ATOM_F)
    const r = await rebuildBacklinks({ root, dryRun: true })
    expect(r.edgeCount).toBeGreaterThan(0)
    await expect(readFile(r.outputPath)).rejects.toThrow()
  })

  it('handles empty gks/ gracefully (no file created)', async () => {
    const root = await makeRoot()
    const r = await rebuildBacklinks({ root })
    expect(r.atomCount).toBe(0)
    expect(r.edgeCount).toBe(0)
    expect(r.changed).toBe(false)
    // Empty input → no file write (idempotent with non-existent file).
    await expect(readFile(r.outputPath)).rejects.toThrow()
  })

  it('skips gks/00_index/', async () => {
    const root = await makeRoot()
    await mkdir(join(root, 'gks/00_index'), { recursive: true })
    await writeFile(join(root, 'gks/00_index/atomic_index.jsonl'), 'noise')
    await writeFile(join(root, 'gks/00_index/should-skip.md'), ATOM_F)
    await writeFile(join(root, 'gks/feat/FEAT--FOO.md'), ATOM_F)
    const r = await rebuildBacklinks({ root })
    // FEAT--FOO has 2 edges; should-skip would add 2 more if not skipped
    expect(r.edgeCount).toBe(2)
  })
})
