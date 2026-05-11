import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { performance } from 'node:perf_hooks'

import type { Episode } from '../../../memory/episodic/types.js'
import type { SourceHit, SourceResult } from '../types.js'

export interface EpisodicSourceOptions {
  root: string
  namespace: string
  query: string
  topK: number
}

function episodicMemoryPath(root: string, namespace: string): string {
  return resolve(
    root,
    '.brain/msp/projects',
    namespace,
    'memory/episodic_memory.json',
  )
}

function tokenise(text: string): string[] {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s_-]/g, ' ')
    .split(/\s+/)
    .filter((t) => t.length > 0)
}

function jaccard(a: Set<string>, b: Set<string>): number {
  if (a.size === 0 || b.size === 0) return 0
  let inter = 0
  for (const t of a) if (b.has(t)) inter += 1
  const union = a.size + b.size - inter
  return union === 0 ? 0 : inter / union
}

async function readEpisodes(path: string): Promise<Episode[]> {
  let raw: string
  try {
    raw = await readFile(path, 'utf8')
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return []
    throw err
  }
  if (raw.trim() === '') return []
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch {
    return []
  }
  if (!Array.isArray(parsed)) return []
  return parsed as Episode[]
}

interface ScoredEpisode {
  episode: Episode
  score: number
}

/**
 * Score an episode against a tokenised query.
 *  - Jaccard overlap of `summary` tokens with query tokens.
 *  - +0.2 bonus per exact tag match (case-insensitive).
 */
export function scoreEpisode(
  episode: Episode,
  queryBag: Set<string>,
): number {
  const summary = episode.content?.summary ?? ''
  const summaryBag = new Set(tokenise(summary))
  const overlap = jaccard(summaryBag, queryBag)

  let tagBonus = 0
  const tags = episode.tags ?? []
  for (const t of tags) {
    if (typeof t !== 'string') continue
    const tl = t.toLowerCase()
    // Tag matches if its tokens overlap the query bag, OR exact match.
    if (queryBag.has(tl)) {
      tagBonus += 0.2
      continue
    }
    for (const tk of tokenise(t)) {
      if (queryBag.has(tk)) {
        tagBonus += 0.2
        break
      }
    }
  }

  return overlap + tagBonus
}

/**
 * Episodic memory source. Reads `episodic_memory.json` for the namespace,
 * scores each episode by token overlap + tag bonuses against the query,
 * sorts desc, returns top-K. Missing file → empty (NOT an error — fresh
 * project).
 */
export async function episodicSource(
  opts: EpisodicSourceOptions,
): Promise<SourceResult> {
  const start = performance.now()
  const path = episodicMemoryPath(opts.root, opts.namespace)
  const queryBag = new Set(tokenise(opts.query))

  if (queryBag.size === 0) {
    return {
      source: 'episodic',
      hits: [],
      latencyMs: Math.round(performance.now() - start),
    }
  }

  let episodes: Episode[]
  try {
    episodes = await readEpisodes(path)
  } catch (err) {
    const msg =
      err instanceof Error ? err.message : String(err ?? 'unknown error')
    return {
      source: 'episodic',
      hits: [],
      latencyMs: Math.round(performance.now() - start),
      error: msg,
    }
  }

  const scored: ScoredEpisode[] = episodes
    .map((episode) => ({ episode, score: scoreEpisode(episode, queryBag) }))
    .filter((s) => s.score > 0)

  scored.sort((a, b) => {
    if (b.score !== a.score) return b.score - a.score
    // Tie-break: prefer newer episode (timestamp DESC) so "recent context
    // is high-signal" intent from the ADR is honoured.
    const ta = a.episode.timestamp ?? ''
    const tb = b.episode.timestamp ?? ''
    if (ta !== tb) return ta < tb ? 1 : -1
    return a.episode.episodicId < b.episode.episodicId ? -1 : 1
  })

  const hits: SourceHit[] = scored
    .slice(0, opts.topK)
    .map((s, i) => ({
      atomId: s.episode.episodicId,
      rank: i + 1,
      snippet: s.episode.content.summary,
      source: 'episodic',
    }))

  return {
    source: 'episodic',
    hits,
    latencyMs: Math.round(performance.now() - start),
  }
}
