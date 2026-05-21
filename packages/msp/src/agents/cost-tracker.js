// Pricing snapshot: as of 2026-05-14. Public list prices, USD per million
// tokens. These are deliberately approximate — the goal is "directionally
// accurate", not actuarially correct. Refresh from vendor pricing pages when
// the rough ratios shift more than ~20%.
//
// - T1 (Qwen local): free, no inference cost.
// - T2 (Gemini 2.0 Flash ballpark): ~$0.075 in, $0.30 out per 1M tokens.
// - T3 (Claude Sonnet ballpark):    ~$3.00 in, $15.00 out per 1M tokens.
//
// See CONCEPT--COST-TRACKING and BLUEPRINT--COST-TRACKING for context.
export const PRICING = [
    { tier: 'T1', per_million_input_usd: 0, per_million_output_usd: 0 },
    { tier: 'T2', per_million_input_usd: 0.075, per_million_output_usd: 0.3 },
    { tier: 'T3', per_million_input_usd: 3.0, per_million_output_usd: 15.0 },
];
const PRICING_BY_TIER = new Map(PRICING.map((row) => [row.tier, row]));
/**
 * Estimate cost in USD for a (tier, input_tokens, output_tokens) tuple using
 * the PRICING table. Returns 0 for T1 (free local inference). Returns 0 for
 * unknown tiers (defensive — type system already constrains, but the runtime
 * fallback keeps callers safe).
 */
export function estimateCost(tier, input_tokens, output_tokens) {
    const row = PRICING_BY_TIER.get(tier);
    if (row === undefined)
        return 0;
    const inputCost = (input_tokens * row.per_million_input_usd) / 1_000_000;
    const outputCost = (output_tokens * row.per_million_output_usd) / 1_000_000;
    return inputCost + outputCost;
}
/**
 * Rough token estimate from raw text using the industry-standard ~4 chars per
 * token heuristic. Cheap and predictable; off by ~10-20% from a real tokenizer
 * but adequate for ballpark cost projection.
 */
export function estimateTokens(text) {
    if (text.length === 0)
        return 0;
    return Math.ceil(text.length / 4);
}
