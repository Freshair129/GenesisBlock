/**
 * Shared helpers for the Symbol Graph CLI + MCP tools — namespace resolution
 * and the canonical on-disk path layout.
 *
 * Mirrors `src/memory/backlinks/indexer.ts` (DEFAULT_NAMESPACE='evaAI', path
 * `.brain/msp/projects/<ns>/...`).
 */

import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'

export const DEFAULT_NAMESPACE = 'evaAI'
export const DB_FILE = 'graph.db'
export const META_FILE = 'meta.json'

export function symbolsDir(root: string, namespace: string = DEFAULT_NAMESPACE): string {
  return resolve(root, '.brain/msp/projects', namespace, 'symbols')
}

export function dbPath(root: string, namespace: string = DEFAULT_NAMESPACE): string {
  return join(symbolsDir(root, namespace), DB_FILE)
}

export function metaPath(root: string, namespace: string = DEFAULT_NAMESPACE): string {
  return join(symbolsDir(root, namespace), META_FILE)
}

export function graphExists(root: string, namespace: string = DEFAULT_NAMESPACE): boolean {
  return existsSync(dbPath(root, namespace))
}
