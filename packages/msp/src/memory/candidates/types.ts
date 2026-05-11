export const ATOM_TYPES_UPPER = [
  'CONCEPT',
  'ADR',
  'FEAT',
  'BLUEPRINT',
  'FRAME',
  'AUDIT',
  'PROTO',
] as const

export type AtomTypeUpper = (typeof ATOM_TYPES_UPPER)[number]

export interface CandidateWriteInput {
  type: string
  proposed_id: string
  title: string
  body: string
  rationale?: string
  confidence?: number
}

export interface CandidateFrontmatter {
  proposed_id: string
  type: string
  status: 'candidate'
  proposed_at: string
  proposed_by: 'agent' | 'human'
  rationale?: string
  confidence?: number
}

export interface CandidateSummary extends CandidateFrontmatter {
  title: string
  path: string
}

export interface CandidateRecord extends CandidateSummary {
  body: string
}

export interface CandidateWriteResult {
  path: string
  overwritten: boolean
}

export interface CandidateWriterOpts {
  root: string
  namespace?: string
  proposedBy?: 'agent' | 'human'
}

export class CandidateIdError extends Error {
  constructor(id: string) {
    super(
      `Invalid proposed_id "${id}" — must match /^(CONCEPT|ADR|FEAT|BLUEPRINT|FRAME|AUDIT|PROTO)--[A-Z0-9-]+$/`,
    )
    this.name = 'CandidateIdError'
  }
}

export class CandidateNotFoundError extends Error {
  constructor(id: string) {
    super(`Candidate not found: ${id}`)
    this.name = 'CandidateNotFoundError'
  }
}
