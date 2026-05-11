import type { Tokeniser } from './types.js'

/**
 * Conservative char-count token estimator. 3.5 chars/token holds for mixed
 * English / Thai / code. English alone averages ~4 chars/token; using 3.5
 * over-estimates slightly so the compressor never over-fills the budget.
 *
 * `Math.ceil(0)` is 0 — the empty string costs no tokens. Negative-length
 * strings are impossible in JS but we still clamp to a non-negative integer.
 */
export const DEFAULT_TOKENISER: Tokeniser = (s: string): number => {
  if (!s) return 0
  return Math.ceil(s.length / 3.5)
}

/**
 * Convenience wrapper that allows callers to pass an injected `Tokeniser`
 * or fall back to the default. Used internally so every estimator call
 * goes through one chokepoint (handy for testing and future swap-out).
 */
export function estimateText(
  text: string,
  tokeniser: Tokeniser = DEFAULT_TOKENISER,
): number {
  return tokeniser(text)
}
