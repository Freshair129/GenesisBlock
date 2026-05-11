import { useEffect, useMemo, useState } from 'react'
import { api, type SymbolCommunity, type SymbolKind, type SymbolNode } from '../api'

interface Props {
  symbols: SymbolNode[]
  communities: SymbolCommunity[]
  selectedId: string | null
  onSelect: (id: string) => void
}

const ALL_KINDS: ('all' | SymbolKind)[] = [
  'all',
  'function',
  'method',
  'class',
  'interface',
  'type',
  'enum',
  'const',
  'module',
]

interface CommunityGroup {
  community: SymbolCommunity | null
  files: Map<string, SymbolNode[]>
}

function groupSymbols(
  symbols: SymbolNode[],
  communities: SymbolCommunity[],
): CommunityGroup[] {
  const byId = new Map<number, SymbolCommunity>()
  for (const c of communities) byId.set(c.id, c)
  const groups = new Map<number | 'none', CommunityGroup>()
  for (const sym of symbols) {
    const key: number | 'none' = sym.community_id ?? 'none'
    let g = groups.get(key)
    if (!g) {
      g = {
        community: sym.community_id !== null ? byId.get(sym.community_id) ?? null : null,
        files: new Map(),
      }
      groups.set(key, g)
    }
    let fileGroup = g.files.get(sym.file)
    if (!fileGroup) {
      fileGroup = []
      g.files.set(sym.file, fileGroup)
    }
    fileGroup.push(sym)
  }
  // Stable sort: communities by id (none last), files alpha, symbols by start_line.
  const result: CommunityGroup[] = []
  const ids = [...groups.keys()]
  ids.sort((a, b) => {
    if (a === 'none') return 1
    if (b === 'none') return -1
    return (a as number) - (b as number)
  })
  for (const id of ids) {
    const g = groups.get(id)!
    const files: Map<string, SymbolNode[]> = new Map()
    const fileNames = [...g.files.keys()].sort()
    for (const f of fileNames) {
      const list = g.files.get(f)!
      list.sort((a, b) => a.start_line - b.start_line)
      files.set(f, list)
    }
    result.push({ community: g.community, files })
  }
  return result
}

export default function SymbolList({ symbols, communities, selectedId, onSelect }: Props) {
  const [kindFilter, setKindFilter] = useState<'all' | SymbolKind>('all')
  const [exportedOnly, setExportedOnly] = useState(false)
  const [communityFilter, setCommunityFilter] = useState<number | 'all'>('all')
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<SymbolNode[] | null>(null)
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set())

  // Debounced search
  useEffect(() => {
    const q = searchQuery.trim()
    if (!q) {
      setSearchResults(null)
      return
    }
    const handle = setTimeout(() => {
      api
        .searchSymbols(q, 50)
        .then((hits) => setSearchResults(hits))
        .catch(() => setSearchResults([]))
    }, 200)
    return () => clearTimeout(handle)
  }, [searchQuery])

  const filtered = useMemo(() => {
    const source = searchResults ?? symbols
    return source.filter((s) => {
      if (kindFilter !== 'all' && s.kind !== kindFilter) return false
      if (exportedOnly && !s.exported) return false
      if (communityFilter !== 'all' && s.community_id !== communityFilter) return false
      return true
    })
  }, [symbols, searchResults, kindFilter, exportedOnly, communityFilter])

  const grouped = useMemo(() => groupSymbols(filtered, communities), [filtered, communities])

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
      <div
        className="atom-list-header"
        style={{ display: 'flex', flexDirection: 'column', gap: 6, padding: 8 }}
      >
        <input
          type="search"
          placeholder="Search symbols…"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          aria-label="symbol-search-input"
          style={{ padding: 4 }}
        />
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          <select
            value={kindFilter}
            onChange={(e) => setKindFilter(e.target.value as 'all' | SymbolKind)}
            aria-label="symbol-kind-filter"
          >
            {ALL_KINDS.map((k) => (
              <option key={k} value={k}>
                {k}
              </option>
            ))}
          </select>
          <select
            value={String(communityFilter)}
            onChange={(e) => {
              const v = e.target.value
              setCommunityFilter(v === 'all' ? 'all' : Number(v))
            }}
            aria-label="symbol-community-filter"
          >
            <option value="all">All communities</option>
            {communities.map((c) => (
              <option key={c.id} value={c.id}>
                {c.label ?? `community ${c.id}`} ({c.size})
              </option>
            ))}
          </select>
          <label style={{ fontSize: '0.85em', display: 'flex', alignItems: 'center', gap: 4 }}>
            <input
              type="checkbox"
              checked={exportedOnly}
              onChange={(e) => setExportedOnly(e.target.checked)}
              aria-label="symbol-exported-only"
            />
            exported
          </label>
        </div>
      </div>
      <div className="atom-list" aria-label="symbol-list">
        {grouped.length === 0 && (
          <div style={{ padding: 12, opacity: 0.6 }}>No symbols match.</div>
        )}
        {grouped.map((group) => {
          const cId =
            group.community?.id !== undefined ? String(group.community.id) : 'none'
          const cKey = `c-${cId}`
          const cCollapsed = collapsed.has(cKey)
          const cLabel = group.community
            ? group.community.label ?? `community ${group.community.id}`
            : 'no community'
          return (
            <div key={cKey} style={{ marginBottom: 4 }}>
              <div
                onClick={() => toggle(cKey)}
                style={{
                  cursor: 'pointer',
                  padding: '4px 8px',
                  background: '#eef',
                  fontWeight: 'bold',
                  fontSize: '0.85em',
                }}
              >
                {cCollapsed ? '▶' : '▼'} {cLabel}
              </div>
              {!cCollapsed &&
                [...group.files.entries()].map(([file, syms]) => {
                  const fKey = `${cKey}/f-${file}`
                  const fCollapsed = collapsed.has(fKey)
                  return (
                    <div key={fKey} style={{ marginLeft: 8 }}>
                      <div
                        onClick={() => toggle(fKey)}
                        style={{
                          cursor: 'pointer',
                          padding: '2px 8px',
                          fontSize: '0.78em',
                          opacity: 0.75,
                          fontFamily: 'monospace',
                        }}
                      >
                        {fCollapsed ? '▶' : '▼'} {file}
                      </div>
                      {!fCollapsed &&
                        syms.map((sym) => (
                          <div
                            key={sym.id}
                            className={`atom-item ${
                              selectedId === sym.id ? 'selected' : ''
                            }`}
                            onClick={() => onSelect(sym.id)}
                            style={{
                              marginLeft: 16,
                              padding: '2px 6px',
                              cursor: 'pointer',
                              background: selectedId === sym.id ? '#dde' : 'transparent',
                              fontSize: '0.85em',
                            }}
                          >
                            <span style={{ opacity: 0.6, marginRight: 6 }}>{sym.kind}</span>
                            <span style={{ fontWeight: sym.exported ? 'bold' : 'normal' }}>
                              {sym.name}
                            </span>
                            <span style={{ opacity: 0.5, marginLeft: 6, fontSize: '0.85em' }}>
                              :{sym.start_line}
                            </span>
                          </div>
                        ))}
                    </div>
                  )
                })}
            </div>
          )
        })}
      </div>
    </>
  )
}
