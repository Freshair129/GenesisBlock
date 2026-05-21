import { resolve, join } from 'node:path'
import { stat } from 'node:fs/promises'
import { forEachJsonl } from '../lib/jsonl.js'
import type { AtomicEntry } from './types.js'

/**
 * Loads an atomic index from a remote path (currently local filesystem).
 */
export async function loadRemoteIndex(
  remotePath: string,
  root: string = process.cwd()
): Promise<AtomicEntry[]> {
  // Resolve path relative to current root if not absolute
  const absolutePath = resolve(root, remotePath)
  
  // Assume index is at <remote>/gks/00_index/atomic_index.jsonl or <remote>/00_index/...
  // We'll try a few common locations
  const candidates = [
    absolutePath, // Direct path to jsonl
    join(absolutePath, 'gks', '00_index', 'atomic_index.jsonl'),
    join(absolutePath, '00_index', 'atomic_index.jsonl'),
  ]

  for (const path of candidates) {
    try {
      const s = await stat(path)
      if (!s.isFile()) continue

      const entries: AtomicEntry[] = []
      await forEachJsonl<AtomicEntry>(path, (row) => {
        if (row.id) entries.push(row)
      })
      console.error(`[remote] loaded ${entries.length} atoms from ${path}`)
      return entries
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') continue
      throw err
    }
  }

  throw new Error(`Could not find atomic index at ${remotePath} (tried ${candidates.join(', ')})`)
}
