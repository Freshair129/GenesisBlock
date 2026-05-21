// Default thresholds for scoring and boundary detection.
// boundary is intentionally low so bag-of-words similarity within a topic
// reliably stays above it; explicit thresholds in tests override this.
export const DEFAULT_THRESHOLDS = {
    low: 0.3,
    high: 0.65,
    boundary: 0.15,
};
// Default timeout for Tier-2 LLM calls
export const DEFAULT_LLM_TIMEOUT_MS = 8000;
// Default maximum number of LLM calls per session
export const DEFAULT_MAX_LLM_CALLS = 20;
