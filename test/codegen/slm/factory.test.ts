import { describe, expect, it } from 'vitest'

import { createSlmClient } from '../../../src/codegen/slm/factory.js'
import { SlmError } from '../../../src/codegen/slm/errors.js'

describe('createSlmClient', () => {
  it('returns a callable client for provider:ollama (no network in this test)', () => {
    const client = createSlmClient({ provider: 'ollama', ollama: { fetchImpl: (async () => new Response('{}')) as typeof fetch } })
    expect(typeof client).toBe('function')
  })

  it('returns the deterministic mock for provider:mock', async () => {
    const client = createSlmClient({ provider: 'mock' })
    const out = await client({ prompt: 'no hint here', model: 'x', attempt: 1 })
    expect(out).toMatch(/handler/)
  })

  it('throws SlmError(config) for an unknown provider', () => {
    expect(() => createSlmClient({ provider: 'gpt99' as never })).toThrow(SlmError)
  })

  it('honours MSP_SLM_PROVIDER env var when no opts.provider', () => {
    const old = process.env.MSP_SLM_PROVIDER
    process.env.MSP_SLM_PROVIDER = 'mock'
    try {
      const client = createSlmClient()
      expect(typeof client).toBe('function')
    } finally {
      process.env.MSP_SLM_PROVIDER = old
    }
  })
})
