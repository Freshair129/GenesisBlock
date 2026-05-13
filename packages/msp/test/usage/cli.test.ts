import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { main } from '../../src/usage/cli.js'

interface Streams {
  stdout: string
  stderr: string
}

function captureIO(argv: string[]): { streams: Streams; restore: () => void } {
  const streams: Streams = { stdout: '', stderr: '' }
  const origArgv = process.argv
  const origStdoutWrite = process.stdout.write.bind(process.stdout)
  const origStderrWrite = process.stderr.write.bind(process.stderr)

  process.argv = ['node', '/fake/cli.js', ...argv]
  process.stdout.write = ((chunk: string | Uint8Array) => {
    streams.stdout += typeof chunk === 'string' ? chunk : Buffer.from(chunk).toString('utf8')
    return true
  }) as typeof process.stdout.write
  process.stderr.write = ((chunk: string | Uint8Array) => {
    streams.stderr += typeof chunk === 'string' ? chunk : Buffer.from(chunk).toString('utf8')
    return true
  }) as typeof process.stderr.write

  return {
    streams,
    restore: () => {
      process.argv = origArgv
      process.stdout.write = origStdoutWrite
      process.stderr.write = origStderrWrite
    },
  }
}

async function writeFakeDaily(
  root: string,
  date: string,
  totalCost: number,
  byTier: { T1?: number; T2?: number; T3?: number } = {},
): Promise<void> {
  const dir = resolve(root, 'gks', 'usage')
  await mkdir(dir, { recursive: true })
  const body = {
    total_cost_usd: totalCost,
    call_count: (byTier.T1 ?? 0) + (byTier.T2 ?? 0) + (byTier.T3 ?? 0),
    by_tier: {
      T1: { count: byTier.T1 ?? 0, cost_usd: 0 },
      T2: { count: byTier.T2 ?? 0, cost_usd: totalCost },
      T3: { count: byTier.T3 ?? 0, cost_usd: 0 },
    },
    top_episodes: [],
    updated_at: `${date}T12:00:00.000Z`,
  }
  const content = [
    '---',
    `id: USAGE--DAILY-${date}`,
    'phase: 5',
    'type: usage',
    'status: stable',
    'tier: genesis',
    'source_type: episodic',
    'vault_id: default',
    `title: "USAGE — Daily cost bucket ${date}"`,
    'tags:',
    '  - agents',
    '  - usage',
    '  - cost',
    '  - daily',
    `created_at: ${date}T00:00:00.000Z`,
    '---',
    '',
    `# Daily ${date}`,
    '',
    '<!-- USAGE-SUMMARY-START -->',
    '```json',
    JSON.stringify(body, null, 2),
    '```',
    '<!-- USAGE-SUMMARY-END -->',
    '',
  ].join('\n')
  await writeFile(join(dir, `USAGE--DAILY-${date}.md`), content, 'utf8')
}

let captured: { streams: Streams; restore: () => void } | undefined

afterEach(() => {
  captured?.restore()
  captured = undefined
})

describe('msp-usage CLI — argument parsing', () => {
  let root: string

  beforeEach(async () => {
    root = await mkdtemp(join(tmpdir(), 'msp-usage-cli-'))
  })

  afterEach(async () => {
    await rm(root, { recursive: true, force: true })
  })

  it('--help prints usage and returns 0', async () => {
    captured = captureIO(['--help'])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/msp-usage/)
    expect(captured.streams.stdout).toMatch(/Usage:/)
  })

  it('returns 2 when no subcommand is given', async () => {
    captured = captureIO([])
    const code = await main()
    expect(code).toBe(2)
    expect(captured.streams.stderr).toMatch(/missing subcommand/)
  })

  it('returns 2 on unknown subcommand', async () => {
    captured = captureIO(['bogus'])
    const code = await main()
    expect(code).toBe(2)
    expect(captured.streams.stderr).toMatch(/unknown subcommand/)
  })

  it('rollup-week without --iso returns 2', async () => {
    captured = captureIO(['rollup-week', '--write', `--root=${root}`])
    const code = await main()
    expect(code).toBe(2)
    expect(captured.streams.stderr).toMatch(/rollup-week requires --iso/)
  })

  it('rollup-month without --iso returns 2', async () => {
    captured = captureIO(['rollup-month', '--write', `--root=${root}`])
    const code = await main()
    expect(code).toBe(2)
    expect(captured.streams.stderr).toMatch(/rollup-month requires --iso/)
  })
})

describe('msp-usage CLI — read subcommands', () => {
  let root: string

  beforeEach(async () => {
    root = await mkdtemp(join(tmpdir(), 'msp-usage-cli-read-'))
  })

  afterEach(async () => {
    await rm(root, { recursive: true, force: true })
  })

  it('week --iso prints aggregate (human table)', async () => {
    await writeFakeDaily(root, '2026-05-04', 1, { T2: 1 })
    await writeFakeDaily(root, '2026-05-07', 2, { T2: 1 })

    captured = captureIO(['week', '--iso=2026-W19', `--root=${root}`])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/USAGE — week 2026-W19/)
    expect(captured.streams.stdout).toMatch(/total_cost_usd: \$3\.0000/)
    expect(captured.streams.stdout).toMatch(/days_covered:\s+2/)
  })

  it('week --iso --json emits parseable UsageSummary', async () => {
    await writeFakeDaily(root, '2026-05-04', 1.5, { T2: 1 })

    captured = captureIO(['week', '--iso=2026-W19', '--json', `--root=${root}`])
    const code = await main()
    expect(code).toBe(0)
    const parsed = JSON.parse(captured.streams.stdout) as {
      total_cost_usd: number
      days_covered: number
      calls_by_tier: { T1: number; T2: number; T3: number }
    }
    expect(parsed.total_cost_usd).toBeCloseTo(1.5, 9)
    expect(parsed.days_covered).toBe(1)
    expect(parsed.calls_by_tier.T2).toBe(1)
  })

  it('month --iso aggregates calendar month', async () => {
    await writeFakeDaily(root, '2026-05-01', 1, { T2: 1 })
    await writeFakeDaily(root, '2026-05-31', 2, { T2: 1 })
    await writeFakeDaily(root, '2026-06-01', 999, { T3: 1 })

    captured = captureIO(['month', '--iso=2026-05', '--json', `--root=${root}`])
    const code = await main()
    expect(code).toBe(0)
    const parsed = JSON.parse(captured.streams.stdout) as {
      total_cost_usd: number
      days_covered: number
    }
    expect(parsed.total_cost_usd).toBeCloseTo(3, 9)
    expect(parsed.days_covered).toBe(2)
  })

  it('today returns empty summary when no daily exists for today', async () => {
    // We do not write a daily for today's date — aggregateSingleDay
    // returns the empty shape.
    captured = captureIO(['today', '--json', `--root=${root}`])
    const code = await main()
    expect(code).toBe(0)
    const parsed = JSON.parse(captured.streams.stdout) as {
      total_cost_usd: number
      days_covered: number
    }
    expect(parsed.total_cost_usd).toBe(0)
    expect(parsed.days_covered).toBe(0)
  })

  it('week with no --iso defaults to current ISO week', async () => {
    captured = captureIO(['week', '--json', `--root=${root}`])
    const code = await main()
    expect(code).toBe(0)
    const parsed = JSON.parse(captured.streams.stdout) as {
      total_cost_usd: number
    }
    expect(typeof parsed.total_cost_usd).toBe('number')
  })

  it('month with no --iso defaults to current calendar month', async () => {
    captured = captureIO(['month', '--json', `--root=${root}`])
    const code = await main()
    expect(code).toBe(0)
    const parsed = JSON.parse(captured.streams.stdout) as {
      total_cost_usd: number
    }
    expect(typeof parsed.total_cost_usd).toBe('number')
  })
})

describe('msp-usage CLI — rollup subcommands', () => {
  let root: string

  beforeEach(async () => {
    root = await mkdtemp(join(tmpdir(), 'msp-usage-cli-rollup-'))
  })

  afterEach(async () => {
    await rm(root, { recursive: true, force: true })
  })

  it('rollup-week without --write performs dry-run (no file written)', async () => {
    await writeFakeDaily(root, '2026-05-04', 1, { T2: 1 })

    captured = captureIO(['rollup-week', '--iso=2026-W19', `--root=${root}`])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/dry-run/)

    // No file written.
    const path = resolve(root, 'gks', 'usage', 'USAGE--WEEKLY-2026-W19.md')
    let existed = true
    try {
      await readFile(path, 'utf8')
    } catch {
      existed = false
    }
    expect(existed).toBe(false)
  })

  it('rollup-week --write persists the atom', async () => {
    await writeFakeDaily(root, '2026-05-04', 2, { T2: 2 })

    captured = captureIO([
      'rollup-week',
      '--iso=2026-W19',
      '--write',
      `--root=${root}`,
    ])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/wrote /)
    expect(captured.streams.stdout).toMatch(/USAGE--WEEKLY-2026-W19\.md/)

    const content = await readFile(
      resolve(root, 'gks', 'usage', 'USAGE--WEEKLY-2026-W19.md'),
      'utf8',
    )
    expect(content).toContain('id: USAGE--WEEKLY-2026-W19')
  })

  it('rollup-month --write persists the atom', async () => {
    await writeFakeDaily(root, '2026-05-15', 5, { T2: 5 })

    captured = captureIO([
      'rollup-month',
      '--iso=2026-05',
      '--write',
      `--root=${root}`,
    ])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/wrote /)

    const content = await readFile(
      resolve(root, 'gks', 'usage', 'USAGE--MONTHLY-2026-05.md'),
      'utf8',
    )
    expect(content).toContain('id: USAGE--MONTHLY-2026-05')
    expect(content).toContain('period_start: 2026-05-01')
    expect(content).toContain('period_end: 2026-05-31')
  })

  it('rollup-week --json returns aggregate as JSON in dry-run', async () => {
    await writeFakeDaily(root, '2026-05-04', 3, { T2: 1 })

    captured = captureIO([
      'rollup-week',
      '--iso=2026-W19',
      '--json',
      `--root=${root}`,
    ])
    const code = await main()
    expect(code).toBe(0)
    const parsed = JSON.parse(captured.streams.stdout) as {
      total_cost_usd: number
    }
    expect(parsed.total_cost_usd).toBeCloseTo(3, 9)
  })
})
