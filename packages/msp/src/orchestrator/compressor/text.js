/**
 * Render a list of session turns to a single string for token-counting and
 * LLM prompts. Format mirrors `buildTier2Prompt` from M7b for consistency
 * (`[speakerId] content`, joined with newlines).
 */
export function joinTurns(turns) {
    return turns.map((t) => `[${t.speakerId}] ${t.content}`).join('\n');
}
