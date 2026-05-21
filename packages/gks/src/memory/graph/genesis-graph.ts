/**
 * Genesis Block backend — Phase 0 (TypeScript-only).
 *
 * Implements GraphBackend with an event-sourced JSONL store under a single
 * directory, plus an opt-in Cypher v0 surface (`.cypher(query)`).
 *
 * Layout (per `ADR--GENESIS-BLOCK-AS-GKS-BACKEND` §"Reconciling with ADR-001"):
 *   <path>/
 *     manifest.json        — { schema_version: '1.0.0' }
 *     genesis-graph.jsonl  — append-only event log
 *
 * Eventual upgrade path (`BLUEPRINT--GENESIS-BLOCK-INTEGRATION` P3.1-P3.6):
 *   the Rust crate at `packages/gks/native/genesis-graph/` takes over the
 *   same directory. The on-disk format is forward-compatible because the
 *   Rust binary will accept either a `.db` binary file or the existing
 *   `.jsonl` event log; the TS adapter shape (`createGenesisGraphBackend`
 *   returning a `GraphBackend`) stays unchanged.
 *
 * Active-engine framing (`CONCEPT--GENESIS-BLOCK-ENGINE` §1B):
 *   Parser / Syncer / Analytic / Interface live above this backend — see
 *   `packages/msp/src/cognitive/index.ts` for the wiring.
 */

import { mkdir, rename } from 'node:fs/promises'
import { createHash, randomUUID } from 'node:crypto'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import { appendJsonl, forEachJsonl, readJsonSafe, writeJson, writeJsonl } from '../../lib/jsonl.js'
import { createLogger } from '../../lib/logger.js'
import {
  CURRENT_SCHEMA_VERSION,
  enforceSchemaCompatibility,
} from '../../lib/schema-version.js'

import type {
  AddEdgeArgs,
  AddNodeArgs,
  GraphBackend,
  GraphEdge,
  GraphNode,
  GraphQuery,
  NeighborQuery,
  NeighborResult,
} from '../graph.js'

import { parseCypherV0, type CypherV0Plan, type ReturnItem } from './cypher-v0.js'
import { GenesisGraphUnsupportedCypher } from './genesis-graph-errors.js'

const log = createLogger('graph:genesis-graph')

export interface GenesisGraphBackendOptions {
  /**
   * Directory that owns the genesis-graph store. `manifest.json` +
   * `genesis-graph.jsonl` are created here. If the directory does not
   * exist, it is created on first write.
   */
  path: string
}

interface Event {
  kind: 'node' | 'edge' | 'edge_retract'
  payload: unknown
}

interface Manifest {
  schema_version: string
}

/**
 * Factory: returns the native Rust-backed graph if its prebuilt `.node`
 * binary is available for the host triple, otherwise the pure-TS fallback.
 *
 * The native module is resolved at call-time via `require()` and any failure
 * — missing optional dep, missing platform binary, throwing constructor — is
 * caught and logged once at info level. Callers always receive a working
 * `GraphBackend`.
 */
/**
 * Public surface both backends satisfy. Extends `GraphBackend` with the
 * opt-in Cypher v0 method — needed so callers of the factory can use
 * `.cypher(...)` against the returned object regardless of whether the
 * native or pure-TS path is in play.
 */
export interface GenesisGraphBackendApi extends GraphBackend {
  cypher(query: string): Promise<Array<Record<string, unknown>>>
  compact(): Promise<void>
}

export function createGenesisGraphBackend(
  opts: GenesisGraphBackendOptions,
): GenesisGraphBackendApi {
  const native = tryLoadNativeBackend(opts)
  if (native) return native
  return new GenesisGraphBackend(opts)
}

function tryLoadNativeBackend(
  opts: GenesisGraphBackendOptions,
): NativeGenesisGraphBackend | null {
  try {
    // Lazy CJS require — keeps the optional dep truly optional. Use
    // `createRequire` because this file is compiled to ESM and `require`
    // is not a top-level global there.
    const requireFn = createRequire(import.meta.url)
    const mod = requireFn('@freshair129/gks-genesis-block-native') as {
      GenesisDatabase?: GenesisDatabaseCtor
    }
    if (!mod || typeof mod.GenesisDatabase?.open !== 'function') {
      log.info('falling back to pure-TS backend', {
        reason: 'native module loaded but GenesisDatabase.open is missing',
      })
      return null
    }
    const db = mod.GenesisDatabase.open({ path: opts.path })
    return new NativeGenesisGraphBackend(db)
  } catch (err) {
    const reason = err instanceof Error ? err.message : String(err)
    log.info('falling back to pure-TS backend', { reason })
    return null
  }
}

// ─── native adapter ─────────────────────────────────────────────────────────

// Structural type sketch of the native surface we depend on. The real types
// live in `packages/gks/native/genesis-block/index.d.ts` but we redeclare a
// minimal mirror here so we don't import-link to an optional dep.

interface NodeOutputLike {
  id: string
  labels: string[]
  props: unknown
}

interface EdgeOutputLike {
  id: string
  from: string
  to: string
  rel: string
  props: unknown
  validFrom: string
  validTo?: string | null
  recordedAt: string
  supersededBy?: string | null
}

interface NeighborOutputLike {
  node: NodeOutputLike
  path: EdgeOutputLike[]
  depth: number
}

interface NodeInputLike {
  id?: string
  labels: string[]
  props?: unknown
}

interface EdgeInputLike {
  id?: string
  from: string
  to: string
  rel: string
  props?: unknown
  validFrom?: string
  supersede?: boolean
}

interface QueryInputLike {
  from?: string
  to?: string
  rel?: string
  asOf?: string
  includeInvalid?: boolean
  limit?: number
}

interface NeighborInputLike {
  depth?: number
  rel?: string
  direction?: string
  asOf?: string
  includeInvalid?: boolean
  limit?: number
}

interface GenesisDatabaseLike {
  addNode(args: NodeInputLike): Promise<NodeOutputLike>
  addEdge(args: EdgeInputLike): Promise<EdgeOutputLike>
  retractEdge(id: string, at?: string | null): Promise<EdgeOutputLike | null>
  query(args: QueryInputLike): Promise<EdgeOutputLike[]>
  neighbors(seed: string, args: NeighborInputLike): Promise<NeighborOutputLike[]>
  cypher(query: string): Promise<unknown>
  compact(): Promise<void>
}

interface GenesisDatabaseCtor {
  open(opts: { path: string }): GenesisDatabaseLike
}

class NativeGenesisGraphBackend implements GenesisGraphBackendApi {
  private readonly nodeCount = { value: 0 }
  private readonly edgeCount = { value: 0 }

  constructor(private readonly db: GenesisDatabaseLike) {}

  // Native `open` already loaded the JSONL log — nothing to do here.
  async load(): Promise<void> {
    return
  }

  // The native crate does not expose a counter; we report best-effort counts
  // accumulated by this adapter. This is good enough for parity with the
  // pure-TS backend's `size()` callers (currently informational only).
  size(): { nodes: number; edges: number } {
    return { nodes: this.nodeCount.value, edges: this.edgeCount.value }
  }

  async addNode(args: AddNodeArgs): Promise<GraphNode> {
    const out = await this.db.addNode({
      id: args.id,
      labels: args.labels,
      props: args.props ?? {},
    })
    this.nodeCount.value++
    return toGraphNode(out)
  }

  async addEdge(args: AddEdgeArgs): Promise<GraphEdge> {
    const out = await this.db.addEdge({
      id: args.id,
      from: args.from,
      to: args.to,
      rel: args.rel,
      props: args.props ?? {},
      validFrom: args.valid_from,
      supersede: args.supersede,
    })
    this.edgeCount.value++
    return toGraphEdge(out)
  }

  async retractEdge(id: string, at?: string): Promise<GraphEdge | null> {
    const out = await this.db.retractEdge(id, at ?? null)
    return out ? toGraphEdge(out) : null
  }

  async query(q: GraphQuery = {}): Promise<GraphEdge[]> {
    const out = await this.db.query({
      from: q.from,
      to: q.to,
      rel: q.rel,
      asOf: q.asOf,
      includeInvalid: q.includeInvalid,
      limit: q.limit,
    })
    return out.map(toGraphEdge)
  }

  async neighbors(seed: string, q: NeighborQuery = {}): Promise<NeighborResult[]> {
    const out = await this.db.neighbors(seed, {
      depth: q.depth,
      rel: q.rel,
      direction: q.direction,
      asOf: q.asOf,
      includeInvalid: q.includeInvalid,
      limit: q.limit,
    })
    return out.map(toNeighborResult)
  }

  async cypher(query: string): Promise<Array<Record<string, unknown>>> {
    const out = await this.db.cypher(query)
    return out as Array<Record<string, unknown>>
  }

  async compact(): Promise<void> {
    await this.db.compact()
  }
}

// ─── boundary converters (camelCase ↔ snake_case) ───────────────────────────

function toGraphNode(no: NodeOutputLike): GraphNode {
  return {
    id: no.id,
    labels: no.labels,
    props: (no.props ?? {}) as Record<string, unknown>,
  }
}

function toGraphEdge(eo: EdgeOutputLike): GraphEdge {
  const edge: GraphEdge = {
    id: eo.id,
    from: eo.from,
    to: eo.to,
    rel: eo.rel,
    props: (eo.props ?? {}) as Record<string, unknown>,
    valid_from: eo.validFrom,
    valid_to: eo.validTo ?? null,
    recorded_at: eo.recordedAt,
  }
  if (eo.supersededBy != null) edge.superseded_by = eo.supersededBy
  return edge
}

function toNeighborResult(no: NeighborOutputLike): NeighborResult {
  return {
    node: toGraphNode(no.node),
    path: no.path.map(toGraphEdge),
    depth: no.depth,
  }
}

export class GenesisGraphBackend implements GenesisGraphBackendApi {
  private readonly dir: string
  private readonly jsonlPath: string
  private readonly manifestPath: string
  private readonly nodes = new Map<string, GraphNode>()
  private readonly edges = new Map<string, GraphEdge>()
  private readonly outIdx = new Map<string, Set<string>>()
  private readonly inIdx = new Map<string, Set<string>>()
  private loaded = false

  constructor(opts: GenesisGraphBackendOptions) {
    this.dir = opts.path
    this.jsonlPath = join(this.dir, 'genesis-graph.jsonl')
    this.manifestPath = join(this.dir, 'manifest.json')
  }

  async load(): Promise<void> {
    if (this.loaded) return
    await mkdir(this.dir, { recursive: true })

    const manifest = await readJsonSafe<Manifest>(this.manifestPath)
    if (manifest) {
      enforceSchemaCompatibility(manifest.schema_version)
    } else {
      await writeJson(this.manifestPath, { schema_version: CURRENT_SCHEMA_VERSION })
    }

    try {
      await forEachJsonl<Event>(this.jsonlPath, (evt) => this.apply(evt))
    } catch (err) {
      const e = err as NodeJS.ErrnoException
      if (e.code !== 'ENOENT') throw err
    }

    this.loaded = true
    log.info('genesis-graph loaded', {
      dir: this.dir,
      nodes: this.nodes.size,
      edges: this.edges.size,
    })
  }

  size(): { nodes: number; edges: number } {
    return { nodes: this.nodes.size, edges: this.edges.size }
  }

  async addNode(args: AddNodeArgs): Promise<GraphNode> {
    await this.ensureLoaded()
    const id = args.id ?? stableId('N', args.labels.join(':'), JSON.stringify(args.props ?? {}))
    const existing = this.nodes.get(id)
    if (existing) {
      const merged: GraphNode = {
        id,
        labels: [...new Set([...existing.labels, ...args.labels])],
        props: { ...existing.props, ...(args.props ?? {}) },
      }
      this.nodes.set(id, merged)
      await this.persist({ kind: 'node', payload: merged })
      return merged
    }
    const node: GraphNode = { id, labels: args.labels, props: args.props ?? {} }
    this.nodes.set(id, node)
    await this.persist({ kind: 'node', payload: node })
    return node
  }

  async addEdge(args: AddEdgeArgs): Promise<GraphEdge> {
    await this.ensureLoaded()
    if (!this.nodes.has(args.from)) {
      throw new Error(`addEdge: unknown from-node ${args.from}`)
    }
    if (!this.nodes.has(args.to)) {
      throw new Error(`addEdge: unknown to-node ${args.to}`)
    }

    const now = new Date().toISOString()
    const id = args.id ?? randomUUID()
    const edge: GraphEdge = {
      id,
      from: args.from,
      to: args.to,
      rel: args.rel,
      props: args.props ?? {},
      valid_from: args.valid_from ?? now,
      valid_to: null,
      recorded_at: now,
    }

    if (args.supersede) {
      const victims: GraphEdge[] = []
      for (const eid of this.outIdx.get(args.from) ?? []) {
        const e = this.edges.get(eid)!
        if (e.rel === args.rel && e.valid_to == null && e.id !== id) {
          victims.push(e)
        }
      }
      for (const v of victims) {
        const retired: GraphEdge = { ...v, valid_to: edge.valid_from, superseded_by: id }
        this.edges.set(v.id, retired)
        await this.persist({ kind: 'edge', payload: retired })
      }
    }

    this.edges.set(edge.id, edge)
    this.indexEdge(edge)
    await this.persist({ kind: 'edge', payload: edge })
    return edge
  }

  async retractEdge(id: string, at: string = new Date().toISOString()): Promise<GraphEdge | null> {
    await this.ensureLoaded()
    const e = this.edges.get(id)
    if (!e) return null
    if (e.valid_to != null) return e
    const retired: GraphEdge = { ...e, valid_to: at }
    this.edges.set(id, retired)
    await this.persist({ kind: 'edge_retract', payload: retired })
    return retired
  }

  query(q: GraphQuery = {}): GraphEdge[] {
    const asOfMs = q.asOf ? Date.parse(q.asOf) : undefined
    const out: GraphEdge[] = []

    const iter = q.from
      ? iterateSet(this.outIdx.get(q.from), this.edges)
      : q.to
        ? iterateSet(this.inIdx.get(q.to), this.edges)
        : this.edges.values()

    for (const e of iter) {
      if (q.from && e.from !== q.from) continue
      if (q.to && e.to !== q.to) continue
      if (q.rel && e.rel !== q.rel) continue
      if (!q.includeInvalid && !isEdgeValidAt(e, asOfMs)) continue
      out.push(e)
      if (q.limit && out.length >= q.limit) break
    }
    return out
  }

  neighbors(seed: string, q: NeighborQuery = {}): NeighborResult[] {
    const depth = Math.max(1, q.depth ?? 1)
    const direction = q.direction ?? 'out'
    const asOfMs = q.asOf ? Date.parse(q.asOf) : undefined
    const limit = q.limit ?? Infinity

    if (!this.nodes.has(seed)) return []

    const results: NeighborResult[] = []
    const visited = new Set<string>([seed])
    type Frame = { id: string; path: GraphEdge[]; hops: number }
    const queue: Frame[] = [{ id: seed, path: [], hops: 0 }]
    let head = 0

    while (head < queue.length && results.length < limit) {
      const frame = queue[head++]!
      if (frame.hops >= depth) continue

      const edgeIds =
        direction === 'out'
          ? this.outIdx.get(frame.id) ?? new Set<string>()
          : direction === 'in'
            ? this.inIdx.get(frame.id) ?? new Set<string>()
            : new Set<string>([
                ...(this.outIdx.get(frame.id) ?? []),
                ...(this.inIdx.get(frame.id) ?? []),
              ])

      for (const eid of edgeIds) {
        const edge = this.edges.get(eid)!
        if (q.rel && edge.rel !== q.rel) continue
        if (!q.includeInvalid && !isEdgeValidAt(edge, asOfMs)) continue
        const next = edge.from === frame.id ? edge.to : edge.from
        if (visited.has(next)) continue
        visited.add(next)
        const node = this.nodes.get(next)
        if (!node) continue
        const path = [...frame.path, edge]
        const hops = frame.hops + 1
        results.push({ node, path, depth: hops })
        if (results.length >= limit) break
        queue.push({ id: next, path, hops })
      }
    }
    return results
  }

  /**
   * Opt-in Cypher v0 surface. Parses the query, runs it through neighbors(),
   * and projects RETURN items. Throws `GenesisGraphUnsupportedCypher` for
   * anything outside the BLUEPRINT v0 subset.
   *
   * Supported shape:
   *   MATCH (a:Label {id: 'literal'})-[r:rel1|rel2*1..6]->(b:Label)
   *   [WHERE b.prop = 'literal']
   *   RETURN b.id [, length(r) AS hops]
   */
  async cypher(query: string): Promise<Array<Record<string, unknown>>> {
    await this.ensureLoaded()
    const plan = parseCypherV0(query)
    return this.executePlan(plan)
  }

  async compact(): Promise<void> {
    await this.ensureLoaded()
    const tmpPath = `${this.jsonlPath}.tmp`

    const events: Event[] = []
    for (const node of this.nodes.values()) {
      events.push({ kind: 'node', payload: node })
    }
    for (const edge of this.edges.values()) {
      if (edge.valid_to == null) {
        events.push({ kind: 'edge', payload: edge })
      }
    }

    await writeJsonl(tmpPath, events)
    await rename(tmpPath, this.jsonlPath)
    log.info('genesis-graph compacted', { path: this.jsonlPath })
  }

  private executePlan(plan: CypherV0Plan): Array<Record<string, unknown>> {
    const seed = this.nodes.get(plan.seedId)
    if (!seed) return []
    if (!seed.labels.includes(plan.seedLabel)) return []

    // Apply 'a.*' predicates against the seed up front — they're constant.
    for (const p of plan.predicates) {
      if (p.alias !== 'a') continue
      if (String(seed.props[p.prop] ?? '') !== p.equals) return []
    }

    // Walk out to maxHops; we filter by minHops + label + predicates after.
    const opts: NeighborQuery = { depth: plan.maxHops, direction: 'out' }
    // Single-rel optimisation: GraphBackend.neighbors only accepts one rel
    // filter; for unions we filter in TS.
    if (plan.rels.length === 1) opts.rel = plan.rels[0]
    const reached = this.neighbors(plan.seedId, opts)

    const rows: Array<Record<string, unknown>> = []
    for (const hit of reached) {
      if (hit.depth < plan.minHops) continue
      if (!hit.node.labels.includes(plan.targetLabel)) continue
      if (plan.rels.length > 1) {
        // Path must contain only rels from the union.
        if (!hit.path.every((e) => plan.rels.includes(e.rel))) continue
      }
      let ok = true
      for (const p of plan.predicates) {
        if (p.alias !== 'b') continue
        if (String(hit.node.props[p.prop] ?? '') !== p.equals) {
          ok = false
          break
        }
      }
      if (!ok) continue
      rows.push(this.project(plan.returns, seed, hit))
    }
    return rows
  }

  private project(
    items: ReturnItem[],
    seed: GraphNode,
    hit: NeighborResult,
  ): Record<string, unknown> {
    const row: Record<string, unknown> = {}
    for (const item of items) {
      if (item.kind === 'length') {
        row[item.as] = hit.depth
        continue
      }
      if (item.kind === 'property') {
        const src = item.source!
        if (src === 'a.id') row[item.as] = seed.id
        else if (src === 'b.id') row[item.as] = hit.node.id
        else if (src.startsWith('a.')) row[item.as] = seed.props[src.slice(2)] ?? null
        else if (src.startsWith('b.')) row[item.as] = hit.node.props[src.slice(2)] ?? null
        else throw new GenesisGraphUnsupportedCypher(src, 'RETURN source unrecognised')
        continue
      }
    }
    return row
  }

  // ─── internal ──────────────────────────────────────────────────────────

  private async ensureLoaded(): Promise<void> {
    if (!this.loaded) await this.load()
  }

  private apply(evt: Event): void {
    if (evt.kind === 'node') {
      const n = evt.payload as GraphNode
      this.nodes.set(n.id, n)
    } else if (evt.kind === 'edge' || evt.kind === 'edge_retract') {
      const e = evt.payload as GraphEdge
      this.edges.set(e.id, e)
      this.indexEdge(e)
    }
  }

  private indexEdge(e: GraphEdge): void {
    const out = this.outIdx.get(e.from) ?? new Set<string>()
    out.add(e.id)
    this.outIdx.set(e.from, out)
    const inn = this.inIdx.get(e.to) ?? new Set<string>()
    inn.add(e.id)
    this.inIdx.set(e.to, inn)
  }

  private async persist(evt: Event): Promise<void> {
    await appendJsonl(this.jsonlPath, evt)
  }
}

// ─── helpers ─────────────────────────────────────────────────────────────

function stableId(prefix: string, ...parts: string[]): string {
  const h = createHash('sha256').update(parts.join('::')).digest('hex').slice(0, 16)
  return `${prefix}-${h}`
}

function* iterateSet<T>(
  ids: Set<string> | undefined,
  lookup: Map<string, T>,
): Generator<T> {
  if (!ids) return
  for (const id of ids) {
    const v = lookup.get(id)
    if (v !== undefined) yield v
  }
}

function isEdgeValidAt(edge: GraphEdge, asOfMs?: number): boolean {
  if (asOfMs === undefined) return edge.valid_to == null
  const fromMs = Date.parse(edge.valid_from)
  if (asOfMs < fromMs) return false
  if (edge.valid_to == null) return true
  return asOfMs < Date.parse(edge.valid_to)
}
