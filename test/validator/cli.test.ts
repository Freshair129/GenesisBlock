import { spawn } from 'node:child_process'
import { mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const cliPath = fileURLToPath(new URL('../../src/validator/cli.ts', import.meta.url))
const repoRoot = fileURLToPath(new URL('../..', import.meta.url))

interface RunResult {
  code: number
  stdout: string
  stderr: string
}

function run(args: string[]): Promise<RunResult> {
  return new Promise((resolve, reject) => {
    const child = spawn('npx', ['tsx', cliPath, ...args], {
      env: { ...process.env, NO_COLOR: '1' },
      cwd: repoRoot,
    })
    let stdout = ''
    let stderr = ''
    child.stdout.on('data', (d) => (stdout += d.toString()))
    child.stderr.on('data', (d) => (stderr += d.toString()))
    child.on('error', reject)
    child.on('close', (code) => resolve({ code: code ?? -1, stdout, stderr }))
  })
}

describe('msp-validate CLI', () => {
  it('exits 0 on a valid fixture', async () => {
    const r = await run([
      `--root=${repoRoot}`,
      `${repoRoot}/test/fixtures/CONCEPT--TEST-VALID.md`,
    ])
    expect(r.code).toBe(0)
    expect(r.stdout).toMatch(/✓/)
  }, 30_000)

  it('exits 1 on a forbidden-field fixture and names the rule', async () => {
    const r = await run([
      `--root=${repoRoot}`,
      `${repoRoot}/test/fixtures/CONCEPT--TEST-FORBIDDEN.md`,
    ])
    expect(r.code).toBe(1)
    expect(r.stdout).toMatch(/\[forbidden-fields\]/)
  }, 30_000)

  it('exits 1 on a dangling wikilink fixture', async () => {
    const r = await run([
      `--root=${repoRoot}`,
      `${repoRoot}/test/fixtures/CONCEPT--TEST-DANGLING.md`,
    ])
    expect(r.code).toBe(1)
    expect(r.stdout).toMatch(/\[dangling-wikilink\]/)
  }, 30_000)

  it('emits valid JSON with --json on a failing fixture', async () => {
    const r = await run([
      '--json',
      `--root=${repoRoot}`,
      `${repoRoot}/test/fixtures/CONCEPT--TEST-FORBIDDEN.md`,
    ])
    expect(r.code).toBe(1)
    const parsed = JSON.parse(r.stdout)
    expect(Array.isArray(parsed)).toBe(true)
    expect(parsed[0].errors[0].rule).toBe('forbidden-fields')
  }, 30_000)

  it('exits 2 when atomic index is missing', async () => {
    const tmp = await mkdtemp(join(tmpdir(), 'msp-validate-'))
    await writeFile(join(tmp, 'atom.md'), '---\nid: CONCEPT--FOO\n---\nbody\n')
    const r = await run([
      `--root=${tmp}`,
      `--index=${tmp}/missing.jsonl`,
      `${tmp}/atom.md`,
    ])
    expect(r.code).toBe(2)
    expect(r.stderr).toMatch(/atomic index not found/)
  }, 30_000)

  it('--all on the repo dogfoods successfully', async () => {
    const r = await run([`--root=${repoRoot}`, '--all'])
    if (r.code !== 0) {
      // CI debug: dump full output when assertion will fail
      // eslint-disable-next-line no-console
      console.error('=== DOGFOODS DEBUG ===\nSTDOUT:\n' + r.stdout + '\nSTDERR:\n' + r.stderr + '\n=== END ===')
    }
    expect(r.code).toBe(0)
    expect(r.stdout).toMatch(/Total: \d+ passed, 0 failed/)
  }, 60_000)
})
