#!/usr/bin/env node
/**
 * msp-graph — Symbol Graph CLI (PR-4 of 6).
 *
 * Six subcommands per FEAT--MSP-GRAPH-CLI:
 *
 *   build         scan files, parse, persist, run Leiden, dump JSONL
 *   query <name>  exact/prefix lookup
 *   community     list members of a community (by id or symbol)
 *   impact <id>   reverse closure on calls + references
 *   stats         summary from meta.json
 *   dump-jsonl    re-emit JSONL from existing SQLite
 *
 * See:
 *   - FEAT--MSP-GRAPH-CLI    — exact subcommand contracts + exit codes
 *   - ADR--SYMBOL-GRAPH-PERSISTENCE — SQLite + JSONL hybrid layout
 *   - BLUEPRINT--SYMBOL-GRAPH-CORE  — schema + parser interfaces
 */

import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, relative as relativePath, resolve, sep } from 'node:path'
import { parseArgs } from 'node:util'

import { detectCommunities } from './communities/leiden.js'
import { parseFile } from './parser/typescript.js'
import { dumpJsonl } from './store/jsonl.js'
import { SymbolStore } from './store/sqlite.js'
import type { Community, Edge, EdgeType, Symbol, SymbolGraphMeta, SymbolKind } from './types.js'
import { DB_FILE, DEFAULT_NAMESPACE, META_FILE, symbolsDir } from './util.js'

const HELP = `msp-graph — Symbol Graph CLI

Usage:
  msp-graph build [--root=<dir>] [--out=<dir>] [--resolution=<n>] [--seed=<n>] [--include="<glob>,<glob>"]
  msp-graph query <name> [--kind=<k>] [--json] [--root=<dir>]
  msp-graph community [--id=<n> | --symbol=<id>] [--visualize=mermaid|cytoscape-json|stdout] [--resolution=<n>] [--root=<dir>]
  msp-graph impact <id> [--depth=<n>] [--json] [--root=<dir>]
  msp-graph stats [--json] [--root=<dir>]
  msp-graph dump-jsonl [--out=<dir>] [--root=<dir>]
  msp-graph --help

Flags:
  --root=<dir>          project root (default: cwd)
  --out=<dir>           output directory (default: .brain/msp/projects/<ns>/symbols)
  --resolution=<n>      Leiden resolution (default 1.0)
  --seed=<n>            Leiden RNG seed (default 42)
  --include=<patterns>  comma-separated globs (default: src/**/*.ts,web/src/**/*.tsx)
  --namespace=<ns>      project namespace (default 'evaAI')
  --json                emit machine-readable JSON
  --kind=<k>            symbol kind filter
  --depth=<n>           BFS depth (default 5 for impact)
  --id=<n>              community id
  --symbol=<id>         resolve community via this symbol id
  --visualize=<mode>    mermaid | cytoscape-json | stdout (default stdout)

Exit codes:
  build:        0 ok / 1 parse-fail / 2 persist-fail
  query:        0 found / 1 not-found
  community:    0 ok / 2 stale-graph
  impact:       0 ok
  stats:        0 ok / 2 not-built
  dump-jsonl:   0 ok
`

const DEFAULT_INCLUDE = 'src/**/*.ts,web/src/**/*.tsx'
const DEFAULT_RESOLUTION = 1.0
const DEFAULT_SEED = 42
const DEFAULT_IMPACT_DEPTH = 5

const SKIP_DIRS = new Set(['node_modules', '.brain', 'dist', '.git', 'build', 'coverage'])

interface CommonOpts {
  root: string
  namespace: string
}

function toPosix(p: string): string {
  return p.split(sep).join('/')
}

// ─────────────────────────────────────────────────────────────────────────────
// Glob → file walking. Tiny implementation just enough for our patterns
// ("src/**/*.ts", "web/src/**/*.tsx"). No external dep.
// ─────────────────────────────────────────────────────────────────────────────

interface GlobPattern {
  // Anchored prefix segments (before any double-star). e.g. ['src'] for 'src/<star><star>/*.ts'.
  prefix: string[]
  // Extension match (e.g. '.ts', '.tsx'); empty = match all.
  ext: string
}

function parseGlob(pat: string): GlobPattern {
  // Normalize and split.
  const norm = pat.trim().split(/[\\/]+/).filter((s) => s.length > 0)
  const prefix: string[] = []
  let ext = ''
  for (const seg of norm) {
    if (seg === '**') break
    if (seg.includes('*')) {
      // e.g. '*.ts' — last segment with glob; extract extension.
      const dot = seg.indexOf('.')
      if (dot !== -1) ext = seg.slice(dot)
      break
    }
    prefix.push(seg)
  }
  // Also handle non-** patterns where only the file segment has a star.
  if (ext === '') {
    const last = norm[norm.length - 1] ?? ''
    if (last.includes('*')) {
      const dot = last.indexOf('.')
      if (dot !== -1) ext = last.slice(dot)
    }
  }
  return { prefix, ext }
}

function* walkDir(absDir: string): Generator<string> {
  let entries
  try {
    entries = readdirSync(absDir, { withFileTypes: true })
  } catch {
    return
  }
  // Sort for determinism.
  entries.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  for (const ent of entries) {
    if (ent.name.startsWith('.') && ent.name !== '.') {
      // Skip dotfiles/dotdirs by default; keep .brain etc. excluded explicitly.
    }
    const full = join(absDir, ent.name)
    if (ent.isDirectory()) {
      if (SKIP_DIRS.has(ent.name)) continue
      yield* walkDir(full)
    } else if (ent.isFile()) {
      yield full
    }
  }
}

function discoverFiles(root: string, includePatterns: string[]): string[] {
  const matched = new Set<string>()
  for (const raw of includePatterns) {
    const pat = parseGlob(raw)
    const startDir = pat.prefix.length > 0 ? resolve(root, ...pat.prefix) : root
    if (!existsSync(startDir)) continue
    let st
    try {
      st = statSync(startDir)
    } catch {
      continue
    }
    if (!st.isDirectory()) {
      // Single file shortcut.
      if (pat.ext === '' || startDir.endsWith(pat.ext)) matched.add(startDir)
      continue
    }
    for (const f of walkDir(startDir)) {
      if (pat.ext !== '' && !f.endsWith(pat.ext)) continue
      matched.add(f)
    }
  }
  return [...matched].sort()
}

// ─────────────────────────────────────────────────────────────────────────────
// build
// ─────────────────────────────────────────────────────────────────────────────

interface BuildOpts extends CommonOpts {
  out: string
  resolution: number
  seed: number
  include: string[]
}

async function runBuild(opts: BuildOpts): Promise<number> {
  const start = Date.now()
  process.stderr.write(`[graph] discovering files in ${opts.root}…\n`)

  const files = discoverFiles(opts.root, opts.include)
  if (files.length === 0) {
    process.stderr.write(
      `[graph] no files matched include=${opts.include.join(',')} under ${opts.root}\n`,
    )
  }

  // Phase 1: parse files; collect symbols + edges and parse errors.
  const allSymbols: Symbol[] = []
  const allEdges: Edge[] = []
  const parseErrors: { file: string; message: string }[] = []
  for (const abs of files) {
    try {
      const result = parseFile(abs, opts.root)
      if (result.symbols.length === 0 && result.edges.length === 0) {
        // Treat fully-empty parse as a soft error iff the file is non-empty.
        try {
          const text = readFileSync(abs, 'utf8')
          if (text.trim().length > 0) {
            parseErrors.push({
              file: toPosix(relativePath(opts.root, abs)),
              message: 'parser produced no symbols or edges',
            })
          }
        } catch {
          // ignore
        }
      }
      allSymbols.push(...result.symbols)
      allEdges.push(...result.edges)
    } catch (err) {
      parseErrors.push({
        file: toPosix(relativePath(opts.root, abs)),
        message: (err as Error).message,
      })
    }
  }

  // Phase 2: cross-file edge resolution. Mark edges resolved=true when we find
  // a real symbol with the matching id; otherwise leave as-is (parser already
  // emits resolved=false for unresolved cases).
  const symbolIds = new Set(allSymbols.map((s) => s.id))
  for (const edge of allEdges) {
    if (!edge.resolved && symbolIds.has(edge.dst_id)) {
      edge.resolved = true
    }
  }

  process.stderr.write(
    `[graph] parsed ${files.length} files → ${allSymbols.length} symbols, ${allEdges.length} edges${parseErrors.length > 0 ? `, ${parseErrors.length} parse errors` : ''}\n`,
  )

  // Phase 3: open SQLite, clear, bulk insert.
  const outDir = opts.out
  try {
    mkdirSync(outDir, { recursive: true })
  } catch (err) {
    process.stderr.write(`[graph] failed to mkdir ${outDir}: ${(err as Error).message}\n`)
    return 2
  }
  const dbPath = join(outDir, DB_FILE)
  const store = new SymbolStore()
  try {
    store.open(dbPath)
    store.migrate()
    store.clearAll()
    for (const s of allSymbols) store.upsertSymbol(s)
    for (const e of allEdges) store.upsertEdge(e)
  } catch (err) {
    process.stderr.write(`[graph] persist failed: ${(err as Error).message}\n`)
    try {
      store.close()
    } catch {
      // ignore
    }
    return 2
  }

  // Phase 4: Leiden community detection.
  let communityCount = 0
  let modularity = 0
  let algorithm: 'leiden' | 'louvain' = 'leiden'
  try {
    const det = await detectCommunities(allSymbols, allEdges, {
      resolution: opts.resolution,
      seed: opts.seed,
    })
    algorithm = det.algorithm
    modularity = det.modularity
    // Group by community id, build size table, then upsert.
    const communities = new Map<number, Symbol[]>()
    for (const sym of allSymbols) {
      const cid = det.partition.get(sym.id)
      if (cid === undefined) continue
      store.setSymbolCommunity(sym.id, cid)
      const arr = communities.get(cid) ?? []
      arr.push(sym)
      communities.set(cid, arr)
    }
    // Compute labels via the leiden module's helper.
    const { deriveLabel } = await import('./communities/leiden.js')
    const sortedIds = [...communities.keys()].sort((a, b) => a - b)
    for (const cid of sortedIds) {
      const members = communities.get(cid) ?? []
      const community: Community = {
        id: cid,
        size: members.length,
        label: deriveLabel(members, allEdges),
        modularity,
        parent_id: null,
      }
      store.upsertCommunity(community)
    }
    communityCount = sortedIds.length
  } catch (err) {
    process.stderr.write(`[graph] community detection failed: ${(err as Error).message}\n`)
    // Don't fail the build over Leiden errors; just record zero communities.
  }

  // Phase 5: write meta.
  const nowIso = new Date().toISOString()
  const meta: SymbolGraphMeta = {
    schema_version: 1,
    last_built_at: nowIso,
    parser: 'typescript',
    algorithm,
    leiden_resolution: opts.resolution,
    leiden_seed: opts.seed,
    symbol_count: allSymbols.length,
    edge_count: allEdges.length,
    community_count: communityCount,
    parse_errors: parseErrors,
  }
  store.setMeta('schema_version', '1')
  store.setMeta('last_built_at', nowIso)
  store.setMeta('parser', 'typescript')
  store.setMeta('algorithm', algorithm)
  store.setMeta('leiden_resolution', String(opts.resolution))
  store.setMeta('leiden_seed', String(opts.seed))
  store.setMeta('symbol_count', String(allSymbols.length))
  store.setMeta('edge_count', String(allEdges.length))
  store.setMeta('community_count', String(communityCount))
  store.setMeta('parse_errors', JSON.stringify(parseErrors))

  // Phase 6: dump JSONL alongside SQLite.
  try {
    await dumpJsonl(store, outDir)
  } catch (err) {
    process.stderr.write(`[graph] JSONL dump failed: ${(err as Error).message}\n`)
    store.close()
    return 2
  }

  try {
    writeFileSync(join(outDir, META_FILE), JSON.stringify(meta, null, 2) + '\n', 'utf8')
  } catch (err) {
    process.stderr.write(`[graph] meta.json write failed: ${(err as Error).message}\n`)
    store.close()
    return 2
  }
  store.close()

  const elapsed = ((Date.now() - start) / 1000).toFixed(1)
  process.stderr.write(
    `[graph] built ${allSymbols.length} symbols / ${allEdges.length} edges / ${communityCount} communities, modularity=${modularity.toFixed(3)}, took ${elapsed}s\n`,
  )

  return parseErrors.length > 0 && allSymbols.length === 0 ? 1 : 0
}

// ─────────────────────────────────────────────────────────────────────────────
// query
// ─────────────────────────────────────────────────────────────────────────────

interface QueryOpts extends CommonOpts {
  name: string
  kind: string | null
  json: boolean
}

function fileDepth(file: string): number {
  return file.split('/').length
}

function rankSymbols(symbols: Symbol[]): Symbol[] {
  return [...symbols].sort((a, b) => {
    if (a.exported !== b.exported) return a.exported ? -1 : 1
    const da = fileDepth(a.file)
    const db = fileDepth(b.file)
    if (da !== db) return da - db
    if (a.file !== b.file) return a.file < b.file ? -1 : 1
    return a.id < b.id ? -1 : a.id > b.id ? 1 : 0
  })
}

async function runQuery(opts: QueryOpts): Promise<number> {
  const dir = symbolsDir(opts.root, opts.namespace)
  const dbPath = join(dir, DB_FILE)
  if (!existsSync(dbPath)) {
    process.stderr.write(
      `[graph] graph not built — run 'npm run msp:graph build' first\n`,
    )
    return 1
  }
  const store = new SymbolStore()
  store.open(dbPath)
  try {
    const all = store.allSymbols()
    let matches = all.filter((s) => s.name === opts.name)
    if (matches.length === 0) {
      matches = all.filter((s) => s.name.startsWith(opts.name))
    }
    if (opts.kind) {
      matches = matches.filter((s) => s.kind === opts.kind)
    }
    const ranked = rankSymbols(matches)
    if (ranked.length === 0) {
      if (opts.json) {
        process.stdout.write(JSON.stringify({ ok: false, hits: [] }) + '\n')
      } else {
        process.stderr.write(`no symbol named "${opts.name}"\n`)
      }
      return 1
    }
    if (opts.json) {
      process.stdout.write(JSON.stringify({ ok: true, hits: ranked }, null, 2) + '\n')
    } else {
      for (const s of ranked) {
        process.stdout.write(
          `${s.id}\n  kind=${s.kind} exported=${s.exported} ${s.file}:${s.start_line}\n`,
        )
        if (s.signature) process.stdout.write(`  ${s.signature}\n`)
      }
    }
    return 0
  } finally {
    store.close()
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// community
// ─────────────────────────────────────────────────────────────────────────────

interface CommunityOpts extends CommonOpts {
  id: number | null
  symbol: string | null
  visualize: 'mermaid' | 'cytoscape-json' | 'stdout'
  resolution: number | null
}

async function runCommunity(opts: CommunityOpts): Promise<number> {
  const dir = symbolsDir(opts.root, opts.namespace)
  const dbPath = join(dir, DB_FILE)
  if (!existsSync(dbPath)) {
    process.stderr.write(`[graph] graph not built\n`)
    return 2
  }
  const store = new SymbolStore()
  store.open(dbPath)
  try {
    let cid: number | null = opts.id
    if (cid === null && opts.symbol) {
      const sym = store.getSymbol(opts.symbol)
      if (!sym) {
        process.stderr.write(`[graph] no symbol with id ${opts.symbol}\n`)
        return 2
      }
      cid = sym.community_id
    }
    if (cid === null) {
      process.stderr.write(`[graph] --id=<n> or --symbol=<id> required\n`)
      return 2
    }
    const members = store.getCommunityMembers(cid)
    if (members.length === 0) {
      process.stderr.write(`[graph] community ${cid} empty or unknown\n`)
      return 2
    }
    const allCommunities = store.allCommunities()
    const community = allCommunities.find((c) => c.id === cid)
    if (opts.visualize === 'mermaid') {
      process.stdout.write(`graph TD\n`)
      for (const m of members) {
        const safe = m.id.replace(/[^a-zA-Z0-9]/g, '_')
        process.stdout.write(`  ${safe}["${m.name} (${m.kind})"]\n`)
      }
    } else if (opts.visualize === 'cytoscape-json') {
      const elements = members.map((m) => ({
        data: { id: m.id, label: m.name, kind: m.kind, file: m.file },
      }))
      process.stdout.write(JSON.stringify({ elements }, null, 2) + '\n')
    } else {
      process.stdout.write(
        `community ${cid}: size=${community?.size ?? members.length} label=${community?.label ?? ''} modularity=${community?.modularity ?? 'n/a'}\n`,
      )
      for (const m of members) {
        process.stdout.write(`  ${m.id}\n`)
      }
    }
    return 0
  } finally {
    store.close()
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// impact (reverse BFS over calls + references)
// ─────────────────────────────────────────────────────────────────────────────

interface ImpactOpts extends CommonOpts {
  id: string
  depth: number
  json: boolean
}

interface ImpactHit {
  symbol_id: string
  distance: number
}

async function runImpact(opts: ImpactOpts): Promise<number> {
  const dir = symbolsDir(opts.root, opts.namespace)
  const dbPath = join(dir, DB_FILE)
  if (!existsSync(dbPath)) {
    process.stderr.write(`[graph] graph not built\n`)
    return 2
  }
  const store = new SymbolStore()
  store.open(dbPath)
  try {
    const reverseTypes: EdgeType[] = ['calls', 'references']
    const visited = new Map<string, number>()
    let frontier = [opts.id]
    visited.set(opts.id, 0)
    for (let hop = 1; hop <= opts.depth && frontier.length > 0; hop++) {
      const allEdges = store.allEdges()
      const next: string[] = []
      const frontierSet = new Set(frontier)
      for (const e of allEdges) {
        if (!reverseTypes.includes(e.type)) continue
        if (!frontierSet.has(e.dst_id)) continue
        if (visited.has(e.src_id)) continue
        visited.set(e.src_id, hop)
        next.push(e.src_id)
      }
      frontier = next
    }
    visited.delete(opts.id)
    const hits: ImpactHit[] = [...visited.entries()]
      .map(([symbol_id, distance]) => ({ symbol_id, distance }))
      .sort((a, b) => {
        if (a.distance !== b.distance) return a.distance - b.distance
        return a.symbol_id < b.symbol_id ? -1 : 1
      })
    if (opts.json) {
      process.stdout.write(JSON.stringify({ ok: true, callers: hits, count: hits.length }, null, 2) + '\n')
    } else {
      process.stdout.write(`${hits.length} callers of ${opts.id}\n`)
      for (const h of hits) {
        process.stdout.write(`  ${h.distance} ${h.symbol_id}\n`)
      }
    }
    return 0
  } finally {
    store.close()
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// stats
// ─────────────────────────────────────────────────────────────────────────────

interface StatsOpts extends CommonOpts {
  json: boolean
}

async function runStats(opts: StatsOpts): Promise<number> {
  const dir = symbolsDir(opts.root, opts.namespace)
  const metaPath = join(dir, META_FILE)
  if (!existsSync(metaPath)) {
    process.stderr.write(
      `[graph] graph not built — run 'npm run msp:graph build' first\n`,
    )
    return 2
  }
  let meta: SymbolGraphMeta
  try {
    meta = JSON.parse(readFileSync(metaPath, 'utf8')) as SymbolGraphMeta
  } catch (err) {
    process.stderr.write(`[graph] meta.json unreadable: ${(err as Error).message}\n`)
    return 2
  }
  if (opts.json) {
    process.stdout.write(JSON.stringify(meta, null, 2) + '\n')
  } else {
    process.stdout.write(
      `built_at=${meta.last_built_at}\n` +
        `parser=${meta.parser} algorithm=${meta.algorithm}\n` +
        `symbol_count=${meta.symbol_count} edge_count=${meta.edge_count} community_count=${meta.community_count}\n` +
        `parse_errors=${meta.parse_errors.length}\n`,
    )
  }
  return 0
}

// ─────────────────────────────────────────────────────────────────────────────
// dump-jsonl
// ─────────────────────────────────────────────────────────────────────────────

interface DumpOpts extends CommonOpts {
  out: string
}

async function runDump(opts: DumpOpts): Promise<number> {
  const dir = symbolsDir(opts.root, opts.namespace)
  const dbPath = join(dir, DB_FILE)
  if (!existsSync(dbPath)) {
    process.stderr.write(`[graph] graph not built\n`)
    return 0
  }
  const store = new SymbolStore()
  store.open(dbPath)
  try {
    mkdirSync(opts.out, { recursive: true })
    await dumpJsonl(store, opts.out)
    process.stderr.write(`[graph] dumped JSONL to ${opts.out}\n`)
    return 0
  } finally {
    store.close()
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// arg parsing helpers
// ─────────────────────────────────────────────────────────────────────────────

function asNumber(v: string | undefined, fallback: number): number {
  if (v === undefined) return fallback
  const n = Number(v)
  if (Number.isNaN(n)) throw new Error(`expected numeric value, got "${v}"`)
  return n
}

function isVisualizeMode(v: string): v is 'mermaid' | 'cytoscape-json' | 'stdout' {
  return v === 'mermaid' || v === 'cytoscape-json' || v === 'stdout'
}

function isSymbolKind(v: string): v is SymbolKind {
  return (
    v === 'function' ||
    v === 'method' ||
    v === 'class' ||
    v === 'interface' ||
    v === 'type' ||
    v === 'enum' ||
    v === 'const' ||
    v === 'module'
  )
}

// ─────────────────────────────────────────────────────────────────────────────
// main
// ─────────────────────────────────────────────────────────────────────────────

async function main(): Promise<number> {
  // Avoid an unused-import lint hit; `dirname` reserved for future bin usage.
  void dirname

  let parsed
  try {
    parsed = parseArgs({
      args: process.argv.slice(2),
      options: {
        root: { type: 'string' },
        out: { type: 'string' },
        resolution: { type: 'string' },
        seed: { type: 'string' },
        include: { type: 'string' },
        namespace: { type: 'string' },
        json: { type: 'boolean' },
        kind: { type: 'string' },
        depth: { type: 'string' },
        id: { type: 'string' },
        symbol: { type: 'string' },
        visualize: { type: 'string' },
        help: { type: 'boolean' },
      },
      allowPositionals: true,
    })
  } catch (err) {
    process.stderr.write(`error: ${(err as Error).message}\n${HELP}`)
    return 2
  }

  const { values, positionals } = parsed
  if (values.help) {
    process.stdout.write(HELP)
    return 0
  }
  if (positionals.length === 0) {
    process.stderr.write(`error: no subcommand\n${HELP}`)
    return 2
  }

  const [subcommand, ...rest] = positionals
  const root = resolve(values.root ?? process.cwd())
  const namespace = values.namespace ?? DEFAULT_NAMESPACE
  const common: CommonOpts = { root, namespace }

  switch (subcommand) {
    case 'build': {
      const out = values.out ? resolve(values.out) : symbolsDir(root, namespace)
      const include = (values.include ?? DEFAULT_INCLUDE)
        .split(',')
        .map((p) => p.trim())
        .filter((p) => p.length > 0)
      try {
        return await runBuild({
          ...common,
          out,
          resolution: asNumber(values.resolution, DEFAULT_RESOLUTION),
          seed: asNumber(values.seed, DEFAULT_SEED),
          include,
        })
      } catch (err) {
        process.stderr.write(`[graph] build error: ${(err as Error).message}\n`)
        return 1
      }
    }
    case 'query': {
      const name = rest[0]
      if (!name) {
        process.stderr.write(`error: query requires a <name> argument\n`)
        return 1
      }
      const kind = values.kind ?? null
      if (kind !== null && !isSymbolKind(kind)) {
        process.stderr.write(`error: unknown kind "${kind}"\n`)
        return 1
      }
      return runQuery({
        ...common,
        name,
        kind,
        json: values.json === true,
      })
    }
    case 'community': {
      const visMode = values.visualize ?? 'stdout'
      if (!isVisualizeMode(visMode)) {
        process.stderr.write(`error: unknown --visualize mode "${visMode}"\n`)
        return 2
      }
      const id = values.id !== undefined ? asNumber(values.id, NaN) : NaN
      return runCommunity({
        ...common,
        id: Number.isFinite(id) ? id : null,
        symbol: values.symbol ?? null,
        visualize: visMode,
        resolution: values.resolution !== undefined ? asNumber(values.resolution, NaN) : null,
      })
    }
    case 'impact': {
      const id = rest[0]
      if (!id) {
        process.stderr.write(`error: impact requires a <symbol-id> argument\n`)
        return 0
      }
      return runImpact({
        ...common,
        id,
        depth: Math.max(1, Math.floor(asNumber(values.depth, DEFAULT_IMPACT_DEPTH))),
        json: values.json === true,
      })
    }
    case 'stats': {
      return runStats({ ...common, json: values.json === true })
    }
    case 'dump-jsonl': {
      const out = values.out ? resolve(values.out) : symbolsDir(root, namespace)
      return runDump({ ...common, out })
    }
    default:
      process.stderr.write(`error: unknown subcommand "${subcommand}"\n${HELP}`)
      return 2
  }
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(`✗ msp-graph: ${(err as Error).message}\n`)
    process.exit(2)
  })
