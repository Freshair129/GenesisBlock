import { spawn, spawnSync } from 'node:child_process'
import { copyFile, mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { afterAll, beforeAll, describe, expect, it } from 'vitest'

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))
const hookSrc = join(repoRoot, 'examples/hooks/pre-push-verify.sh')

interface RunResult {
  code: number
  stdout: string
  stderr: string
}

function run(cmd: string, args: string[], cwd: string, opts: { input?: string } = {}): RunResult {
  const r = spawnSync(cmd, args, {
    cwd,
    encoding: 'utf8',
    input: opts.input,
    env: { ...process.env, NO_COLOR: '1' },
  })
  return { code: r.status ?? -1, stdout: r.stdout ?? '', stderr: r.stderr ?? '' }
}

function chmodX(path: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const c = spawn('chmod', ['+x', path])
    c.on('close', (code) => (code === 0 ? resolve() : reject(new Error(`chmod ${code}`))))
  })
}

const STABLE_FEAT = `---
id: FEAT--TEST-OK
phase: 2
type: feat
status: stable
vault_id: TEST
title: Test FEAT (chain OK)
tags: [test]
crosslinks:
  references: [CONCEPT--TEST-OK]
created_at: 2026-04-01T00:00:00Z
---

# FEAT — chain OK

Body.
`

const STABLE_CONCEPT = `---
id: CONCEPT--TEST-OK
phase: 1
type: concept
status: stable
vault_id: TEST
title: Test CONCEPT
tags: [test]
created_at: 2026-04-01T00:00:00Z
---

# CONCEPT

Body.
`

const BROKEN_FEAT = `---
id: FEAT--TEST-BROKEN
phase: 2
type: feat
status: stable
vault_id: TEST
title: Test FEAT (chain broken)
tags: [test]
crosslinks:
  references: [CONCEPT--DOES-NOT-EXIST]
created_at: 2026-04-01T00:00:00Z
---

# FEAT — broken

Body.
`

let repoDir: string

beforeAll(async () => {
  repoDir = await mkdtemp(join(tmpdir(), 'msp-prepush-'))

  // Set up: bare repo "origin" + working repo
  const origin = join(repoDir, 'origin.git')
  const wt = join(repoDir, 'wt')
  await mkdir(origin, { recursive: true })
  await mkdir(wt, { recursive: true })
  run('git', ['init', '--bare', '--initial-branch=main'], origin)

  run('git', ['init', '--initial-branch=main'], wt)
  run('git', ['config', 'user.email', 'test@msp.local'], wt)
  run('git', ['config', 'user.name', 'msp-test'], wt)
  run('git', ['config', 'commit.gpgsign', 'false'], wt)
  run('git', ['remote', 'add', 'origin', origin], wt)

  // Make wt look like a real MSP repo for verify-flow
  // Symlink node_modules + atomic_index from the real repo so npx gks works
  await mkdir(join(wt, 'gks/00_index'), { recursive: true })
  await mkdir(join(wt, 'gks/feat'), { recursive: true })
  await mkdir(join(wt, 'gks/concept'), { recursive: true })
  run('ln', ['-s', join(repoRoot, 'node_modules'), join(wt, 'node_modules')], wt)
  // Tiny package.json so npx works
  await writeFile(
    join(wt, 'package.json'),
    JSON.stringify({ name: 'fixture', private: true }, null, 2),
  )

  // Initial commit + push (so origin has main)
  await writeFile(join(wt, 'README.md'), '# fixture\n')
  run('git', ['add', '.'], wt)
  run('git', ['commit', '-m', 'init'], wt)
  run('git', ['push', '-u', 'origin', 'main'], wt)

  // Install hook
  await mkdir(join(wt, '.git/hooks'), { recursive: true })
  await copyFile(hookSrc, join(wt, '.git/hooks/pre-push'))
  await chmodX(join(wt, '.git/hooks/pre-push'))
}, 60_000)

afterAll(async () => {
  // best-effort tmp cleanup; OS will GC
})

describe('pre-push hook', () => {
  it('exits 0 when no FEAT files were touched in the push range', () => {
    const wt = join(repoDir, 'wt')
    // Touch only a non-FEAT file
    spawnSync('bash', ['-c', `echo 'extra' >> README.md && git add README.md && git commit -m 'tweak'`], {
      cwd: wt,
    })
    const r = run('git', ['push', 'origin', 'main'], wt)
    // Push should succeed; hook silent
    expect(r.code).toBe(0)
  }, 30_000)

  it('exits 0 when a touched FEAT chain is OK', async () => {
    const wt = join(repoDir, 'wt')
    await writeFile(join(wt, 'gks/concept/CONCEPT--TEST-OK.md'), STABLE_CONCEPT)
    await writeFile(join(wt, 'gks/feat/FEAT--TEST-OK.md'), STABLE_FEAT)
    // Build a minimal atomic_index for verify-flow
    const idx = [
      JSON.stringify({ id: 'CONCEPT--TEST-OK', phase: 1, type: 'concept', status: 'stable', vault_id: 'TEST', path: 'concept/CONCEPT--TEST-OK.md' }),
      JSON.stringify({ id: 'FEAT--TEST-OK', phase: 2, type: 'feat', status: 'stable', vault_id: 'TEST', path: 'feat/FEAT--TEST-OK.md', crosslinks: { references: ['CONCEPT--TEST-OK'] } }),
    ].join('\n') + '\n'
    await writeFile(join(wt, 'gks/00_index/atomic_index.jsonl'), idx)
    run('git', ['add', '.'], wt)
    run('git', ['commit', '-m', 'add OK chain'], wt)
    const r = run('git', ['push', 'origin', 'main'], wt)
    expect(r.code).toBe(0)
    expect(r.stdout + r.stderr).toMatch(/MSP pre-push: 1 FEAT\(s\) verified|FEAT--TEST-OK/)
  }, 60_000)

  it('exits 1 when a touched FEAT chain is broken', async () => {
    const wt = join(repoDir, 'wt')
    await writeFile(join(wt, 'gks/feat/FEAT--TEST-BROKEN.md'), BROKEN_FEAT)
    // Index doesn't have CONCEPT--DOES-NOT-EXIST → verify-flow fails
    const idx = [
      JSON.stringify({ id: 'CONCEPT--TEST-OK', phase: 1, type: 'concept', status: 'stable', vault_id: 'TEST', path: 'concept/CONCEPT--TEST-OK.md' }),
      JSON.stringify({ id: 'FEAT--TEST-OK', phase: 2, type: 'feat', status: 'stable', vault_id: 'TEST', path: 'feat/FEAT--TEST-OK.md', crosslinks: { references: ['CONCEPT--TEST-OK'] } }),
      JSON.stringify({ id: 'FEAT--TEST-BROKEN', phase: 2, type: 'feat', status: 'stable', vault_id: 'TEST', path: 'feat/FEAT--TEST-BROKEN.md', crosslinks: { references: ['CONCEPT--DOES-NOT-EXIST'] } }),
    ].join('\n') + '\n'
    await writeFile(join(wt, 'gks/00_index/atomic_index.jsonl'), idx)
    run('git', ['add', '.'], wt)
    run('git', ['commit', '-m', 'add broken chain'], wt)
    const r = run('git', ['push', 'origin', 'main'], wt)
    expect(r.code).not.toBe(0)
    expect(r.stdout + r.stderr).toMatch(/MSP pre-push:.*failed/)
  }, 60_000)

  it('--no-verify bypasses even when chain broken', () => {
    const wt = join(repoDir, 'wt')
    const r = run('git', ['push', '--no-verify', 'origin', 'main'], wt)
    expect(r.code).toBe(0)
  }, 30_000)
})
