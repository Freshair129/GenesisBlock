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
  }
}
