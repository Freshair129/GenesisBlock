import { parse as yamlParse } from 'yaml'
import { buildAliases, lookupType } from '../../validator/utils/registry.js'
import {
  CandidateIdError,
  type CandidateFrontmatter,
  type CandidateWriteInput,
} from './types.js'

const ID_PATTERN = /^(CONCEPT|ADR|FEAT|BLUEPRINT|FRAME|AUDIT|PROTO)--[A-Z0-9-]+$/

export function assertValidProposedId(id: string): void {
  if (!ID_PATTERN.test(id)) throw new CandidateIdError(id)
}

function yamlScalar(s: string): string {
  if (/[:#\n"\\]/.test(s) || s !== s.trim() || s === '') return JSON.stringify(s)
  return s
}

export function composeFrontmatter(
  input: CandidateWriteInput,
  proposedAt: string,
  proposedBy: 'agent' | 'human',
): string {
  const lines: string[] = ['---']
  lines.push(`proposed_id: ${input.proposed_id}`)
  lines.push(`type: ${input.type}`)
  lines.push(`status: candidate`)
  const aliases = buildAliases(input.proposed_id, undefined, process.cwd())
  lines.push('aliases:')
  for (const alias of aliases) {
    lines.push(`  - ${alias}`)
  }
  const prefix = input.proposed_id.split('--')[0]!
  const typeDef = lookupType(prefix, process.cwd())
  if (typeDef) {
    lines.push(`cluster: ${yamlScalar(typeDef.cluster)}`)
    lines.push(`role: ${yamlScalar(typeDef.role)}`)
  }
  lines.push(`proposed_at: ${proposedAt}`)
  lines.push(`proposed_by: ${proposedBy}`)
  if (input.rationale !== undefined) lines.push(`rationale: ${yamlScalar(input.rationale)}`)
  if (input.confidence !== undefined) lines.push(`confidence: ${input.confidence}`)
  lines.push('---')
  return lines.join('\n')
}

export function parseFrontmatter(raw: string): { fm: CandidateFrontmatter; body: string } {
  const match = raw.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/)
  if (!match) throw new Error('Candidate file: missing or malformed frontmatter')
  const yamlText = match[1]!
  const body = match[2] ?? ''
  let fm: any
  try {
    fm = yamlParse(yamlText)
  } catch (err: any) {
    throw new Error(`Candidate file: invalid YAML frontmatter: ${err.message}`)
  }
  return { fm: fm as unknown as CandidateFrontmatter, body }
}

export function extractTitle(body: string): string {
  for (const line of body.split(/\r?\n/)) {
    const h = line.match(/^#\s+(.*)$/)
    if (h) return h[1]!.trim()
  }
  for (const line of body.split(/\r?\n/)) {
    const t = line.trim()
    if (t) return t
  }
  return ''
}

export function composeBody(title: string, body: string): string {
  const trimmed = body.trimEnd()
  if (/^#\s+/.test(body.trimStart())) return trimmed
  if (title.trim() === '') return trimmed
  return `# ${title}\n\n${trimmed}`
}
