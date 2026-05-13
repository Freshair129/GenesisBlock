import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import {
  aggregateDaily,
  aggregateMonth,
  aggregateSingleDay,
  aggregateWeek,
  formatIsoMonth,
  formatIsoWeek,
  isoWeekEnd,
  isoWeekOf,
  isoWeekStart,
  parseIsoMonth,
  parseIsoWeek,
} from '../../src/usage/aggregator.js'

interface FakeDaily {
  date: string // YYYY-MM-DD
  total_cost_usd: number
  by_tier?: {
    T1?: { count: number; cost_usd: number }
    T2?: { count: number; cost_usd: number }
    T3?: { count: number; cost_usd: number }
  }
  top_episodes?: { id: string; cost_usd: number; tier?: string }[]
}

async function writeFakeDaily(root: string, daily: FakeDaily): Promise<void> {
  const dir = resolve(root, 'gks', 'usage')
  await mkdir(dir, { recursive: true })
  const id = `USAGE--DAILY-${daily.date}`
  const body = {
    total_cost_usd: daily.total_cost_usd,
    call_count:
      (daily.by_tier?.T1?.count ?? 0) +
      (daily.by_tier?.T2?.count ?? 0) +
      (daily.by_tier?.T3?.count ?? 0),
    by_tier: {
      T1: daily.by_tier?.T1 ?? { count: 0, cost_usd: 0 },
      T2: daily.by_tier?.T2 ?? { count: 0, cost_usd: 0 },
      T3: daily.by_tier?.T3 ?? { count: 0, cost_usd: 0 },
    },
    top_episodes: daily.top_episodes ?? [],
    updated_at: `${daily.date}T12:00:00.000Z`,
  }
  const content = [
    '---',
    `id: ${id}`,
    'phase: 5',
    'type: usage',
    'status: stable',
    'tier: genesis',
    'source_type: episodic',
    'vault_id: default',
    `title: "USAGE — Daily cost bucket ${daily.date}"`,
    'tags:',
    '  - agents',
    '  - usage',
    '  - cost',
    '  - daily',
    `created_at: ${daily.date}T00:00:00.000Z`,
    '---',
    '',
    `# USAGE — Daily cost bucket ${daily.date}`,
    '',
    '## Summary',
    '',
    '<!-- USAGE-SUMMARY-START -->',
    '```json',
    JSON.stringify(body, null, 2),
    '```',
    '<!-- USAGE-SUMMARY-END -->',
    '',
  ].join('\n')
  await writeFile(join(dir, `${id}.md`), content, 'utf8')
}

describe('aggregator — ISO week helpers', () => {
  it('isoWeekOf(2026-05-07) === 2026-W19 (Thursday)', () => {
    // 2026-05-07 is a Thursday; ISO week 19 of 2026.
    const w = isoWeekOf(new Date(Date.UTC(2026, 4, 7)))
    expect(formatIsoWeek(w.year, w.week)).toBe('2026-W19')
  })

  it('isoWeekOf(2026-05-04) === 2026-W19 (Monday start)', () => {
    const w = isoWeekOf(new Date(Date.UTC(2026, 4, 4)))
    expect(formatIsoWeek(w.year, w.week)).toBe('2026-W19')
  })

  it('isoWeekOf(2026-05-10) === 2026-W19 (Sunday end)', () => {
    const w = isoWeekOf(new Date(Date.UTC(2026, 4, 10)))
    expect(formatIsoWeek(w.year, w.week)).toBe('2026-W19')
  })

  it('isoWeekStart(2026,19) is Monday 2026-05-04', () => {
    const d = isoWeekStart(2026, 19)
    expect(d.toISOString().slice(0, 10)).toBe('2026-05-04')
  })

  it('isoWeekEnd(2026,19) is Sunday 2026-05-10', () => {
    const d = isoWeekEnd(2026, 19)
    expect(d.toISOString().slice(0, 10)).toBe('2026-05-10')
  })

  it('parseIsoWeek round-trip', () => {
    expect(parseIsoWeek('2026-W19')).toEqual({ year: 2026, week: 19 })
    expect(parseIsoWeek('2025-W01')).toEqual({ year: 2025, week: 1 })
  })

  it('parseIsoWeek rejects malformed input', () => {
    expect(() => parseIsoWeek('2026-W')).toThrow()
    expect(() => parseIsoWeek('2026-W99')).toThrow()
    expect(() => parseIsoWeek('week-2026')).toThrow()
  })

  it('parseIsoMonth + formatIsoMonth round-trip', () => {
    expect(parseIsoMonth('2026-05')).toEqual({ year: 2026, month: 5 })
    expect(formatIsoMonth(2026, 5)).toBe('2026-05')
    expect(formatIsoMonth(2026, 12)).toBe('2026-12')
  })

  it('parseIsoMonth rejects malformed input', () => {
    expect(() => parseIsoMonth('2026-13')).toThrow()
    expect(() => parseIsoMonth('05-2026')).toThrow()
    expect(() => parseIsoMonth('2026-5')).toThrow()
  })
})

describe('aggregator — file scanning + summing', () => {
  let root: string

  beforeEach(async () => {
    root = await mkdtemp(join(tmpdir(), 'msp-usage-aggregator-'))
  })

  afterEach(async () => {
    await rm(root, { recursive: true, force: true })
  })

  it('aggregateDaily returns empty summary when usage dir is missing', async () => {
    const s = await aggregateDaily(root)
    expect(s.total_cost_usd).toBe(0)
    expect(s.days_covered).toBe(0)
    expect(s.calls_by_tier).toEqual({ T1: 0, T2: 0, T3: 0 })
    expect(s.top_episodes).toEqual([])
  })

  it('aggregateDaily sums across all daily atoms', async () => {
    await writeFakeDaily(root, {
      date: '2026-05-04',
      total_cost_usd: 0.01,
      by_tier: { T1: { count: 3, cost_usd: 0 }, T2: { count: 1, cost_usd: 0.01 } },
    })
    await writeFakeDaily(root, {
      date: '2026-05-05',
      total_cost_usd: 0.05,
      by_tier: { T3: { count: 1, cost_usd: 0.05 } },
    })
    await writeFakeDaily(root, {
      date: '2026-05-06',
      total_cost_usd: 0.002,
      by_tier: { T2: { count: 2, cost_usd: 0.002 } },
    })

    const s = await aggregateDaily(root)
    expect(s.days_covered).toBe(3)
    expect(s.total_cost_usd).toBeCloseTo(0.062, 9)
    expect(s.calls_by_tier).toEqual({ T1: 3, T2: 3, T3: 1 })
  })

  it('aggregateDaily honours since/until window', async () => {
    await writeFakeDaily(root, {
      date: '2026-05-01',
      total_cost_usd: 1,
      by_tier: { T2: { count: 1, cost_usd: 1 } },
    })
    await writeFakeDaily(root, {
      date: '2026-05-05',
      total_cost_usd: 2,
      by_tier: { T2: { count: 1, cost_usd: 2 } },
    })
    await writeFakeDaily(root, {
      date: '2026-05-09',
      total_cost_usd: 4,
      by_tier: { T2: { count: 1, cost_usd: 4 } },
    })

    const s = await aggregateDaily(
      root,
      new Date(Date.UTC(2026, 4, 4)),
      new Date(Date.UTC(2026, 4, 8)),
    )
    expect(s.days_covered).toBe(1)
    expect(s.total_cost_usd).toBeCloseTo(2, 9)
    expect(s.calls_by_tier.T2).toBe(1)
  })

  it('aggregateWeek filters to ISO week dailies', async () => {
    // ISO week 2026-W19 runs 2026-05-04 → 2026-05-10
    await writeFakeDaily(root, {
      date: '2026-05-03', // Sun -> week W18
      total_cost_usd: 10,
      by_tier: { T3: { count: 1, cost_usd: 10 } },
    })
    await writeFakeDaily(root, {
      date: '2026-05-04', // Mon -> W19
      total_cost_usd: 1,
      by_tier: { T2: { count: 1, cost_usd: 1 } },
    })
    await writeFakeDaily(root, {
      date: '2026-05-10', // Sun -> W19
      total_cost_usd: 2,
      by_tier: { T2: { count: 1, cost_usd: 2 } },
    })
    await writeFakeDaily(root, {
      date: '2026-05-11', // Mon -> W20
      total_cost_usd: 50,
      by_tier: { T3: { count: 1, cost_usd: 50 } },
    })

    const s = await aggregateWeek(root, '2026-W19')
    expect(s.days_covered).toBe(2)
    expect(s.total_cost_usd).toBeCloseTo(3, 9)
    expect(s.calls_by_tier.T2).toBe(2)
    expect(s.calls_by_tier.T3).toBe(0)
  })

  it('aggregateMonth filters to calendar month', async () => {
    await writeFakeDaily(root, {
      date: '2026-04-30',
      total_cost_usd: 100,
      by_tier: { T3: { count: 1, cost_usd: 100 } },
    })
    await writeFakeDaily(root, {
      date: '2026-05-01',
      total_cost_usd: 1,
      by_tier: { T2: { count: 1, cost_usd: 1 } },
    })
    await writeFakeDaily(root, {
      date: '2026-05-31',
      total_cost_usd: 2,
      by_tier: { T2: { count: 2, cost_usd: 2 } },
    })
    await writeFakeDaily(root, {
      date: '2026-06-01',
      total_cost_usd: 999,
      by_tier: { T3: { count: 1, cost_usd: 999 } },
    })

    const s = await aggregateMonth(root, '2026-05')
    expect(s.days_covered).toBe(2)
    expect(s.total_cost_usd).toBeCloseTo(3, 9)
    expect(s.calls_by_tier.T2).toBe(3)
  })

  it('aggregateSingleDay returns just one day', async () => {
    await writeFakeDaily(root, {
      date: '2026-05-14',
      total_cost_usd: 0.0234,
      by_tier: {
        T1: { count: 8, cost_usd: 0 },
        T2: { count: 3, cost_usd: 0.0034 },
        T3: { count: 1, cost_usd: 0.02 },
      },
    })
    await writeFakeDaily(root, {
      date: '2026-05-15',
      total_cost_usd: 50,
      by_tier: { T3: { count: 5, cost_usd: 50 } },
    })

    const s = await aggregateSingleDay(root, '2026-05-14')
    expect(s.days_covered).toBe(1)
    expect(s.total_cost_usd).toBeCloseTo(0.0234, 9)
    expect(s.calls_by_tier).toEqual({ T1: 8, T2: 3, T3: 1 })
  })

  it('aggregateSingleDay returns empty when no atom matches', async () => {
    await writeFakeDaily(root, {
      date: '2026-05-14',
      total_cost_usd: 0.01,
      by_tier: { T2: { count: 1, cost_usd: 0.01 } },
    })
    const s = await aggregateSingleDay(root, '2026-05-13')
    expect(s.days_covered).toBe(0)
    expect(s.total_cost_usd).toBe(0)
  })

  it('top_episodes are sorted by cost desc and capped at 5', async () => {
    await writeFakeDaily(root, {
      date: '2026-05-04',
      total_cost_usd: 30,
      by_tier: { T3: { count: 3, cost_usd: 30 } },
      top_episodes: [
        { id: 'EP-A', cost_usd: 10 },
        { id: 'EP-B', cost_usd: 5 },
      ],
    })
    await writeFakeDaily(root, {
      date: '2026-05-05',
      total_cost_usd: 50,
      by_tier: { T3: { count: 5, cost_usd: 50 } },
      top_episodes: [
        { id: 'EP-C', cost_usd: 15 },
        { id: 'EP-D', cost_usd: 12 },
        { id: 'EP-E', cost_usd: 8 },
        { id: 'EP-F', cost_usd: 6 },
        { id: 'EP-G', cost_usd: 2 },
      ],
    })
    const s = await aggregateWeek(root, '2026-W19')
    expect(s.top_episodes.length).toBe(5)
    expect(s.top_episodes[0]!.id).toBe('EP-C')
    expect(s.top_episodes[0]!.cost_usd).toBe(15)
    // Sorted by cost desc: C(15), D(12), A(10), E(8), F(6).
    expect(s.top_episodes.map((e) => e.id)).toEqual([
      'EP-C',
      'EP-D',
      'EP-A',
      'EP-E',
      'EP-F',
    ])
  })

  it('ignores non-daily files (roll-ups, junk)', async () => {
    await writeFakeDaily(root, {
      date: '2026-05-04',
      total_cost_usd: 1,
      by_tier: { T2: { count: 1, cost_usd: 1 } },
    })
    const dir = resolve(root, 'gks', 'usage')
    await writeFile(
      join(dir, 'USAGE--WEEKLY-2026-W19.md'),
      '---\nid: USAGE--WEEKLY-2026-W19\n---\n',
      'utf8',
    )
    await writeFile(join(dir, 'random.md'), 'noise', 'utf8')

    const s = await aggregateDaily(root)
    expect(s.days_covered).toBe(1)
    expect(s.total_cost_usd).toBe(1)
  })
})
