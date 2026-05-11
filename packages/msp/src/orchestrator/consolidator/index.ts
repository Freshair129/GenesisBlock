import { createInterface } from 'node:readline'
import { createReadStream } from 'node:fs'
import { resolve } from 'node:path'

import { detectBoundaries } from './boundary.js'
import { callTier2 } from './llm.js'
import { computeSessionStats, scoreChunk } from './score.js'
import {
  deterministicSummary,
  extractDeterministicTags,
} from './summarise.js'
import {
  DEFAULT_LLM_TIMEOUT_MS,
  DEFAULT_MAX_LLM_CALLS,
  DEFAULT_THRESHOLDS,
} from './types.js'
import type {
  ConsolidateOptions,
  Episode,
  ScoreSource,
  Turn,
} from './types.js'

const DEFAULT_NAMESPACE = 'evaAI'

export type {
  ConsolidateOptions,
  Episode,
  Turn,
  Verdict,
  ScoreSource,
  Tier1Result,
  Tier2Result,
  SessionStats,
  Thresholds,
  LlmClient,
  Chunk,
} from './types.js'

export { scoreChunk, computeSessionStats } from './score.js'
export { detectBoundaries } from './boundary.js'
export { callTier2 } from './llm.js'
export { deterministicSummary, extractDeterministicTags } from './summarise.js'

function sessionPath(root: string, namespace: string, sessionId: string): string {
  return resolve(
    root,
    '.brain/msp/projects',
    namespace,
    'sessions',
    `${sessionId}.jsonl`,
  )
}

/**
 * Read a session.jsonl file into an in-memory `Turn[]`. Missing file → [].
 */
export async function readSessionTurns(
  root: string,
  namespace: string,
  sessionId: string,
): Promise<Turn[]> {
  const path = sessionPath(root, namespace, sessionId)
  const out: Turn[] = []
  try {
    const stream = createReadStream(path, { encoding: 'utf8' })
    const rl = createInterface({ input: stream, crlfDelay: Infinity })
    for await (const line of rl) {
      const trimmed = line.trim()
      if (!trimmed) continue
      try {
        out.push(JSON.parse(trimmed) as Turn)
      } catch {
        /* skip malformed line */
      }
    }
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code !== 'ENOENT') throw err
  }
  // Stable ordering by turnId to defend against out-of-order writes.
  out.sort((a, b) => a.turnId - b.turnId)
  return out
}

/**
 * Hybrid (deterministic gate + LLM borderline) consolidation pass.
 *
 * Reads session turns, partitions into topic-coherent chunks, scores each
 * chunk via tier-1 deterministic features, escalates borderline chunks to
 * tier-2 LLM (capped by `maxLlmCallsPerSession`), and emits Episode[] in
 * memory. Caller decides persistence (does NOT write to episodic store).
 */
export async function consolidate(
  opts: ConsolidateOptions,
): Promise<Episode[]> {
  const root = opts.root ?? process.cwd()
  const namespace = opts.namespace ?? DEFAULT_NAMESPACE
  const thresholds = {
    low: opts.thresholds?.low ?? DEFAULT_THRESHOLDS.low,
    high: opts.thresholds?.high ?? DEFAULT_THRESHOLDS.high,
    boundary: opts.thresholds?.boundary ?? DEFAULT_THRESHOLDS.boundary,
  }
  const maxLlmCalls = opts.maxLlmCallsPerSession ?? DEFAULT_MAX_LLM_CALLS
  const llmTimeoutMs = opts.llmCallTimeoutMs ?? DEFAULT_LLM_TIMEOUT_MS
  const now = opts.now ?? (() => new Date())

  const turns = await readSessionTurns(root, namespace, opts.sessionId)
  if (turns.length === 0) return []

  const stats = computeSessionStats(turns)
  const ranges = detectBoundaries(turns, { thresholds })

  const episodes: Episode[] = []
  let llmCallsUsed = 0
  let prevChunk: Turn[] | null = null

  for (const [start, end] of ranges) {
    const chunk = turns.slice(start, end + 1)
    if (chunk.length === 0) continue

    const tier1 = scoreChunk(chunk, stats, thresholds, prevChunk)
    prevChunk = chunk

    if (tier1.verdict === 'drop') continue

    if (tier1.verdict === 'keep') {
      episodes.push(
        makeEpisode({
          sessionId: opts.sessionId,
          turnRange: [start, end],
          summary: deterministicSummary(chunk),
          tags: extractDeterministicTags(chunk),
          score: tier1.score,
          scoreSource: 'tier1',
          createdAt: now().toISOString(),
        }),
      )
      continue
    }

    // borderline: tier-2 if budget allows, else default-keep.
    if (llmCallsUsed < maxLlmCalls && opts.llm) {
      const t2 = await callTier2(chunk, {
        llm: opts.llm,
        timeoutMs: llmTimeoutMs,
      })
      llmCallsUsed += 1
      // Only keep if the LLM agreed (≥ 0.5) — or the call failed (default-keep
      // in which case we still keep, per ADR failure-modes).
      if (t2.source === 'tier2-default') {
        episodes.push(
          makeEpisode({
            sessionId: opts.sessionId,
            turnRange: [start, end],
            summary: deterministicSummary(chunk),
            tags: extractDeterministicTags(chunk),
            score: tier1.score, // keep tier-1 score; LLM didn't disagree
            scoreSource: 'tier2-default',
            createdAt: now().toISOString(),
          }),
        )
      } else if (t2.score >= 0.5) {
        episodes.push(
          makeEpisode({
            sessionId: opts.sessionId,
            turnRange: [start, end],
            summary: t2.summary || deterministicSummary(chunk),
            tags: t2.tags.length > 0 ? t2.tags : extractDeterministicTags(chunk),
            score: t2.score,
            scoreSource: 'tier2',
            createdAt: now().toISOString(),
          }),
        )
      }
      continue
    }

    // Budget exhausted (or no llm provided) → default-keep.
    episodes.push(
      makeEpisode({
        sessionId: opts.sessionId,
        turnRange: [start, end],
        summary: deterministicSummary(chunk),
        tags: extractDeterministicTags(chunk),
        score: tier1.score,
        scoreSource: 'tier2-default',
        createdAt: now().toISOString(),
      }),
    )
  }

  return episodes
}

interface EpisodeFields {
  sessionId: string
  turnRange: [number, number]
  summary: string
  tags: string[]
  score: number
  scoreSource: ScoreSource
  createdAt: string
}

function makeEpisode(f: EpisodeFields): Episode {
  return {
    sessionId: f.sessionId,
    turnRange: f.turnRange,
    summary: f.summary,
    tags: f.tags,
    score: f.score,
    scoreSource: f.scoreSource,
    createdAt: f.createdAt,
  }
}
