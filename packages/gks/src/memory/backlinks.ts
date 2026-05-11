import { writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { mkdir } from 'node:fs/promises'
import type { AtomicEntry } from './types.js'

/** One directed edge extracted from a crosslinks predicate. */
export interface BacklinkEdge {
  from: string
  to: string
  type: string
}

export interface BacklinksOptions {
  /** Only emit edges whose crosslinks key matches one of these predicates. */
  filterTypes?: string[]
  /** Sort edges by from→to→type for git-diff stability. Default true. */
  sort?: boolean
}

/**
 * Derive all backlink edges from a map of loaded atomic entries.
 * Iterates every atom's `crosslinks` map and emits one edge per
 * (from, to, predicate) triple. GKS does not persist backlinks — callers
 * (orchestrators, CLI, MCP tools) cache the output themselves.
 */
export function deriveBacklinksFromEntries(
  entries: Iterable<AtomicEntry>,
  opts: BacklinksOptions = {},
): BacklinkEdge[] {
  const edges: BacklinkEdge[] = []
  for (const atom of entries) {
    if (!atom.crosslinks) continue
    for (const [predicate, targets] of Object.entries(atom.crosslinks)) {
      if (opts.filterTypes && !opts.filterTypes.includes(predicate)) continue
      if (!Array.isArray(targets)) continue
      for (const target of targets) {
        if (typeof target === 'string' && target.length > 0) {
          edges.push({ from: atom.id, to: target, type: predicate })
        }
      }
    }
  }
  if (opts.sort !== false) {
    edges.sort(
      (a, b) =>
        a.from.localeCompare(b.from) ||
        a.to.localeCompare(b.to) ||
        a.type.localeCompare(b.type),
    )
  }
  return edges
}

/**
 * Write derived backlink edges to a JSONL file (one JSON object per line).
 * Creates parent directories as needed.
 */
export async function emitBacklinksJsonl(
  entries: Iterable<AtomicEntry>,
  outPath: string,
  opts: BacklinksOptions = {},
): Promise<{ edgeCount: number; bytes: number }> {
  const edges = deriveBacklinksFromEntries(entries, opts)
  const lines = edges.map((e) => JSON.stringify(e)).join('\n')
  const content = lines.length > 0 ? lines + '\n' : ''
  const abs = resolve(outPath)
  await mkdir(dirname(abs), { recursive: true })
  await writeFile(abs, content, 'utf8')
  return { edgeCount: edges.length, bytes: Buffer.byteLength(content, 'utf8') }
}
