#!/usr/bin/env node
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'

import { gcEpisodes } from './episode-gc.js'
import type { GcOpts, GcReport } from './episode-gc.js'

const HELP = `msp-episode-gc — prune old episode atoms under <root>/gks/episode/

Usage:
  msp-episode-gc [options]
  msp-episode-gc --help

Default behaviour is implicit dry-run: nothing is mutated unless --apply is
given. Eligible old episodes are archived to <root>/gks/episode/_archive/<YYYY-MM>/
unless --delete is also passed.

Options:
  --keep-days=<N>   keep episodes newer than N days (default 30)
  --apply           actually mutate the filesystem (default: dry-run)
  --delete          permanently unlink eligible episodes instead of archiving
                    (requires --apply to take effect)
  --dry-run         force dry-run even if --apply is set
  --root=<path>     repo root that contains gks/episode/ (default: cwd)
  --json            emit GcReport as JSON on stdout
  --help            show this help

Retention rule (see ADR--EPISODE-GC-POLICY):
  - keep all episodes from the last N days
  - keep all older episodes that failed (ok=false) or were severity=critical
  - archive (or delete with --delete) the rest

Exit codes:
  0  GC completed (report emitted)
  1  GC completed with at least one filesystem error
  2  bad usage / parse error
`

interface ParsedCliArgs {
  keepDays: number
  apply: boolean
  delete: boolean
  dryRun: boolean
  root: string
  json: boolean
}

function parsePositiveInt(value: string, flag: string): number {
  const n = Number.parseInt(value, 10)
  if (!Number.isFinite(n) || n < 0 || String(n) !== value.trim()) {
    throw new Error(`${flag} must be a non-negative integer, got "${value}"`)
  }
  return n
}

export async function main(): Promise<number> {
  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        'keep-days': { type: 'string' },
        apply: { type: 'boolean' },
        delete: { type: 'boolean' },
        'dry-run': { type: 'boolean' },
        root: { type: 'string' },
        json: { type: 'boolean' },
        help: { type: 'boolean' },
      },
      allowPositionals: false,
    })
  } catch (err) {
    process.stderr.write(`error: ${(err as Error).message}\n${HELP}`)
    return 2
  }

  const { values } = parsed

  if (values.help) {
    process.stdout.write(HELP)
    return 0
  }

  let keepDays = 30
  if (values['keep-days'] !== undefined) {
    try {
      keepDays = parsePositiveInt(values['keep-days'], '--keep-days')
    } catch (err) {
      process.stderr.write(`error: ${(err as Error).message}\n${HELP}`)
      return 2
    }
  }

  const cli: ParsedCliArgs = {
    keepDays,
    apply: values.apply === true,
    delete: values.delete === true,
    dryRun: values['dry-run'] === true,
    root: values.root ?? process.cwd(),
    json: values.json === true,
  }

  // Conservative default: without --apply, treat as dry-run.
  // --dry-run forces dry-run even if --apply was supplied.
  const dryRun = cli.dryRun || !cli.apply

  const opts: GcOpts = {
    keep_days: cli.keepDays,
    delete: cli.delete,
    dry_run: dryRun,
  }

  let report: GcReport
  try {
    report = await gcEpisodes(cli.root, opts)
  } catch (err) {
    process.stderr.write(`✗ gcEpisodes error: ${(err as Error).message}\n`)
    return 1
  }

  if (cli.json) {
    process.stdout.write(JSON.stringify(report, null, 2) + '\n')
  } else {
    writeHumanReport(cli, report, dryRun)
  }

  return report.errors.length > 0 ? 1 : 0
}

function writeHumanReport(
  cli: ParsedCliArgs,
  report: GcReport,
  dryRun: boolean,
): void {
  const mode = dryRun
    ? '[dry-run]'
    : cli.delete
      ? '[apply --delete]'
      : '[apply archive]'
  const verbScanned = dryRun ? 'would scan' : 'scanned'
  const verbArchive = dryRun ? 'would archive' : 'archived'
  const verbDelete = dryRun ? 'would delete' : 'deleted'
  process.stdout.write(`msp-episode-gc ${mode} — root=${cli.root}\n`)
  process.stdout.write(`  keep_days     : ${cli.keepDays}\n`)
  process.stdout.write(`  ${verbScanned.padEnd(13)}: ${report.total_scanned}\n`)
  process.stdout.write(`  kept          : ${report.kept}\n`)
  process.stdout.write(`  ${verbArchive.padEnd(13)}: ${report.archived}\n`)
  process.stdout.write(`  ${verbDelete.padEnd(13)}: ${report.deleted}\n`)
  if (report.errors.length > 0) {
    process.stdout.write(`  errors        : ${report.errors.length}\n`)
    for (const e of report.errors) {
      process.stdout.write(`    - ${e}\n`)
    }
  }
  if (dryRun && !cli.apply) {
    process.stdout.write(
      '\n(no changes made — re-run with --apply to actually move/delete files)\n',
    )
  }
}

// Auto-run when invoked as a script. When the module is imported (e.g. by
// tests that drive main() directly with a stubbed process.argv), the
// entrypoint path won't match and the side-effect is skipped.
function isDirectInvocation(): boolean {
  const entry = process.argv[1]
  if (entry === undefined) return false
  const here = fileURLToPath(import.meta.url)
  return entry === here || entry === here.replace(/\.ts$/, '.js')
}

if (isDirectInvocation()) {
  main()
    .then((code) => process.exit(code))
    .catch((err) => {
      process.stderr.write(`✗ unexpected: ${(err as Error).message}\n`)
      process.exit(2)
    })
}
