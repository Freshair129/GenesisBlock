#!/usr/bin/env node
import { readFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { parseArgs } from 'node:util'
import { parse as parseYaml } from 'yaml'
import { loadPolicies } from './loader.js'
import { evaluatePolicy } from './pdp.js'
import { makeContext, makeResource, makeSubject } from './types.js'
import { forEachJsonl } from '@freshair129/gks'

const HELP = `msp-policy — UCF policy management and shadow reporting

Usage:
  msp-policy lint <file>...
  msp-policy explain <subject_id> <resource_id> <action> [--root=<dir>]
  msp-policy report [--root=<dir>]

Flags:
  --root=<dir>       project root (default: cwd)
  --help             this message
`

async function main(): Promise<number> {
  const { values, positionals } = parseArgs({
    options: {
      root: { type: 'string' },
      help: { type: 'boolean' },
    },
    allowPositionals: true,
  })

  if (values.help || positionals.length === 0) {
    process.stdout.write(HELP)
    return 0
  }

  const root = resolve(values.root ?? process.cwd())
  const command = positionals[0]

  switch (command) {
    case 'lint':
      return await doLint(positionals.slice(1))
    case 'explain':
      return await doExplain(root, positionals[1], positionals[2], positionals[3])
    case 'report':
      return await doReport(root)
    default:
      process.stderr.write(`error: unknown command ${command}\n${HELP}`)
      return 1
  }
}

async function doLint(files: string[]): Promise<number> {
  let errors = 0
  for (const f of files) {
    try {
      const text = await readFile(f, 'utf8')
      const parsed = parseYaml(text)
      // Basic schema validation happens here (implicitly via zod if we used it in a separate validator)
      // For now, loadPolicies already logs errors.
      process.stdout.write(`✓ ${f} parsed successfully\n`)
    } catch (err) {
      process.stderr.write(`✗ ${f}: ${(err as Error).message}\n`)
      errors++
    }
  }
  return errors > 0 ? 1 : 0
}

async function doExplain(
  root: string,
  subId?: string,
  resId?: string,
  action?: string,
): Promise<number> {
  if (!subId || !resId || !action) {
    process.stderr.write('error: explain requires <subject_id> <resource_id> <action>\n')
    return 1
  }

  const policiesDir = join(root, 'policies')
  const policySet = await loadPolicies(policiesDir)

  const subject = makeSubject('user', subId)
  const resource = makeResource('atom', resId)
  const context = makeContext('cli', 'explain-cli')

  const decision = evaluatePolicy(subject, resource, action as any, context, policySet)

  process.stdout.write(`Decision: ${decision.effect.toUpperCase()}\n\nReasoning:\n`)
  for (const t of decision.reasoning) {
    const status = t.matched ? '[MATCH]' : '[SKIP]'
    process.stdout.write(`${status} ${t.rule_id ?? 'default'}: ${t.description}\n`)
  }

  return 0
}

async function doReport(root: string): Promise<number> {
  const logPath = join(root, '.brain', 'msp', 'audit', 'shadow-policy.jsonl')
  let total = 0
  let wouldDeny = 0
  const deniedActions = new Map<string, number>()

  try {
    await forEachJsonl<any>(logPath, (entry) => {
      total++
      if (entry.decision.effect === 'deny') {
        wouldDeny++
        const key = `${entry.action}:${entry.subject.kind}`
        deniedActions.set(key, (deniedActions.get(key) ?? 0) + 1)
      }
    })
  } catch (err) {
    process.stderr.write(`error: could not read shadow log: ${(err as Error).message}\n`)
    return 1
  }

  process.stdout.write(`Shadow Policy Report\n`)
  process.stdout.write(`====================\n`)
  process.stdout.write(`Total decisions:  ${total}\n`)
  process.stdout.write(`Would have denied: ${wouldDeny} (${((wouldDeny / total) * 100).toFixed(1)}%)\n\n`)

  if (wouldDeny > 0) {
    process.stdout.write(`Top "Denied" Actions:\n`)
    for (const [action, count] of deniedActions.entries()) {
      process.stdout.write(`  - ${action}: ${count}\n`)
    }
  }

  return 0
}

main().then((code) => process.exit(code))
