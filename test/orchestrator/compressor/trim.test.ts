import { describe, expect, it } from 'vitest'

import { trimEpisode } from '../../../src/orchestrator/compressor/trim.js'
import type { Turn } from '../../../src/orchestrator/consolidator/types.js'
import type { CompressorEpisode } from '../../../src/orchestrator/compressor/types.js'

function turn(content: string, speakerId = 'user', turnId = 0): Turn {
  return {
    sessionId: 's1',
    episodicId: 'e1',
    turnId,
    msgId: `m${turnId}`,
    speakerId,
    content,
  }
}

function makeEpisode(turns: Turn[], score = 0.7): CompressorEpisode {
  return {
    sessionId: 's1',
    turnRange: [0, Math.max(0, turns.length - 1)],
    summary: 'test summary',
    score,
    turns,
  }
}

const HIGH_SIGNAL = 'we will use pgvector for retrieval per ADR--RETRIEVAL'
const HIGH_SIGNAL_ALT = "let's ship the consolidator at v1.2.3 in src/foo.ts"
const FILLER = 'hi thanks ok'
const FILLER_ALT = 'sure noted yep'
const FILLER_LONG = 'hello hi thanks ok sure sounds good noted yes yep'

describe('trimEpisode', () => {
  it('returns no drops when budget already fits the whole episode', () => {
    const ep = makeEpisode([turn(HIGH_SIGNAL, 'a', 0), turn('great', 'u', 1)])
    const r = trimEpisode(ep, 10_000)
    expect(r.droppedIndices).toEqual([])
    expect(r.fits).toBe(true)
    expect(r.text).toContain(HIGH_SIGNAL)
  })

  it('drops low-score filler turns first to make budget', () => {
    // 1 high-signal + 4 fillers — high should survive.
    const turns = [
      turn(HIGH_SIGNAL, 'a', 0),
      turn(FILLER, 'u', 1),
      turn(FILLER_ALT, 'u', 2),
      turn(FILLER, 'u', 3),
      turn(FILLER_ALT, 'u', 4),
    ]
    const ep = makeEpisode(turns)
    // Budget tight enough that fillers must go.
    const r = trimEpisode(ep, 20)
    expect(r.text).toContain(HIGH_SIGNAL)
    expect(r.droppedIndices.length).toBeGreaterThan(0)
    // High-signal turn (index 0) must NOT have been dropped.
    expect(r.droppedIndices).not.toContain(0)
  })

  it('never drops high-signal turns even when budget is impossibly tight', () => {
    const turns = [
      turn(HIGH_SIGNAL, 'a', 0),
      turn(HIGH_SIGNAL_ALT, 'a', 1),
      turn(FILLER, 'u', 2),
    ]
    const ep = makeEpisode(turns)
    const r = trimEpisode(ep, 5) // way too tight
    // The two high-signal turns should not appear in droppedIndices.
    expect(r.droppedIndices).not.toContain(0)
    expect(r.droppedIndices).not.toContain(1)
    // But fits should be false since budget is unachievable.
    expect(r.fits).toBe(false)
  })

  it('reports fits=false when no further candidates can be dropped', () => {
    const turns = [
      turn(HIGH_SIGNAL, 'a', 0),
      turn(HIGH_SIGNAL_ALT, 'a', 1),
    ]
    const ep = makeEpisode(turns)
    const r = trimEpisode(ep, 3)
    expect(r.fits).toBe(false)
    expect(r.droppedIndices.length).toBe(0)
  })

  it('returns droppedIndices in chronological order (sorted asc)', () => {
    const turns = [
      turn(FILLER, 'u', 0),
      turn(HIGH_SIGNAL, 'a', 1),
      turn(FILLER_ALT, 'u', 2),
      turn(FILLER_LONG, 'u', 3),
      turn(HIGH_SIGNAL_ALT, 'a', 4),
    ]
    const ep = makeEpisode(turns)
    const r = trimEpisode(ep, 30)
    // Whatever was dropped, indices must be ascending.
    const sorted = [...r.droppedIndices].sort((a, b) => a - b)
    expect(r.droppedIndices).toEqual(sorted)
  })

  it('preserves chronological order in output text', () => {
    const turns = [
      turn(HIGH_SIGNAL, 'a', 0),
      turn(FILLER, 'u', 1),
      turn(HIGH_SIGNAL_ALT, 'a', 2),
    ]
    const ep = makeEpisode(turns)
    const r = trimEpisode(ep, 50)
    if (r.fits) {
      const idxFirst = r.text.indexOf(HIGH_SIGNAL)
      const idxSecond = r.text.indexOf(HIGH_SIGNAL_ALT)
      expect(idxFirst).toBeGreaterThanOrEqual(0)
      expect(idxSecond).toBeGreaterThan(idxFirst)
    }
  })

  it('is idempotent on identical input (no mutation, deterministic)', () => {
    const turns = [
      turn(HIGH_SIGNAL, 'a', 0),
      turn(FILLER, 'u', 1),
      turn(HIGH_SIGNAL_ALT, 'a', 2),
      turn(FILLER_LONG, 'u', 3),
    ]
    const ep = makeEpisode(turns)
    const original = JSON.parse(JSON.stringify(ep))
    const r1 = trimEpisode(ep, 25)
    const r2 = trimEpisode(ep, 25)
    expect(r1).toEqual(r2)
    // No mutation of input.
    expect(ep).toEqual(original)
  })
})
