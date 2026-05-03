import { describe, expect, it, vi } from 'vitest'

import { createOllamaClient } from '../../../src/codegen/slm/ollama.js'
import { SlmError } from '../../../src/codegen/slm/errors.js'

const CALL = {
  prompt: 'write hello',
  model: 'qwen2.5-coder:7b',
  attempt: 1,
}

function mockFetch(response: Response | Promise<Response>): typeof fetch {
  return vi.fn().mockResolvedValue(response) as unknown as typeof fetch
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

describe('createOllamaClient — happy path', () => {
  it('returns the response string from a 200 JSON body', async () => {
    const client = createOllamaClient({ fetchImpl: mockFetch(jsonResponse({ response: 'export const x = 1\n' })) })
    const out = await client(CALL)
    expect(out).toBe('export const x = 1\n')
  })

  it('POSTs to <host>/api/generate with model + prompt + stream:false', async () => {
    const fetchSpy = vi.fn().mockResolvedValue(jsonResponse({ response: 'ok' }))
    const client = createOllamaClient({
      host: 'http://1.2.3.4:9999',
      model: 'tiny',
      fetchImpl: fetchSpy as unknown as typeof fetch,
    })
    await client(CALL)
    expect(fetchSpy).toHaveBeenCalledOnce()
    const [url, init] = fetchSpy.mock.calls[0]!
    expect(url).toBe('http://1.2.3.4:9999/api/generate')
    expect(init.method).toBe('POST')
    const body = JSON.parse(init.body as string)
    expect(body.model).toBe('tiny')
    expect(body.prompt).toBe('write hello')
    expect(body.stream).toBe(false)
  })

  it('strips trailing slash from host', async () => {
    const fetchSpy = vi.fn().mockResolvedValue(jsonResponse({ response: 'ok' }))
    const client = createOllamaClient({
      host: 'http://1.2.3.4:9999///',
      fetchImpl: fetchSpy as unknown as typeof fetch,
    })
    await client(CALL)
    const [url] = fetchSpy.mock.calls[0]!
    expect(url).toBe('http://1.2.3.4:9999/api/generate')
  })
})

describe('createOllamaClient — errors', () => {
  it('throws SlmError(http) on non-2xx', async () => {
    const client = createOllamaClient({
      fetchImpl: mockFetch(new Response('boom', { status: 500 })),
    })
    await expect(client(CALL)).rejects.toMatchObject({ kind: 'http' })
  })

  it('throws SlmError(network) when fetch throws', async () => {
    const client = createOllamaClient({
      fetchImpl: vi.fn().mockRejectedValue(new Error('ECONNREFUSED')) as unknown as typeof fetch,
    })
    await expect(client(CALL)).rejects.toMatchObject({ kind: 'network' })
  })

  it('throws SlmError(parse) on invalid JSON body', async () => {
    const client = createOllamaClient({
      fetchImpl: mockFetch(new Response('not json', { status: 200, headers: { 'content-type': 'application/json' } })),
    })
    await expect(client(CALL)).rejects.toMatchObject({ kind: 'parse' })
  })

  it('throws SlmError(parse) when response field is missing', async () => {
    const client = createOllamaClient({
      fetchImpl: mockFetch(jsonResponse({ noResponseField: true })),
    })
    await expect(client(CALL)).rejects.toMatchObject({ kind: 'parse' })
  })

  it('throws SlmError(timeout) when fetch hangs past timeoutMs', async () => {
    const slowFetch = vi.fn().mockImplementation((_url: unknown, init: RequestInit) =>
      new Promise<Response>((_resolve, reject) => {
        init.signal!.addEventListener('abort', () => reject(new Error('aborted')))
      }),
    ) as unknown as typeof fetch
    const client = createOllamaClient({ fetchImpl: slowFetch, timeoutMs: 25 })
    await expect(client(CALL)).rejects.toMatchObject({ kind: 'timeout' })
  })

  it('throws SlmError(config) when fetchImpl is not a function', async () => {
    expect(() =>
      createOllamaClient({ fetchImpl: undefined as never }),
    ).not.toThrow() // OK if globalThis.fetch present
    // simulate missing global fetch
    const oldFetch = globalThis.fetch
    // @ts-expect-error — testing the missing-fetch path
    globalThis.fetch = undefined
    try {
      expect(() => createOllamaClient()).toThrow(SlmError)
    } finally {
      globalThis.fetch = oldFetch
    }
  })
})

describe('env-var fallbacks', () => {
  it('uses OLLAMA_HOST and OLLAMA_MODEL env vars by default', async () => {
    const old = { host: process.env.OLLAMA_HOST, model: process.env.OLLAMA_MODEL }
    process.env.OLLAMA_HOST = 'http://env-host:1234'
    process.env.OLLAMA_MODEL = 'env-model'
    const fetchSpy = vi.fn().mockResolvedValue(jsonResponse({ response: 'ok' }))
    const client = createOllamaClient({ fetchImpl: fetchSpy as unknown as typeof fetch })
    await client(CALL)
    const [url, init] = fetchSpy.mock.calls[0]!
    expect(url).toBe('http://env-host:1234/api/generate')
    expect(JSON.parse(init.body as string).model).toBe('env-model')

    process.env.OLLAMA_HOST = old.host
    process.env.OLLAMA_MODEL = old.model
  })
})
