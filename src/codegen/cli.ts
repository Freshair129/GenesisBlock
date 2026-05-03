#!/usr/bin/env node
import { resolve } from 'node:path'
import { parseArgs } from 'node:util'

import { runTask } from './runner.js'

const HELP = `msp-run-task — execute one T*.task.yaml under the codegen contract

Usage:
  msp-run-task <path/to/T*.task.yaml> [flags]

Flags:
  --model=<name>     SLM identifier (default: mock-slm)
  --max-retries=<n>  default 3
  --no-escalate      fail at exit 1 instead of escalating to Gemini
  --dry-run          print the assembled prompt; don't call SLM
  --json             machine-readable output
  --help             this message

Exit codes:
  0  acceptance passed
  1  all retries failed (no escalation, or escalation disabled)
  2  internal error (YAML malformed, blueprint missing/draft, IO)
  3  escalated to Gemini and Gemini succeeded
  4  escalated and human review required (Opus layer)
`

async function main(): Promise<number> {
  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        model: { type: 'string' },
        'max-retries': { type: 'string' },
        'no-escalate': { type: 'boolean' },
        'dry-run': { type: 'boolean' },
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
    process.stderr.write(`error: no task path given\n${HELP}`)
    return 2
  }

  try {
    const result = await runTask(resolve(positionals[0]!), {
      model: values.model,
      maxRetries: values['max-retries'] ? Number.parseInt(values['max-retries'], 10) : undefined,
      escalate: values['no-escalate'] !== true,
      dryRun: values['dry-run'] === true,
    })
    if (values.json) {
      process.stdout.write(JSON.stringify(result, null, 2) + '\n')
    } else {
      process.stdout.write(`task ${result.taskId}: ${result.finalStatus} after ${result.attempts.length} attempt(s)\n`)
      if (result.escalation) process.stdout.write(`  escalation: ${result.escalation.layer} → ${result.escalation.outcome}\n`)
    }
    return result.exitCode
  } catch (err) {
    process.stderr.write(`✗ ${(err as Error).message}\n`)
    return 2
  }
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(`✗ unexpected: ${(err as Error).message}\n`)
    process.exit(2)
  })
