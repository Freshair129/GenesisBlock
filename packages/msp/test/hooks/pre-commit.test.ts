import { spawn, spawnSync } from 'node:child_process'
import { copyFile, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { afterAll, beforeAll, describe, expect, it } from 'vitest'

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))
const hookSrc = join(repoRoot, 'examples/hooks/pre-commit-validator.sh')

const VALID = `---
id: CONCEPT--TEST-VALID-HOOK
phase: 1
type: concept
status: stable
vault_id: TEST
title: Valid hook fixture
tags: [test]
created_at: 2026-04-01T00:00:00Z
---

# Valid

Hook smoke test fixture.
`

const FORBIDDEN = `---
id: CONCEPT--TEST-FORBIDDEN-HOOK
phase: 1
type: concept
status: stable
vault_id: TEST
title: Atom with forbidden field
commit_hash: deadbeef
created_at: 2026-04-01T00:00:00Z
---

# Forbidden

Should be rejected.
`

interface RunResult {
  code: number
  stdout: string
  stderr: string
}

function run(cmd: string, args: string[], cwd: string, env?: Record<string, string>): RunResult {
  const r = spawnSync(cmd, args, {
    cwd,
    env: { ...process.env, ...env, NO_COLOR: '1' },
    encoding: 'utf8',
  })
  return { code: r.status ?? -1, stdout: r.stdout ?? '', stderr: r.stderr ?? '' }
}

async function makeFixtureRepo(): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), 'msp-precommit-'))

  // Init git repo + minimal identity.
  run('git', ['init', '--initial-branch=main'], dir)
  run('git', ['config', 'user.email', 'test@msp.local'], dir)
  run('git', ['config', 'user.name', 'msp-test'], dir)
  run('git', ['config', 'commit.gpgsign', 'false'], dir)

  // Symlink the real validator infra into the fixture so the hook can find npm + node_modules.
  // Simpler: make the fixture call into the real repo via a wrapper package.json.
  await writeFile(
    join(dir, 'package.json'),
    JSON.stringify(
      {
        name: 'msp-hook-fixture',
        private: true,
        scripts: {
          'msp:validate': `tsx ${join(repoRoot, 'src/validator/cli.ts')} --root=${repoRoot}`,
        },
      },
      null,
      2,
    ),
  )

  // Bring in node_modules (validator + tsx) by symlinking to the real one.
  // On systems where symlinks are restricted, fall back to a copy. For CI we keep symlinks.
  run('ln', ['-s', join(repoRoot, 'node_modules'), join(dir, 'node_modules')], dir)

  // Install our hook.
  await mkdir(join(dir, '.git/hooks'), { recursive: true })
  await copyFile(hookSrc, join(dir, '.git/hooks/pre-commit'))
  await new Promise<void>((resolve, reject) => {
    const c = spawn('chmod', ['+x', join(dir, '.git/hooks/pre-commit')])
    c.on('close', (code) => (code === 0 ? resolve() : reject(new Error(`chmod exit ${code}`))))
  })

  // Make the gks/concept dir so committed atoms have a valid path.
  await mkdir(join(dir, 'gks/concept'), { recursive: true })

  return dir
}

let repo: string

beforeAll(async () => {
  repo = await makeFixtureRepo()
}, 60_000)

afterAll(async () => {
  // Best-effort cleanup; the temp dir is small and OS will GC.
})

describe('pre-commit hook', () => {
  it('exits 0 when no atom files are staged', () => {
    const r = run('git', ['commit', '--allow-empty', '-m', 'empty'], repo)
    expect(r.code).toBe(0)
  }, 30_000)

  it('exits 0 on a valid atom commit', async () => {
    const path = 'gks/concept/CONCEPT--TEST-VALID-HOOK.md'
    await writeFile(join(repo, path), VALID)
    run('git', ['add', path], repo)
    const r = run('git', ['commit', '-m', 'add valid'], repo)
    expect(r.code).toBe(0)
    expect(r.stdout + r.stderr).toMatch(/MSP validator: 1 file\(s\) passed/)
  }, 60_000)

  it('exits 1 on a forbidden-field atom commit', async () => {
    const path = 'gks/concept/CONCEPT--TEST-FORBIDDEN-HOOK.md'
    await writeFile(join(repo, path), FORBIDDEN)
    run('git', ['add', path], repo)
    const r = run('git', ['commit', '-m', 'add forbidden'], repo)
    expect(r.code).not.toBe(0)
    expect(r.stdout + r.stderr).toMatch(/\[forbidden-fields\]/)
  }, 60_000)

  it('--no-verify bypasses the hook even on a bad atom', async () => {
    // The previous test left the bad atom staged.
    const r = run('git', ['commit', '--no-verify', '-m', 'force'], repo)
    expect(r.code).toBe(0)
  }, 30_000)

  it('hotfix gate: src/ commit allowed when no overdue HOTFIX exists', async () => {
    // Stage a non-gks/, non-.brain/ file. With no HOTFIX-- atom, the hook should pass.
    await mkdir(join(repo, 'src'), { recursive: true })
    await writeFile(join(repo, 'src/foo.ts'), 'export const x = 1\n')
    run('git', ['add', 'src/foo.ts'], repo)
    const r = run('git', ['commit', '-m', 'add src/foo'], repo)
    expect(r.code).toBe(0)
  }, 30_000)

  it('hotfix gate: src/ commit blocked when an overdue HOTFIX references the file', async () => {
    // Plant an overdue HOTFIX atom referencing src/foo.ts.
    const hotfixDir = join(repo, 'gks/hotfix')
    await mkdir(hotfixDir, { recursive: true })
    const overdue = `---
id: HOTFIX--abc1234
phase: 5
type: hotfix
status: stable
title: test overdue
created_at: 2024-01-01T00:00:00Z
valid_from: 2024-01-01T00:00:00Z
valid_to: 2024-01-03T00:00:00Z
linked_symbols:
  - {"file":"src/foo.ts"}
meta: {"commit_sha":"deadbeef","reason":"test"}
---

# HOTFIX
body
`
    await writeFile(join(hotfixDir, 'HOTFIX--abc1234.md'), overdue)
    // Edit src/foo.ts so the hook stages it for hotfix check
    await writeFile(join(repo, 'src/foo.ts'), 'export const x = 2\n')
    run('git', ['add', 'src/foo.ts', 'gks/hotfix/HOTFIX--abc1234.md'], repo)
    const r = run('git', ['commit', '-m', 'tweak src/foo'], repo)
    expect(r.code).not.toBe(0)
    expect(r.stdout + r.stderr).toMatch(/MSP hotfix check failed|hotfix gate/)
    // Cleanup: remove the planted hotfix so subsequent tests aren't affected
    run('git', ['reset', '--', 'src/foo.ts', 'gks/hotfix/HOTFIX--abc1234.md'], repo)
    await rm(join(hotfixDir, 'HOTFIX--abc1234.md'), { force: true })
  }, 30_000)
})
