export interface Atom {
  id: string
  type: string
  title: string
  status: string
  path?: string
  created?: string
  links?: string[]
}

export interface GraphData {
  nodes: { id: string; type: string; title?: string }[]
  edges: { source: string; target: string }[]
}

export interface RecallHit {
  id: string
  source: string
  score: number
  snippet: string
}

export interface RecallResult {
  query: string
  hits: RecallHit[]
  tookMs: number
}

export interface CandidateSummary {
  proposed_id: string
  type: string
  status: 'candidate'
  proposed_at: string
  proposed_by: 'agent' | 'human'
  rationale?: string
  confidence?: number
  title: string
  path: string
}

export interface CandidateRecord extends CandidateSummary {
  body: string
}

// ---- Symbol Graph types (mirrors src/symbols/types.ts SQLite schema) ----

export type SymbolKind =
  | 'function'
  | 'method'
  | 'class'
  | 'interface'
  | 'type'
  | 'enum'
  | 'const'
  | 'module'

export type SymbolEdgeType =
  | 'defines'
  | 'imports'
  | 'calls'
  | 'extends'
  | 'implements'
  | 'references'

export interface SymbolNode {
  id: string
  name: string
  kind: SymbolKind
  file: string
  start_line: number
  end_line: number
  exported: boolean
  parent_id: string | null
  signature: string | null
  community_id: number | null
  created_at: string
}

export interface SymbolEdge {
  src_id: string
  dst_id: string
  type: SymbolEdgeType
  weight: number
  resolved: boolean
}

export interface SymbolCommunity {
  id: number
  size: number
  label: string | null
  modularity: number | null
  parent_id: number | null
}

export interface SymbolGraphStats {
  schema_version: number
  last_built_at: string
  parser: string
  algorithm: string
  leiden_resolution: number
  leiden_seed: number
  symbol_count: number
  edge_count: number
  community_count: number
  parse_errors: { file: string; message: string }[]
}

export interface SymbolDetail {
  symbol: SymbolNode
  neighborPreview: SymbolNode[]
}

export interface SymbolNeighbors {
  nodes: SymbolNode[]
  edges: SymbolEdge[]
}

export interface SymbolCommunityDetail {
  community: SymbolCommunity
  members: SymbolNode[]
  edges: SymbolEdge[]
}

export interface SymbolSearchHit extends SymbolNode {
  score: number
}

interface OkEnvelope {
  ok: true
}

interface ErrEnvelope {
  ok: false
  error: string
}

async function unwrapOrThrow<T extends OkEnvelope>(res: Response): Promise<T> {
  const json = (await res.json()) as T | ErrEnvelope
  if (!res.ok || !('ok' in json) || json.ok !== true) {
    const message =
      typeof json === 'object' && json && 'error' in json
        ? (json as ErrEnvelope).error
        : `request failed (${res.status})`
    throw new Error(message)
  }
  return json
}

export const api = {
  getAtoms: async (type?: string, status?: string): Promise<Atom[]> => {
    const params = new URLSearchParams()
    if (type) params.append('type', type)
    if (status) params.append('status', status)
    const res = await fetch(`/api/atoms?${params.toString()}`)
    return res.json()
  },

  getAtom: async (id: string): Promise<unknown> => {
    const res = await fetch(`/api/atoms/${id}`)
    if (!res.ok) throw new Error('Atom not found')
    return res.json()
  },

  getGraph: async (): Promise<GraphData> => {
    const res = await fetch('/api/graph')
    return res.json()
  },

  listCandidates: async (): Promise<CandidateSummary[]> => {
    const res = await fetch('/api/candidates')
    return res.json()
  },

  readCandidate: async (id: string): Promise<CandidateRecord> => {
    const res = await fetch(`/api/candidates/${encodeURIComponent(id)}`)
    if (!res.ok) throw new Error(`Candidate not found: ${id}`)
    return res.json()
  },

  deleteCandidate: async (id: string): Promise<void> => {
    const res = await fetch(`/api/candidates/${encodeURIComponent(id)}`, { method: 'DELETE' })
    if (!res.ok) throw new Error(`Failed to delete: ${id}`)
  },

  getHotfixes: async (): Promise<string[]> => {
    const res = await fetch('/api/hotfixes')
    return res.json()
  },

  getSessions: async (): Promise<string[]> => {
    const res = await fetch('/api/sessions')
    return res.json()
  },

  recall: async (query: string, topK = 5): Promise<RecallResult> => {
    const res = await fetch('/api/recall', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query, topK })
    })
    return res.json()
  },

  getBrains: async (): Promise<{ brains: { name: string; path: string }[]; activeBrainIndex: number }> => {
    const res = await fetch('/api/brains')
    return res.json()
  },

  addBrain: async (name: string, path: string): Promise<unknown> => {
    const res = await fetch('/api/brains', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, path })
    })
    return res.json()
  },

  switchBrain: async (index: number): Promise<unknown> => {
    const res = await fetch('/api/brains/switch', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ index })
    })
    return res.json()
  },

  // ---- Symbol Graph (PR-5 of 6) ----

  getSymbols: async (
    opts: { offset?: number; limit?: number } = {},
  ): Promise<{ symbols: SymbolNode[]; communities: SymbolCommunity[] }> => {
    const params = new URLSearchParams()
    if (opts.offset !== undefined) params.set('offset', String(opts.offset))
    if (opts.limit !== undefined) params.set('limit', String(opts.limit))
    const qs = params.toString()
    const res = await fetch(`/api/symbols${qs ? `?${qs}` : ''}`)
    const json = await unwrapOrThrow<
      { ok: true; symbols: SymbolNode[]; communities: SymbolCommunity[] }
    >(res)
    return { symbols: json.symbols, communities: json.communities }
  },

  getSymbol: async (id: string): Promise<SymbolDetail> => {
    const res = await fetch(`/api/symbols/${encodeURIComponent(id)}`)
    const json = await unwrapOrThrow<
      { ok: true; symbol: SymbolNode; neighborPreview: SymbolNode[] }
    >(res)
    return { symbol: json.symbol, neighborPreview: json.neighborPreview }
  },

  getSymbolNeighbors: async (id: string, depth = 1): Promise<SymbolNeighbors> => {
    const res = await fetch(
      `/api/symbols/${encodeURIComponent(id)}/neighbors?depth=${depth}`,
    )
    const json = await unwrapOrThrow<
      { ok: true; nodes: SymbolNode[]; edges: SymbolEdge[] }
    >(res)
    return { nodes: json.nodes, edges: json.edges }
  },

  getSymbolCommunity: async (id: number): Promise<SymbolCommunityDetail> => {
    const res = await fetch(`/api/symbols/community/${id}`)
    const json = await unwrapOrThrow<
      { ok: true; community: SymbolCommunity; members: SymbolNode[]; edges: SymbolEdge[] }
    >(res)
    return { community: json.community, members: json.members, edges: json.edges }
  },

  searchSymbols: async (q: string, limit = 20): Promise<SymbolSearchHit[]> => {
    const params = new URLSearchParams({ q, limit: String(limit) })
    const res = await fetch(`/api/symbols/search?${params.toString()}`)
    const json = await unwrapOrThrow<{ ok: true; hits: SymbolSearchHit[] }>(res)
    return json.hits
  },

  /**
   * Returns null when the graph has not been built yet (404 from the server).
   * Throws on other errors.
   */
  getSymbolStats: async (): Promise<SymbolGraphStats | null> => {
    const res = await fetch('/api/symbols/stats')
    if (res.status === 404) return null
    const json = await unwrapOrThrow<{ ok: true } & SymbolGraphStats>(res)
    // Strip the envelope marker for downstream typing.
    const { ok: _ok, ...rest } = json
    return rest as SymbolGraphStats
  },
}
