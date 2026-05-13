#!/usr/bin/env node
// `msp-usage` — read-only dashboard CLI over USAGE--DAILY-* atoms, plus
// roll-up writers (gated on --write). See CONCEPT--USAGE-ROLLUPS,
// SPEC--USAGE-ROLLUP-ATOM, SPEC--USAGE-ATOM.

import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'

import {
  aggregateMonth,
  aggregateSingleDay,
  aggregateWeek,
  formatIsoMonth,
  formatIsoWeek,
  isoWeekOf,
  type UsageSummary,
} from './aggregator.js'
import { writeMonthlyRollup, writeWeeklyRollup } from './rollup-writer.js'

const HELP = `msp-usage — read cost dashboards over USAGE--DAILY-* atoms

Usage:
  msp-usage today                                show today's totals
  msp-usage week  [--iso=YYYY-Www]               aggregate ISO week (default: current)
  msp-usage month [--iso=YYYY-MM]                aggregate calendar month (default: current)
  msp-usage rollup-week  --iso=YYYY-Www --write  write USAGE--WEEKLY-<iso>.md
  msp-usage rollup-month --iso=YYYY-MM  --write  write USAGE--MONTHLY-<iso>.md
  msp-usage --help

Options:
  --iso=<period>      ISO 8601 week (YYYY-Www) or month (YYYY-MM)
  --root=<dir>        repo root (default: process.cwd())
  --write             actually write the roll-up atom (else just print)
  --json              emit UsageSummary JSON instead of human table
  --help              show this help

Exit codes:
  0  success
  1  runtime error (read / parse / write)
  2  bad usage
`

type Subcommand =
  | 'today'
  | 'week'
  | 'month'
  | 'rollup-week'
  | 'rollup-month'

const VALID_SUBCOMMANDS: readonly Subcommand[] = [
  'today',
  'week',
  'month',
  'rollup-week',
  'rollup-month',
]

function isSubcommand(value: string): value is Subcommand {
  return (VALID_SUBCOMMANDS as readonly string[]).includes(value)
}

function todayIsoDate(): string {
  return new Date().toISOString().slice(0, 10)
}

function currentIsoWeek(): string {
  const { year, week } = isoWeekOf(new Date())
  return formatIsoWeek(year, week)
}

function currentIsoMonth(): string {
  const d = new Date()
  return formatIsoMonth(d.getUTCFullYear(), d.getUTCMonth() + 1)
}

// ---------------------------------------------------------------------------
// Output formatters
// ---------------------------------------------------------------------------

function useColor(): boolean {
  if (process.env['NO_COLOR']) return false
  return Boolean(process.stdout.isTTY)
}

function color(s: string, code: number): string {
  if (!useColor()) return s
  return `[${code}m${s}[0m`
}

function formatCost(n: number): string {
  return `$${n.toFixed(4)}`
}

function tierColor(tier: 'T1' | 'T2' | 'T3', cost: number): string {
  const label = `${tier}: ${formatCost(cost)}`
  if (tier === 'T1') return color(label, 32) // green
  if (tier === 'T2') return color(label, 33) // yellow
  return color(label, 31) // red — T3
}

function formatTable(label: string, summary: UsageSummary): string {
  const lines: string[] = []
  lines.push(color(`USAGE — ${label}`, 1))
  lines.push('')
  lines.push(`  total_cost_usd: ${formatCost(summary.total_cost_usd)}`)
  lines.push(`  days_covered:   ${summary.days_covered}`)
  lines.push('')
  lines.push('  calls_by_tier:')
  lines.push(`    T1: ${summary.calls_by_tier.T1}`)
  lines.push(`    T2: ${summary.calls_by_tier.T2}`)
  lines.push(`    T3: ${summary.calls_by_tier.T3}`)

  // Per-tier cost not directly in summary (only call counts) — display a
  // sparingly-coloured banner. We do not have per-tier cost separated here,
  // so we colour only the totals row by dominant tier in count.
  const counts = summary.calls_by_tier
  const dominantTier =
    counts.T3 > 0 ? 'T3' : counts.T2 > 0 ? 'T2' : 'T1'
  lines.push('')
  lines.push(
    `  ${tierColor(dominantTier as 'T1' | 'T2' | 'T3', summary.total_cost_usd)} (dominant tier by call count)`,
  )

  if (summary.top_episodes.length > 0) {
    lines.push('')
    lines.push('  top_episodes:')
    for (const ep of summary.top_episodes) {
      lines.push(`    - ${ep.id} (${formatCost(ep.cost_usd)})`)
    }
  }
  lines.push('')
  return lines.join('\n')
}

function emitOutput(label: string, summary: UsageSummary, asJson: boolean): void {
  if (asJson) {
    process.stdout.write(JSON.stringify(summary, null, 2) + '\n')
  } else {
    process.stdout.write(formatTable(label, summary))
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

export async function main(): Promise<number> {
  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        iso: { type: 'string' },
        root: { type: 'string' },
        write: { type: 'boolean' },
        json: { type: 'boolean' },
        help: { type: 'boolean' },
      },
      allowPositionals: true,
    })
  } catch (err) {
    process.stderr.write(`error: ${(err as Error).message}\n${HELP}`)
    return 2
  }
  const { values, positionals } = parsed

  if (values.help) {
    process.stdout.write(HELP)
    return 0
  }

  if (positionals.length === 0) {
    process.stderr.write(`error: missing subcommand\n${HELP}`)
    return 2
  }
  const sub = positionals[0]!
  if (!isSubcommand(sub)) {
    process.stderr.write(
      `error: unknown subcommand "${sub}". Expected one of ${VALID_SUBCOMMANDS.join('|')}\n${HELP}`,
    )
    return 2
  }

  const root = values.root ?? process.cwd()
  const asJson = values.json === true

  try {
    if (sub === 'today') {
      const date = todayIsoDate()
      const summary = await aggregateSingleDay(root, date)
      emitOutput(`today (${date})`, summary, asJson)
      return 0
    }

    if (sub === 'week') {
      const iso = values.iso ?? currentIsoWeek()
      const summary = await aggregateWeek(root, iso)
      emitOutput(`week ${iso}`, summary, asJson)
      return 0
    }

    if (sub === 'month') {
      const iso = values.iso ?? currentIsoMonth()
      const summary = await aggregateMonth(root, iso)
      emitOutput(`month ${iso}`, summary, asJson)
      return 0
    }

    if (sub === 'rollup-week') {
      if (values.iso === undefined) {
        process.stderr.write(`error: rollup-week requires --iso=YYYY-Www\n${HELP}`)
        return 2
      }
      if (values.write !== true) {
        // Dry run — aggregate and print, do not write.
        const summary = await aggregateWeek(root, values.iso)
        emitOutput(`rollup-week ${values.iso} (dry-run — pass --write to persist)`, summary, asJson)
        return 0
      }
      const path = await writeWeeklyRollup(root, values.iso)
      process.stdout.write(`wrote ${path}\n`)
      return 0
    }

    if (sub === 'rollup-month') {
      if (values.iso === undefined) {
        process.stderr.write(`error: rollup-month requires --iso=YYYY-MM\n${HELP}`)
        return 2
      }
      if (values.write !== true) {
        const summary = await aggregateMonth(root, values.iso)
        emitOutput(`rollup-month ${values.iso} (dry-run — pass --write to persist)`, summary, asJson)
        return 0
      }
      const path = await writeMonthlyRollup(root, values.iso)
      process.stdout.write(`wrote ${path}\n`)
      return 0
    }
  } catch (err) {
    process.stderr.write(`error: ${(err as Error).message}\n`)
    return 1
  }

  // Exhaustiveness fallthrough (shouldn't reach here — isSubcommand gate above).
  return 2
}

function isDirectInvocation(): boolean {
  const entry = process.argv[1]
  if (!entry) return false
  const here = fileURLToPath(import.meta.url)
  return entry === here || entry === here.replace(/\.ts$/, '.js')
}

if (isDirectInvocation()) {
  main()
    .then((code) => process.exit(code))
    .catch((err) => {
      process.stderr.write(`unexpected: ${(err as Error).message}\n`)
      process.exit(1)
    })
}
