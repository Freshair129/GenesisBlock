import { useState, useMemo } from 'react'
import type { Atom } from '../api'

interface Props {
  atoms: Atom[]
  selectedId: string | null
  onSelect: (id: string) => void
}

export default function AtomList({ atoms, selectedId, onSelect }: Props) {
  const [filterType, setFilterType] = useState<string>('ALL')

  const types = useMemo(() => {
    const t = new Set(atoms.map(a => a.type))
    return ['ALL', ...Array.from(t).sort()]
  }, [atoms])

  const filtered = useMemo(() => {
    if (filterType === 'ALL') return atoms
    return atoms.filter(a => a.type === filterType)
  }, [atoms, filterType])

  return (
    <>
      <div className="atom-list-header">
        <select value={filterType} onChange={e => setFilterType(e.target.value)}>
          {types.map(t => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>
      </div>
      <div className="atom-list">
        {filtered.map(atom => (
          <div 
            key={atom.id} 
            className={`atom-item ${selectedId === atom.id ? 'selected' : ''}`}
            onClick={() => onSelect(atom.id)}
          >
            <div className="atom-id">{atom.id}</div>
            <div className="atom-title" title={atom.title}>{atom.title || 'Untitled'}</div>
          </div>
        ))}
      </div>
    </>
  )
}
