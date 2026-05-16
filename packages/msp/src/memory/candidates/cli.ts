#!/usr/bin/env node
/**
 * `msp-candidate` — propose and inspect candidate atoms via MSP.
 *
 * This is the CLI entry point for non-MCP agents (Gemini, Qwen) that need
 * to write atoms through MSP without direct GKS access. MCP-capable agents
 * (Claude) should use the `msp_candidate` MCP tool instead.
 *
 * Authority: ADR--AGENT-WRITE-BOUNDARIES (MSP as sole write gateway).
 */
import { resolve } from 'node:path'
import { parseArgs } from 'node:util'

import { CandidateWriter } from './writer.js'
import {
  ATOM_TYPES_UPPER,
  CandidateIdError,
  CandidateNotFoundError,
} from './types.js'

const TYPES_DISPLAY = ATOM_TYPES_UPPER.join(' | ')

const HELP = `msp-candidate — propose and inspect candidate atoms via MSP

Usage:
  msp-candidate propose --id=<ID> --type=<TYPE> --title=<TITLE> [options]
  msp-candidate list [--namespace=<ns>] [--root=<dir>]
  msp-candidate read <ID> [--namespace=<ns>] [--root=<dir>]
  msp-candidate delete <ID> [--namespace=<ns>] [--root=<dir>]
  msp-candidate --help

Subcommands:
  propose     Write a candidate atom to the MSP inbound queue
  list        List all pending candidates in the namespace
  read        Show full content of a candidate (JSON)
  delete      Remove a candidate from the queue

Options (propose):
  --id=<ID>           Atom ID, e.g. FEAT--MY-FEATURE (required)
  --type=<TYPE>       ${TYPES_DISPLAY} (required)
  --title=<TITLE>     Human-readable title (required)
  --body=<TEXT>       Initial markdown body (default: empty)
  --rationale=<TEXT>  Why this atom is proposed
  --confidence=<N>    Agent confidence 0.0–1.0
  --namespace=<ns>    MSP project namespace (default: evaAI)
  --root=<dir>        Project root (default: cwd)
  --help              Show this message

Exit codes:
  0  success
  1  not found / no candidates
  2  bad arguments or IO failure
`

function resolveRoot(raw: string | undefined): string {
  return resolve(raw ?? process.cwd())
}

async function runPropose(values: {
  id?: string
  type?: string
  title?: string
  body?: string
  rationale?: string
  confidence?: string
  namespace?: string
  root?: string
}): Promise<number> {
  const { id, type, title, body, rationale, confidence, namespace, root } =
    values

  if (!id) {
    process.stderr.write(`error: --id is required\n${HELP}`)
    return 2
  }
  if (!type) {
    process.stderr.write(`error: --type is required\n${HELP}`)
    return 2
  }
  if (!title) {
    process.stderr.write(`error: --title is required\n${HELP}`)
    return 2
  }

  const writer = new CandidateWriter({
    root: resolveRoot(root),
    namespace,
    proposedBy: 'agent',
  })

  try {
    const result = await writer.write({
      proposed_id: id,
      type: type.toLowerCase(),
      title,
      body: body ?? '',
      rationale,
      confidence:
        confidence !== undefined ? parseFloat(confidence) : undefined,
    })
    process.stdout.write(
      `✓ candidate ${result.overwritten ? 'updated' : 'created'}: ${result.path}\n`,
    )
    return 0
  } catch (err) {
    if (err instanceof CandidateIdError) {
      process.stderr.write(`error: ${err.message}\n`)
      return 2
    }
    process.stderr.write(`error: ${(err as Error).message}\n`)
    return 2
  }
}

async function runList(values: {
  namespace?: string
  root?: string
}): Promise<number> {
  const writer = new CandidateWriter({
    root: resolveRoot(values.root),
    namespace: values.namespace,
  })
  const candidates = await writer.list()
  if (candidates.length === 0) {
    process.stdout.write('no candidates found\n')
    return 1
  }
  for (const c of candidates) {
    process.stdout.write(
      `${c.proposed_id.padEnd(40)}  ${c.type.padEnd(12)}  ${c.proposed_at ?? ''}  ${c.title}\n`,
    )
  }
  return 0
}

async function runRead(
  id: string,
  values: { namespace?: string; root?: string },
): Promise<number> {
  const writer = new CandidateWriter({
    root: resolveRoot(values.root),
    namespace: values.namespace,
  })
  try {
    const record = await writer.read(id)
    process.stdout.write(JSON.stringify(record, null, 2) + '\n')
    return 0
  } catch (err) {
    if (err instanceof CandidateNotFoundError) {
      process.stderr.write(`error: ${err.message}\n`)
      return 1
    }
    process.stderr.write(`error: ${(err as Error).message}\n`)
    return 2
  }
}

async function runDelete(
  id: string,
  values: { namespace?: string; root?: string },
): Promise<number> {
  const writer = new CandidateWriter({
    root: resolveRoot(values.root),
    namespace: values.namespace,
  })
  try {
    await writer.delete(id)
    process.stdout.write(`✓ deleted: ${id}\n`)
    return 0
  } catch (err) {
    if (err instanceof CandidateNotFoundError) {
      process.stderr.write(`error: ${err.message}\n`)
      return 1
    }
    process.stderr.write(`error: ${(err as Error).message}\n`)
    return 2
  }
}

async function main(): Promise<number> {
  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        id: { type: 'string' },
        type: { type: 'string' },
        title: { type: 'string' },
        body: { type: 'string' },
        rationale: { type: 'string' },
        confidence: { type: 'string' },
        namespace: { type: 'string' },
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

  const sub = positionals[0]

  if (!sub || sub === 'propose') {
    return runPropose(values)
  }

  if (sub === 'list') {
    return runList(values)
  }

  if (sub === 'read') {
    const id = positionals[1]
    if (!id) {
      process.stderr.write(`error: read requires an atom ID\n${HELP}`)
      return 2
    }
    return runRead(id, values)
  }

  if (sub === 'delete') {
    const id = positionals[1]
    if (!id) {
      process.stderr.write(`error: delete requires an atom ID\n${HELP}`)
      return 2
    }
    return runDelete(id, values)
  }

  process.stderr.write(`error: unknown subcommand "${sub}"\n${HELP}`)
  return 2
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(`✗ unexpected error: ${(err as Error).message}\n`)
    process.exit(2)
  })
