/**
 * Default per-source RRF weights from `ADR--RETRIEVAL-RRF-FUSION`.
 * Tuning is M9 work via a `PARAM--` atom.
 */
export const DEFAULT_WEIGHTS = {
    'gks-vector': 1.0,
    'obsidian-text': 0.8,
    grep: 0.6,
    episodic: 1.2,
    backlinks: 0.5,
    graph: 1.5,
    narrative: 1.4,
    identity: 1.8,
};
/**
 * Default per-source timeouts (ms) from `ADR--RETRIEVAL-RRF-FUSION`.
 */
export const DEFAULT_PER_SOURCE_TIMEOUTS = {
    'gks-vector': 800,
    'obsidian-text': 400,
    grep: 600,
    episodic: 100,
    backlinks: 100,
    graph: 300,
    narrative: 200,
    identity: 200,
};
export const DEFAULT_TOTAL_TIMEOUT_MS = 1500;
export const DEFAULT_TOP_K = 10;
export const DEFAULT_RRF_K = 60;
export const DEFAULT_NAMESPACE = 'evaAI';
