import { createSlmClient } from '../codegen/slm/factory.js'
import type { LlmClient } from '../orchestrator/consolidator/types.js'
import type { GitDiffResult } from '../utils/git.js'

export interface AdrEngineOpts {
  provider?: 'ollama' | 'mock' | 'qwen' | 'gemini'
  model?: string
  hint?: string
}

export interface AdrDraft {
  context: string
  decision: string
  consequences: string
  suggestedSlug: string
}

const PROMPT_TEMPLATE = (diff: string, hint: string) => `You are an expert software architect. Draft a standards-compliant Architecture Decision Record (ADR) based on the following code changes and optional user hint.

CODE CHANGES (git diff):
---
${diff}
---

USER HINT:
---
${hint || 'No hint provided.'}
---

INSTRUCTIONS:
1. Analyse the intent of the changes.
2. Provide content for three specific ADR sections:
   - CONTEXT: Why is this change being made? What was the previous state?
   - DECISION: What is the specific technical choice made here?
   - CONSEQUENCES: What are the pros and cons of this choice?
3. Suggest a semantic SCREAMING-KEBAB-CASE slug for the ADR ID (e.g. DYNAMIC-COMPOUND-ID).

OUTPUT FORMAT (JSON):
{
  "context": "string",
  "decision": "string",
  "consequences": "string",
  "suggestedSlug": "string"
}`

/**
 * Core engine for generating ADR content using an LLM.
 */
export async function draftAdrContent(
  diffResult: GitDiffResult,
  opts: AdrEngineOpts = {},
): Promise<AdrDraft> {
  const provider = opts.provider ?? 'ollama'
  const model = opts.model ?? 'qwen-coder' // Default to a coding-specialised model
  const slmFunc = createSlmClient({ provider })

  const prompt = PROMPT_TEMPLATE(diffResult.diff, opts.hint || '')

  try {
    const raw = await slmFunc({ prompt, model, attempt: 1 })
    
    // Attempt to parse JSON from the response (LLMs sometimes wrap JSON in markdown blocks)
    const jsonMatch = raw.match(/\{[\s\S]*\}/)
    if (!jsonMatch) {
      throw new Error('LLM response did not contain valid JSON metadata.')
    }
    
    const parsed = JSON.parse(jsonMatch[0])
    
    return {
      context: parsed.context || 'No context generated.',
      decision: parsed.decision || 'No decision generated.',
      consequences: parsed.consequences || 'No consequences generated.',
      suggestedSlug: parsed.suggestedSlug || 'UNKNOWN-DECISION',
    }
  } catch (err) {
    throw new Error(`ADR Engine failed: ${(err as Error).message}`)
  }
}
