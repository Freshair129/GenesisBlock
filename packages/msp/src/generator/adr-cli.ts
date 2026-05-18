#!/usr/bin/env node
import { writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { parseArgs } from 'node:util'

import { draftAdrContent } from './adr-engine.js'
import { getStagedDiff } from '../utils/git.js'

const HELP = `msp-adr — automate the drafting of GKS Architecture Decision Records (ADRs)

Usage:
  msp-adr draft [--hint "<text>"] [--staged] [--root <dir>] [--provider <p>]

Flags:
  --staged           Draft based on currently staged git changes
  --hint="<text>"    Guidance for the LLM's understanding of the decision
  --root=<dir>       Project root (default: cwd)
  --provider=<p>     LLM backend: ollama|mock|qwen|gemini (default: ollama)
  --help             This message

Example:
  msp-adr draft --staged --hint "Using a central registry for all vault paths"
`

/**
 * Generates the current timestamp with ICT (+07:00) offset.
 */
function getIctTimestamp() {
  const now = new Date()
  const y = now.getFullYear()
  const m = String(now.getMonth() + 1).padStart(2, '0')
  const d = String(now.getDate()).padStart(2, '0')
  const h = String(now.getHours()).padStart(2, '0')
  const min = String(now.getMinutes()).padStart(2, '0')
  const s = String(now.getSeconds()).padStart(2, '0')
  return `${y}-${m}-${d}T${h}:${min}:${s}+07:00`
}

async function main(): Promise<number> {
  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        staged: { type: 'boolean' },
        hint: { type: 'string' },
        root: { type: 'string' },
        provider: { type: 'string', default: 'ollama' },
        help: { type: 'boolean', short: 'h' },
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

  try {
    process.stdout.write('🔍 Analysing staged changes...\n')
    const diffResult = await getStagedDiff()
    
    if (values.staged && diffResult.files.length === 0) {
      process.stderr.write('✗ No staged changes found. Use `git add` first or omit --staged.\n')
      return 1
    }

    process.stdout.write(`📝 Drafting ADR content using ${values.provider}...\n`)
    const draft = await draftAdrContent(diffResult, {
      provider: values.provider as any,
      hint: values.hint,
    })

    const atomId = `ADR--${draft.suggestedSlug.toUpperCase()}`
    const timestamp = getIctTimestamp()
    
    const content = `---
id: ${atomId}
phase: 2
type: adr
status: raw
vault_id: default
tier: process
source_type: axiomatic
title: ADR — ${draft.suggestedSlug.replace(/-/g, ' ')}
tags: [msp, architecture, automation, generated]
aliases: [ADR, implementation_flow, Architecture decision record]
cluster: implementation_flow
role: Architecture decision record
created_at: ${timestamp}
---

# ADR — ${draft.suggestedSlug.replace(/-/g, ' ')}

## Context

${draft.context}

## Decision

${draft.decision}

## Consequences

${draft.consequences}

## Source

- Auto-generated via msp-adr from staged diff.
- Hint: ${values.hint || 'None'}
`

    const filePath = join(root, 'gks', 'adr', `${atomId}.md`)
    await writeFile(filePath, content)

    process.stdout.write(`\n✅ ADR successfully drafted!\n`)
    process.stdout.write(`ID:   ${atomId}\n`)
    process.stdout.write(`Path: ${filePath}\n`)
    process.stdout.write(`\nReview and update the status to 'draft' or 'stable' when ready.\n`)

  } catch (err) {
    process.stderr.write(`✗ ADR drafting failed: ${(err as Error).message}\n`)
    return 1
  }

  return 0
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(`✗ unexpected error: ${(err as Error).message}\n`)
    process.exit(2)
  })
