import { createHash } from 'node:crypto'

import type { Blueprint, Task } from './types.js'

export interface BuildOpts {
  task: Task
  blueprint: Blueprint
  attempt: number
  lastFailure?: { kind: 'pattern' | 'test'; details: string[] }
}

/**
 * Render the SLM prompt. Per ADR--CODEGEN-RETRY-POLICY:
 * - On retry, include the failed test / pattern match
 * - Strip previous attempt from context (we never include it; the caller
 *   constructs a fresh prompt every retry)
 */
export function buildPrompt({ task, blueprint, attempt, lastFailure }: BuildOpts): string {
  const sections: string[] = []
  sections.push(`# Task ${task.id} (attempt ${attempt})`)
  sections.push('')
  sections.push(`Parent blueprint: ${blueprint.id}`)
  sections.push('')
  sections.push('## Geography')
  for (const f of task.geography) sections.push(`- ${f}`)
  sections.push('')
  sections.push('## Acceptance')
  for (const c of task.acceptance) sections.push(`- ${c}`)
  sections.push('')
  if (lastFailure) {
    sections.push(`## Previous attempt failure (${lastFailure.kind})`)
    for (const d of lastFailure.details) sections.push(`- ${d}`)
    sections.push('')
    sections.push('Address the failure above. Output code only — no commentary, no fenced wrapping.')
    sections.push('')
  }
  sections.push('## System Instructions')
  sections.push('- Output code only. No commentary, no fenced wrapping.')
  sections.push('- If your task requires information outside your current context domains (e.g. missing atoms, denied access), use the `escalate()` tool to request a scope extension rather than guessing.')
  sections.push('')
  sections.push('## Prompt')
  sections.push(task.prompt)
  return sections.join('\n')
}

export function promptHash(prompt: string): string {
  return createHash('sha256').update(prompt).digest('hex').slice(0, 16)
}
