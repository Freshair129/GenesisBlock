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
  const yaml = match[1]!
  const body = match[2] ?? ''
  const fm: Record<string, unknown> = {}
  for (const line of yaml.split(/\r?\n/)) {
    const m = line.match(/^([a-z_]+):\s*(.*)$/i)
    if (!m) continue
    const key = m[1]!
    const rawVal = m[2]!
    if (key === 'confidence') {
      fm[key] = Number(rawVal)
    } else if (rawVal.startsWith('"') && rawVal.endsWith('"')) {
      fm[key] = JSON.parse(rawVal)
    } else {
      fm[key] = rawVal
    }
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
