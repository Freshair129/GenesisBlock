/**
 * SQLite-backed Symbol Graph store.
 *
 * Schema mirrors ADR--SYMBOL-GRAPH-PERSISTENCE §"Schema sketch" verbatim
 * (see also BLUEPRINT--SYMBOL-GRAPH-CORE for migration policy).
 *
 * Uses `better-sqlite3`'s synchronous API on the hot path (it's fast and
 * synchronous by design) but exposes async wrappers for caller convenience
 * so we can swap to an async driver later without API churn.
 */

import Database from 'better-sqlite3'

import type { Community, Edge, EdgeType, Symbol, SymbolKind } from '../types.js'

const SCHEMA_VERSION = 1

const SCHEMA_SQL = `
CREATE TABLE IF NOT EXISTS symbols (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  file TEXT NOT NULL,
  start_line INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  exported INTEGER NOT NULL,
  parent_id TEXT,
  signature TEXT,
  community_id INTEGER,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file);
CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind);
CREATE INDEX IF NOT EXISTS idx_symbols_community ON symbols(community_id);

CREATE TABLE IF NOT EXISTS edges (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  src_id TEXT NOT NULL,
  dst_id TEXT NOT NULL,
  type TEXT NOT NULL,
  weight REAL NOT NULL DEFAULT 1.0,
  resolved INTEGER NOT NULL,
  UNIQUE(src_id, dst_id, type)
);

CREATE INDEX IF NOT EXISTS idx_edges_src_type ON edges(src_id, type);
CREATE INDEX IF NOT EXISTS idx_edges_dst_type ON edges(dst_id, type);

CREATE TABLE IF NOT EXISTS communities (
  id INTEGER PRIMARY KEY,
  size INTEGER NOT NULL,
  label TEXT,
  modularity REAL,
  parent_id INTEGER
);

CREATE TABLE IF NOT EXISTS meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
`

interface SymbolRow {
  id: string
  name: string
  kind: string
  file: string
  start_line: number
  end_line: number
  exported: number
  parent_id: string | null
  signature: string | null
  community_id: number | null
  created_at: string
}

interface EdgeRow {
  src_id: string
  dst_id: string
  type: string
  weight: number
  resolved: number
}

interface CommunityRow {
  id: number
  size: number
  label: string | null
  modularity: number | null
  parent_id: number | null
}

function rowToSymbol(r: SymbolRow): Symbol {
  return {
    id: r.id,
    name: r.name,
    kind: r.kind as SymbolKind,
    file: r.file,
    start_line: r.start_line,
    end_line: r.end_line,
    exported: r.exported === 1,
    parent_id: r.parent_id,
    signature: r.signature,
    community_id: r.community_id,
    created_at: r.created_at,
  }
}

function rowToEdge(r: EdgeRow): Edge {
  return {
    src_id: r.src_id,
    dst_id: r.dst_id,
    type: r.type as EdgeType,
    weight: r.weight,
    resolved: r.resolved === 1,
  }
}

function rowToCommunity(r: CommunityRow): Community {
  return {
    id: r.id,
    size: r.size,
    label: r.label,
    modularity: r.modularity,
    parent_id: r.parent_id,
  }
}

export class SymbolStore {
  private db: Database.Database | null = null

  /** Open (or create) the database at `dbPath` and run migrations. */
  open(dbPath: string): void {
    this.db = new Database(dbPath)
    this.db.pragma('journal_mode = WAL')
    this.migrate()
  }

  close(): void {
    if (this.db) {
      this.db.close()
      this.db = null
    }
  }

  /** Idempotent — runs `CREATE TABLE IF NOT EXISTS` and stamps schema_version. */
  migrate(): void {
    const db = this.requireDb()
    db.exec(SCHEMA_SQL)
    const stored = this.getMeta('schema_version')
    if (!stored) {
      this.setMeta('schema_version', String(SCHEMA_VERSION))
    }
  }

  /** Reset all tables. Used for full rebuilds. */
  clearAll(): void {
    const db = this.requireDb()
    db.exec(`
      DELETE FROM symbols;
      DELETE FROM edges;
      DELETE FROM communities;
      DELETE FROM meta;
    `)
    // Re-stamp schema_version so getMeta('schema_version') still works.
    this.setMeta('schema_version', String(SCHEMA_VERSION))
  }

  // ---- Symbols ----

  upsertSymbol(s: Symbol): void {
    const db = this.requireDb()
    db.prepare(
      `INSERT INTO symbols (id, name, kind, file, start_line, end_line, exported, parent_id, signature, community_id, created_at)
       VALUES (@id, @name, @kind, @file, @start_line, @end_line, @exported, @parent_id, @signature, @community_id, @created_at)
       ON CONFLICT(id) DO UPDATE SET
         name=excluded.name, kind=excluded.kind, file=excluded.file,
         start_line=excluded.start_line, end_line=excluded.end_line,
         exported=excluded.exported, parent_id=excluded.parent_id,
         signature=excluded.signature, community_id=excluded.community_id,
         created_at=excluded.created_at`,
    ).run({
      id: s.id,
      name: s.name,
      kind: s.kind,
      file: s.file,
      start_line: s.start_line,
      end_line: s.end_line,
      exported: s.exported ? 1 : 0,
      parent_id: s.parent_id,
      signature: s.signature,
      community_id: s.community_id,
      created_at: s.created_at,
    })
  }

  getSymbol(id: string): Symbol | null {
    const db = this.requireDb()
    const row = db.prepare(`SELECT * FROM symbols WHERE id = ?`).get(id) as SymbolRow | undefined
    return row ? rowToSymbol(row) : null
  }

  /** All symbols, sorted by id (used by JSONL exporter for determinism). */
  allSymbols(): Symbol[] {
    const db = this.requireDb()
    const rows = db.prepare(`SELECT * FROM symbols ORDER BY id`).all() as SymbolRow[]
    return rows.map(rowToSymbol)
  }

  setSymbolCommunity(id: string, communityId: number | null): void {
    const db = this.requireDb()
    db.prepare(`UPDATE symbols SET community_id = ? WHERE id = ?`).run(communityId, id)
  }

  // ---- Edges ----

  upsertEdge(e: Edge): void {
    const db = this.requireDb()
    db.prepare(
      `INSERT INTO edges (src_id, dst_id, type, weight, resolved)
       VALUES (?, ?, ?, ?, ?)
       ON CONFLICT(src_id, dst_id, type) DO UPDATE SET
         weight=excluded.weight, resolved=excluded.resolved`,
    ).run(e.src_id, e.dst_id, e.type, e.weight, e.resolved ? 1 : 0)
  }

  /** All edges, sorted by (src_id, dst_id, type) for deterministic export. */
  allEdges(): Edge[] {
    const db = this.requireDb()
    const rows = db
      .prepare(`SELECT src_id, dst_id, type, weight, resolved FROM edges ORDER BY src_id, dst_id, type`)
      .all() as EdgeRow[]
    return rows.map(rowToEdge)
  }

  /**
   * BFS over edges from `id` up to `depth` hops. Returns deduped neighbor list
   * (excluding the seed) plus the edges traversed. Edge filter optional.
   */
  getNeighbors(
    id: string,
    depth: number,
    types?: EdgeType[],
  ): { nodes: Symbol[]; edges: Edge[] } {
    const db = this.requireDb()
    if (depth <= 0) return { nodes: [], edges: [] }

    let edgeQuery = `SELECT src_id, dst_id, type, weight, resolved FROM edges WHERE src_id IN (SELECT value FROM json_each(?))`
    if (types && types.length > 0) {
      edgeQuery += ` AND type IN (${types.map(() => '?').join(',')})`
    }
    edgeQuery += ` ORDER BY src_id, dst_id, type`

    const visitedNodes = new Set<string>([id])
    const collectedEdges: Edge[] = []
    let frontier = [id]

    for (let hop = 0; hop < depth && frontier.length > 0; hop++) {
      const stmt = db.prepare(edgeQuery)
      const params: unknown[] = [JSON.stringify(frontier)]
      if (types) params.push(...types)
      const rows = stmt.all(...params) as EdgeRow[]
      const next: string[] = []
      for (const row of rows) {
        const edge = rowToEdge(row)
        const k = `${edge.src_id} ${edge.dst_id} ${edge.type}`
        if (collectedEdges.some((e) => `${e.src_id} ${e.dst_id} ${e.type}` === k)) continue
        collectedEdges.push(edge)
        if (!visitedNodes.has(edge.dst_id)) {
          visitedNodes.add(edge.dst_id)
          next.push(edge.dst_id)
        }
      }
      frontier = next
    }

    visitedNodes.delete(id)
    const nodes: Symbol[] = []
    for (const nodeId of visitedNodes) {
      const sym = this.getSymbol(nodeId)
      if (sym) nodes.push(sym)
    }
    nodes.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))

    return { nodes, edges: collectedEdges }
  }

  // ---- Communities ----

  upsertCommunity(c: Community): void {
    const db = this.requireDb()
    db.prepare(
      `INSERT INTO communities (id, size, label, modularity, parent_id)
       VALUES (?, ?, ?, ?, ?)
       ON CONFLICT(id) DO UPDATE SET
         size=excluded.size, label=excluded.label,
         modularity=excluded.modularity, parent_id=excluded.parent_id`,
    ).run(c.id, c.size, c.label, c.modularity, c.parent_id)
  }

  allCommunities(): Community[] {
    const db = this.requireDb()
    const rows = db.prepare(`SELECT * FROM communities ORDER BY id`).all() as CommunityRow[]
    return rows.map(rowToCommunity)
  }

  getCommunityMembers(communityId: number): Symbol[] {
    const db = this.requireDb()
    const rows = db
      .prepare(`SELECT * FROM symbols WHERE community_id = ? ORDER BY id`)
      .all(communityId) as SymbolRow[]
    return rows.map(rowToSymbol)
  }

  // ---- Meta ----

  getMeta(key: string): string | null {
    const db = this.requireDb()
    const row = db.prepare(`SELECT value FROM meta WHERE key = ?`).get(key) as
      | { value: string }
      | undefined
    return row ? row.value : null
  }

  setMeta(key: string, value: string): void {
    const db = this.requireDb()
    db.prepare(
      `INSERT INTO meta (key, value) VALUES (?, ?)
       ON CONFLICT(key) DO UPDATE SET value=excluded.value`,
    ).run(key, value)
  }

  // ---- Internals ----

  private requireDb(): Database.Database {
    if (!this.db) {
      throw new Error('SymbolStore: open(dbPath) must be called before use')
    }
    return this.db
  }
}
