import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import {
  writeMonthlyRollup,
  writeWeeklyRollup,
} from '../../src/usage/rollup-writer.js'

const SUMMARY_START = '<!-- USAGE-SUMMARY-START -->'
const SUMMARY_END = '<!-- USAGE-SUMMARY-END -->'

interface RollupBody {
  bucket: 'weekly' | 'monthly'
  iso_period: string
  period_start: string
  period_end: string
  total_cost_usd: number
  calls_by_tier: { T1: number; T2: number; T3: number }
  days_covered: number
  top_episodes: { id: string; cost_usd: number }[]
  generated_at: string
}

function extractSummary(content: string): RollupBody {
  const startIdx = content.indexOf(SUMMARY_START)
  const endIdx = content.indexOf(SUMMARY_END)
  expect(startIdx).toBeGreaterThan(-1)
  expect(endIdx).toBeGreaterThan(startIdx)
  const block = content.slice(startIdx + SUMMARY_START.length, endIdx)
  const fenceMatch = block.match(/```json\s*([\s\S]*?)\s*```/)
  expect(fenceMatch).not.toBeNull()
  return JSON.parse(fenceMatch![1]!) as RollupBody
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
    `# USAGE — Daily cost bucket ${date}`,
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

describe('rollup-writer — weekly', () => {
  let root: string

  beforeEach(async () => {
    root = await mkdtemp(join(tmpdir(), 'msp-rollup-week-'))
  })

  afterEach(async () => {
    await rm(root, { recursive: true, force: true })
  })

  it('writes USAGE--WEEKLY-<iso>.md with correct frontmatter + body', async () => {
    await writeFakeDaily(root, '2026-05-04', 1.5, { T2: 3 })
    await writeFakeDaily(root, '2026-05-07', 2.5, { T2: 4 })

    const path = await writeWeeklyRollup(root, '2026-W19')
    expect(path).toBe(
      resolve(root, 'gks', 'usage', 'USAGE--WEEKLY-2026-W19.md'),
    )

    const content = await readFile(path, 'utf8')
    expect(content).toContain('id: USAGE--WEEKLY-2026-W19')
    expect(content).toContain('phase: 5')
    expect(content).toContain('type: usage')
    expect(content).toContain('status: stable')
    expect(content).toContain('tier: genesis')
    expect(content).toContain('source_type: episodic')
    expect(content).toContain('  - rollup')
    expect(content).toContain('  - weekly')
    expect(content).toContain('period_start: 2026-05-04')
    expect(content).toContain('period_end: 2026-05-10')

    const body = extractSummary(content)
    expect(body.bucket).toBe('weekly')
    expect(body.iso_period).toBe('2026-W19')
    expect(body.period_start).toBe('2026-05-04')
    expect(body.period_end).toBe('2026-05-10')
    expect(body.days_covered).toBe(2)
    expect(body.total_cost_usd).toBeCloseTo(4, 9)
    expect(body.calls_by_tier.T2).toBe(7)
  })

  it('overwrites the JSON block in place when re-run with new data', async () => {
    await writeFakeDaily(root, '2026-05-04', 1, { T2: 1 })
    const path = await writeWeeklyRollup(root, '2026-W19')
    const firstContent = await readFile(path, 'utf8')
    const firstCreatedAt = firstContent.match(/^created_at: (.+)$/m)?.[1]
    expect(firstCreatedAt).toBeDefined()

    // Add another daily, re-run rollup.
    await writeFakeDaily(root, '2026-05-05', 9, { T2: 1 })
    await new Promise((r) => setTimeout(r, 5))
    const path2 = await writeWeeklyRollup(root, '2026-W19')
    expect(path2).toBe(path)

    const secondContent = await readFile(path, 'utf8')
    const secondCreatedAt = secondContent.match(/^created_at: (.+)$/m)?.[1]
    // Frontmatter created_at preserved.
    expect(secondCreatedAt).toBe(firstCreatedAt)

    const body = extractSummary(secondContent)
    expect(body.days_covered).toBe(2)
    expect(body.total_cost_usd).toBeCloseTo(10, 9)
  })

  it('writes empty roll-up when no dailies fall in the week', async () => {
    const path = await writeWeeklyRollup(root, '2026-W19')
    const body = extractSummary(await readFile(path, 'utf8'))
    expect(body.bucket).toBe('weekly')
    expect(body.days_covered).toBe(0)
    expect(body.total_cost_usd).toBe(0)
    expect(body.calls_by_tier).toEqual({ T1: 0, T2: 0, T3: 0 })
  })
})

describe('rollup-writer — monthly', () => {
  let root: string

  beforeEach(async () => {
    root = await mkdtemp(join(tmpdir(), 'msp-rollup-month-'))
  })

  afterEach(async () => {
    await rm(root, { recursive: true, force: true })
  })

  it('writes USAGE--MONTHLY-<YYYY-MM>.md with month-spanning aggregate', async () => {
    await writeFakeDaily(root, '2026-04-30', 100, { T3: 1 }) // out
    await writeFakeDaily(root, '2026-05-01', 1, { T2: 1 })
    await writeFakeDaily(root, '2026-05-15', 2, { T2: 1 })
    await writeFakeDaily(root, '2026-05-31', 4, { T2: 1 })
    await writeFakeDaily(root, '2026-06-01', 200, { T3: 1 }) // out

    const path = await writeMonthlyRollup(root, '2026-05')
    expect(path).toBe(
      resolve(root, 'gks', 'usage', 'USAGE--MONTHLY-2026-05.md'),
    )

    const content = await readFile(path, 'utf8')
    expect(content).toContain('id: USAGE--MONTHLY-2026-05')
    expect(content).toContain('  - monthly')
    expect(content).toContain('  - rollup')
    expect(content).toContain('period_start: 2026-05-01')
    expect(content).toContain('period_end: 2026-05-31')

    const body = extractSummary(content)
    expect(body.bucket).toBe('monthly')
    expect(body.iso_period).toBe('2026-05')
    expect(body.days_covered).toBe(3)
    expect(body.total_cost_usd).toBeCloseTo(7, 9)
    expect(body.calls_by_tier.T2).toBe(3)
    expect(body.calls_by_tier.T3).toBe(0)
  })

  it('throws on invalid month iso', async () => {
    await expect(writeMonthlyRollup(root, 'not-a-month')).rejects.toThrow()
    await expect(writeMonthlyRollup(root, '2026-13')).rejects.toThrow()
  })
})
