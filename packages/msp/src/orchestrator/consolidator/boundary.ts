import type { Turn, Thresholds } from './types.js'
import { DEFAULT_THRESHOLDS } from './types.js'

const STOPWORDS = new Set([
  'the','a','an','and','or','but','if','then','else','of','in','on','at',
  'to','for','from','by','with','is','are','was','were','be','been','being',
  'have','has','had','do','does','did','will','would','could','should','may',
  'might','this','that','these','those','it','its','i','you','we','they',
  'he','she','him','her','them','our','your','my','me','us','as','so','not',
  'no','yes','can','about','into','out','up','down','over','under','also',
  'than','too','very','just','only','any','all','some','more','most','other',
  'such','what','which','who','whom','when','where','why','how',
])

/**
 * Break a string into normalised content tokens (lowercase, alphanumeric,
 * stopwords removed, length ≥ 2).
 */
export function tokenise(text: string): string[] {
  if (!text) return []
  const out: string[] = []
  const matches = text.toLowerCase().match(/[a-z0-9_-]+/g)
  if (!matches) return []
  for (const m of matches) {
    if (m.length < 2) continue
    if (STOPWORDS.has(m)) continue
    out.push(m)
  }
  return out
}

/**
 * Cosine similarity between two token bags. Returns 0..1.
 * Empty bags → 0 (treated as a topic shift / boundary).
 */
export function bagCosine(a: string[], b: string[]): number {
  if (a.length === 0 || b.length === 0) return 0
  const fa: Record<string, number> = {}
  const fb: Record<string, number> = {}
  for (const t of a) fa[t] = (fa[t] ?? 0) + 1
  for (const t of b) fb[t] = (fb[t] ?? 0) + 1
  let dot = 0
  let na = 0
  let nb = 0
  for (const k of Object.keys(fa)) {
    na += fa[k]! * fa[k]!
    if (fb[k] !== undefined) dot += fa[k]! * fb[k]!
  }
  for (const k of Object.keys(fb)) nb += fb[k]! * fb[k]!
  if (na === 0 || nb === 0) return 0
  return dot / (Math.sqrt(na) * Math.sqrt(nb))
}

export interface BoundaryOptions {
  /** Sliding window size in turns. Defaults to 3. */
  window?: number
  /** Topic-continuity threshold below which we cut. Defaults from ADR (0.25). */
  thresholds?: Thresholds
}

/**
 * Detect episode boundaries via windowed topic-continuity.
 *
 * Algorithm (windowed cosine drop):
 *   1. Slide a window of `window` turns over the session.
 *   2. For each step, compare the window token bag to the previous window's.
 *   3. If cosine < boundary_threshold AND the next window starts a new
 *      contiguous span, emit a boundary at that point.
 *   4. Always emit the start (0) and end (turns.length-1) bookends.
 *
 * Returns inclusive [startIdx, endIdx] tuples covering the full session
 * partition (no overlaps, no gaps).
 */
export function detectBoundaries(
  turns: Turn[],
  opts: BoundaryOptions = {},
): Array<[number, number]> {
  if (turns.length === 0) return []
  if (turns.length === 1) return [[0, 0]]

  const window = Math.max(1, opts.window ?? 3)
  const threshold = opts.thresholds?.boundary ?? DEFAULT_THRESHOLDS.boundary

  // For very short sessions: a single chunk.
  if (turns.length <= window) return [[0, turns.length - 1]]

  // Per-turn token bags so we can rebuild window bags cheaply.
  const turnTokens: string[][] = turns.map((t) => tokenise(t.content))

  function windowBag(start: number, end: number): string[] {
    const out: string[] = []
    for (let i = start; i <= end && i < turns.length; i++) out.push(...turnTokens[i]!)
    return out
  }

  const cuts: number[] = [0]
  // Step the window's right-edge through the session; compare each new
  // window of `window` turns to the previous adjacent window. To avoid
  // spurious cuts at the tail (where the forward window would be smaller
  // than `window`), we only consider boundaries while a full forward
  // window is available.
  for (let i = window; i + window - 1 < turns.length; i++) {
    const prev = windowBag(i - window, i - 1)
    const cur = windowBag(i, i + window - 1)
    const sim = bagCosine(prev, cur)
    if (sim < threshold) {
      // Cut at boundary `i` (i becomes start of the next chunk).
      // Avoid emitting a degenerate split if previous cut was just emitted.
      if (cuts[cuts.length - 1]! < i) cuts.push(i)
    }
  }

  // Convert cut points → inclusive [start, end] ranges.
  const ranges: Array<[number, number]> = []
  for (let i = 0; i < cuts.length; i++) {
    const start = cuts[i]!
    const end = i + 1 < cuts.length ? cuts[i + 1]! - 1 : turns.length - 1
    ranges.push([start, end])
  }
  return ranges
}
