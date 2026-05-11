import { useEffect, useMemo, useState } from 'react'
import {
  api,
  type SymbolEdge,
  type SymbolEdgeType,
  type SymbolGraphStats,
  type SymbolNode,
} from '../api'

interface Props {
  symbolId: string | null
  stats: SymbolGraphStats | null
  onSelect: (id: string) => void
}

interface DetailState {
  symbol: SymbolNode
  parent: SymbolNode | null
  groupedNeighbors: Map<SymbolEdgeType | 'called-by', { edge: SymbolEdge; node: SymbolNode }[]>
}

const EDGE_GROUP_LABELS: { key: SymbolEdgeType | 'called-by'; label: string }[] = [
  { key: 'calls', label: 'Calls' },
  { key: 'called-by', label: 'Called by' },
  { key: 'imports', label: 'Imports' },
  { key: 'extends', label: 'Extends' },
  { key: 'implements', label: 'Implements' },
  { key: 'references', label: 'References' },
  { key: 'defines', label: 'Defines' },
]

export default function SymbolDetail({ symbolId, stats, onSelect }: Props) {
  const [detail, setDetail] = useState<DetailState | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set())

  useEffect(() => {
    if (!symbolId) {
      setDetail(null)
      setError(null)
      return
    }
    let cancelled = false
    setLoading(true)
    setError(null)

    Promise.all([api.getSymbol(symbolId), api.getSymbolNeighbors(symbolId, 1)])
      .then(async ([info, neighbors]) => {
        if (cancelled) return
        let parent: SymbolNode | null = null
        if (info.symbol.parent_id) {
          try {
            const parentInfo = await api.getSymbol(info.symbol.parent_id)
            if (!cancelled) parent = parentInfo.symbol
          } catch {
            // parent might be missing; ignore.
          }
        }
        if (cancelled) return
        const grouped = new Map<
          SymbolEdgeType | 'called-by',
          { edge: SymbolEdge; node: SymbolNode }[]
        >()
        const nodesById = new Map<string, SymbolNode>()
        for (const n of neighbors.nodes) nodesById.set(n.id, n)
        // Outgoing edges: src is the seed
        for (const e of neighbors.edges) {
          let key: SymbolEdgeType | 'called-by'
          let nodeId: string
          if (e.src_id === info.symbol.id) {
            key = e.type
            nodeId = e.dst_id
          } else if (e.dst_id === info.symbol.id && e.type === 'calls') {
            key = 'called-by'
            nodeId = e.src_id
          } else {
            continue
          }
          const node = nodesById.get(nodeId)
          if (!node) continue
          if (!grouped.has(key)) grouped.set(key, [])
          grouped.get(key)!.push({ edge: e, node })
        }
        setDetail({ symbol: info.symbol, parent, groupedNeighbors: grouped })
      })
      .catch((err: Error) => {
        if (cancelled) return
        setError(err.message)
        setDetail(null)
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [symbolId])

  const vscodeLink = useMemo(() => {
    if (!detail) return null
    // vscode://file/<abs>:<line> — `file` is repo-relative; the user's editor will
    // resolve relative to the workspace if the absolute path can't be inferred.
    return `vscode://file/${detail.symbol.file}:${detail.symbol.start_line}`
  }, [detail])

  if (!symbolId) {
    return (
      <div className="detail-body" style={{ padding: 12, opacity: 0.7 }}>
        Select a symbol on the left or in the graph.
        {stats && (
          <div style={{ marginTop: 16, fontSize: '0.85em' }}>
            <div>Built: {stats.last_built_at}</div>
            <div>
              {stats.symbol_count} symbols · {stats.edge_count} edges ·{' '}
              {stats.community_count} communities
            </div>
            <div style={{ opacity: 0.6 }}>
              parser={stats.parser} algorithm={stats.algorithm}
            </div>
          </div>
        )}
      </div>
    )
  }

  if (loading) return <div className="detail-body" style={{ padding: 12 }}>Loading…</div>
  if (error) return <div className="detail-body" style={{ padding: 12, color: '#b00020' }}>Error: {error}</div>
  if (!detail) return null

  const toggle = (key: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }

  return (
    <>
      <div className="detail-header" style={{ padding: 12, borderBottom: '1px solid #ddd' }}>
        <h3 style={{ margin: 0, fontSize: '1.05em' }} aria-label="symbol-detail-name">
          {detail.symbol.name}
          <span style={{ marginLeft: 6, opacity: 0.6, fontSize: '0.85em' }}>
            {detail.symbol.kind}
          </span>
          {detail.symbol.exported && (
            <span
              style={{
                marginLeft: 8,
                fontSize: '0.7em',
                padding: '2px 6px',
                background: '#2ecc71',
                color: '#fff',
                borderRadius: 3,
              }}
            >
              exported
            </span>
          )}
        </h3>
        <div style={{ marginTop: 4, fontSize: '0.85em', fontFamily: 'monospace' }}>
          {detail.symbol.file}:{detail.symbol.start_line}
          {vscodeLink && (
            <a
              href={vscodeLink}
              style={{
                marginLeft: 8,
                fontSize: '0.85em',
                padding: '1px 6px',
                border: '1px solid #888',
                borderRadius: 3,
                textDecoration: 'none',
              }}
              title="Open in VS Code"
            >
              Open
            </a>
          )}
        </div>
        {detail.parent && (
          <div style={{ marginTop: 4, fontSize: '0.85em' }}>
            Parent:{' '}
            <a
              href="#"
              onClick={(e) => {
                e.preventDefault()
                if (detail.parent) onSelect(detail.parent.id)
              }}
            >
              {detail.parent.name} ({detail.parent.kind})
            </a>
          </div>
        )}
      </div>
      <div className="detail-body" style={{ padding: 12 }}>
        {detail.symbol.signature && (
          <pre
            style={{
              background: '#f7f7f7',
              padding: 8,
              borderRadius: 4,
              overflow: 'auto',
              fontSize: '0.85em',
            }}
          >
            {detail.symbol.signature}
          </pre>
        )}
        <div style={{ marginTop: 8, fontSize: '0.85em' }}>
          <strong>id:</strong>{' '}
          <code style={{ fontSize: '0.85em' }}>{detail.symbol.id}</code>
        </div>
        {detail.symbol.community_id !== null && (
          <div style={{ marginTop: 4, fontSize: '0.85em' }}>
            <strong>community:</strong> {detail.symbol.community_id}
          </div>
        )}
        <hr style={{ margin: '12px 0', border: 'none', borderTop: '1px solid #ddd' }} />
        <div aria-label="symbol-neighbor-groups">
          {EDGE_GROUP_LABELS.map(({ key, label }) => {
            const list = detail.groupedNeighbors.get(key) ?? []
            if (list.length === 0) return null
            const cKey = `g-${key}`
            const cCollapsed = collapsed.has(cKey)
            return (
              <div key={cKey} style={{ marginBottom: 6 }}>
                <div
                  onClick={() => toggle(cKey)}
                  style={{
                    cursor: 'pointer',
                    fontWeight: 'bold',
                    fontSize: '0.85em',
                    padding: '2px 0',
                  }}
                >
                  {cCollapsed ? '▶' : '▼'} {label} ({list.length})
                </div>
                {!cCollapsed &&
                  list.map(({ node }) => (
                    <div
                      key={`${cKey}-${node.id}`}
                      onClick={() => onSelect(node.id)}
                      style={{
                        cursor: 'pointer',
                        padding: '2px 8px 2px 16px',
                        fontSize: '0.85em',
                      }}
                    >
                      <span style={{ opacity: 0.6, marginRight: 4 }}>{node.kind}</span>
                      <span>{node.name}</span>
                      <span style={{ opacity: 0.5, marginLeft: 4, fontSize: '0.85em' }}>
                        {' '}
                        — {node.file}:{node.start_line}
                      </span>
                    </div>
                  ))}
              </div>
            )
          })}
        </div>
      </div>
    </>
  )
}
