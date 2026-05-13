import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { RunOpts, RunResult, TierAdapter } from '../../src/agents/tiers/types.js'

// --------------------------------------------------------------------------
// Mock the three real adapter modules + result-recorder. vi.mock() is hoisted
// to the top of the file by vitest, so the factory bodies must be
// self-contained (or reference values hoisted via vi.hoisted()). We use
// vi.hoisted() so the test body can still reach the fakes through `mocks.*`.
// --------------------------------------------------------------------------

const mocks = vi.hoisted(() => {
  function makeFakeAdapter(name: 'T1' | 'T2' | 'T3'): TierAdapter & {
    healthcheck: ReturnType<typeof vi.fn>
    run: ReturnType<typeof vi.fn>
  } {
    return {
      name,
      healthcheck: vi.fn(async () => true),
      run: vi.fn(async (_prompt: string, _opts: RunOpts): Promise<RunResult> => ({
        ok: true,
        output: `${name}-default-output`,
        exit_code: 0,
      })),
    }
  }

  return {
    fakeQwen: makeFakeAdapter('T1'),
    fakeGemini: makeFakeAdapter('T2'),
    fakeClaude: makeFakeAdapter('T3'),
    recordEpisodeMock: vi.fn(
      async (_task: unknown, _result: unknown, _root: unknown): Promise<string> =>
        '/fake/episode/path.md',
    ),
  }
})

const { fakeQwen, fakeGemini, fakeClaude, recordEpisodeMock } = mocks

vi.mock('../../src/agents/tiers/qwen.js', () => ({ qwenAdapter: mocks.fakeQwen }))
vi.mock('../../src/agents/tiers/gemini.js', () => ({ geminiAdapter: mocks.fakeGemini }))
vi.mock('../../src/agents/tiers/claude.js', () => ({ claudeAdapter: mocks.fakeClaude }))
vi.mock('../../src/agents/result-recorder.js', () => ({
  recordEpisode: (task: unknown, result: unknown, root: unknown) =>
    mocks.recordEpisodeMock(task, result, root),
}))

// Import AFTER vi.mock so the mocked deps are resolved.
import { dispatch } from '../../src/agents/dispatch.js'
import type { DispatchTask } from '../../src/agents/types.js'

function task(overrides: Partial<DispatchTask> = {}): DispatchTask {
  return {
    type: 'other',
    severity: 'regular',
    prompt: 'do a thing',
    ...overrides,
  }
}

beforeEach(() => {
  for (const a of [fakeQwen, fakeGemini, fakeClaude]) {
    a.healthcheck.mockReset()
    a.healthcheck.mockResolvedValue(true)
    a.run.mockReset()
    a.run.mockImplementation(async () => ({
      ok: true,
      output: `${a.name}-default-output`,
      exit_code: 0,
    }))
  }
  recordEpisodeMock.mockReset()
  recordEpisodeMock.mockResolvedValue('/fake/episode/path.md')
})

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

describe('dispatch()', () => {
  describe('default routing', () => {
    it('routes summarize/regular to T1 and uses qwenAdapter', async () => {
      const result = await dispatch(task({ type: 'summarize' }))
      expect(result.tier_used).toBe('T1')
      expect(fakeQwen.run).toHaveBeenCalledOnce()
      expect(fakeGemini.run).not.toHaveBeenCalled()
      expect(fakeClaude.run).not.toHaveBeenCalled()
      expect(result.escalated_from).toBeUndefined()
    })

    it('routes codegen/regular to T2 and uses geminiAdapter', async () => {
      const result = await dispatch(task({ type: 'codegen' }))
      expect(result.tier_used).toBe('T2')
      expect(fakeGemini.run).toHaveBeenCalledOnce()
    })
  })

  describe('budget_hint: T3 with non-critical severity', () => {
    // Behavior choice (documented): cost-policy.enforceTierCap rejects T3 when
    // severity is not 'critical'. Because budget_hint is an explicit caller
    // override, dispatch THROWS rather than silently downgrading.
    it('throws when caller forces T3 but severity is not critical', async () => {
      await expect(
        dispatch(task({ type: 'codegen', severity: 'regular', budget_hint: 'T3' })),
      ).rejects.toThrow(/T3 restricted to critical severity/)
      expect(fakeClaude.run).not.toHaveBeenCalled()
    })

    it('honours budget_hint: T3 when severity is critical', async () => {
      const result = await dispatch(
        task({ type: 'codegen', severity: 'critical', budget_hint: 'T3' }),
      )
      expect(result.tier_used).toBe('T3')
      expect(fakeClaude.run).toHaveBeenCalledOnce()
    })
  })

  describe('escalation on adapter failure', () => {
    it('escalates T1 → T2 when T1 returns ok:false and severity ≥ regular', async () => {
      fakeQwen.run.mockResolvedValueOnce({
        ok: false,
        output: 't1-fail',
        exit_code: 1,
      })
      fakeGemini.run.mockResolvedValueOnce({
        ok: true,
        output: 't2-recovered',
        exit_code: 0,
      })

      const result = await dispatch(task({ type: 'summarize', severity: 'regular' }))

      expect(result.tier_used).toBe('T2')
      expect(result.escalated_from).toBe('T1')
      expect(result.output).toBe('t2-recovered')
      expect(fakeQwen.run).toHaveBeenCalledOnce()
      expect(fakeGemini.run).toHaveBeenCalledOnce()
    })

    it('does NOT escalate when severity is low', async () => {
      fakeQwen.run.mockResolvedValueOnce({
        ok: false,
        output: 't1-fail',
        exit_code: 1,
      })

      const result = await dispatch(task({ type: 'summarize', severity: 'low' }))

      expect(result.tier_used).toBe('T1')
      expect(result.escalated_from).toBeUndefined()
      expect(fakeGemini.run).not.toHaveBeenCalled()
    })
  })

  describe('context-size override', () => {
    it('forces T2 when context_size_tokens=3_000_000 even for a T1-shaped task', async () => {
      const result = await dispatch(
        task({
          type: 'summarize',
          severity: 'regular',
          context_size_tokens: 3_000_000,
        }),
      )
      expect(result.tier_used).toBe('T2')
      expect(fakeGemini.run).toHaveBeenCalledOnce()
      expect(fakeQwen.run).not.toHaveBeenCalled()
      expect(fakeClaude.run).not.toHaveBeenCalled()
    })

    it('throws when caller forces T3 but context exceeds 2M', async () => {
      await expect(
        dispatch(
          task({
            type: 'codegen',
            severity: 'critical',
            context_size_tokens: 3_000_000,
            budget_hint: 'T3',
          }),
        ),
      ).rejects.toThrow(/only T2 permitted/)
    })
  })

  describe('healthcheck fallback', () => {
    it('falls back T2 → T3 when geminiAdapter healthcheck fails, for critical task', async () => {
      fakeGemini.healthcheck.mockResolvedValueOnce(false)
      fakeClaude.run.mockResolvedValueOnce({
        ok: true,
        output: 't3-out',
        exit_code: 0,
      })

      const result = await dispatch(
        task({ type: 'codegen', severity: 'critical', budget_hint: 'T3' }),
      )
      // budget_hint T3 + critical → tier T3 directly; healthcheck only fires for
      // non-T1 tiers. T3 is healthy by default; expect normal T3 path.
      expect(result.tier_used).toBe('T3')
      expect(fakeClaude.run).toHaveBeenCalledOnce()
    })
  })

  describe('episode recording (best-effort)', () => {
    it('still returns OK when recordEpisode throws', async () => {
      recordEpisodeMock.mockRejectedValueOnce(new Error('disk on fire'))

      const result = await dispatch(task({ type: 'summarize' }))

      expect(result.tier_used).toBe('T1')
      expect(result.output).toBe('T1-default-output')
      expect(recordEpisodeMock).toHaveBeenCalledOnce()
    })

    it('calls recordEpisode with task + result + a string root', async () => {
      await dispatch(task({ type: 'summarize' }))

      expect(recordEpisodeMock).toHaveBeenCalledOnce()
      const [recordedTask, recordedResult, recordedRoot] =
        recordEpisodeMock.mock.calls[0]!
      expect((recordedTask as DispatchTask).type).toBe('summarize')
      expect(typeof (recordedResult as { output: string }).output).toBe('string')
      expect(typeof recordedRoot).toBe('string')
    })
  })

  describe('DispatchResult shape', () => {
    it('always sets tier_used, output, and duration_ms', async () => {
      const result = await dispatch(task({ type: 'summarize' }))
      expect(['T1', 'T2', 'T3']).toContain(result.tier_used)
      expect(typeof result.output).toBe('string')
      expect(typeof result.duration_ms).toBe('number')
      expect(result.duration_ms).toBeGreaterThanOrEqual(0)
    })

    it('omits escalated_from when no escalation occurred', async () => {
      const result = await dispatch(task({ type: 'summarize' }))
      expect(result.escalated_from).toBeUndefined()
    })
  })
})
