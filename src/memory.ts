import { MemoryStore, createRestObsidianAdapter, wrapObsidianWithCache } from '@freshair129/gks'

let _store: MemoryStore | null = null

export function getStore(): MemoryStore {
  if (_store) return _store

  const obsidianUrl = process.env.OBSIDIAN_URL
  const obsidianKey = process.env.OBSIDIAN_API_KEY

  const obsidian =
    obsidianUrl
      ? wrapObsidianWithCache(
          createRestObsidianAdapter({
            baseUrl: obsidianUrl,
            apiKey: obsidianKey,
          }),
        )
      : undefined

  _store = new MemoryStore({
    root: process.env.GKS_ROOT ?? './data',
    obsidian,
  })

  return _store
}
