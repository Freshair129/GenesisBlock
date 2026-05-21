/** Default LLM call timeout, mirrors M7b `DEFAULT_LLM_TIMEOUT_MS`. */
export const DEFAULT_LLM_TIMEOUT_MS = 8000;
/**
 * Tier-2 trim threshold: turns whose tier-1 score sits below this are
 * candidates for dropping. 0.30 mirrors `DEFAULT_THRESHOLDS.low` from
 * M7b — a turn that would tier-1-drop on its own is fair game.
 */
export const TRIM_THRESHOLD = 0.3;
/**
 * Minimum droppable fraction for the trim tier to fire. Per
 * ADR--COMPRESSOR-THREE-TIER: only trim when ≥ 30% of the turns are
 * candidates AND the trimmed result actually fits.
 */
export const TRIM_DROP_FRACTION = 0.3;
/**
 * Resummarise target ratio: aim for 60% of original token count (or
 * remaining budget, whichever is smaller).
 */
export const RESUMMARISE_RATIO = 0.6;
