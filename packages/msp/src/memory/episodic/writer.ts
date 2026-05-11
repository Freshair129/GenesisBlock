import { readFile } from 'node:fs/promises'
import { createInterface } from 'node:readline'
import { createReadStream } from 'node:fs'
import { resolve } from 'node:path'

import type { SessionTurn } from '../sessions/types.js'
import { atomicWrite } from './atomic-write.js'
import { validateEpisode } from './schema.js'
import { heuristicSummariser } from './summarisers/heuristic.js'
import type { AppendOpts, Episode, FromTurnsOpts } from './types.js'

const DEFAULT_NAMESPACE = 'evaAI'

function episodicMemoryPath(root: string, namespace: string): string {
  return resolve(root, '.brain/msp/projects', namespace, 'memory/episodic_memory.json')
}

function sessionPath(root: string, namespace: string, episodicId: string): string {
  return resolve(root, '.brain/msp/projects', namespace, 'sessions', `${episodicId}.jsonl`)
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
  const parsed = JSON.parse(raw)
  if (!Array.isArray(parsed)) {
    throw new Error(`episodic_memory.json: top-level must be an array, got ${typeof parsed}`)
  }
  return parsed as Episode[]
}

function byTimestamp(a: Episode, b: Episode): number {
  const ta = a.timestamp ?? ''
  const tb = b.timestamp ?? ''
  if (ta === tb) return 0
  return ta < tb ? -1 : 1
}

export async function appendEpisode(
  episode: Episode,
  opts: AppendOpts,
): Promise<void> {
  const namespace = opts.namespace ?? DEFAULT_NAMESPACE
  const path = episodicMemoryPath(opts.root, namespace)

  const ep: Episode = {
    ...episode,
    timestamp: episode.timestamp ?? new Date().toISOString(),
  }
  validateEpisode(ep)

  const existing = await readEpisodes(path)
  const idx = existing.findIndex((e) => e.episodicId === ep.episodicId)
  if (idx >= 0) {
    existing[idx] = ep
  } else {
    existing.push(ep)
  }
  existing.sort(byTimestamp)

  await atomicWrite(path, JSON.stringify(existing, null, 2) + '\n')
}

function inRange(turnId: number, range: string[]): boolean {
  for (const r of range) {
    const m = r.match(/^turnIdx-(\d+)(?:\.\.turnIdx-(\d+))?$/)
    if (!m) continue
    const lo = Number.parseInt(m[1]!, 10)
    const hi = m[2] !== undefined ? Number.parseInt(m[2], 10) : lo
    if (turnId >= lo && turnId <= hi) return true
  }
  return false
}

async function readTurnsInRange(
  root: string,
  namespace: string,
  episodicId: string,
  range: string[],
): Promise<SessionTurn[]> {
  const path = sessionPath(root, namespace, episodicId)
  const out: SessionTurn[] = []
  try {
    const stream = createReadStream(path, { encoding: 'utf8' })
    const rl = createInterface({ input: stream, crlfDelay: Infinity })
    for await (const line of rl) {
      const trimmed = line.trim()
      if (trimmed === '') continue
      const turn = JSON.parse(trimmed) as SessionTurn
      if (inRange(turn.turnId, range)) out.push(turn)
    }
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code !== 'ENOENT') throw err
  }
  return out
}

appendEpisode.fromTurns = async function fromTurns(opts: FromTurnsOpts): Promise<void> {
  const namespace = opts.namespace ?? DEFAULT_NAMESPACE
  const turns = await readTurnsInRange(opts.root, namespace, opts.episodicId, opts.range)
  const summariser = opts.summariser ?? heuristicSummariser
  const content = await summariser(turns)

  const episode: Episode = {
    episodicId: opts.episodicId,
    sessionId: opts.sessionId,
    projectId: opts.projectId,
    importance_score: opts.importance_score,
    range: opts.range,
    content,
    context: opts.context,
    tags: opts.tags,
    associations: opts.associations,
  }
  await appendEpisode(episode, { root: opts.root, namespace })
}

export { heuristicSummariser } from './summarisers/heuristic.js'
