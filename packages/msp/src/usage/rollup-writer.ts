// USAGE--WEEKLY-<isoWeek> / USAGE--MONTHLY-<YYYY-MM> roll-up atom writer.
//
// Aggregates dailies via packages/msp/src/usage/aggregator.ts then writes the
// resulting UsageSummary as a roll-up atom per SPEC--USAGE-ROLLUP-ATOM. Writes
// are explicit (caller-driven); this module never runs from dispatcher hot path.
//
// Body layout mirrors usage-recorder.ts: frontmatter + a fenced JSON block
// between HTML comment markers. Subsequent calls for the same period rewrite
// the JSON in place; frontmatter is preserved.

import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'

import {
  aggregateMonth,
  aggregateWeek,
  formatIsoMonth,
  formatIsoWeek,
  isoWeekEnd,
  isoWeekStart,
  parseIsoMonth,
  parseIsoWeek,
  type UsageSummary,
} from './aggregator.js'

const SUMMARY_START = '<!-- USAGE-SUMMARY-START -->'
const SUMMARY_END = '<!-- USAGE-SUMMARY-END -->'

interface RollupPayload extends UsageSummary {
  bucket: 'weekly' | 'monthly'
  iso_period: string
  period_start: string
  period_end: string
  generated_at: string
}

function escapeYaml(s: string): string {
  return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`
}

function toIsoDate(d: Date): string {
  return d.toISOString().slice(0, 10)
}

async function fileExists(path: string): Promise<boolean> {
  try {
    await readFile(path, 'utf8')
    return true
  } catch (err) {
    if (
      typeof err === 'object' &&
      err !== null &&
      'code' in err &&
      (err as { code: unknown }).code === 'ENOENT'
    ) {
      return false
    }
    throw err
  }
}

function buildFrontmatter(
  id: string,
  title: string,
  createdAt: string,
  bucket: 'weekly' | 'monthly',
  payload: RollupPayload,
): string {
  return [
    '---',
    `id: ${id}`,
    'phase: 5',
    'type: usage',
    'status: stable',
    'tier: genesis',
    'source_type: episodic',
    'vault_id: default',
    `title: ${escapeYaml(title)}`,
    'tags:',
    '  - agents',
    '  - usage',
    '  - cost',
    '  - rollup',
    `  - ${bucket}`,
    `period_start: ${payload.period_start}`,
    `period_end: ${payload.period_end}`,
    `days_covered: ${payload.days_covered}`,
    `total_cost_usd: ${payload.total_cost_usd}`,
    `created_at: ${createdAt}`,
    '---',
    '',
  ].join('\n')
}

function buildBody(title: string, bucket: 'weekly' | 'monthly', payload: RollupPayload): string {
  const json = JSON.stringify(payload, null, 2)
  const intro =
    bucket === 'weekly'
      ? `Weekly roll-up of dispatcher cost telemetry (ISO week ${payload.iso_period}). Generated from \`USAGE--DAILY-*\` atoms — see \`SPEC--USAGE-ROLLUP-ATOM\` for contract.`
      : `Monthly roll-up of dispatcher cost telemetry (calendar month ${payload.iso_period}). Generated from \`USAGE--DAILY-*\` atoms — see \`SPEC--USAGE-ROLLUP-ATOM\` for contract.`
  return [
    `# ${title}`,
    '',
    intro,
    '',
    '## Summary',
    '',
    SUMMARY_START,
    '```json',
    json,
    '```',
    SUMMARY_END,
    '',
  ].join('\n')
}

function replaceSummaryBlock(content: string, payload: RollupPayload): string {
  const startIdx = content.indexOf(SUMMARY_START)
  const endIdx = content.indexOf(SUMMARY_END)
  if (startIdx === -1 || endIdx === -1) {
    // Malformed existing file — fall back to appending a fresh block.
    const json = JSON.stringify(payload, null, 2)
    return `${content}\n\n${SUMMARY_START}\n\`\`\`json\n${json}\n\`\`\`\n${SUMMARY_END}\n`
  }
  const before = content.slice(0, startIdx + SUMMARY_START.length)
  const after = content.slice(endIdx)
  const json = JSON.stringify(payload, null, 2)
  return `${before}\n\`\`\`json\n${json}\n\`\`\`\n${after}`
}

async function writeRollup(
  root: string,
  id: string,
  title: string,
  bucket: 'weekly' | 'monthly',
  payload: RollupPayload,
): Promise<string> {
  const filename = `${id}.md`
  const absDir = resolve(root, 'gks', 'usage')
  const absPath = join(absDir, filename)
  await mkdir(dirname(absPath), { recursive: true })

  const exists = await fileExists(absPath)
  if (!exists) {
    const frontmatter = buildFrontmatter(id, title, payload.generated_at, bucket, payload)
    const body = buildBody(title, bucket, payload)
    await writeFile(absPath, frontmatter + body, 'utf8')
    return absPath
  }

  const existing = await readFile(absPath, 'utf8')
  const next = replaceSummaryBlock(existing, payload)
  await writeFile(absPath, next, 'utf8')
  return absPath
}

/**
 * Aggregate the given ISO week's dailies and write a `USAGE--WEEKLY-<isoWeek>`
 * atom. Returns the absolute path of the written file.
 */
export async function writeWeeklyRollup(
  root: string,
  weekIso: string,
): Promise<string> {
  const { year, week } = parseIsoWeek(weekIso)
  const canonicalIso = formatIsoWeek(year, week)
  const start = isoWeekStart(year, week)
  const end = isoWeekEnd(year, week)
  const summary = await aggregateWeek(root, canonicalIso)
  const payload: RollupPayload = {
    ...summary,
    bucket: 'weekly',
    iso_period: canonicalIso,
    period_start: toIsoDate(start),
    period_end: toIsoDate(end),
    generated_at: new Date().toISOString(),
  }
  const id = `USAGE--WEEKLY-${canonicalIso}`
  const title = `USAGE — Weekly roll-up ${canonicalIso}`
  return writeRollup(root, id, title, 'weekly', payload)
}

/**
 * Aggregate the given calendar month's dailies and write a
 * `USAGE--MONTHLY-<YYYY-MM>` atom. Returns the absolute path.
 */
export async function writeMonthlyRollup(
  root: string,
  monthIso: string,
): Promise<string> {
  const { year, month } = parseIsoMonth(monthIso)
  const canonicalIso = formatIsoMonth(year, month)
  const start = new Date(Date.UTC(year, month - 1, 1))
  const end = new Date(Date.UTC(year, month, 0))
  const summary = await aggregateMonth(root, canonicalIso)
  const payload: RollupPayload = {
    ...summary,
    bucket: 'monthly',
    iso_period: canonicalIso,
    period_start: toIsoDate(start),
    period_end: toIsoDate(end),
    generated_at: new Date().toISOString(),
  }
  const id = `USAGE--MONTHLY-${canonicalIso}`
  const title = `USAGE — Monthly roll-up ${canonicalIso}`
  return writeRollup(root, id, title, 'monthly', payload)
}
