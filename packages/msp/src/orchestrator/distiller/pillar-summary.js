const NARRATIVE_PROMPT = (episodes) => `You are an expert memory synthesis engine for the cognitive_system. 
Your task is to synthesize a cluster of session episode summaries into a single, high-density Narrative unit (Tier 2 memory).

### INPUT EPISODES:
${episodes.map((ep, i) => `${i + 1}. [Session ${ep.sessionId}]: ${ep.summary}`).join('\n')}

### INSTRUCTIONS:
1. Identify the overarching technical narrative arc or theme across these episodes.
2. Extract specific technical, architectural, or project-governance decisions made.
3. Identify recurring behavioral patterns (e.g. "agent always checks logs before fix") or project facts.
4. List any major unresolved technical questions or pending roadmap items.

### OUTPUT FORMAT (JSON):
Output ONLY a valid JSON object with the following fields:
{
  "summary": "Coherent synthesis of the narrative arc (max 500 tokens).",
  "key_decisions": ["Specific decision 1", "Specific decision 2", ...],
  "unresolved_questions": ["Question 1", ...],
  "patterns_observed": ["Behavioral or factual pattern 1", ...]
}`;
const IDENTITY_PROMPT = (narratives) => `You are an expert identity-synthesis engine for the cognitive_system. 
Your task is to synthesize multiple Narrative arcs into durable Identity-level beliefs and a behavioral profile (Tier 3 memory).

### INPUT NARRATIVES:
${narratives.map((n, i) => `${i + 1}. [${n.id}]: ${n.content.summary}`).join('\n')}

### INSTRUCTIONS:
1. Identify fundamental, durable beliefs about the project, the user, or the system's own purpose that are consistent across these narratives.
2. For each belief, assign a confidence score (0.0 to 1.0) and cite the IDs of the narratives that support it.
3. Synthesize a concise "behavioral profile" describing how the agent acts or makes decisions based on this history.

### OUTPUT FORMAT (JSON):
Output ONLY a valid JSON object with the following fields:
{
  "beliefs": [
    { "belief": "Fundamental belief statement", "confidence": 0.85, "evidence_cite": ["NARRATIVE--ID1", ...] },
    ...
  ],
  "behavioral_profile": "Concise summary of agent's behavioral DNA."
}`;
/**
 * Pillar 2: SUMMARY
 * Uses an LLM to synthesize multiple episodes into a single narrative summary.
 * Aligns with the 8-8-8 protocol (8 Sessions -> 1 Narrative).
 */
export async function synthesizeNarrative(episodes, llm, timeoutMs = 45000) {
    if (episodes.length === 0) {
        throw new Error('Cannot synthesize narrative from zero episodes');
    }
    const prompt = NARRATIVE_PROMPT(episodes);
    try {
        const raw = await Promise.race([
            llm({ prompt, model: 'memory-distiller', attempt: 1 }),
            new Promise((_, reject) => setTimeout(() => reject(new Error('Narrative synthesis timed out')), timeoutMs)),
        ]);
        const jsonMatch = raw.match(/\{[\s\S]*\}/);
        if (!jsonMatch)
            throw new Error('LLM response did not contain a valid JSON object');
        const parsed = JSON.parse(jsonMatch[0]);
        return {
            summary: parsed.summary || 'No summary generated.',
            key_decisions: parsed.key_decisions || [],
            unresolved_questions: parsed.unresolved_questions || [],
            patterns_observed: parsed.patterns_observed || [],
        };
    }
    catch (err) {
        console.error(`[distiller] synthesis error: ${err.message}`);
        return {
            summary: `Fallback synthesis of ${episodes.length} episodes: ` + episodes.map(e => e.summary).join(' | '),
            key_decisions: [],
            unresolved_questions: [],
            patterns_observed: [],
        };
    }
}
/**
 * Pillar 2: SUMMARY (Identity Variant)
 * Synthesizes multiple narratives into Tier 3 Identity beliefs.
 */
export async function synthesizeIdentity(narratives, llm, timeoutMs = 60000) {
    if (narratives.length === 0) {
        throw new Error('Cannot synthesize identity from zero narratives');
    }
    const prompt = IDENTITY_PROMPT(narratives);
    try {
        const raw = await Promise.race([
            llm({ prompt, model: 'memory-distiller-opus', attempt: 1 }),
            new Promise((_, reject) => setTimeout(() => reject(new Error('Identity synthesis timed out')), timeoutMs)),
        ]);
        const jsonMatch = raw.match(/\{[\s\S]*\}/);
        if (!jsonMatch)
            throw new Error('LLM response did not contain a valid JSON object');
        const parsed = JSON.parse(jsonMatch[0]);
        return {
            beliefs: parsed.beliefs || [],
            behavioral_profile: parsed.behavioral_profile || 'No profile generated.'
        };
    }
    catch (err) {
        console.error(`[distiller] identity synthesis error: ${err.message}`);
        return { beliefs: [], behavioral_profile: 'Fallback behavioral profile based on limited history.' };
    }
}
