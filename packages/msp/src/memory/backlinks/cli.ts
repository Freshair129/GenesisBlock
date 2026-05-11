#!/usr/bin/env node
import { resolve } from 'node:path'
import { parseArgs } from 'node:util'

import { rebuildBacklinks } from './indexer.js'

const HELP = `msp-backlinks — full-rebuild backlinks.jsonl from atom crosslinks

Usage:
  msp-backlinks                       # rebuild .brain/.../vector/backlinks.jsonl
  msp-backlinks --root=<dir>          # project root (default: cwd)
  msp-backlinks --namespace=<ns>      # default 'evaAI' per ADR--PATH-ENCODING
  msp-backlinks --dry-run             # preview counts; don't write
  msp-backlinks --check               # exit 1 if file would change (CI assert)

Exit codes:
  0  rebuild successful (or --dry-run / --check showing no change)
  1  --check found drift between on-disk file and recomputed content
`

async function main(): Promise<number> {
  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        root: { type: 'string' },
        namespace: { type: 'string' },
        'dry-run': { type: 'boolean' },
        check: { type: 'boolean' },
        help: { type: 'boolean' },
      },
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

  const root = resolve(values.root ?? process.cwd())
  const result = await rebuildBacklinks({
    root,
    namespace: values.namespace,
    dryRun: values['dry-run'] === true,
    check: values.check === true,
  })

  if (values.check) {
    if (result.changed) {
      console.log(`✗ backlinks drift: ${result.outputPath} differs from recomputed content (${result.edgeCount} edges from ${result.atomCount} atoms)`)
      return 1
    }
    console.log(`✓ backlinks up-to-date (${result.edgeCount} edges from ${result.atomCount} atoms)`)
    return 0
  }

  if (values['dry-run']) {
    console.log(`[dry-run] ${result.edgeCount} edges from ${result.atomCount} atoms (would write to ${result.outputPath})`)
    return 0
  }

  console.log(`✓ wrote ${result.outputPath} (${result.edgeCount} edges from ${result.atomCount} atoms)${result.changed ? '' : ' [unchanged]'}`)
  return 0
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(`✗ msp-backlinks: ${(err as Error).message}\n`)
    process.exit(2)
  })
