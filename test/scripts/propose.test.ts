import { spawnSync } from 'node:child_process'
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { afterEach, beforeAll, describe, expect, it } from 'vitest'

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))
const inboundDir = join(repoRoot, '.brain/msp/projects/evaAI/inbound')

interface RunResult {
  code: number
  stdout: string
  stderr: string
}

function npmRun(script: string, args: string[]): RunResult {
  const r = spawnSync('npm', ['run', script, '--silent', '--', ...args], {
    cwd: repoRoot,
    encoding: 'utf8',
    env: { ...process.env, NO_COLOR: '1' },
  })
  return { code: r.status ?? -1, stdout: r.stdout ?? '', stderr: r.stderr ?? '' }
}

async function findInbound(prefix: string): Promise<string | undefined> {
  try {
    const entries = await readdir(inboundDir)
    const match = entries.find((n) => n.startsWith(`${prefix}.rev-`) && n.endsWith('.md'))
    return match ? join(inboundDir, match) : undefined
  } catch {
    return undefined
  }
}

const cleanup: string[] = []
afterEach(async () => {
  for (const id of cleanup.splice(0)) {
    const p = await findInbound(id)
    if (p) await rm(p, { force: true })
  }
})

describe('msp:propose wrapper (phase 6 passthrough)', () => {
  it('passes phase 5 through unchanged', async () => {
    const id = 'CONCEPT--TEST-WRAPPER-P5'
    cleanup.push(id)
    const r = npmRun('msp:propose', [id, '--title=Test', '--body=test', '--phase=5', '--type=concept', '--root=' + repoRoot])
    expect(r.code).toBe(0)
    const path = await findInbound(id)
    expect(path).toBeDefined()
    const text = await readFile(path!, 'utf8')
    expect(text).toMatch(/^phase: 5$/m)
  }, 30_000)

  it('translates phase 6 → propose at 5 then patches file to phase 6', async () => {
    const id = 'AUDIT--TEST-WRAPPER-P6'
    cleanup.push(id)
    const r = npmRun('msp:propose', [id, '--title=Test', '--body=test', '--phase=6', '--type=audit', '--root=' + repoRoot])
    expect(r.code).toBe(0)
    expect(r.stdout + r.stderr).toMatch(/patched .+ to phase: 6/)
    const path = await findInbound(id)
    expect(path).toBeDefined()
    const text = await readFile(path!, 'utf8')
    expect(text).toMatch(/^phase: 6$/m)
    expect(text).not.toMatch(/^phase: 5$/m)
  }, 30_000)

  it('exits non-zero when GKS rejects (invalid ID format)', async () => {
    // ID without TYPE--SLUG pattern — GKS atomic-id refuses
    const r = npmRun('msp:propose', ['lowercase-id', '--title=t', '--body=t', '--phase=5', '--type=concept', '--root=' + repoRoot])
    expect(r.code).not.toBe(0)
  }, 30_000)
})
