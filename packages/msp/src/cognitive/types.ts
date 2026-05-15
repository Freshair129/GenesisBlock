/**
 * Cognitive Layer types — public surface for `createCognitiveLayer`.
 *
 * The facade unifies the GKS storage primitives, the MSP passport
 * (identity / orchestration), and the codegen runner behind one entry
 * point so consumers (EVA, Claude Code, Hermes, openclaw, Cursor, custom
 * MCP agents) wire one factory call instead of stitching MemoryStore,
 * identity, mcp-server, and runner manually.
 *
 * The shape honours seven points from FRAMEWORK_MASTER_SPEC.md (see the
 * inline §-references below).
 */

import type {
  GraphBackend,
  MemoryStore,
  Namespace,
  RetrievalHit,
  RetrievalOptions,
  RetrievalResult,
} from '@freshair129/gks'
import type { SlmFactoryOpts } from '../codegen/slm/types.js'
import type { RunOptions, RunResult } from '../codegen/types.js'
import type { Action, RequestContext, Subject } from '../policy/types.js'
import type { SubagentScope } from '../policy/task-scope.js'

/** §17.3 — 3-tier agent mapping (T1 = Ollama+qwen2.5-coder, T2 = mid LLM, T3 = large LLM). */
export type CognitiveTier = 'T1' | 'T2' | 'T3'

/** §7.7.2 — scale-level gate (L1 = quick task, L2 = feature, L3 = critical/core). */
export type ScaleLevel = 'L1' | 'L2' | 'L3'

export interface CognitiveSlmOptions {
  /** Tier hint. Default `T1`. */
  tier?: CognitiveTier
  /** Direct provider override (passed through to `createSlmClient`). */
  provider?: SlmFactoryOpts['provider']
  /** Model override (Ollama only; Gemini reads $GEMINI_MODEL). */
  model?: string
  /** Pass-through to the SLM factory for advanced options. */
  factory?: SlmFactoryOpts
}

export interface CognitiveLayerOptions {
  /** Repo root — used by MemoryStore + codegen runner. */
  root: string
  /** Optional GraphBackend override; default = built-in GraphStore (JSONL). */
  graphBackend?: GraphBackend | ((root: string) => Promise<GraphBackend> | GraphBackend)
  /** SLM routing for `runTask`. */
  slm?: CognitiveSlmOptions
  /** Namespace stamped on retain / recall calls. */
  defaultNamespace?: Namespace
}

export interface PolicyContext {
  subject?: Subject
  action?: Action
  context?: RequestContext
}

export interface CognitiveRunTaskOptions extends RunOptions, PolicyContext {
  /** Scale level — drives the §7.7.2 gate. Default `L2`. */
  scale?: ScaleLevel
  /** Override the tier set at facade construction. */
  tier?: CognitiveTier
}

export type CognitiveRecallHit = RetrievalHit & {
  atomId: string
  /**
   * §7.5 — Memory-for-Audit guardrail. Stamped on hits from episodic /
   * session sources so callers know the content is for traceability /
   * summarisation only, not bulk context re-load.
   */
  audit_only?: boolean
}

export interface CognitiveRecallResult extends Omit<RetrievalResult, 'hits'> {
  hits: CognitiveRecallHit[]
  tookMs: number
  fallback_reasons?: string[]
}

export interface EscalationRequest {
  request_scope_extension?: string[]
  reason: string
}

export interface EscalationResult {
  approved: boolean
  updated_scope?: SubagentScope
}

export interface CognitiveLayer {
  /** Read path — §13 hybrid retrieval (atomic → FTS → vector → graph + RRF). */
  recall(query: string, opts?: RetrievalOptions & PolicyContext): Promise<CognitiveRecallResult>
  /** Write path — wraps `retain(store, …)`. */
  remember(content: string, opts?: RememberOptions & PolicyContext): Promise<{ id: string }>
  /** §9.3 — Subagent context expansion request. */
  escalate(req: EscalationRequest): Promise<EscalationResult>
  /** Session-end consolidation. */
  consolidate(sessionId: string): Promise<void>
  /** Codegen runner with tier routing + §7.7.2 gate. */
  runTask(taskPath: string, opts?: CognitiveRunTaskOptions): Promise<RunResult>
  /** Walk crosslinks from a FEAT (delegates to GKS verifyFlow). */
  verifyFlow(featId: string): Promise<unknown>
  /** §14.1 SSOT authority hierarchy resolver. */
  resolveSSOT(citations: AtomCitation[]): AtomCitation | null
  /** §6.4 hotfix escape hatch — re-exports the HotfixStore surface (lazy). */
  hotfix: HotfixHandle
  /** Underlying MemoryStore (escape hatch). */
  store: MemoryStore
  /** Underlying graph backend (escape hatch). */
  graph: GraphBackend
  /** Spawn an MCP server pre-wired with the 19 MSP tools. */
  mcpServer(): unknown
}

export interface RememberOptions {
  metadata?: Record<string, unknown>
  tags?: string[]
}

export interface AtomCitation {
  id: string
  /** atomic type — e.g. 'proto', 'master', 'adr', 'frame', 'concept'. */
  type: string
  source: 'code' | 'atom'
}

export interface HotfixHandle {
  open(args: { sha: string; reason: string }): Promise<unknown>
  list(): Promise<unknown[]>
  close(sha: string): Promise<unknown>
  check(): Promise<unknown>
}

export class ScaleLevelGateError extends Error {
  readonly scale: ScaleLevel
  readonly missing: string[]
  constructor(scale: ScaleLevel, missing: string[]) {
    super(
      `Scale-level ${scale} gate failed — required atoms missing or not stable: ${missing.join(', ')}`,
    )
    this.name = 'ScaleLevelGateError'
    this.scale = scale
    this.missing = missing
  }
}
