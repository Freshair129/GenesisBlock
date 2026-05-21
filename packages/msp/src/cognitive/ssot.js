/**
 * §14.1 — SSOT authority hierarchy.
 *
 * When two atoms disagree, the winning source is the one with the highest
 * authority. Code (runtime behaviour) beats anything; PROTO beats MASTER
 * beats ADR beats FRAME beats KNOWLEDGE-TYPES beats CONCEPT/FEAT/BLUEPRINT.
 *
 * Returns the winning citation, or `null` if the input is empty.
 */
/** Lower index = higher authority. */
const AUTHORITY_ORDER = [
    // 1. Code is implicit — represented by AtomCitation.source === 'code'.
    'proto', // 2. machine-enforced invariants
    'master', // 3. root-level policy
    'adr', // 4. architectural decision
    'framework', // 5a. governance / architecture frameworks (v2.3+)
    'genesis', // 5b. block manifest (v2.3+)
    'frame', // 5c. deprecated — legacy framework alias (pre-v2.3) / v2.3 placeholder for genesis
    'knowledge-types', // 6. canonical taxonomy
    'concept', // 7. requirements
    'feat',
    'blueprint',
    // everything else falls through to the lowest tier
];
export function resolveSSOT(citations) {
    if (citations.length === 0)
        return null;
    let winner = null;
    let winnerRank = Number.POSITIVE_INFINITY;
    for (const c of citations) {
        const rank = rankOf(c);
        if (rank < winnerRank) {
            winner = c;
            winnerRank = rank;
        }
    }
    return winner;
}
function rankOf(c) {
    if (c.source === 'code')
        return -1;
    const idx = AUTHORITY_ORDER.indexOf((c.type || '').toLowerCase());
    return idx === -1 ? AUTHORITY_ORDER.length : idx;
}
