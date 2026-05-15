import type { SubagentBudget, SubagentScope } from '../policy/task-scope.js'

export interface Task {
  id: string
  parent_blueprint: string
  status?: string
  prompt: string
  acceptance: string[]
  geography: string[]
  assignee?: string
  created_at?: string
  /** §9 — Task-level scope filtering (UCF Phase 2). */
  scope?: SubagentScope
  /** §11.3 — Context resolution budget (UCF Phase 3). */
  budget?: SubagentBudget
}

export interface Blueprint {
  id: string
  status: string
  body: string
  geography?: string[]
}

export interface RunOptions {
  model?: string
  maxRetries?: number
  escalate?: boolean
  dryRun?: boolean
  sandbox?: string
  /** Pluggable SLM client; defaults to a deterministic mock for tests. */
  slmClient?: SlmClient
  /** Pluggable acceptance runner; defaults to evaluating a JS expression. */
  acceptanceRunner?: AcceptanceRunner
  /** Pluggable Gemini escalator; defaults to no-op. */
  escalator?: Escalator
}

export interface AttemptRecord {
  attempt: number
  promptHash: string
  slmModel: string
  rawOutput: string
  cleanedOutput: string
  patternErrors: string[]
  acceptanceErrors: string[]
}

export type FinalStatus =
  | 'success'
  | 'pattern-fail'
  | 'acceptance-fail'
  | 'escalated-success'
  | 'escalated-fail'

export interface RunResult {
  taskId: string
  attempts: AttemptRecord[]
  finalStatus: FinalStatus
  exitCode: 0 | 1 | 2 | 3 | 4
  escalation?: { layer: 'gemini' | 'opus'; outcome: 'pass' | 'fail' }
  output?: string
}

export interface SlmCall {
  prompt: string
  model: string
  attempt: number
  lastFailure?: { kind: 'pattern' | 'test'; details: string[] }
}

export type SlmClient = (call: SlmCall) => Promise<string>
export type AcceptanceRunner = (task: Task, code: string) => Promise<string[]>
export type Escalator = (task: Task, blueprint: Blueprint, history: AttemptRecord[]) => Promise<{ ok: boolean; output?: string }>

export class CodegenError extends Error {
  constructor(message: string, public readonly cause?: unknown) {
    super(message)
    this.name = 'CodegenError'
  }
}
