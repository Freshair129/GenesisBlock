import { resolveEnv } from './env.js'
import { makeFilesystemClient } from './filesystem.js'
import { makeRestClient, probe } from './rest.js'
import type { ClientOpts, ObsidianClient } from './types.js'

export type { ClientOpts, ObsidianClient, SearchHit } from './types.js'

export async function createObsidianClient(
  opts: ClientOpts = {},
): Promise<ObsidianClient> {
  const root = opts.root ?? process.cwd()
  const warn = opts.warn ?? ((m) => process.stderr.write(`${m}\n`))
  const env = resolveEnv({ url: opts.url, apiKey: opts.apiKey }, warn)
  const timeoutMs = opts.timeoutMs ?? 1500
  const fetchImpl = opts.fetch ?? fetch

  if (env.url && env.apiKey) {
    const ok = await probe({
      url: env.url,
      apiKey: env.apiKey,
      timeoutMs,
      fetchImpl,
    })
    if (ok) {
      return makeRestClient({
        url: env.url,
        apiKey: env.apiKey,
        timeoutMs,
        fetchImpl,
      })
    }
  }
  return makeFilesystemClient({ root })
}
