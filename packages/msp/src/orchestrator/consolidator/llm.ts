import type { LlmClient, Tier2Result, Turn } from './types.js'
import { DEFAULT_LLM_TIMEOUT_MS } from './types.js'

const PROMPT_HEAD = `You are scoring the importance of an agent conversation chunk for long-term memory.

Conversation chunk:
---`

const PROMPT_TAIL = `---

Return JSON ONLY in this exact shape (no prose, no markdown fences):
{ "score": <0..1>, "summary": "<1 sentence>", "tags": ["<keyword>", ...] }

Scoring rubric:
  0.0–0.3: clearly forgettable (greeting, dead end, redundant)
  0.3–0.6: ambiguous — slight value if context is needed later
  0.6–1.0: clear keeper (decision, fact, learning, code reference)

Pick 3–5 tags (lowercase keywords or atom IDs).`

/**
 * Default-keep result returned when the LLM is unavailable, times out,
 * returns malformed JSON, or any other failure mode (per ADR table).
 */
export function defaultKeepResult(): Tier2Result {
  return {
    score: 0.6,
    summary: '',
    tags: [],
    source: 'tier2-default',
  }
}

/**
 * Build the prompt for tier-2 importance scoring.
 */
export function buildTier2Prompt(chunk: Turn[]): string {
  const body = chunk
    .map((t) => `[${t.speakerId}] ${t.content}`)
    .join('\n')
  return `${PROMPT_HEAD}\n${body}\n${PROMPT_TAIL}\n`
}

/**
 * Extract a JSON object from a possibly noisy LLM response.
 * Supports plain JSON, JSON inside ```json fences, and fenceless objects.
 */
export function extractJsonObject(text: string): unknown | null {
  if (!text) return null
  const trimmed = text.trim()

  // Try direct parse first.
  try {
    return JSON.parse(trimmed)
  } catch {
    /* fall through */
  }

  // Try to strip a leading fence.
  const fenceMatch = trimmed.match(/```(?:json)?\s*([\s\S]*?)```/i)
  if (fenceMatch && fenceMatch[1]) {
    try {
      return JSON.parse(fenceMatch[1].trim())
    } catch {
      /* fall through */
    }
  }

  // Last resort: find the first balanced { ... } block.
  const start = trimmed.indexOf('{')
  if (start < 0) return null
  let depth = 0
  for (let i = start; i < trimmed.length; i++) {
    const c = trimmed[i]
    if (c === '{') depth++
    else if (c === '}') {
      depth--
      if (depth === 0) {
        const candidate = trimmed.slice(start, i + 1)
        try {
          return JSON.parse(candidate)
        } catch {
          return null
        }
      }
    }
  }
  return null
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v)
}

/**
 * Validate + normalise a tier-2 JSON response.
 * Returns null if the shape is unusable.
 */
export function parseTier2Response(parsed: unknown): {
  score: number
  summary: string
  tags: string[]
} | null {
  if (!isPlainObject(parsed)) return null
  const rawScore = parsed.score
  const score =
    typeof rawScore === 'number'
      ? rawScore
      : typeof rawScore === 'string'
        ? Number.parseFloat(rawScore)
        : NaN
  if (!Number.isFinite(score)) return null
  const clamped = score < 0 ? 0 : score > 1 ? 1 : score

  const summary =
    typeof parsed.summary === 'string' ? parsed.summary.trim() : ''

  let tags: string[] = []
  if (Array.isArray(parsed.tags)) {
    tags = parsed.tags
      .filter((t): t is string => typeof t === 'string')
      .map((t) => t.trim())
      .filter((t) => t.length > 0)
      .slice(0, 5)
  }

  return { score: clamped, summary, tags }
}

export interface CallTier2Opts {
  llm?: LlmClient
  timeoutMs?: number
  /** Optional model name forwarded to the SLM client. Default 'consolidator'. */
  model?: string
  /** Optional injected `setTimeout`/`clearTimeout` for test determinism. */
  setTimeoutImpl?: typeof setTimeout
  clearTimeoutImpl?: typeof clearTimeout
}

/**
 * Tier-2 caller. Builds the JSON-importance prompt, invokes the LLM with
 * a hard timeout, parses the response, and falls back to a default-keep
 * result on any failure mode.
 */
export async function callTier2(
  chunk: Turn[],
  opts: CallTier2Opts = {},
): Promise<Tier2Result> {
  if (!opts.llm) return defaultKeepResult()

  const timeoutMs = opts.timeoutMs ?? DEFAULT_LLM_TIMEOUT_MS
  const model = opts.model ?? 'consolidator'
  const prompt = buildTier2Prompt(chunk)

  const setT = opts.setTimeoutImpl ?? setTimeout
  const clearT = opts.clearTimeoutImpl ?? clearTimeout

  // Race the LLM call against a timeout. We don't have an AbortController
  // path through SlmClient, so we fall back to Promise.race + the LLM
  // implementation's own internal timeout (Ollama has one).
  let timer: ReturnType<typeof setTimeout> | null = null
  const timeoutPromise = new Promise<'__timeout__'>((resolve) => {
    timer = setT(() => resolve('__timeout__'), timeoutMs)
  })

  let result: string | '__timeout__'
  try {
    result = await Promise.race([
      opts.llm({ prompt, model, attempt: 1 }),
      timeoutPromise,
    ])
  } catch {
    return defaultKeepResult()
  } finally {
    if (timer !== null) clearT(timer)
  }

  if (result === '__timeout__') return defaultKeepResult()

  const parsed = extractJsonObject(result)
  if (parsed === null) return defaultKeepResult()
  const ok = parseTier2Response(parsed)
  if (!ok) return defaultKeepResult()

  return {
    score: ok.score,
    summary: ok.summary,
    tags: ok.tags,
    source: 'tier2',
  }
}
