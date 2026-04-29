import 'dotenv/config'
import express from 'express'
import { retain, recall } from '@freshair129/gks/memory'
import { getStore } from './memory.js'

const app = express()
app.use(express.json())

const store = getStore()

app.get('/health', (_req, res) => {
  res.json({ status: 'ok' })
})

// Index new knowledge into GKS
// POST /index { content, tags?, source? }
app.post('/index', async (req, res) => {
  const { content, tags = [], source } = req.body as {
    content: string
    tags?: string[]
    source?: string
  }
  if (!content) {
    res.status(400).json({ error: 'content required' })
    return
  }
  const result = await retain(store, {
    content,
    metadata: { tags, ...(source ? { source } : {}) },
  })
  res.json({ id: result.vectorDocId })
})

// Query — recall relevant context from GKS
// POST /query { query, topK?, strategy? }
app.post('/query', async (req, res) => {
  const { query, topK = 5, strategy } = req.body as {
    query: string
    topK?: number
    strategy?: 'atomic' | 'vector' | 'episodic' | 'multi'
  }
  if (!query) {
    res.status(400).json({ error: 'query required' })
    return
  }
  const result = await recall(store, query, { topK, strategy })
  res.json({
    query: result.query,
    hits: result.hits.map((h) => ({
      id: h.id,
      source: h.source,
      score: h.score,
      snippet: h.snippet,
    })),
    tookMs: result.tookMs,
  })
})

import fs from 'fs/promises'
import path from 'path'

// Helper to read GKS root
const GKS_ROOT = process.env.GKS_BROWSE_ROOT || process.env.GKS_ROOT || './data'

// Serve static frontend
app.use(express.static(path.join(process.cwd(), 'web', 'dist')))

// ---------------------------------------------------------
// KNOWLEDGE BROWSER API
// ---------------------------------------------------------

app.get('/api/atoms', async (req, res) => {
  try {
    const typeFilter = req.query.type as string | undefined
    const statusFilter = req.query.status as string | undefined
    
    const indexPath = path.join(GKS_ROOT, 'gks', '00_index', 'atomic_index.jsonl')
    const atoms: any[] = []
    
    try {
      const data = await fs.readFile(indexPath, 'utf-8')
      const lines = data.split('\n').filter(l => l.trim())
      for (const line of lines) {
        try {
          const entry = JSON.parse(line)
          if (typeFilter && entry.type !== typeFilter) continue
          if (statusFilter && entry.status !== statusFilter) continue
          atoms.push(entry)
        } catch (e) {}
      }
    } catch (e) {
      // Fallback: scan directories if index not found (simplified)
      // This is a minimal fallback, ideally the index exists.
      console.warn('Index not found, fallback scanning not fully implemented', e)
    }
    
    res.json(atoms)
  } catch (error: any) {
    res.status(500).json({ error: error.message })
  }
})

app.get('/api/atoms/:id', async (req, res) => {
  try {
    const id = req.params.id
    const typeMatch = id.match(/^([A-Z]+)--/)
    if (!typeMatch) {
      res.status(400).json({ error: 'Invalid ID format' })
      return
    }
    const type = typeMatch[1].toLowerCase()
    const filePath = path.join(GKS_ROOT, 'gks', type, `${id}.md`)
    
    const content = await fs.readFile(filePath, 'utf-8')
    
    // Parse frontmatter
    let frontmatter: Record<string, any> = {}
    let body = content
    const match = content.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/)
    if (match) {
      body = match[2]
      const fmStr = match[1]
      // Simple parse
      const lines = fmStr.split('\n')
      let currentArray: string[] | null = null
      let currentArrayKey = ''
      
      for (const line of lines) {
        if (line.startsWith('  - ')) {
          if (currentArray) currentArray.push(line.replace('  - ', '').trim())
          continue
        }
        const colonIdx = line.indexOf(':')
        if (colonIdx > -1) {
          const key = line.slice(0, colonIdx).trim()
          const val = line.slice(colonIdx + 1).trim()
          if (!val) {
            currentArray = []
            currentArrayKey = key
            frontmatter[key] = currentArray
          } else {
            frontmatter[key] = val
            currentArray = null
          }
        }
      }
    }
    
    res.json({ id, ...frontmatter, body })
  } catch (error: any) {
    res.status(404).json({ error: 'Atom not found' })
  }
})

app.get('/api/graph', async (req, res) => {
  try {
    const indexPath = path.join(GKS_ROOT, 'gks', '00_index', 'atomic_index.jsonl')
    const nodes: any[] = []
    const edges: any[] = []
    
    const data = await fs.readFile(indexPath, 'utf-8')
    const lines = data.split('\n').filter(l => l.trim())
    
    for (const line of lines) {
      try {
        const entry = JSON.parse(line)
        nodes.push({ id: entry.id, type: entry.type, title: entry.title })
        if (entry.links && Array.isArray(entry.links)) {
          for (const target of entry.links) {
            edges.push({ source: entry.id, target })
          }
        }
      } catch (e) {}
    }
    
    res.json({ nodes, edges })
  } catch (error: any) {
    res.status(500).json({ error: error.message })
  }
})

app.get('/api/inbound', async (req, res) => {
  try {
    const inboundDir = path.join(GKS_ROOT, '.brain', 'msp', 'projects', 'evaAI', 'inbound')
    let files: string[] = []
    try {
      files = await fs.readdir(inboundDir)
    } catch (e) {
      // Directory might not exist
    }
    
    const candidates = []
    for (const f of files) {
      if (f.endsWith('.json')) {
        const data = await fs.readFile(path.join(inboundDir, f), 'utf-8')
        candidates.push(JSON.parse(data))
      }
    }
    res.json(candidates)
  } catch (error: any) {
    res.status(500).json({ error: error.message })
  }
})

app.get('/api/hotfixes', async (req, res) => {
  try {
    // Read directly from gks/hotfix/*.md as a simple list
    const hotfixDir = path.join(GKS_ROOT, 'gks', 'hotfix')
    let files: string[] = []
    try {
      files = await fs.readdir(hotfixDir)
    } catch (e) {
      // Dir might not exist
    }
    
    const hotfixes = files.filter(f => f.endsWith('.md')).map(f => f.replace('.md', ''))
    res.json(hotfixes)
  } catch (error: any) {
    res.status(500).json({ error: error.message })
  }
})

app.get('/api/sessions', async (req, res) => {
  try {
    const sessionDir = path.join(GKS_ROOT, '.brain', 'msp', 'projects', 'evaAI', 'session')
    let files: string[] = []
    try {
      files = await fs.readdir(sessionDir)
    } catch (e) {}
    res.json(files.filter(f => f.endsWith('.jsonl') || f.endsWith('.json')))
  } catch (error: any) {
    res.status(500).json({ error: error.message })
  }
})

// Alias /query to /api/recall
app.post('/api/recall', async (req, res) => {
  // Delegate to existing query handler logic or just redirect internally
  const { query, topK = 5, strategy } = req.body
  if (!query) return res.status(400).json({ error: 'query required' })
  try {
    const result = await recall(store, query, { topK, strategy })
    res.json({
      query: result.query,
      hits: result.hits.map((h: any) => ({
        id: h.id,
        source: h.source,
        score: h.score,
        snippet: h.snippet,
      })),
      tookMs: result.tookMs,
    })
  } catch (e: any) {
    res.status(500).json({ error: e.message })
  }
})

const PORT = process.env.PORT ?? 3000
app.listen(PORT, () => {

  console.log(`MSP listening on :${PORT}`)
})
