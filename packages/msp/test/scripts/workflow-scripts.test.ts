/**
 * Tests for msp:atom-date / msp:scaffold-atom / msp:supersede workflow scripts.
 *
 * Strategy: spawn each script via tsx and inspect stdout / filesystem effects.
 * Uses tmpdir fixtures + workspace node_modules walk-up (per the established
 * pre-push.test.ts pattern, since spawned tsx needs node_modules to resolve).
 */
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

const repoRoot = fileURLToPath(new URL('../../../..', import.meta.url))
const atomDateScript = join(repoRoot, 'scripts/msp/atom-date.ts')
const scaffoldScript = join(repoRoot, 'scripts/msp/msp-atom.ts')
const supersedeScript = join(repoRoot, 'scripts/msp/supersede.ts')

function runScript(scriptPath: string, args: string[]): { code: number; stdout: string; stderr: string } {
  const r = spawnSync('npx', ['tsx', scriptPath, ...args], {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  })
  return { code: r.status ?? -1, stdout: r.stdout ?? '', stderr: r.stderr ?? '' }
}

describe('msp:atom-date', () => {
  it('prints an ISO 8601 string with +07:00 offset by default', () => {
    const r = runScript(atomDateScript, [])
    expect(r.code).toBe(0)
    expect(r.stdout.trim()).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}\+07:00$/)
  })

  it('prints UTC absolute (Z suffix) with --utc flag', () => {
    const r = runScript(atomDateScript, ['--utc'])
    expect(r.code).toBe(0)
    expect(r.stdout.trim()).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/)
  })

  it('default and --utc represent the same UTC instant (within tsx spawn overhead)', () => {
    const a = runScript(atomDateScript, []).stdout.trim()
    const b = runScript(atomDateScript, ['--utc']).stdout.trim()
    const aMs = Date.parse(a)
    const bMs = Date.parse(b)
    // 5s tolerance accounts for tsx + spawn cold start on slow CI runners
    expect(Math.abs(aMs - bMs)).toBeLessThan(5000)
  })
})

describe('msp:scaffold-atom', () => {
  let tmpRoot: string

  beforeEach(() => {
    tmpRoot = mkdtempSync(join(tmpdir(), 'msp-scaffold-'))
    // Mock the minimal gks/ tree the script writes into
    mkdirSync(join(tmpRoot, 'gks/concept'), { recursive: true })
    mkdirSync(join(tmpRoot, 'gks/adr'), { recursive: true })
    // Mock minimal atom_registry.yaml
    writeFileSync(
      join(tmpRoot, 'atom_registry.yaml'),
      `
taxonomy:
  clusters:
    process:
      types:
        concept:
          phase: 1
          folder: concept
          tier: process
          sections: [Problem, Hypothesis, Scope]
`,
    )
  })

  afterEach(() => {
    rmSync(tmpRoot, { recursive: true, force: true })
  })

  it('creates a CONCEPT atom with valid frontmatter', () => {
    const r = runScript(scaffoldScript, ['scaffold', `--type=concept`, `--slug=TEST-FOO`, `--root=${tmpRoot}`])
    expect(r.code).toBe(0)
    const path = join(tmpRoot, 'gks/concept/CONCEPT--TEST-FOO.md')
    expect(existsSync(path)).toBe(true)
    const content = readFileSync(path, 'utf8')
    expect(content).toMatch(/^id: CONCEPT--TEST-FOO$/m)
    expect(content).toMatch(/^phase: 1$/m)
    expect(content).toMatch(/^type: concept$/m)
    expect(content).toMatch(/^status: draft$/m)
    expect(content).toMatch(/^created_at: \d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}\+07:00$/m)
  })

  it('refuses to overwrite an existing file without --force', () => {
    const path = join(tmpRoot, 'gks/concept/CONCEPT--EXIST.md')
    writeFileSync(path, 'existing content')
    const r = runScript(scaffoldScript, ['scaffold', `--type=concept`, `--slug=EXIST`, `--root=${tmpRoot}`])
    expect(r.code).not.toBe(0)
    expect(r.stderr).toMatch(/already exists/)
    // Original content preserved
    expect(readFileSync(path, 'utf8')).toBe('existing content')
  })

  it('rejects invalid slug', () => {
    const r = runScript(scaffoldScript, ['scaffold', `--type=concept`, `--slug=lowercase-bad`, `--root=${tmpRoot}`])
    expect(r.code).not.toBe(0)
    expect(r.stderr).toMatch(/invalid slug/)
  })

  it('rejects unknown type', () => {
    const r = runScript(scaffoldScript, ['scaffold', `--type=notatype`, `--slug=FOO`, `--root=${tmpRoot}`])
    expect(r.code).not.toBe(0)
    expect(r.stderr).toMatch(/unknown --type/)
  })
})

describe('msp:supersede', () => {
  let tmpRoot: string

  beforeEach(() => {
    tmpRoot = mkdtempSync(join(tmpdir(), 'msp-supersede-'))
    mkdirSync(join(tmpRoot, 'gks/concept'), { recursive: true })
    mkdirSync(join(tmpRoot, 'gks/adr'), { recursive: true })
    mkdirSync(join(tmpRoot, 'gks/00_index'), { recursive: true })

    // Atomic index with 2 atoms
    writeFileSync(
      join(tmpRoot, 'gks/00_index/atomic_index.jsonl'),
      [
        JSON.stringify({ id: 'CONCEPT--OLD', path: 'concept\\CONCEPT--OLD.md' }),
        JSON.stringify({ id: 'CONCEPT--NEW', path: 'concept\\CONCEPT--NEW.md' }),
      ].join('\n') + '\n',
    )

    writeFileSync(
      join(tmpRoot, 'gks/concept/CONCEPT--OLD.md'),
      `---
id: CONCEPT--OLD
phase: 1
type: concept
status: active
tier: process
source_type: axiomatic
vault_id: default
title: old atom
crosslinks: {"references":["FRAME--FOO"]}
created_at: 2026-05-01T00:00:00.000+07:00
---

# CONCEPT — old
`,
    )

    writeFileSync(
      join(tmpRoot, 'gks/concept/CONCEPT--NEW.md'),
      `---
id: CONCEPT--NEW
phase: 1
type: concept
status: active
tier: process
source_type: axiomatic
vault_id: default
title: new atom
crosslinks: {}
created_at: 2026-05-12T00:00:00.000+07:00
---

# CONCEPT — new
`,
    )
  })

  afterEach(() => {
    rmSync(tmpRoot, { recursive: true, force: true })
  })

  it('flips old status to superseded + adds reciprocal crosslinks', () => {
    const r = runScript(supersedeScript, [
      `--old=CONCEPT--OLD`,
      `--new=CONCEPT--NEW`,
      `--root=${tmpRoot}`,
    ])
    expect(r.code).toBe(0)

    const oldContent = readFileSync(join(tmpRoot, 'gks/concept/CONCEPT--OLD.md'), 'utf8')
    expect(oldContent).toMatch(/^status: superseded$/m)
    expect(oldContent).toMatch(/"superseded_by":\["CONCEPT--NEW"\]/)
    // existing crosslinks preserved
    expect(oldContent).toMatch(/"references":\["FRAME--FOO"\]/)

    const newContent = readFileSync(join(tmpRoot, 'gks/concept/CONCEPT--NEW.md'), 'utf8')
    expect(newContent).toMatch(/"supersedes":\["CONCEPT--OLD"\]/)
  })

  it('refuses if old atom is already superseded', () => {
    // First call succeeds
    runScript(supersedeScript, [`--old=CONCEPT--OLD`, `--new=CONCEPT--NEW`, `--root=${tmpRoot}`])
    // Second call must refuse
    const r = runScript(supersedeScript, [`--old=CONCEPT--OLD`, `--new=CONCEPT--NEW`, `--root=${tmpRoot}`])
    expect(r.code).not.toBe(0)
    expect(r.stderr).toMatch(/already status: superseded/)
  })

  it('refuses if any new atom not in index', () => {
    const r = runScript(supersedeScript, [
      `--old=CONCEPT--OLD`,
      `--new=CONCEPT--MISSING`,
      `--root=${tmpRoot}`,
    ])
    expect(r.code).not.toBe(0)
    expect(r.stderr).toMatch(/not found in index/)
  })
})
