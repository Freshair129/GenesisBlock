import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../../../src/agents/dispatch.js', () => ({
  dispatch: vi.fn(),
}))

import { dispatch as mockedDispatch } from '../../../src/agents/dispatch.js'
import { handler, name } from '../../../src/mcp/tools/dispatch.js'

const dispatchMock = mockedDispatch as unknown as ReturnType<typeof vi.fn>

describe('msp_dispatch tool', () => {
  beforeEach(() => {
    dispatchMock.mockReset()
  })

  it('has the right name', () => {
    expect(name).toBe('msp_dispatch')
  })

  it('invokes dispatch() with a normalised task and returns the result', async () => {
    dispatchMock.mockResolvedValueOnce({
      tier_used: 'T1',
      output: 'mocked summary',
      duration_ms: 12,
    })

    const result = await handler({ root: '/tmp/repo' })({
      prompt: 'hi',
      type: 'summarize',
      severity: 'regular',
    })

    expect(dispatchMock).toHaveBeenCalledTimes(1)
    expect(dispatchMock).toHaveBeenCalledWith({
      type: 'summarize',
      severity: 'regular',
      prompt: 'hi',
    })
    expect(result.isError).toBeUndefined()
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.ok).toBe(true)
    expect(parsed.tier_used).toBe('T1')
    expect(parsed.output).toBe('mocked summary')
    expect(parsed.duration_ms).toBe(12)
  })

  it('uses defaults for type and severity when omitted', async () => {
    dispatchMock.mockResolvedValueOnce({
      tier_used: 'T2',
      output: '',
      duration_ms: 1,
    })
    await handler({ root: '/tmp/repo' })({ prompt: 'p' })
    expect(dispatchMock).toHaveBeenCalledWith({
      type: 'other',
      severity: 'regular',
      prompt: 'p',
    })
  })

  it('passes optional fields through to dispatch()', async () => {
    dispatchMock.mockResolvedValueOnce({
      tier_used: 'T3',
      output: 'ok',
      duration_ms: 5,
      escalated_from: 'T2',
      cost_usd: 0.42,
    })

    const result = await handler({ root: '/tmp/repo' })({
      prompt: 'p',
      context_size_tokens: 100,
      budget_hint: 'T3',
      deadline_ms: 30000,
    })

    expect(dispatchMock).toHaveBeenCalledWith({
      type: 'other',
      severity: 'regular',
      prompt: 'p',
      context_size_tokens: 100,
      budget_hint: 'T3',
      deadline_ms: 30000,
    })
    const parsed = JSON.parse(result.content[0]!.text)
    expect(parsed.escalated_from).toBe('T2')
    expect(parsed.cost_usd).toBe(0.42)
  })

  it('returns isError on invalid task type', async () => {
    const result = await handler({ root: '/tmp/repo' })({ prompt: 'p', type: 'nope' })
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/invalid task type/)
    expect(dispatchMock).not.toHaveBeenCalled()
  })

  it('returns isError on invalid severity', async () => {
    const result = await handler({ root: '/tmp/repo' })({ prompt: 'p', severity: 'meh' })
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/invalid severity/)
  })

  it('returns isError on invalid budget_hint', async () => {
    const result = await handler({ root: '/tmp/repo' })({ prompt: 'p', budget_hint: 'T4' })
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/invalid budget_hint/)
  })

  it('returns isError when dispatch() throws', async () => {
    dispatchMock.mockRejectedValueOnce(new Error('cost cap denied'))
    const result = await handler({ root: '/tmp/repo' })({ prompt: 'p' })
    expect(result.isError).toBe(true)
    expect(result.content[0]!.text).toMatch(/dispatch failed/)
    expect(result.content[0]!.text).toMatch(/cost cap denied/)
  })
})
