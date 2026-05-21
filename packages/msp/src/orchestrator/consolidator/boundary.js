import { DEFAULT_THRESHOLDS } from './config.js';
// Common function words stripped before similarity comparison.
const STOP = new Set([
    'the', 'a', 'an', 'is', 'are', 'was', 'were', 'be', 'been', 'being',
    'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'should',
    'could', 'can', 'may', 'might', 'shall', 'must',
    'in', 'on', 'at', 'to', 'for', 'of', 'and', 'or', 'but', 'not', 'no',
    'this', 'that', 'these', 'those', 'it', 'its',
    'i', 'you', 'he', 'she', 'we', 'they',
    'with', 'as', 'if', 'from', 'by', 'into', 'about',
]);
/**
 * Tokenise text into a filtered word array. Strips stop-words and
 * tokens shorter than 3 characters so that function words do not
 * dominate cosine similarity.
 */
export function tokenise(text) {
    return (text.toLowerCase().match(/\w+/g) ?? []).filter((w) => w.length > 2 && !STOP.has(w));
}
/**
 * Cosine similarity between two token arrays (bag-of-words model).
 * Returns 0 when either array is empty.
 */
export function bagCosine(a, b) {
    if (a.length === 0 || b.length === 0)
        return 0;
    const freqA = new Map();
    const freqB = new Map();
    for (const w of a)
        freqA.set(w, (freqA.get(w) ?? 0) + 1);
    for (const w of b)
        freqB.set(w, (freqB.get(w) ?? 0) + 1);
    const all = new Set([...freqA.keys(), ...freqB.keys()]);
    let dot = 0;
    let mag1 = 0;
    let mag2 = 0;
    for (const w of all) {
        const c1 = freqA.get(w) ?? 0;
        const c2 = freqB.get(w) ?? 0;
        dot += c1 * c2;
        mag1 += c1 * c1;
        mag2 += c2 * c2;
    }
    if (mag1 === 0 || mag2 === 0)
        return 0;
    return dot / (Math.sqrt(mag1) * Math.sqrt(mag2));
}
/**
 * Partition `turns` into topic-coherent episode ranges using a sliding
 * bag-of-words window. Returns an array of non-overlapping [start, end]
 * index pairs that together cover the full turn array.
 *
 * A boundary is inserted before turn i when the cosine similarity between
 * the combined tokens of the preceding `window` turns and turn i falls
 * below `thresholds.boundary`.
 */
export function detectBoundaries(turns, opts = {}) {
    if (turns.length === 0)
        return [];
    const threshold = opts.thresholds?.boundary ?? DEFAULT_THRESHOLDS.boundary;
    const window = opts.window ?? 2;
    const ranges = [];
    let currentStart = 0;
    for (let i = window; i < turns.length; i++) {
        const prevTokens = turns
            .slice(i - window, i)
            .flatMap((t) => tokenise(t.content));
        const curTokens = tokenise(turns[i].content);
        const similarity = bagCosine(prevTokens, curTokens);
        if (similarity < threshold) {
            ranges.push([currentStart, i - 1]);
            currentStart = i;
        }
    }
    ranges.push([currentStart, turns.length - 1]);
    return ranges;
}
