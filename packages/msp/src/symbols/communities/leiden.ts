/**
 * Community detection adapter — primary Leiden, fallback Louvain.
 *
 * See ADR--LEIDEN-COMMUNITY-DETECTION:
 *   - primary: `@aflsolutions/graphology-communities-leiden@1.1.x`
 *   - fallback: `graphology-communities-louvain@2.0.x` (when leiden import fails)
 *   - resolution=1.0 default, seed=42 default
 *
 * Both algorithms accept an `rng` function rather than a numeric seed; we
 * derive a deterministic mulberry32 RNG from the integer seed to keep builds
 * git-identical.
 *
 * The graph is treated as **undirected** for community detection — directed
 * call/import edges collapse to undirected co-occurrence, matching the
 * "logical module" intuition.
 */

// graphology@0.25 publishes a CommonJS module — under Node's ESM-of-CJS rules
// the named export `UndirectedGraph` is not visible, but the default export
// exposes all classes. Default-import + destructure is portable.
import graphology from 'graphology'
import type { AbstractGraph } from 'graphology-types'

const { UndirectedGraph } = graphology

import type { CommunityDetectionResult, CommunityDetector, Edge, Symbol } from '../types.js'

type GraphInstance = AbstractGraph

/** Tiny seeded RNG (mulberry32) — pure function of the integer seed. */
function mulberry32(seed: number): () => number {
  let s = seed >>> 0
  return function rng(): number {
    s = (s + 0x6d2b79f5) >>> 0
    let t = s
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

interface LeidenLikeFn {
  (graph: GraphInstance, options: Record<string, unknown>): { [node: string]: number }
}

interface LeidenLikeDetailed {
  detailed?: (
    graph: GraphInstance,
    options: Record<string, unknown>,
  ) => { communities: { [node: string]: number }; modularity: number }
}

type LeidenLike = LeidenLikeFn & LeidenLikeDetailed

interface LeidenModule {
  default: LeidenLike
}

interface LouvainModule {
  default: LeidenLike
}

async function tryLoadLeiden(): Promise<LeidenLike | null> {
  try {
    const mod = (await import('@aflsolutions/graphology-communities-leiden')) as unknown as LeidenModule
    return mod.default
  } catch {
    return null
  }
}

async function tryLoadLouvain(): Promise<LeidenLike | null> {
  try {
    const mod = (await import('graphology-communities-louvain')) as unknown as LouvainModule
    return mod.default
  } catch {
    return null
  }
}

function buildUndirectedGraph(symbols: Symbol[], edges: Edge[]): GraphInstance {
  const g: GraphInstance = new UndirectedGraph()
  const symbolIds = new Set<string>()
  for (const s of symbols) {
    g.addNode(s.id)
    symbolIds.add(s.id)
  }
  for (const e of edges) {
    // Skip edges that point to symbols not in the graph (e.g. external imports).
    if (!symbolIds.has(e.src_id) || !symbolIds.has(e.dst_id)) continue
    if (e.src_id === e.dst_id) continue
    if (g.hasEdge(e.src_id, e.dst_id) || g.hasEdge(e.dst_id, e.src_id)) continue
    g.addEdge(e.src_id, e.dst_id, { weight: e.weight })
  }
  return g
}

/** Heuristic label for a community: `<top-dir>/<top-symbol-name>`, capped at 60 chars. */
export function deriveLabel(members: Symbol[], edges: Edge[]): string | null {
  if (members.length === 0) return null

  // Top directory: most-common file's first directory segment
  const dirCounts = new Map<string, number>()
  for (const m of members) {
    const slashIdx = m.file.indexOf('/')
    const dir = slashIdx === -1 ? m.file : m.file.slice(0, slashIdx)
    dirCounts.set(dir, (dirCounts.get(dir) ?? 0) + 1)
  }
  let topDir = ''
  let topDirCount = -1
  for (const [dir, count] of [...dirCounts.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1))) {
    if (count > topDirCount) {
      topDir = dir
      topDirCount = count
    }
  }

  // Top symbol: highest out-degree exported symbol (excluding modules)
  const memberIds = new Set(members.map((m) => m.id))
  const outDegree = new Map<string, number>()
  for (const e of edges) {
    if (!memberIds.has(e.src_id)) continue
    outDegree.set(e.src_id, (outDegree.get(e.src_id) ?? 0) + 1)
  }
  let topSymbol: Symbol | null = null
  let topScore = -1
  // Stable order by id for deterministic ties.
  const sortedMembers = [...members].sort((a, b) => (a.id < b.id ? -1 : 1))
  for (const m of sortedMembers) {
    if (m.kind === 'module') continue
    if (!m.exported) continue
    const deg = outDegree.get(m.id) ?? 0
    if (deg > topScore) {
      topSymbol = m
      topScore = deg
    }
  }
  // If no exported symbols, fall back to any non-module symbol with highest degree.
  if (!topSymbol) {
    for (const m of sortedMembers) {
      if (m.kind === 'module') continue
      const deg = outDegree.get(m.id) ?? 0
      if (deg > topScore) {
        topSymbol = m
        topScore = deg
      }
    }
  }

  const label = topSymbol ? `${topDir}/${topSymbol.name}` : topDir
  return label.slice(0, 60)
}

function runDetector(
  detector: LeidenLike,
  graph: GraphInstance,
  resolution: number,
  seed: number,
  algorithm: 'leiden' | 'louvain',
): { partition: Map<string, number>; modularity: number } {
  const opts: Record<string, unknown> = {
    resolution,
    rng: mulberry32(seed),
  }
  // Empty graphs trip both algorithms — short-circuit.
  if (graph.order === 0) {
    return { partition: new Map(), modularity: 0 }
  }

  // Try detailed() first to get modularity; fall back to plain call if not available.
  if (typeof detector.detailed === 'function') {
    try {
      const result = detector.detailed(graph, opts)
      return {
        partition: new Map(Object.entries(result.communities).map(([k, v]) => [k, v as number])),
        modularity: result.modularity,
      }
    } catch {
      // Fall through to plain call below.
    }
  }
  const mapping = detector(graph, opts)
  return {
    partition: new Map(Object.entries(mapping).map(([k, v]) => [k, v as number])),
    modularity: 0,
  }
}

/**
 * Async because the leiden/louvain modules are loaded via dynamic import.
 * Returns the detection result; throws if BOTH detectors fail to load.
 */
export async function detectCommunities(
  symbols: Symbol[],
  edges: Edge[],
  opts: { resolution: number; seed: number },
): Promise<CommunityDetectionResult> {
  const graph = buildUndirectedGraph(symbols, edges)

  const leiden = await tryLoadLeiden()
  if (leiden) {
    const r = runDetector(leiden, graph, opts.resolution, opts.seed, 'leiden')
    return { ...r, algorithm: 'leiden' }
  }

  process.stderr.write(
    'symbols/communities/leiden: @aflsolutions/graphology-communities-leiden failed to load — falling back to Louvain\n',
  )
  const louvain = await tryLoadLouvain()
  if (louvain) {
    const r = runDetector(louvain, graph, opts.resolution, opts.seed, 'louvain')
    return { ...r, algorithm: 'louvain' }
  }

  throw new Error(
    'symbols/communities/leiden: neither Leiden nor Louvain could be loaded; install @aflsolutions/graphology-communities-leiden or graphology-communities-louvain',
  )
}

/**
 * Synchronous adapter wrapper exposing the `CommunityDetector` interface.
 * Callers that need synchronous detection should preload the modules; this
 * adapter assumes both have been imported successfully.
 */
export function makeDetector(
  leiden: LeidenLike | null,
  louvain: LeidenLike | null,
): CommunityDetector {
  return {
    run(symbols, edges, opts) {
      const graph = buildUndirectedGraph(symbols, edges)
      if (leiden) {
        const r = runDetector(leiden, graph, opts.resolution, opts.seed, 'leiden')
        return { ...r, algorithm: 'leiden' }
      }
      if (louvain) {
        const r = runDetector(louvain, graph, opts.resolution, opts.seed, 'louvain')
        return { ...r, algorithm: 'louvain' }
      }
      throw new Error(
        'makeDetector: at least one of leiden/louvain must be provided',
      )
    },
  }
}
