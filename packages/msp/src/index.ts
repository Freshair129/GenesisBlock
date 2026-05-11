import 'dotenv/config'
import express from 'express'
import { retain, recall } from '@freshair129/gks/memory'
import { getStore } from './memory.js'
import { CandidateWriter } from './memory/candidates/writer.js'
import { CandidateNotFoundError } from './memory/candidates/types.js'
import { registerSymbolApi } from './symbols/api.js'

const app = express()
app.use(express.json())

app.use((req, res, next) => {
  res.setHeader(
    'Content-Security-Policy',
    "default-src 'self'; script-src 'self' 'unsafe-eval' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'"
  )
  next()
})

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

// Brain Management
const CONFIG_FILE = path.join(process.cwd(), 'brains-config.json')
let brains: { name: string; path: string }[] = [
  { name: 'Default', path: process.env.GKS_BROWSE_ROOT || process.env.GKS_ROOT || './data' }
]
let activeBrainIndex = 0

async function loadBrains() {
  try {
    const data = await fs.readFile(CONFIG_FILE, 'utf-8')
    const config = JSON.parse(data)
    brains = config.brains || brains
    activeBrainIndex = config.activeBrainIndex ?? 0
  } catch (e) {}
}
loadBrains()

async function saveBrains() {
  await fs.writeFile(CONFIG_FILE, JSON.stringify({ brains, activeBrainIndex }, null, 2))
}

function getActiveRoot() {
  return brains[activeBrainIndex]?.path || './data'
}

// Serve static frontend
app.use(express.static(path.join(process.cwd(), 'web', 'dist')))

// ---------------------------------------------------------
// KNOWLEDGE BROWSER API
// ---------------------------------------------------------

async function scanAtomsFallback(gksRoot: string) {
  const atoms: any[] = []
  const gksDir = path.join(gksRoot, 'gks')
  let types: string[] = []
  try {
    types = await fs.readdir(gksDir)
  } catch (e) {
    return atoms // No gks dir
  }

  for (const typeFolder of types) {
    if (typeFolder === '00_index' || typeFolder === 'hotfix') continue
    const typePath = path.join(gksDir, typeFolder)
    try {
      const stat = await fs.stat(typePath)
      if (!stat.isDirectory()) continue
      
      const files = await fs.readdir(typePath)
      for (const file of files) {
        if (!file.endsWith('.md')) continue
        try {
          const content = await fs.readFile(path.join(typePath, file), 'utf-8')
          const match = content.match(/^---\n([\s\S]*?)\n---/)
          if (!match) continue
          
          const fmStr = match[1]
          const lines = fmStr.split('\n')
          let entry: any = { type: typeFolder.toUpperCase() }
          let currentArray: string[] | null = null
          
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
                entry[key] = currentArray
              } else {
                entry[key] = val
                currentArray = null
              }
            }
          }
          atoms.push(entry)
        } catch (e) {}
      }
    } catch (e) {}
  }
  return atoms
}

app.get('/api/atoms', async (req, res) => {
  try {
    const typeFilter = req.query.type as string | undefined
    const statusFilter = req.query.status as string | undefined
    
    const indexPath = path.join(getActiveRoot(), 'gks', '00_index', 'atomic_index.jsonl')
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
      // Fallback: scan directories if index not found
      const scanned = await scanAtomsFallback(getActiveRoot())
      for (const entry of scanned) {
        if (typeFilter && entry.type !== typeFilter) continue
        if (statusFilter && entry.status !== statusFilter) continue
        atoms.push(entry)
      }
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
    const filePath = path.join(getActiveRoot(), 'gks', type, `${id}.md`)
    
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
    const indexPath = path.join(getActiveRoot(), 'gks', '00_index', 'atomic_index.jsonl')
    const nodes: any[] = []
    const edges: any[] = []
    let atomEntries: any[] = []
    
    try {
      const data = await fs.readFile(indexPath, 'utf-8')
      const lines = data.split('\n').filter(l => l.trim())
      for (const line of lines) {
        try {
          atomEntries.push(JSON.parse(line))
        } catch (e) {}
      }
    } catch (e) {
      atomEntries = await scanAtomsFallback(getActiveRoot())
    }
    
    for (const entry of atomEntries) {
      nodes.push({ id: entry.id, type: entry.type, title: entry.title })
      if (entry.links && Array.isArray(entry.links)) {
        for (const target of entry.links) {
          edges.push({ source: entry.id, target })
        }
      }
    }
    
    res.json({ nodes, edges })
  } catch (error: any) {
    res.status(500).json({ error: error.message })
  }
})

app.get('/api/candidates', async (_req, res) => {
  try {
    const writer = new CandidateWriter({ root: getActiveRoot() })
    const summaries = await writer.list()
    res.json(summaries)
  } catch (error: any) {
    res.status(500).json({ error: error.message })
  }
})

app.get('/api/candidates/:id', async (req, res) => {
  try {
    const writer = new CandidateWriter({ root: getActiveRoot() })
    const record = await writer.read(req.params.id)
    res.json(record)
  } catch (error: any) {
    if (error instanceof CandidateNotFoundError) {
      res.status(404).json({ error: error.message })
      return
    }
    res.status(400).json({ error: error.message })
  }
})

app.delete('/api/candidates/:id', async (req, res) => {
  try {
    const writer = new CandidateWriter({ root: getActiveRoot() })
    await writer.delete(req.params.id)
    res.json({ ok: true })
  } catch (error: any) {
    if (error instanceof CandidateNotFoundError) {
      res.status(404).json({ error: error.message })
      return
    }
    res.status(400).json({ error: error.message })
  }
})

// Symbol Graph API (PR-5 of 6) — see src/symbols/api.ts.
registerSymbolApi(app, getActiveRoot)

app.get('/api/hotfixes', async (req, res) => {
  try {
    // Read directly from gks/hotfix/*.md as a simple list
    const hotfixDir = path.join(getActiveRoot(), 'gks', 'hotfix')
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
    const sessionDir = path.join(getActiveRoot(), '.brain', 'msp', 'projects', 'evaAI', 'session')
    let files: string[] = []
    try {
      files = await fs.readdir(sessionDir)
    } catch (e) {}
    res.json(files.filter(f => f.endsWith('.jsonl') || f.endsWith('.json')))
  } catch (error: any) {
    res.status(500).json({ error: error.message })
  }
})

// Brain APIs
app.get('/api/brains', (req, res) => {
  res.json({ brains, activeBrainIndex })
})

app.post('/api/brains', async (req, res) => {
  const { name, path: brainPath } = req.body
  if (!name || !brainPath) return res.status(400).json({ error: 'name and path required' })
  brains.push({ name, path: brainPath })
  await saveBrains()
  res.json({ success: true, brains })
})

app.post('/api/brains/switch', async (req, res) => {
  const { index } = req.body
  if (typeof index !== 'number' || index < 0 || index >= brains.length) {
    return res.status(400).json({ error: 'invalid index' })
  }
  activeBrainIndex = index
  await saveBrains()
  res.json({ success: true, activeBrainIndex })
})

const PORT = process.env.PORT ?? 3000
app.listen(PORT, () => {

  console.log(`MSP listening on :${PORT}`)
})
