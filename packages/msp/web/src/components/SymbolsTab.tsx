import { useEffect, useMemo, useState } from 'react'
import {
  api,
  type SymbolCommunity,
  type SymbolGraphStats,
  type SymbolNode,
} from '../api'
import SymbolList from './SymbolList'
import SymbolGraphView from './SymbolGraphView'
import SymbolDetail from './SymbolDetail'

type LoadState = 'loading' | 'empty' | 'ready' | 'error'

const BUILD_COMMAND = 'npm run msp:graph build'

export default function SymbolsTab() {
  const [loadState, setLoadState] = useState<LoadState>('loading')
  const [stats, setStats] = useState<SymbolGraphStats | null>(null)
  const [symbols, setSymbols] = useState<SymbolNode[]>([])
  const [communities, setCommunities] = useState<SymbolCommunity[]>([])
  const [selectedSymbolId, setSelectedSymbolId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    let cancelled = false
    setLoadState('loading')
    setError(null)
    api
      .getSymbolStats()
      .then(async (stats) => {
        if (cancelled) return
        if (stats === null) {
          setLoadState('empty')
          return
        }
        setStats(stats)
        const { symbols, communities } = await api.getSymbols()
        if (cancelled) return
        setSymbols(symbols)
        setCommunities(communities)
        setLoadState('ready')
      })
      .catch((err: Error) => {
        if (cancelled) return
        setError(err.message)
        setLoadState('error')
      })
    return () => {
      cancelled = true
    }
  }, [])

  const communitiesById = useMemo(() => {
    const m = new Map<number, SymbolCommunity>()
    for (const c of communities) m.set(c.id, c)
    return m
  }, [communities])

  if (loadState === 'loading') {
    return (
      <div style={{ padding: 16 }} aria-label="symbols-loading">
        Loading symbol graph…
      </div>
    )
  }

  if (loadState === 'error') {
    return (
      <div style={{ padding: 16, color: '#b00020' }} aria-label="symbols-error">
        Error loading symbol graph: {error}
      </div>
    )
  }

  if (loadState === 'empty') {
    return (
      <div
        style={{
          padding: 24,
          maxWidth: 560,
          margin: '40px auto',
          border: '1px solid #ddd',
          borderRadius: 8,
          background: '#fafafa',
        }}
        aria-label="symbols-empty-state"
      >
        <h3 style={{ marginTop: 0 }}>No symbol graph yet</h3>
        <p>
          The Symbol Graph hasn't been built for this brain. Run the build command in your repo
          root to populate it:
        </p>
        <pre
          style={{
            background: '#1d1f21',
            color: '#fafafa',
            padding: '10px 14px',
            borderRadius: 4,
            fontFamily: 'monospace',
          }}
        >
          {BUILD_COMMAND}
        </pre>
        <button
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(BUILD_COMMAND)
              setCopied(true)
              setTimeout(() => setCopied(false), 2000)
            } catch {
              // ignore
            }
          }}
        >
          {copied ? 'Copied!' : 'Copy command'}
        </button>
        <p style={{ marginTop: 16, fontSize: '0.9em', opacity: 0.7 }}>
          Once the graph is built, refresh this page.
        </p>
      </div>
    )
  }

  return (
    <div className="symbols-tab" style={{ display: 'flex', height: '100%', width: '100%' }}>
      <div className="panel-left" style={{ flex: '0 0 280px', overflow: 'auto' }}>
        <SymbolList
          symbols={symbols}
          communities={communities}
          selectedId={selectedSymbolId}
          onSelect={setSelectedSymbolId}
        />
      </div>
      <div className="panel-center" style={{ flex: 1, position: 'relative' }}>
        <SymbolGraphView
          symbols={symbols}
          communities={communitiesById}
          selectedId={selectedSymbolId}
          onSelect={setSelectedSymbolId}
        />
      </div>
      <div className="panel-right" style={{ flex: '0 0 320px', overflow: 'auto' }}>
        <SymbolDetail
          symbolId={selectedSymbolId}
          stats={stats}
          onSelect={setSelectedSymbolId}
        />
      </div>
    </div>
  )
}
