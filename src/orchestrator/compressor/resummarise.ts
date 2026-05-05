import { joinTurns } from './text.js'
import { DEFAULT_TOKENISER, estimateText } from './tokens.js'
import {
  DEFAULT_LLM_TIMEOUT_MS,
  type CompressorEpisode,
  type LlmClient,
  type Tokeniser,
} from './types.js'

const PROMPT_HEAD = `Re-summarise the following conversation chunk in approximately`
const PROMPT_BODY_HEAD = `tokens. Preserve key decisions, facts, and code references. Drop greetings, dead ends, and conversational filler.

Chunk:
---`
const PROMPT_TAIL = `---

Return ONLY the summary text (no preamble, no markdown fences).`

/**
 * Build the resummarise prompt sent to the tier-3 LLM. Includes the
 * target token count so the model can self-budget; instructs it to
 * preserve decisions / facts / code refs and drop conversational filler.
 */
export function buildResummarisePrompt(
  episode: CompressorEpisode,
  targetTokens: number,
): string {
  const body = joinTurns(episode.turns)
  return `${PROMPT_HEAD} ${targetTokens} ${PROMPT_BODY_HEAD}\n${body}\n${PROMPT_TAIL}\n`
}

/**
 * Strip common LLM noise: leading/trailing whitespace, accidental fenced
 * code blocks, leading "Summary:" prefixes. Conservative — only patterns
 * we've seen in tier-2 LLM testing.
 */
export function cleanLlmText(text: string): string {
  let out = text.trim()

  // Strip a single fenced block if the entire body is wrapped.
  const fence = out.match(/^```(?:[a-zA-Z]+)?\s*\n([\s\S]*?)\n```\s*$/)
  if (fence) {
    out = fence[1]!.trim()
  }

  // Strip a leading "Summary:" or "Re-summarised:" prefix.
  out = out.replace(/^(?:summary|re-?summarised|resummarised)\s*:\s*/i, '')

  return out
}

export interface ResummariseOpts {
  llm?: LlmClient
  timeoutMs?: number
  /** Optional model name forwarded to the SLM client. Default 'compressor'. */
  model?: string
  /** Optional injected setTimeout/clearTimeout for test determinism. */
  setTimeoutImpl?: typeof setTimeout
  clearTimeoutImpl?: typeof clearTimeout
}

export interface ResummariseResult {
  text: string
  /** Token cost of `text` under the active tokeniser. */
  tokens: number
}

/**
 * Tier-3 LLM-based resummarisation. Builds the prompt, races the LLM call
 * against a timeout, cleans the response, and verifies it actually fits
 * within `target`. Returns null on:
 *
 *  - no `llm` supplied (caller should fall through to truncate)
 *  - LLM throws / timeouts
 *  - empty cleaned response
 *  - response still exceeds `target` (no point shrinking-to-not-fit)
 *
 * On null, the caller MUST fall through to {@link truncate} so the
 * compressor stays headless-safe.
 */
export async function resummariseLLM(
  episode: CompressorEpisode,
  target: number,
  tokeniser: Tokeniser = DEFAULT_TOKENISER,
  opts: ResummariseOpts = {},
): Promise<ResummariseResult | null> {
  if (!opts.llm) return null
  if (target <= 0) return null

  const timeoutMs = opts.timeoutMs ?? DEFAULT_LLM_TIMEOUT_MS
  const model = opts.model ?? 'compressor'
  const prompt = buildResummarisePrompt(episode, target)

  const setT = opts.setTimeoutImpl ?? setTimeout
  const clearT = opts.clearTimeoutImpl ?? clearTimeout

  let timer: ReturnType<typeof setTimeout> | null = null
  const timeoutPromise = new Promise<'__timeout__'>((resolve) => {
    timer = setT(() => resolve('__timeout__'), timeoutMs)
  })

  let raw: string | '__timeout__'
  try {
    raw = await Promise.race([
      opts.llm({ prompt, model, attempt: 1 }),
      timeoutPromise,
    ])
  } catch {
    return null
  } finally {
    if (timer !== null) clearT(timer)
  }

  if (raw === '__timeout__') return null

  const cleaned = cleanLlmText(raw)
  if (!cleaned) return null

  const tokens = estimateText(cleaned, tokeniser)
  if (tokens > target) return null

  return { text: cleaned, tokens }
}

export interface TruncateResult {
  text: string
  droppedIndices: number[]
  tokens: number
}

/**
 * Deterministic tier-3 fallback: keep turns from the END of the episode
 * (most recent are typically most relevant in chat history) and drop
 * earlier turns until the remainder fits within `target`.
 *
 * Always cuts at turn boundaries — never mid-text. If even the last single
 * turn exceeds `target`, returns the empty string with all indices in
 * `droppedIndices`.
 *
 * Pure: does NOT mutate `episode`.
 */
export function truncate(
  episode: CompressorEpisode,
  target: number,
  tokeniser: Tokeniser = DEFAULT_TOKENISER,
): TruncateResult {
  const turns = episode.turns
  if (turns.length === 0) {
    return { text: '', droppedIndices: [], tokens: 0 }
  }

  // Walk backwards from the end, growing the kept window while it fits.
  let firstKeptIdx = turns.length // means "nothing kept yet"
  let lastFittingText = ''
  let lastFittingTokens = 0

  for (let i = turns.length - 1; i >= 0; i--) {
    const candidate = turns.slice(i, turns.length)
    const text = candidate.map((t) => `[${t.speakerId}] ${t.content}`).join('\n')
    const tokens = estimateText(text, tokeniser)
    if (tokens > target) break
    firstKeptIdx = i
    lastFittingText = text
    lastFittingTokens = tokens
  }

  // Build droppedIndices = everything BEFORE firstKeptIdx (chronological
  // order, by virtue of the loop).
  const droppedIndices: number[] = []
  for (let i = 0; i < firstKeptIdx; i++) droppedIndices.push(i)

  return {
    text: lastFittingText,
    droppedIndices,
    tokens: lastFittingTokens,
  }
}
