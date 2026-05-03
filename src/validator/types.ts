export interface ParsedAtom {
  fm: Record<string, unknown>
  body: string
  source: string
  filepath: string
}

export interface Wikilink {
  id: string
  line: number
  column: number
}

export type Severity = 'error' | 'warning'

export interface ValidationError {
  rule: string
  severity: Severity
  line?: number
  column?: number
  message: string
  offending?: string
}

export interface AtomicIndexEntry {
  id: string
  type: string
  status: string
  path: string
  phase?: number
  vault_id?: string
  title?: string
  tags?: string[]
  crosslinks?: Record<string, string[]>
  linked_symbols?: unknown[]
  geography?: string[]
}

export interface RequiredFieldsConfig {
  default: ReadonlyArray<string>
  byType: ReadonlyMap<string, ReadonlyArray<string>>
}

export interface ValidationContext {
  atomicIndex: Map<string, AtomicIndexEntry>
  forbiddenFields?: ReadonlySet<string>
  requiredFields?: RequiredFieldsConfig
  now?: Date
}

export interface ValidationResult {
  filepath: string
  errors: ValidationError[]
  warnings: ValidationError[]
}

export type Rule = (atom: ParsedAtom, ctx: ValidationContext) => ValidationError[]

export class ValidatorIOError extends Error {
  constructor(
    message: string,
    public readonly cause?: unknown,
  ) {
    super(message)
    this.name = 'ValidatorIOError'
  }
}
