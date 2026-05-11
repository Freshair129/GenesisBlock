import type { SessionTurn } from '../../sessions/types.js'
import type { EpisodeContent, Summariser } from '../types.js'

const DECISION_RE = /\b(decided|chose|will|going to|let's|let us)\b/i

/**
 * Default summariser. Picks the first non-trivial assistant message as the
 * summary anchor and the last decision-shaped sentence (if any) as a
 * key_decisions entry. Cheap, deterministic, no LLM.
 */
export const heuristicSummariser: Summariser = async (turns: SessionTurn[]) => {
  if (turns.length === 0) {
    return { summary: '(no turns in range)' }
  }

  // Summary: first assistant turn whose content is > 20 chars,
  // else first turn of any speaker.
  const assistant = turns.find(
    (t) => t.speakerId !== 'user' && t.content.trim().length > 20,
  )
  const anchor = assistant ?? turns[0]
  const summary = anchor.content.trim().split('\n')[0].slice(0, 240)

  // Key decisions: any sentence in any turn matching DECISION_RE.
  const decisions: string[] = []
  for (const t of turns) {
    for (const sentence of t.content.split(/(?<=[.!?])\s+/)) {
      const trimmed = sentence.trim()
      if (trimmed.length > 10 && trimmed.length < 200 && DECISION_RE.test(trimmed)) {
        decisions.push(trimmed)
      }
    }
  }

  const result: EpisodeContent = { summary }
  if (decisions.length > 0) result.key_decisions = decisions.slice(0, 5)
  return result
}
