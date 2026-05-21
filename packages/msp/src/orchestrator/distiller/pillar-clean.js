/**
 * Pillar 1: CLEAN
 *
 * Removes duplicate episodes, low-salience noise, and normalize text
 * for synthesis.
 */
export function cleanEpisodes(episodes) {
    if (episodes.length === 0)
        return [];
    // 1. Salience Filtering
    // Keep only those with significant score or deep encoding level
    const salienceFiltered = episodes.filter((ep) => {
        // Keep standard/deep/critical regardless of score
        if (ep.encoding_level !== 'L0' && ep.encoding_level !== 'L1')
            return true;
        // For L0/L1, keep only if score is high
        return ep.score >= 0.5;
    });
    // 2. Semantic Deduplication (Strict text normalization)
    const seen = new Set();
    const unique = salienceFiltered.filter((ep) => {
        const norm = ep.summary.trim().toLowerCase().replace(/\s+/g, ' ');
        const key = `${ep.sessionId}:${norm}`;
        if (seen.has(key))
            return false;
        seen.add(key);
        return true;
    });
    return unique;
}
