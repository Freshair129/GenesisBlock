import { basename } from 'node:path'

import { createRestObsidianAdapter } from '@freshair129/gks/memory'

import { isLoopback } from './env.js'
import type { ObsidianClient, SearchHit } from './types.js'

interface RestOpts {
  url: string
  apiKey: string
  timeoutMs: number
  fetchImpl: typeof fetch
}

function smartViewDeepLink(atomId: string): string {
  // Use obsidian://open?file= (resolves in the user's currently-active
  // vault, no vault-name config needed). Avoids the advanced-uri form
  // which requires a vault parameter the wrapper has no way to know.
  return `obsidian://open?file=${encodeURIComponent(atomId)}`
}

/**
 * Convert a relative file path into the wikilink target shape that
 * GKS's `resolveWikilink` expects (basename, no .md extension).
 * Exported for unit tests.
 */
export function wikilinkTargetFor(relPath: string): string {
  return basename(relPath).replace(/\.md$/i, '')
}

export function makeRestClient(opts: RestOpts): ObsidianClient {
  const adapter = createRestObsidianAdapter({
    baseUrl: opts.url.replace(/\/+$/, ''),
    apiKey: opts.apiKey,
    timeoutMs: opts.timeoutMs,
  })
  const headers: Record<string, string> = {
    accept: 'application/json',
    authorization: `Bearer ${opts.apiKey}`,
  }
  return {
    mode: 'rest',
    async search(query, { limit = 10 } = {}) {
      const hits = await adapter.search(query, { limit })
      return hits.map<SearchHit>((h) => ({
        path: h.path,
        title: h.title,
        snippet: h.snippet,
        score: h.score,
      }))
    },
    async readFile(relPath: string) {
      // resolveWikilink expects a wikilink target (basename / atom id),
      // not a full path. e.g. 'gks/adr/ADR--FOO.md' → 'ADR--FOO'.
      const note = await adapter.resolveWikilink(wikilinkTargetFor(relPath))
      if (!note) throw new Error(`obsidian rest: file not found: ${relPath}`)
      return note.body
    },
    async activeFile() {
      const ctrl = new AbortController()
      const t = setTimeout(() => ctrl.abort(), opts.timeoutMs)
      try {
        const res = await opts.fetchImpl(`${opts.url.replace(/\/+$/, '')}/active/`, {
          headers,
          signal: ctrl.signal,
        })
        if (!res.ok) return null
        const body = (await res.json().catch(() => null)) as { path?: string } | null
        return body?.path ?? null
      } catch {
        return null
      } finally {
        clearTimeout(t)
      }
    },
    smartViewDeepLink,
  }
}

export async function probe(opts: {
  url: string
  apiKey: string
  timeoutMs: number
  fetchImpl: typeof fetch
}): Promise<boolean> {
  // Per ADR--MSP-OBSIDIAN-INTEGRATION §TLS: TLS bypass is only allowed
  // for loopback hosts. Non-loopback HTTPS requires OBSIDIAN_INSECURE=true
  // for explicit local-dev override; otherwise we refuse to probe and let
  // the caller fall through to filesystem mode.
  if (
    !isLoopback(opts.url) &&
    opts.url.startsWith('https://') &&
    process.env.OBSIDIAN_INSECURE !== 'true'
  ) {
    return false
  }
  const ctrl = new AbortController()
  const t = setTimeout(() => ctrl.abort(), opts.timeoutMs)
  try {
    const res = await opts.fetchImpl(`${opts.url.replace(/\/+$/, '')}/`, {
      headers: {
        accept: 'application/json',
        authorization: `Bearer ${opts.apiKey}`,
      },
      signal: ctrl.signal,
    })
    // 200/204 = healthy. 401 means server is up but our key is wrong —
    // continuing into REST mode would just fail every call, so fall back
    // to filesystem instead. Other 4xx/5xx → also fall back.
    return res.ok
  } catch {
    return false
  } finally {
    clearTimeout(t)
  }
}
