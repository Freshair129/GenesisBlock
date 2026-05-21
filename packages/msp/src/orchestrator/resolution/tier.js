/**
 * Assign resolution tiers to retrieval hits.
 * MVP Implementation: Top 3 hits get FULL, others get MENTION.
 */
export function assignResolutionTiers(hits, opts = {}) {
    const fullCount = opts.fullCount ?? 3;
    return hits.map((hit, i) => {
        const tier = i < fullCount ? 'FULL' : 'MENTION';
        return { ...hit, tier };
    });
}
