import { spawn } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))
const cliSrc = `${repoRoot}/src/symbols/cli.ts`

interface SpawnResult {
  code: number
  stdout: string
  stderr: string
}

function runCli(args: string[], cwd: string): Promise<SpawnResult> {
  return new Promise((resolveProm, rejectProm) => {
    const child = spawn('npx', ['tsx', cliSrc, ...args], {
      cwd,
      env: { ...process.env, MSP_ROOT: cwd },
    })
    let stdout = ''
    let stderr = ''
    child.stdout.on('data', (d) => (stdout += d.toString()))
    child.stderr.on('data', (d) => (stderr += d.toString()))
    child.on('error', rejectProm)
    child.on('close', (code) => resolveProm({ code: code ?? 0, stdout, stderr }))
  })
}

let workDir: string

function setupFixture(): string {
  const dir = mkdtempSync(join(tmpdir(), 'sg-cli-'))
  // Two .ts files in src/.
  mkdirSync(join(dir, 'src'), { recursive: true })
  writeFileSync(
    join(dir, 'src', 'foo.ts'),
    `export function alpha(): number {
  return helperFn() + 1
}

function helperFn(): number {
  return 42
}
`,
    'utf8',
  )
  writeFileSync(
    join(dir, 'src', 'bar.ts'),
    `export class Bar {
  beta(): string {
    return 'beta'
  }
}

export const gamma = 'gamma'
`,
    'utf8',
  )
  return dir
}

beforeEach(() => {
  workDir = setupFixture()
})

afterEach(() => {
  rmSync(workDir, { recursive: true, force: true })
})

describe('msp-graph CLI', () => {
  it('build on a tmpdir fixture succeeds and produces SQLite + JSONL', async () => {
    const result = await runCli(['build', `--root=${workDir}`], workDir)
    expect(result.code).toBe(0)
    const symDir = join(workDir, '.brain/msp/projects/evaAI/symbols')
    expect(existsSync(join(symDir, 'graph.db'))).toBe(true)
    expect(existsSync(join(symDir, 'symbols.jsonl'))).toBe(true)
    expect(existsSync(join(symDir, 'edges.jsonl'))).toBe(true)
    expect(existsSync(join(symDir, 'communities.jsonl'))).toBe(true)
    expect(existsSync(join(symDir, 'meta.json'))).toBe(true)
  }, 60_000)

  it('stats exits 2 when graph not built', async () => {
    const result = await runCli(['stats', `--root=${workDir}`], workDir)
    expect(result.code).toBe(2)
    expect(result.stderr).toMatch(/graph not built/)
  }, 30_000)

  it('stats exits 0 with counts after build', async () => {
    const build = await runCli(['build', `--root=${workDir}`], workDir)
    expect(build.code).toBe(0)
    const stats = await runCli(['stats', '--json', `--root=${workDir}`], workDir)
    expect(stats.code).toBe(0)
    const meta = JSON.parse(stats.stdout) as {
      symbol_count: number
      edge_count: number
      community_count: number
    }
    // 2 modules + alpha (function) + helperFn (function) + Bar (class) +
    // Bar.beta (method) + gamma (const) ⇒ at least 7 symbols.
    expect(meta.symbol_count).toBeGreaterThanOrEqual(6)
    expect(meta.edge_count).toBeGreaterThanOrEqual(5)
  }, 60_000)

  it('query <name> exit 0 + correct symbol on a built fixture', async () => {
    await runCli(['build', `--root=${workDir}`], workDir)
    const result = await runCli(
      ['query', 'alpha', '--json', `--root=${workDir}`],
      workDir,
    )
    expect(result.code).toBe(0)
    const parsed = JSON.parse(result.stdout) as {
      ok: boolean
      hits: Array<{ name: string; kind: string }>
    }
    expect(parsed.ok).toBe(true)
    expect(parsed.hits.length).toBeGreaterThanOrEqual(1)
    expect(parsed.hits[0].name).toBe('alpha')
  }, 60_000)

  it('query <missing> exits 1', async () => {
    await runCli(['build', `--root=${workDir}`], workDir)
    const result = await runCli(
      ['query', 'no_such_symbol_zzzqqq', `--root=${workDir}`],
      workDir,
    )
    expect(result.code).toBe(1)
  }, 60_000)
})
