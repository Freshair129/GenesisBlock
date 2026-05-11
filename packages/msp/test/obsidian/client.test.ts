import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { createObsidianClient } from '../../src/obsidian/client.js'
import { _resetWarnedForTests, resolveEnv, isLoopback } from '../../src/obsidian/env.js'
import { makeFilesystemClient } from '../../src/obsidian/filesystem.js'
import { makeRestClient, probe, wikilinkTargetFor } from '../../src/obsidian/rest.js'

async function fixtureVault(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-obsidian-'))
  const adr = join(root, 'gks', 'adr')
  const concept = join(root, 'gks', 'concept')
  await mkdir(adr, { recursive: true })
  await mkdir(concept, { recursive: true })
  await writeFile(
    join(adr, 'ADR--FOO.md'),
    '---\nid: ADR--FOO\ntitle: Foo decision passport\n---\n# ADR — foo\nthis is the passport body\n',
    'utf8',
  )
  await writeFile(
    join(concept, 'CONCEPT--BAR.md'),
    '---\nid: CONCEPT--BAR\ntitle: Bar concept\n---\n# CONCEPT — bar\nunrelated content here\n',
    'utf8',
  )
  return root
}

const ENV_KEYS = ['OBSIDIAN_URL', 'OBSIDIAN_HOST', 'OBSIDIAN_API_KEY', 'OBSIDIAN_INSECURE']

function clearEnv(): void {
  for (const k of ENV_KEYS) delete process.env[k]
  _resetWarnedForTests()
}

describe('createObsidianClient — filesystem mode', () => {
  beforeEach(clearEnv)
  afterEach(clearEnv)

  it('falls back to filesystem when no OBSIDIAN_URL is set', async () => {
    const root = await fixtureVault()
    const c = await createObsidianClient({ root })
    expect(c.mode).toBe('filesystem')
    expect(c.smartViewDeepLink).toBeUndefined()
    expect(c.activeFile).toBeUndefined()
  })

  it('falls back to filesystem when OBSIDIAN_API_KEY is missing', async () => {
    const root = await fixtureVault()
    process.env.OBSIDIAN_URL = 'http://127.0.0.1:27124'
    const c = await createObsidianClient({ root })
    expect(c.mode).toBe('filesystem')
  })

  it('search finds atoms by case-insensitive substring', async () => {
    const root = await fixtureVault()
    const c = await createObsidianClient({ root })
    const hits = await c.search('passport', { limit: 5 })
    expect(hits.length).toBeGreaterThan(0)
    expect(hits[0].path).toContain('ADR--FOO.md')
    expect(hits[0].title).toContain('Foo decision passport')
    expect(hits[0].snippet.toLowerCase()).toContain('passport')
  })

  it('search returns empty array for empty query', async () => {
    const root = await fixtureVault()
    const c = await createObsidianClient({ root })
    expect(await c.search('  ')).toEqual([])
  })

  it('readFile reads gks/<type>/*.md by relative path', async () => {
    const root = await fixtureVault()
    const c = await createObsidianClient({ root })
    const body = await c.readFile('gks/adr/ADR--FOO.md')
    expect(body).toContain('passport body')
  })
})

describe('createObsidianClient — rest mode', () => {
  beforeEach(clearEnv)
  afterEach(clearEnv)

  it('selects rest mode when probe succeeds and key is set', async () => {
    process.env.OBSIDIAN_URL = 'http://127.0.0.1:27124'
    process.env.OBSIDIAN_API_KEY = 'test-token'
    const root = await fixtureVault()
    const fakeFetch = (async (url: string | URL) => {
      const u = typeof url === 'string' ? url : url.toString()
      if (u.endsWith('/')) return new Response('{}', { status: 200 })
      return new Response('not found', { status: 404 })
    }) as unknown as typeof fetch
    const c = await createObsidianClient({ root, fetch: fakeFetch })
    expect(c.mode).toBe('rest')
    expect(typeof c.smartViewDeepLink).toBe('function')
    expect(typeof c.activeFile).toBe('function')
  })

  it('falls back to filesystem when probe fails (network error)', async () => {
    process.env.OBSIDIAN_URL = 'http://127.0.0.1:27124'
    process.env.OBSIDIAN_API_KEY = 'test-token'
    const root = await fixtureVault()
    const fakeFetch = (async () => {
      throw new Error('ECONNREFUSED')
    }) as unknown as typeof fetch
    const c = await createObsidianClient({ root, fetch: fakeFetch })
    expect(c.mode).toBe('filesystem')
  })

  it('falls back to filesystem when probe returns 5xx', async () => {
    process.env.OBSIDIAN_URL = 'http://127.0.0.1:27124'
    process.env.OBSIDIAN_API_KEY = 'test-token'
    const root = await fixtureVault()
    const fakeFetch = (async () =>
      new Response('boom', { status: 503 })) as unknown as typeof fetch
    const c = await createObsidianClient({ root, fetch: fakeFetch })
    expect(c.mode).toBe('filesystem')
  })

  it('treats 401 as unhealthy (wrong key → fall back to filesystem)', async () => {
    process.env.OBSIDIAN_URL = 'http://127.0.0.1:27124'
    process.env.OBSIDIAN_API_KEY = 'wrong-token'
    const ok = await probe({
      url: 'http://127.0.0.1:27124',
      apiKey: 'wrong-token',
      timeoutMs: 100,
      fetchImpl: (async () =>
        new Response('unauthorized', { status: 401 })) as unknown as typeof fetch,
    })
    expect(ok).toBe(false)
  })

  it('rejects non-loopback HTTPS without OBSIDIAN_INSECURE (no fetch made)', async () => {
    delete process.env.OBSIDIAN_INSECURE
    let fetchCalled = false
    const ok = await probe({
      url: 'https://example.com:27124',
      apiKey: 'k',
      timeoutMs: 100,
      fetchImpl: (async () => {
        fetchCalled = true
        return new Response('{}', { status: 200 })
      }) as unknown as typeof fetch,
    })
    expect(ok).toBe(false)
    expect(fetchCalled).toBe(false)
  })

  it('allows non-loopback HTTPS when OBSIDIAN_INSECURE=true', async () => {
    process.env.OBSIDIAN_INSECURE = 'true'
    let fetchCalled = false
    const ok = await probe({
      url: 'https://example.com:27124',
      apiKey: 'k',
      timeoutMs: 100,
      fetchImpl: (async () => {
        fetchCalled = true
        return new Response('{}', { status: 200 })
      }) as unknown as typeof fetch,
    })
    expect(ok).toBe(true)
    expect(fetchCalled).toBe(true)
    delete process.env.OBSIDIAN_INSECURE
  })

  it('falls back to filesystem on 401 (REST mode never engaged)', async () => {
    process.env.OBSIDIAN_URL = 'http://127.0.0.1:27124'
    process.env.OBSIDIAN_API_KEY = 'wrong-token'
    const root = await fixtureVault()
    const fakeFetch = (async () =>
      new Response('unauthorized', { status: 401 })) as unknown as typeof fetch
    const c = await createObsidianClient({ root, fetch: fakeFetch })
    expect(c.mode).toBe('filesystem')
  })
})

describe('OBSIDIAN_HOST deprecation', () => {
  beforeEach(clearEnv)
  afterEach(clearEnv)

  it('emits a one-shot deprecation warning when OBSIDIAN_HOST is set instead of OBSIDIAN_URL', async () => {
    process.env.OBSIDIAN_HOST = 'http://127.0.0.1:27124'
    const messages: string[] = []
    const root = await fixtureVault()
    await createObsidianClient({ root, warn: (m) => messages.push(m) })
    expect(messages.some((m) => m.includes('OBSIDIAN_HOST is deprecated'))).toBe(true)
    expect(messages.length).toBe(1)
  })

  it('does not warn when OBSIDIAN_URL is set (even if OBSIDIAN_HOST is also set)', async () => {
    process.env.OBSIDIAN_URL = 'http://127.0.0.1:27124'
    process.env.OBSIDIAN_HOST = 'http://localhost:27124'
    const messages: string[] = []
    const root = await fixtureVault()
    await createObsidianClient({ root, warn: (m) => messages.push(m) })
    expect(messages).toEqual([])
  })

  it('resolveEnv picks up OBSIDIAN_HOST as the URL when only HOST is set', () => {
    process.env.OBSIDIAN_HOST = 'http://127.0.0.1:27124'
    const env = resolveEnv({}, () => {})
    expect(env.url).toBe('http://127.0.0.1:27124')
    expect(env.hostDeprecated).toBe(true)
  })
})

describe('smartViewDeepLink + helpers', () => {
  beforeEach(clearEnv)
  afterEach(clearEnv)

  it('builds an obsidian://open deep link from an atom id (active-vault form)', () => {
    const c = makeRestClient({
      url: 'http://127.0.0.1:27124',
      apiKey: 'k',
      timeoutMs: 100,
      fetchImpl: fetch,
    })
    const link = c.smartViewDeepLink!('FRAME--MSP-ARCHITECTURE-V2')
    expect(link).toBe('obsidian://open?file=FRAME--MSP-ARCHITECTURE-V2')
  })

  it('URL-encodes atom ids that contain special characters', () => {
    const c = makeRestClient({
      url: 'http://127.0.0.1:27124',
      apiKey: 'k',
      timeoutMs: 100,
      fetchImpl: fetch,
    })
    const link = c.smartViewDeepLink!('ADR--FOO BAR/BAZ')
    expect(link).toBe('obsidian://open?file=ADR--FOO%20BAR%2FBAZ')
  })

  it('isLoopback recognises 127.0.0.1 / localhost / ::1', () => {
    expect(isLoopback('http://127.0.0.1:27124')).toBe(true)
    expect(isLoopback('https://localhost:27124')).toBe(true)
    expect(isLoopback('http://[::1]:27124')).toBe(true)
    expect(isLoopback('https://example.com')).toBe(false)
    expect(isLoopback('not-a-url')).toBe(false)
  })

  it('makeFilesystemClient mode reports correctly', () => {
    const c = makeFilesystemClient({ root: process.cwd() })
    expect(c.mode).toBe('filesystem')
    expect(c.smartViewDeepLink).toBeUndefined()
  })

  it('wikilinkTargetFor extracts basename + strips .md (paths → ids)', () => {
    expect(wikilinkTargetFor('gks/adr/ADR--FOO.md')).toBe('ADR--FOO')
    expect(wikilinkTargetFor('gks/concept/CONCEPT--BAR.md')).toBe('CONCEPT--BAR')
    expect(wikilinkTargetFor('ADR--BARE.md')).toBe('ADR--BARE')
    expect(wikilinkTargetFor('FRAME--NO-EXT')).toBe('FRAME--NO-EXT')
    expect(wikilinkTargetFor('deep/nested/path/AUDIT--X.MD')).toBe('AUDIT--X')
  })
})
