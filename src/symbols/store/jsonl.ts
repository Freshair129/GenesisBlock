/**
 * JSONL exporter + loader for the Symbol Graph.
 *
 * Per ADR--SYMBOL-GRAPH-PERSISTENCE: SQLite is the runtime index, JSONL is
 * the git-committed source of truth. Both must round-trip byte-identically:
 *   build → dump → load fresh SQLite → dump → byte-compare
 *
 * Sort order:
 *   - symbols.jsonl     — by Symbol.id (lexicographic)
 *   - edges.jsonl       — by (src_id, dst_id, type)
 *   - communities.jsonl — by Community.id (numeric)
 *
 * Keys are emitted in a fixed order via explicit objects (NOT Object.keys
 * iteration) to stay deterministic across V8 versions.
 */

import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

import type { Community, Edge, Symbol } from '../types.js'
import type { SymbolStore } from './sqlite.js'

const SYMBOLS_FILE = 'symbols.jsonl'
const EDGES_FILE = 'edges.jsonl'
const COMMUNITIES_FILE = 'communities.jsonl'

function symbolToLine(s: Symbol): string {
  // Fixed key order — see header comment.
  const obj: Record<string, unknown> = {
    id: s.id,
    name: s.name,
    kind: s.kind,
    file: s.file,
    start_line: s.start_line,
    end_line: s.end_line,
    exported: s.exported,
    parent_id: s.parent_id,
    signature: s.signature,
    community_id: s.community_id,
    created_at: s.created_at,
  }
  return JSON.stringify(obj)
}

function edgeToLine(e: Edge): string {
  const obj: Record<string, unknown> = {
    src_id: e.src_id,
    dst_id: e.dst_id,
    type: e.type,
    weight: e.weight,
    resolved: e.resolved,
  }
  return JSON.stringify(obj)
}

function communityToLine(c: Community): string {
  const obj: Record<string, unknown> = {
    id: c.id,
    size: c.size,
    label: c.label,
    modularity: c.modularity,
    parent_id: c.parent_id,
  }
  return JSON.stringify(obj)
}

function joinLines(lines: string[]): string {
  if (lines.length === 0) return ''
  return lines.join('\n') + '\n'
}

function sortSymbols(symbols: Symbol[]): Symbol[] {
  return [...symbols].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
}

function sortEdges(edges: Edge[]): Edge[] {
  return [...edges].sort((a, b) => {
    if (a.src_id !== b.src_id) return a.src_id < b.src_id ? -1 : 1
    if (a.dst_id !== b.dst_id) return a.dst_id < b.dst_id ? -1 : 1
    if (a.type !== b.type) return a.type < b.type ? -1 : 1
    return 0
  })
}

function sortCommunities(communities: Community[]): Community[] {
  return [...communities].sort((a, b) => a.id - b.id)
}

/**
 * Dump the store contents to three JSONL files in `outDir`. Sorted writes;
 * trailing newline only when the file is non-empty.
 */
export async function dumpJsonl(store: SymbolStore, outDir: string): Promise<void> {
  await mkdir(outDir, { recursive: true })

  const symbols = sortSymbols(store.allSymbols())
  const edges = sortEdges(store.allEdges())
  const communities = sortCommunities(store.allCommunities())

  await writeFile(join(outDir, SYMBOLS_FILE), joinLines(symbols.map(symbolToLine)), 'utf8')
  await writeFile(join(outDir, EDGES_FILE), joinLines(edges.map(edgeToLine)), 'utf8')
  await writeFile(
    join(outDir, COMMUNITIES_FILE),
    joinLines(communities.map(communityToLine)),
    'utf8',
  )
}

interface JsonlSymbol {
  id?: unknown
  name?: unknown
  kind?: unknown
  file?: unknown
  start_line?: unknown
  end_line?: unknown
  exported?: unknown
  parent_id?: unknown
  signature?: unknown
  community_id?: unknown
  created_at?: unknown
}

interface JsonlEdge {
  src_id?: unknown
  dst_id?: unknown
  type?: unknown
  weight?: unknown
  resolved?: unknown
}

interface JsonlCommunity {
  id?: unknown
  size?: unknown
  label?: unknown
  modularity?: unknown
  parent_id?: unknown
}

function asString(v: unknown): string {
  if (typeof v !== 'string') throw new Error(`expected string, got ${typeof v}`)
  return v
}

function asNumber(v: unknown): number {
  if (typeof v !== 'number') throw new Error(`expected number, got ${typeof v}`)
  return v
}

function asBoolean(v: unknown): boolean {
  if (typeof v !== 'boolean') throw new Error(`expected boolean, got ${typeof v}`)
  return v
}

function asNullableString(v: unknown): string | null {
  if (v === null) return null
  if (typeof v === 'string') return v
  throw new Error(`expected string|null, got ${typeof v}`)
}

function asNullableNumber(v: unknown): number | null {
  if (v === null) return null
  if (typeof v === 'number') return v
  throw new Error(`expected number|null, got ${typeof v}`)
}

function parseSymbolLine(line: string): Symbol {
  const raw = JSON.parse(line) as JsonlSymbol
  return {
    id: asString(raw.id),
    name: asString(raw.name),
    kind: asString(raw.kind) as Symbol['kind'],
    file: asString(raw.file),
    start_line: asNumber(raw.start_line),
    end_line: asNumber(raw.end_line),
    exported: asBoolean(raw.exported),
    parent_id: asNullableString(raw.parent_id),
    signature: asNullableString(raw.signature),
    community_id: asNullableNumber(raw.community_id),
    created_at: asString(raw.created_at),
  }
}

function parseEdgeLine(line: string): Edge {
  const raw = JSON.parse(line) as JsonlEdge
  return {
    src_id: asString(raw.src_id),
    dst_id: asString(raw.dst_id),
    type: asString(raw.type) as Edge['type'],
    weight: asNumber(raw.weight),
    resolved: asBoolean(raw.resolved),
  }
}

function parseCommunityLine(line: string): Community {
  const raw = JSON.parse(line) as JsonlCommunity
  return {
    id: asNumber(raw.id),
    size: asNumber(raw.size),
    label: asNullableString(raw.label),
    modularity: asNullableNumber(raw.modularity),
    parent_id: asNullableNumber(raw.parent_id),
  }
}

async function readJsonlFile<T>(path: string, parser: (line: string) => T): Promise<T[]> {
  let text: string
  try {
    text = await readFile(path, 'utf8')
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return []
    throw err
  }
  if (text.length === 0) return []
  const lines = text.split('\n').filter((l) => l.length > 0)
  return lines.map(parser)
}

/**
 * Load the three JSONL files from `inDir` into a freshly-opened, empty store.
 * Caller is responsible for `store.open()` + `store.clearAll()` first.
 */
export async function loadJsonl(inDir: string, store: SymbolStore): Promise<void> {
  const symbols = await readJsonlFile(join(inDir, SYMBOLS_FILE), parseSymbolLine)
  const edges = await readJsonlFile(join(inDir, EDGES_FILE), parseEdgeLine)
  const communities = await readJsonlFile(join(inDir, COMMUNITIES_FILE), parseCommunityLine)

  for (const c of communities) store.upsertCommunity(c)
  for (const s of symbols) store.upsertSymbol(s)
  for (const e of edges) store.upsertEdge(e)
}

export const JSONL_FILES = {
  SYMBOLS_FILE,
  EDGES_FILE,
  COMMUNITIES_FILE,
} as const
