import { beforeEach, describe, expect, it, vi } from 'vitest'

// Mock the spawn helper BEFORE importing adapters so they pick up the mock.
vi.mock('../../../src/agents/tiers/spawn-helper.js', () => ({
  runCli: vi.fn(),
}))

import { claudeAdapter } from '../../../src/agents/tiers/claude.js'
import { geminiAdapter } from '../../../src/agents/tiers/gemini.js'
import { qwenAdapter } from '../../../src/agents/tiers/qwen.js'
import { runCli } from '../../../src/agents/tiers/spawn-helper.js'
import type { RunResult, TierAdapter } from '../../../src/agents/tiers/types.js'

const DEFAULT_OPTS = { timeout_ms: 1000, capture_stderr: true }
const mockedRunCli = vi.mocked(runCli)

function mockResult(overrides: Partial<RunResult> = {}): RunResult {
  return {
    ok: true,
    output: '',
    exit_code: 0,
    ...overrides,
  }
}

beforeEach(() => {
  mockedRunCli.mockReset()
})

describe('tier adapter structural conformance', () => {
  it('qwenAdapter has name T1', () => {
    expect(qwenAdapter.name).toBe('T1')
  })

  it('geminiAdapter has name T2', () => {
    expect(geminiAdapter.name).toBe('T2')
  })

  it('claudeAdapter has name T3', () => {
    expect(claudeAdapter.name).toBe('T3')
  })

  it('all three adapters conform structurally to TierAdapter', async () => {
    const adapters: readonly TierAdapter[] = [qwenAdapter, geminiAdapter, claudeAdapter]
    const expectedNames: readonly ('T1' | 'T2' | 'T3')[] = ['T1', 'T2', 'T3']

    expect(adapters).toHaveLength(3)

    for (let i = 0; i < adapters.length; i++) {
      const adapter = adapters[i]!
      expect(adapter.name).toBe(expectedNames[i])
      expect(typeof adapter.healthcheck).toBe('function')
      expect(typeof adapter.run).toBe('function')

      mockedRunCli.mockResolvedValueOnce(mockResult({ ok: true, exit_code: 0 }))
      const health = await adapter.healthcheck()
      expect(typeof health).toBe('boolean')

      mockedRunCli.mockResolvedValueOnce(mockResult({ output: 'probe-out' }))
      const result = await adapter.run('probe', DEFAULT_OPTS)
      expect(typeof result.ok).toBe('boolean')
      expect(typeof result.output).toBe('string')
      expect(typeof result.exit_code).toBe('number')
    }
  })
})

describe('qwenAdapter (T1) delegates to runCli', () => {
  it('healthcheck spawns `qwen --help` and returns true on exit 0', async () => {
    // qwen (Python Ollama-wrapper CLI) does NOT support --version. --help
    // returning exit 0 is the binary-installed signal.
    mockedRunCli.mockResolvedValueOnce(mockResult({ ok: true, exit_code: 0 }))
    const ok = await qwenAdapter.healthcheck()
    expect(ok).toBe(true)
    expect(mockedRunCli).toHaveBeenCalledOnce()
    const [bin, args] = mockedRunCli.mock.calls[0]!
    expect(bin).toBe('qwen')
    expect(args).toEqual(['--help'])
  })

  it('healthcheck returns false on non-zero exit', async () => {
    mockedRunCli.mockResolvedValueOnce(mockResult({ ok: false, exit_code: -1 }))
    expect(await qwenAdapter.healthcheck()).toBe(false)
  })

  it('run() spawns `qwen <prompt>` (positional) and forwards opts', async () => {
    mockedRunCli.mockResolvedValueOnce(mockResult({ output: 'hello' }))
    const result = await qwenAdapter.run('hi there', DEFAULT_OPTS)
    expect(result.output).toBe('hello')
    expect(mockedRunCli).toHaveBeenCalledOnce()
    const [bin, args, opts] = mockedRunCli.mock.calls[0]!
    expect(bin).toBe('qwen')
    // qwen takes the prompt as a positional argument; no --prompt flag.
    expect(args).toEqual(['hi there'])
    expect(opts).toEqual(DEFAULT_OPTS)
  })
})

describe('geminiAdapter (T2) delegates to runCli', () => {
  it('healthcheck spawns `gemini --version`', async () => {
    mockedRunCli.mockResolvedValueOnce(mockResult({ ok: true, exit_code: 0 }))
    expect(await geminiAdapter.healthcheck()).toBe(true)
    const [bin, args] = mockedRunCli.mock.calls[0]!
    expect(bin).toBe('gemini')
    expect(args).toEqual(['--version'])
  })

  it('run() spawns `gemini --approval-mode yolo -p <prompt>`', async () => {
    mockedRunCli.mockResolvedValueOnce(mockResult({ output: 'g-out' }))
    const result = await geminiAdapter.run('do thing', DEFAULT_OPTS)
    expect(result.output).toBe('g-out')
    const [bin, args, opts] = mockedRunCli.mock.calls[0]!
    expect(bin).toBe('gemini')
    expect(args).toEqual(['--approval-mode', 'yolo', '-p', 'do thing'])
    expect(opts).toEqual(DEFAULT_OPTS)
  })
})

describe('claudeAdapter (T3) delegates to runCli', () => {
  it('healthcheck spawns `claude --version`', async () => {
    mockedRunCli.mockResolvedValueOnce(mockResult({ ok: true, exit_code: 0 }))
    expect(await claudeAdapter.healthcheck()).toBe(true)
    const [bin, args] = mockedRunCli.mock.calls[0]!
    expect(bin).toBe('claude')
    expect(args).toEqual(['--version'])
  })

  it('run() spawns `claude --print <prompt>`', async () => {
    mockedRunCli.mockResolvedValueOnce(mockResult({ output: 'c-out' }))
    const result = await claudeAdapter.run('plan something', DEFAULT_OPTS)
    expect(result.output).toBe('c-out')
    const [bin, args, opts] = mockedRunCli.mock.calls[0]!
    expect(bin).toBe('claude')
    expect(args).toEqual(['--print', 'plan something'])
    expect(opts).toEqual(DEFAULT_OPTS)
  })
})
