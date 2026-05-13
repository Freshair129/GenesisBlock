// Pure aggregator over USAGE--DAILY-<isoDate>.md atoms.
//
// Read-only consumer of files written by packages/msp/src/agents/usage-recorder.ts
// (contract: SPEC--USAGE-ATOM). Produces a UsageSummary collapsing N dailies
// into a single shape suitable for week / month roll-ups and dashboard CLIs.
//
// No I/O outside read access of <root>/gks/usage/. Never writes.
//
// See CONCEPT--USAGE-ROLLUPS and SPEC--USAGE-ROLLUP-ATOM for the why + contract.

import { readFile, readdir } from 'node:fs/promises'
import { join, resolve } from 'node:path'

import type { Tier } from '../agents/types.js'

const SUMMARY_START = '<!-- USAGE-SUMMARY-START -->'
const SUMMARY_END = '<!-- USAGE-SUMMARY-END -->'
const DAILY_PREFIX = 'USAGE--DAILY-'
const TOP_EPISODES_LIMIT = 5

export interface UsageSummary {
  total_cost_usd: number
  calls_by_tier: { T1: number; T2: number; T3: number }
  days_covered: number
  top_episodes: { id: string; cost_usd: number }[]
}

interface DailyJsonShape {
  total_cost_usd?: number
  call_count?: number
  by_tier?: Partial<Record<Tier, { count?: number; cost_usd?: number }>>
  top_episodes?: { id?: string; cost_usd?: number; tier?: string }[]
  updated_at?: string
}

interface ParsedDaily {
  date: string // YYYY-MM-DD
  summary: DailyJsonShape
}

export function emptySummary(): UsageSummary {
  return {
    total_cost_usd: 0,
    calls_by_tier: { T1: 0, T2: 0, T3: 0 },
    days_covered: 0,
    top_episodes: [],
  }
}

// ---------------------------------------------------------------------------
// ISO week + month helpers (no deps; sufficient for aggregator filtering)
// ---------------------------------------------------------------------------

/**
 * Compute the ISO 8601 week-year + week number for a given UTC date.
 * Returns `{ year, week }` where week is 1..53 and year is the ISO week-year
 * (which can differ from the calendar year for the first / last days of Jan
 * or Dec). All math runs in UTC; weeks start on Monday.
 */
export function isoWeekOf(date: Date): { year: number; week: number } {
  // Copy date in UTC; shift to the Thursday of the current ISO week (ISO
  // weeks: Mon=1..Sun=7). The Thursday's calendar year is the ISO week-year.
  const tmp = new Date(
    Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()),
  )
  const dayNum = tmp.getUTCDay() === 0 ? 7 : tmp.getUTCDay() // Sun -> 7
  tmp.setUTCDate(tmp.getUTCDate() + 4 - dayNum)
  const year = tmp.getUTCFullYear()
  const jan1 = new Date(Date.UTC(year, 0, 1))
  const diffDays = Math.floor((tmp.getTime() - jan1.getTime()) / 86400000)
  const week = Math.floor(diffDays / 7) + 1
  return { year, week }
}

/**
 * Format an ISO week as the canonical `YYYY-Www` string.
 */
export function formatIsoWeek(year: number, week: number): string {
  return `${year}-W${String(week).padStart(2, '0')}`
}

/**
 * Parse an ISO week string (`YYYY-Www`) into `{ year, week }`. Throws on
 * malformed input.
 */
export function parseIsoWeek(s: string): { year: number; week: number } {
  const match = /^(\d{4})-W(\d{2})$/.exec(s)
  if (match === null) {
    throw new Error(`invalid ISO week: "${s}" (expected YYYY-Www)`)
  }
  const year = Number.parseInt(match[1]!, 10)
  const week = Number.parseInt(match[2]!, 10)
  if (week < 1 || week > 53) {
    throw new Error(`ISO week out of range: ${week}`)
  }
  return { year, week }
}

/**
 * Format an ISO month as `YYYY-MM`.
 */
export function formatIsoMonth(year: number, month: number): string {
  return `${year}-${String(month).padStart(2, '0')}`
}

/**
 * Parse an ISO month string (`YYYY-MM`) into `{ year, month }`. Throws on
 * malformed input. `month` is 1..12.
 */
export function parseIsoMonth(s: string): { year: number; month: number } {
  const match = /^(\d{4})-(\d{2})$/.exec(s)
  if (match === null) {
    throw new Error(`invalid ISO month: "${s}" (expected YYYY-MM)`)
  }
  const year = Number.parseInt(match[1]!, 10)
  const month = Number.parseInt(match[2]!, 10)
  if (month < 1 || month > 12) {
    throw new Error(`ISO month out of range: ${month}`)
  }
  return { year, month }
}

/**
 * Compute the UTC date (00:00) of the Monday opening the given ISO week.
 */
export function isoWeekStart(year: number, week: number): Date {
  // Jan 4 is always in ISO week 1 of its year.
  const jan4 = new Date(Date.UTC(year, 0, 4))
  const jan4Dow = jan4.getUTCDay() === 0 ? 7 : jan4.getUTCDay()
  const week1Monday = new Date(jan4)
  week1Monday.setUTCDate(jan4.getUTCDate() - (jan4Dow - 1))
  const target = new Date(week1Monday)
  target.setUTCDate(week1Monday.getUTCDate() + (week - 1) * 7)
  return target
}

/**
 * Compute the UTC date (00:00) of the Sunday closing the given ISO week.
 */
export function isoWeekEnd(year: number, week: number): Date {
  const start = isoWeekStart(year, week)
  const end = new Date(start)
  end.setUTCDate(start.getUTCDate() + 6)
  return end
}

function toIsoDate(d: Date): string {
  return d.toISOString().slice(0, 10)
}

// ---------------------------------------------------------------------------
// File scan + parse
// ---------------------------------------------------------------------------

function extractDailySummary(content: string): DailyJsonShape | null {
  const startIdx = content.indexOf(SUMMARY_START)
  const endIdx = content.indexOf(SUMMARY_END)
  if (startIdx === -1 || endIdx === -1 || endIdx < startIdx) return null
  const block = content.slice(startIdx + SUMMARY_START.length, endIdx)
  const fenceMatch = block.match(/```json\s*([\s\S]*?)\s*```/)
  if (fenceMatch === null) return null
  try {
    return JSON.parse(fenceMatch[1]!) as DailyJsonShape
  } catch {
    return null
  }
}

function dateFromFilename(filename: string): string | null {
  // USAGE--DAILY-YYYY-MM-DD.md
  if (!filename.startsWith(DAILY_PREFIX) || !filename.endsWith('.md')) return null
  const datePart = filename.slice(DAILY_PREFIX.length, filename.length - 3)
  if (!/^\d{4}-\d{2}-\d{2}$/.test(datePart)) return null
  return datePart
}

async function loadDailies(root: string): Promise<ParsedDaily[]> {
  const dir = resolve(root, 'gks', 'usage')
  let entries: string[]
  try {
    entries = await readdir(dir)
  } catch (err) {
    if (
      typeof err === 'object' &&
      err !== null &&
      'code' in err &&
      (err as { code: unknown }).code === 'ENOENT'
    ) {
      return []
    }
    throw err
  }

  const dailies: ParsedDaily[] = []
  for (const entry of entries) {
    const date = dateFromFilename(entry)
    if (date === null) continue
    const content = await readFile(join(dir, entry), 'utf8')
    const summary = extractDailySummary(content)
    if (summary === null) continue
    dailies.push({ date, summary })
  }
  // Stable order — by date ascending.
  dailies.sort((a, b) => a.date.localeCompare(b.date))
  return dailies
}

function reduce(dailies: ParsedDaily[]): UsageSummary {
  const out = emptySummary()
  out.days_covered = dailies.length
  for (const { summary } of dailies) {
    if (typeof summary.total_cost_usd === 'number') {
      out.total_cost_usd += summary.total_cost_usd
    }
    if (summary.by_tier) {
      for (const tier of ['T1', 'T2', 'T3'] as const) {
        const bucket = summary.by_tier[tier]
        if (bucket && typeof bucket.count === 'number') {
          out.calls_by_tier[tier] += bucket.count
        }
      }
    }
    if (Array.isArray(summary.top_episodes)) {
      for (const ep of summary.top_episodes) {
        if (
          ep &&
          typeof ep.id === 'string' &&
          typeof ep.cost_usd === 'number'
        ) {
          out.top_episodes.push({ id: ep.id, cost_usd: ep.cost_usd })
        }
      }
    }
  }
  out.top_episodes.sort((a, b) => b.cost_usd - a.cost_usd)
  if (out.top_episodes.length > TOP_EPISODES_LIMIT) {
    out.top_episodes.length = TOP_EPISODES_LIMIT
  }
  return out
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Aggregate all daily USAGE atoms under `<root>/gks/usage/`, optionally
 * filtered to a `[since, until]` UTC date range (inclusive endpoints by ISO
 * date string compare).
 */
export async function aggregateDaily(
  root: string,
  since?: Date,
  until?: Date,
): Promise<UsageSummary> {
  const dailies = await loadDailies(root)
  const filtered = dailies.filter(({ date }) => {
    if (since !== undefined && date < toIsoDate(since)) return false
    if (until !== undefined && date > toIsoDate(until)) return false
    return true
  })
  return reduce(filtered)
}

/**
 * Aggregate all daily USAGE atoms falling in the given ISO week
 * (e.g. `2026-W19`).
 */
export async function aggregateWeek(
  root: string,
  weekIso: string,
): Promise<UsageSummary> {
  const { year, week } = parseIsoWeek(weekIso)
  const start = isoWeekStart(year, week)
  const end = isoWeekEnd(year, week)
  return aggregateDaily(root, start, end)
}

/**
 * Aggregate all daily USAGE atoms falling in the given calendar month
 * (e.g. `2026-05`).
 */
export async function aggregateMonth(
  root: string,
  monthIso: string,
): Promise<UsageSummary> {
  const { year, month } = parseIsoMonth(monthIso)
  const start = new Date(Date.UTC(year, month - 1, 1))
  const end = new Date(Date.UTC(year, month, 0)) // day 0 of next month = last day
  return aggregateDaily(root, start, end)
}

/**
 * Read the daily USAGE atom for a single ISO date (`YYYY-MM-DD`). Returns an
 * empty summary if no atom exists for the date.
 */
export async function aggregateSingleDay(
  root: string,
  isoDate: string,
): Promise<UsageSummary> {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(isoDate)) {
    throw new Error(`invalid ISO date: "${isoDate}" (expected YYYY-MM-DD)`)
  }
  const dailies = await loadDailies(root)
  return reduce(dailies.filter((d) => d.date === isoDate))
}
