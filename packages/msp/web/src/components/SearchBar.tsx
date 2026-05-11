import { useState } from 'react'
import { api } from '../api'
import type { RecallHit } from '../api'

interface Props {
  onSelectAtom: (id: string) => void
}

export default function SearchBar({ onSelectAtom }: Props) {
  const [query, setQuery] = useState('')
  const [hits, setHits] = useState<RecallHit[]>([])
  const [searching, setSearching] = useState(false)
  const [showResults, setShowResults] = useState(false)

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!query.trim()) return
    
    setSearching(true)
    try {
      const res = await api.recall(query)
      setHits(res.hits)
      setShowResults(true)
    } catch (e) {
      console.error(e)
    } finally {
      setSearching(false)
    }
  }

  const handleSelect = (id: string) => {
    onSelectAtom(id)
    setShowResults(false)
  }

  return (
    <div style={{ position: 'relative', width: '100%' }}>
      <form onSubmit={handleSearch}>
        <input 
          type="text" 
          className="search-input" 
          placeholder="Search atoms (recall)..." 
          value={query}
          onChange={e => setQuery(e.target.value)}
          onFocus={() => { if (hits.length > 0) setShowResults(true) }}
        />
        {searching && <span style={{ marginLeft: '10px' }}>Searching...</span>}
      </form>
      
      {showResults && hits.length > 0 && (
        <div className="search-results">
          {hits.map(hit => (
            <div key={hit.id} className="search-hit" onClick={() => handleSelect(hit.id)}>
              <div className="hit-id">{hit.id} (score: {hit.score.toFixed(2)})</div>
              <div className="hit-snippet">{hit.snippet.substring(0, 100)}...</div>
            </div>
          ))}
          <div style={{ padding: '5px', textAlign: 'center' }}>
            <button 
              onClick={() => setShowResults(false)}
              style={{ background: 'transparent', color: 'white', border: 'none', cursor: 'pointer' }}
            >Close</button>
          </div>
        </div>
      )}
    </div>
  )
}
