import { DEFAULT_THRESHOLDS } from './config.js';
import { bagCosine, tokenise } from './boundary.js';
/**
 * Tier-1 deterministic feature weights — locked to the table in
 * ADR--CONSOLIDATOR-HYBRID-SCORING. Tuning is M9 work.
 */
export const FEATURE_WEIGHTS = {
    decision_markers: 0.35,
    code_artifact_mentions: 0.2,
    numeric_specificity: 0.15,
    length_normalised: 0.1,
    topic_continuity: 0.1,
    dead_end_markers: -0.3,
    greeting_filler: -0.2,
};
const DECISION_RE = /\b(we'?ll|we will|let'?s|let us|going to|gonna|decided|chose|choosing|selected|rejected|accept(?:ed)?|approve(?:d)?|will use|use)\b/i;
const DEAD_END_RE = /\b(nevermind|never mind|scrap that|forget it|forget that|tried that|doesn'?t work|didn'?t work|abandon(?:ed)?|won'?t work|gave up)\b/i;
const GREETING_RE = /\b(hi|hello|hey|thanks|thank you|got it|ok(?:ay)?|sure|sounds good|noted|yes|yep|nope)\b/i;
const CODE_PATH_RE = /[a-z0-9_-]+\/[a-z0-9_./-]+\.(?:ts|tsx|js|jsx|mjs|cjs|json|md|yaml|yml|sh|py|rs|go|java|cpp|c|h)/i;
const FN_CALL_RE = /[a-zA-Z_$][\w$]*\s*\(/;
const ATOM_ID_RE = /\b(?:ADR|FEAT|CONCEPT|BLUEPRINT|FRAME|AUDIT|PARAM|MICROTASK)--[A-Z0-9-]+/;
const NUMBER_RE = /\b\d[\d.,]*\b/g;
const SEMVER_RE = /\bv?\d+\.\d+(?:\.\d+)?(?:-[a-z0-9.-]+)?\b/i;
const DATE_RE = /\b\d{4}-\d{2}-\d{2}\b/;
/**
 * Decision markers — strongest positive signal.
 * Score: count-based, capped at 1.0.
 */
export function decisionMarkers(text) {
    if (!text)
        return 0;
    let n = 0;
    let m;
    const r = new RegExp(DECISION_RE.source, 'gi');
    while ((m = r.exec(text)) !== null) {
        n++;
        if (m.index === r.lastIndex)
            r.lastIndex++; // safety
    }
    return Math.min(1, n / 2);
}
/**
 * Code artifact mentions (file paths, function calls, atom IDs).
 */
export function codeArtifactMentions(text) {
    if (!text)
        return 0;
    let n = 0;
    if (CODE_PATH_RE.test(text))
        n++;
    if (FN_CALL_RE.test(text))
        n++;
    if (ATOM_ID_RE.test(text))
        n++;
    return Math.min(1, n / 2);
}
/**
 * Numeric specificity — proportion of digit characters per kilo-byte
 * (loose proxy for "lasting facts: dates, versions, counts").
 */
export function numericSpecificity(text) {
    if (!text)
        return 0;
    const len = text.length;
    if (len === 0)
        return 0;
    const matches = text.match(NUMBER_RE) ?? [];
    let bonus = 0;
    if (SEMVER_RE.test(text))
        bonus += 0.4;
    if (DATE_RE.test(text))
        bonus += 0.4;
    // Density: number of digit-tokens per 200 chars, capped.
    const density = (matches.length / Math.max(1, len / 200));
    return Math.min(1, density / 4 + bonus);
}
/**
 * Length-normalised feature. z-score against the session mean, mapped to 0..1.
 *
 * Returns:
 *   - 0 for very short chunks (< 0.5 mean)
 *   - 0.5 around the mean
 *   - up to 1 for long chunks (> 2× mean)
 */
export function lengthNormalised(chunk, stats) {
    if (chunk.length === 0)
        return 0;
    const totalBytes = chunk.reduce((acc, t) => acc + t.content.length, 0);
    const meanChunkBytes = totalBytes / chunk.length;
    if (stats.meanTurnBytes <= 0)
        return 0;
    const ratio = meanChunkBytes / stats.meanTurnBytes;
    if (ratio <= 0.5)
        return 0;
    if (ratio >= 2)
        return 1;
    // Linear ramp 0.5..2 → 0..1
    return (ratio - 0.5) / 1.5;
}
/**
 * Topic continuity vs previous chunk. Higher → same topic (continuation
 * → small positive importance bump). Lower → topic shift (boundary signal,
 * already accounted for in boundary detection).
 *
 * Returns the cosine similarity 0..1 directly. If `prevChunk` is empty
 * (this is the first chunk) we return 0.5 (neutral).
 */
export function topicContinuity(chunk, prevChunk) {
    if (!prevChunk || prevChunk.length === 0)
        return 0.5;
    const cur = chunk.flatMap((t) => tokenise(t.content));
    const prev = prevChunk.flatMap((t) => tokenise(t.content));
    return bagCosine(cur, prev);
}
/** Negative: dead-end markers ("nevermind", "scrap that", ...). */
export function deadEndMarkers(text) {
    if (!text)
        return 0;
    return DEAD_END_RE.test(text) ? 1 : 0;
}
/** Negative: greeting / filler ("hi", "thanks"). */
export function greetingFiller(text) {
    if (!text)
        return 0;
    // Penalise stronger if the entire chunk is filler.
    let n = 0;
    let m;
    const r = new RegExp(GREETING_RE.source, 'gi');
    while ((m = r.exec(text)) !== null) {
        n++;
        if (m.index === r.lastIndex)
            r.lastIndex++;
    }
    if (n === 0)
        return 0;
    // Filler density: matches per 100 chars.
    const density = n / Math.max(1, text.length / 100);
    return Math.min(1, density);
}
/**
 * Compute per-session stats over the full turn array (used by length
 * normalisation).
 */
export function computeSessionStats(turns) {
    if (turns.length === 0) {
        return { turnCount: 0, meanTurnBytes: 0, stddevTurnBytes: 0 };
    }
    const lens = turns.map((t) => t.content.length);
    const sum = lens.reduce((a, b) => a + b, 0);
    const mean = sum / lens.length;
    const variance = lens.reduce((acc, l) => acc + (l - mean) * (l - mean), 0) / lens.length;
    return {
        turnCount: turns.length,
        meanTurnBytes: mean,
        stddevTurnBytes: Math.sqrt(variance),
    };
}
function clamp01(n) {
    if (n < 0)
        return 0;
    if (n > 1)
        return 1;
    return n;
}
/**
 * Tier-1 score: weighted sum of the seven features, clamped to 0..1.
 * Verdict from `low`/`high` thresholds (defaults from ADR).
 */
export function scoreChunk(chunk, stats, thresholds = {}, prevChunk = null) {
    const text = chunk.map((t) => t.content).join('\n');
    const breakdown = {
        decision_markers: decisionMarkers(text),
        code_artifact_mentions: codeArtifactMentions(text),
        numeric_specificity: numericSpecificity(text),
        length_normalised: lengthNormalised(chunk, stats),
        topic_continuity: topicContinuity(chunk, prevChunk),
        dead_end_markers: deadEndMarkers(text),
        greeting_filler: greetingFiller(text),
    };
    let score = 0;
    for (const [k, v] of Object.entries(breakdown)) {
        score += FEATURE_WEIGHTS[k] * v;
    }
    score = clamp01(score);
    const low = thresholds.low ?? DEFAULT_THRESHOLDS.low;
    const high = thresholds.high ?? DEFAULT_THRESHOLDS.high;
    let verdict;
    if (score < low)
        verdict = 'drop';
    else if (score > high)
        verdict = 'keep';
    else
        verdict = 'borderline';
    return { score, verdict, breakdown };
}
