#!/usr/bin/env node
import { resolve } from 'node:path'
import { parseArgs } from 'node:util'

import { loadAtomicIndex, validate, validateAll } from './index.js'
import { loadContract } from './contract.js'
import { discoverProtos, runProtos, shouldFailExit } from './proto/loader.js'
import type { ProtoSummary } from './proto/types.js'
import { ValidatorIOError, type ValidationResult } from './types.js'

const HELP = `msp-validate — schema/ID/wikilink/anti-hallucination gate over MSP atoms

Usage:
  msp-validate <file>...
  msp-validate --all [--root=<dir>]
  msp-validate --json <file>...

Flags:
  --all              walk gks/ + .brain/msp/projects/<ns>/inbound/ recursively
  --root=<dir>       project root (default: cwd)
  --index=<path>     atomic index path (default: <root>/gks/00_index/atomic_index.jsonl)
  --json             machine-readable output
  --help             this message

Exit codes:
  0  pass (warnings allowed)
  1  hard-rule violations
  2  internal error (missing index, malformed YAML, unreadable file)
`

function findProjectInbound(root: string): string {
  // Spec defaults to .brain/msp/projects/evaAI/inbound/ but we just probe
  // for any project under .brain/msp/projects/*/inbound/.
  return resolve(root, '.brain/msp/projects')
}

interface PrettyOpts {
  json: boolean
}

function pretty(results: ValidationResult[], opts: PrettyOpts): void {
  if (opts.json) {
    process.stdout.write(JSON.stringify(results, null, 2) + '\n')
    return
  }
  let pass = 0
  let fail = 0
  for (const r of results) {
    if (r.errors.length === 0) {
      pass++
      if (results.length > 1) console.log(`✓ ${r.filepath}`)
      else console.log(`✓ ${r.filepath}`)
    } else {
      fail++
      for (const e of r.errors) {
        const loc = e.line !== undefined ? `:${e.line}${e.column !== undefined ? `:${e.column}` : ''}` : ''
        console.log(`✗ ${r.filepath}${loc} [${e.rule}] ${e.message}`)
      }
    }
    for (const w of r.warnings) {
      const loc = w.line !== undefined ? `:${w.line}${w.column !== undefined ? `:${w.column}` : ''}` : ''
      console.log(`! ${r.filepath}${loc} [${w.rule}] ${w.message}`)
    }
  }
  if (results.length > 1) {
    console.log(`\nTotal: ${pass} passed, ${fail} failed`)
  }
}

async function main(): Promise<number> {
  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        all: { type: 'boolean' },
        root: { type: 'string' },
        index: { type: 'string' },
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

  const root = resolve(values.root ?? process.cwd())
  const indexPath = resolve(values.index ?? `${root}/gks/00_index/atomic_index.jsonl`)

  let atomicIndex
  try {
    atomicIndex = await loadAtomicIndex(indexPath)
  } catch (err) {
    if (err instanceof ValidatorIOError) {
      process.stderr.write(`✗ ${err.message}\n`)
      return 2
    }
    throw err
  }

  // Load runtime contract; falls back to defaults if YAML missing/invalid.
  const contract = await loadContract(root)
  if (contract.warnings.length > 0 && !values.json) {
    for (const w of contract.warnings) process.stderr.write(`⚠ ${w}\n`)
  }

  const ctx = {
    atomicIndex,
    forbiddenFields: contract.forbiddenFields,
    requiredFields: contract.requiredFields,
  }
  let results: ValidationResult[]

  if (values.all) {
    const dirs = [resolve(root, 'gks'), findProjectInbound(root)]
    results = await validateAll(dirs, ctx)
  } else if (positionals.length > 0) {
    results = []
    for (const p of positionals) {
      results.push(await validate(resolve(p), ctx))
    }
  } else {
    process.stderr.write(`error: no files given (use --all or pass paths)\n${HELP}`)
    return 2
  }

  pretty(results, { json: values.json === true })

  // Run PROTO predicates (M8a — gks/proto/PROTO--*.md + src/validator/proto/<name>.ts)
  let protoSummary: ProtoSummary | null = null
  if (values.all && !values.json) {
    const metas = await discoverProtos(root)
    const indexArray = Array.from(atomicIndex.values())
    protoSummary = await runProtos(metas, {
      atomicIndex: indexArray,
      repoRoot: root,
    })
    prettyProtos(protoSummary)
  }

  const hardFail = results.some((r) => r.errors.length > 0)
  const protoFail = protoSummary ? shouldFailExit(protoSummary) : false
  return hardFail || protoFail ? 1 : 0
}

function prettyProtos(summary: ProtoSummary): void {
  if (summary.total === 0) return
  for (const r of summary.results) {
    if (r.meta.status === 'superseded') continue
    const tag =
      r.meta.status === 'draft' ? `[${r.meta.status}, ${r.meta.severity}]` : `[${r.meta.severity}]`
    if (r.result.ok) {
      console.log(`✓ ${r.meta.id} ${tag}`)
    } else {
      console.log(`✗ ${r.meta.id} ${tag}`)
      for (const v of r.result.violations) {
        const where = v.atomId ? ` (${v.atomId})` : ''
        console.log(`    [${v.severity}]${where} ${v.message}`)
      }
    }
  }
  console.log(`PROTOs: ${summary.passed} passed, ${summary.failed} failed`)
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(`✗ unexpected error: ${(err as Error).message}\n`)
    process.exit(2)
  })
