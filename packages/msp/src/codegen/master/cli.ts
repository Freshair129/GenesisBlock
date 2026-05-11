#!/usr/bin/env node
import { resolve } from 'node:path'
import { parseArgs } from 'node:util'

import { composeMasterAtoms, formatAsPromptFragment } from './composer.js'

const HELP = `msp-master — compose Master atom bodies into a system-prompt fragment

Usage:
  msp-master compose <ID1> [<ID2> ...]
  msp-master compose --root=<dir> <ID1> [<ID2> ...]
  msp-master --help

Flags:
  --root=<dir>    project root (default: cwd)
  --help          this message

Behaviour:
  Each <ID> is resolved to gks/master/<ID>.md (with a fallback scan of gks/).
  The atom must carry tier: master in its frontmatter; otherwise the id is
  reported as missing.

  The composed fragment is written to stdout. A summary line is written to
  stderr (e.g. "[master] composed 2 atoms, 145 tokens"). When any requested
  id is missing or non-Master, the ids are listed on stderr.

Exit codes:
  0  all requested atoms composed
  1  one or more requested ids missing or not tier:master
  2  internal error (bad arguments, IO failure)
`

interface CliOptions {
  root: string
}

async function runCompose(ids: string[], opts: CliOptions): Promise<number> {
  if (ids.length === 0) {
    process.stderr.write(`error: compose requires at least one atom id\n${HELP}`)
    return 2
  }
  const result = await composeMasterAtoms(ids, opts.root)
  const fragment = formatAsPromptFragment(result)
  if (fragment.length > 0) {
    process.stdout.write(fragment)
    if (!fragment.endsWith('\n')) process.stdout.write('\n')
  }
  process.stderr.write(
    `[master] composed ${result.composed.length} atoms, ${result.totalTokens} tokens\n`,
  )
  if (result.missing.length > 0) {
    process.stderr.write(
      `[master] missing or not tier:master: ${result.missing.join(', ')}\n`,
    )
    return 1
  }
  return 0
}

async function main(): Promise<number> {
  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        root: { type: 'string' },
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
    process.stderr.write(`error: no subcommand given\n${HELP}`)
    return 2
  }

  const [subcommand, ...rest] = positionals
  const root = resolve(values.root ?? process.cwd())

  switch (subcommand) {
    case 'compose':
      return runCompose(rest, { root })
    default:
      process.stderr.write(`error: unknown subcommand "${subcommand}"\n${HELP}`)
      return 2
  }
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(`✗ unexpected error: ${(err as Error).message}\n`)
    process.exit(2)
  })
