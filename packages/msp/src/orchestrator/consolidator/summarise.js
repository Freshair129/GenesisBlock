const ATOM_ID_RE = /\b(?:ADR|FEAT|CONCEPT|BLUEPRINT|FRAME|AUDIT|PARAM|MICROTASK)--[A-Z0-9-]+/g;
const WIKILINK_RE = /\[\[([^\]|]+?)(?:\|[^\]]+)?\]\]/g;
const HASHTAG_RE = /(?:^|\s)#([a-z0-9][a-z0-9_-]*)/gi;
const MAX_SUMMARY_CHARS = 240;
const MIN_SUMMARY_CHARS = 12;
/**
 * Truncate a string at a word boundary near `max` chars.
 */
function truncateAtWord(text, max) {
    if (text.length <= max)
        return text;
    const cut = text.slice(0, max);
    const lastSpace = cut.lastIndexOf(' ');
    if (lastSpace > max * 0.5)
        return cut.slice(0, lastSpace).trim() + '…';
    return cut.trim() + '…';
}
/**
 * Pick the first reasonable sentence from a turn's content.
 * Returns null if no sentence ≥ MIN_SUMMARY_CHARS is found.
 */
function firstSentence(content) {
    if (!content)
        return null;
    const trimmed = content.trim();
    if (trimmed.length === 0)
        return null;
    // Split on sentence boundaries; prefer the first non-trivial one.
    const sentences = trimmed.split(/(?<=[.!?])\s+/);
    for (const s of sentences) {
        const x = s.trim();
        if (x.length >= MIN_SUMMARY_CHARS) {
            return truncateAtWord(x, MAX_SUMMARY_CHARS);
        }
    }
    return null;
}
/**
 * Deterministic fallback summary for tier-1-keep chunks (when LLM is not
 * invoked).
 *
 * Strategy:
 *   1. First sentence of any non-user turn ≥ 12 chars.
 *   2. Else first sentence of any turn ≥ 12 chars.
 *   3. Else first 120 chars trimmed at word boundary.
 *   4. Else "(no summary available)".
 */
export function deterministicSummary(chunk) {
    if (chunk.length === 0)
        return '(no summary available)';
    // Prefer non-user (assistant/system) turns first.
    for (const t of chunk) {
        if (t.speakerId !== 'user') {
            const s = firstSentence(t.content);
            if (s)
                return s;
        }
    }
    // Fall back to any turn.
    for (const t of chunk) {
        const s = firstSentence(t.content);
        if (s)
            return s;
    }
    // Last resort: first 120 chars.
    const joined = chunk
        .map((t) => t.content.trim())
        .filter((c) => c.length > 0)
        .join(' ');
    if (joined.length === 0)
        return '(no summary available)';
    return truncateAtWord(joined, 120);
}
/**
 * Pull deterministic tags from the chunk:
 *   - `[[wikilink]]` targets
 *   - Atom IDs (ADR--FOO, FEAT--BAR, etc.)
 *   - `#hashtag` markers
 *
 * De-duplicates and caps at 5.
 */
export function extractDeterministicTags(chunk) {
    const seen = new Set();
    const tags = [];
    function add(tag) {
        const t = tag.trim();
        if (!t)
            return;
        if (seen.has(t))
            return;
        seen.add(t);
        tags.push(t);
    }
    for (const turn of chunk) {
        const text = turn.content;
        let m;
        // Wikilinks first (most explicit).
        WIKILINK_RE.lastIndex = 0;
        while ((m = WIKILINK_RE.exec(text)) !== null) {
            add(m[1]);
            if (tags.length >= 5)
                return tags;
        }
        // Atom IDs.
        ATOM_ID_RE.lastIndex = 0;
        while ((m = ATOM_ID_RE.exec(text)) !== null) {
            add(m[0]);
            if (tags.length >= 5)
                return tags;
        }
        // Hashtags.
        HASHTAG_RE.lastIndex = 0;
        while ((m = HASHTAG_RE.exec(text)) !== null) {
            add(m[1]);
            if (tags.length >= 5)
                return tags;
        }
    }
    return tags;
}
