import type { Namespace } from '@freshair129/gks'

/** Opaque key-value metadata bag for domain-specific attributes. */
export type JsonValue = string | number | boolean | null | { [key: string]: JsonValue } | JsonValue[]
export type AttributeBag = Record<string, JsonValue>

/** Initiator of a request. */
export type SubjectKind = 'user' | 'subagent' | 'service' | 'scheduled-job' | 'mcp-client'
export interface Subject {
  kind: SubjectKind
  id: string
  attributes: AttributeBag
}

/** Target of an action. */
export type ResourceKind = 'atom' | 'episode' | 'vector-doc' | 'tool-result' | 'context-slot'
export interface Resource {
  kind: ResourceKind
  id: string
  namespace: Namespace
  attributes: AttributeBag
}

/** Intent of the request. */
export type Action =
  | 'read'
  | 'recall'
  | 'embed'
  | 'expose-to-llm'
  | 'summarize'
  | 'write'
  | 'modify'
  | 'delete'
  | 'cite'

/** Environmental request metadata. */
export type RequestOrigin = 'http' | 'mcp-stdio' | 'cli' | 'internal'
export interface RequestContext {
  time: Date
  origin: RequestOrigin
  trace_id: string
  network?: string
  purpose?: string
  scale_level?: 'L1' | 'L2' | 'L3'
}

/** Policy Decision Point (PDP) response. */
export type DecisionEffect = 'permit' | 'deny' | 'indeterminate'

export interface Obligation {
  kind: string
  params?: Record<string, any>
}

export interface Advice {
  kind: string
  params?: Record<string, any>
}

export interface ReasonTrace {
  rule_id?: string
  description: string
  matched: boolean
}

export interface Decision {
  effect: DecisionEffect
  obligations: Obligation[]
  advice: Advice[]
  reasoning: ReasonTrace[]
  ttl_seconds?: number
}

// Constructors

export function makeSubject(kind: SubjectKind, id: string, attributes: AttributeBag = {}): Subject {
  return { kind, id, attributes }
}

export function makeResource(
  kind: ResourceKind,
  id: string,
  namespace: Namespace = {},
  attributes: AttributeBag = {},
): Resource {
  return { kind, id, namespace, attributes }
}

export function makeContext(
  origin: RequestOrigin,
  trace_id: string,
  overrides: Partial<RequestContext> = {},
): RequestContext {
  return {
    time: overrides.time ?? new Date(),
    origin,
    trace_id,
    ...overrides,
  }
}

export function makeDecision(effect: DecisionEffect, reasoning: string | ReasonTrace[]): Decision {
  return {
    effect,
    obligations: [],
    advice: [],
    reasoning: typeof reasoning === 'string' ? [{ description: reasoning, matched: true }] : reasoning,
  }
}
