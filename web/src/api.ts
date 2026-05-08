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

export const api = {
  getAtoms: async (type?: string, status?: string): Promise<Atom[]> => {
    const params = new URLSearchParams()
    if (type) params.append('type', type)
    if (status) params.append('status', status)
    const res = await fetch(`/api/atoms?${params.toString()}`)
    return res.json()
  },
  
  getAtom: async (id: string): Promise<any> => {
    const res = await fetch(`/api/atoms/${id}`)
    if (!res.ok) throw new Error('Atom not found')
    return res.json()
  },
  
  getGraph: async (): Promise<GraphData> => {
    const res = await fetch('/api/graph')
    return res.json()
  },
  
  getInbound: async (): Promise<any[]> => {
    const res = await fetch('/api/inbound')
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
  
  getBrains: async (): Promise<{ brains: any[]; activeBrainIndex: number }> => {
    const res = await fetch('/api/brains')
    return res.json()
  },
  
  addBrain: async (name: string, path: string): Promise<any> => {
    const res = await fetch('/api/brains', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, path })
    })
    return res.json()
  },
  
  switchBrain: async (index: number): Promise<any> => {
    const res = await fetch('/api/brains/switch', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ index })
    })
    return res.json()
  }
}
