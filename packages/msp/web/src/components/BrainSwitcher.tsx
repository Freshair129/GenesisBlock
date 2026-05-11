import { useState, useEffect } from 'react'
import { api } from '../api'

interface Props {
  onChanged: () => void
}

export default function BrainSwitcher({ onChanged }: Props) {
  const [brains, setBrains] = useState<any[]>([])
  const [activeIndex, setActiveIndex] = useState(0)
  const [showAdd, setShowAdd] = useState(false)
  const [newName, setNewName] = useState('')
  const [newPath, setNewPath] = useState('')

  const load = async () => {
    const res = await api.getBrains()
    setBrains(res.brains)
    setActiveIndex(res.activeBrainIndex)
  }

  useEffect(() => {
    load()
  }, [])

  const handleSwitch = async (idx: number) => {
    await api.switchBrain(idx)
    setActiveIndex(idx)
    onChanged()
  }

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!newName || !newPath) return
    await api.addBrain(newName, newPath)
    setNewName('')
    setNewPath('')
    setShowAdd(false)
    load()
  }

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
      <select 
        value={activeIndex} 
        onChange={(e) => handleSwitch(parseInt(e.target.value))}
        style={{ background: '#333', color: 'white', padding: '5px', border: '1px solid #555' }}
      >
        {brains.map((b, i) => (
          <option key={i} value={i}>{b.name}</option>
        ))}
      </select>
      
      <button 
        onClick={() => setShowAdd(!showAdd)}
        style={{ background: '#007acc', color: 'white', border: 'none', padding: '5px 10px', cursor: 'pointer', borderRadius: '4px' }}
      >
        + Add Brain
      </button>

      {showAdd && (
        <div style={{
          position: 'absolute', top: '50px', left: '20px', background: '#1e1e1e', padding: '15px', 
          border: '1px solid #333', zIndex: 1000, borderRadius: '8px', boxShadow: '0 4px 10px rgba(0,0,0,0.5)'
        }}>
          <form onSubmit={handleAdd} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            <input 
              placeholder="Brain Name (e.g. My Vault)" 
              value={newName} 
              onChange={e => setNewName(e.target.value)}
              style={{ background: '#121212', color: 'white', border: '1px solid #333', padding: '5px' }}
            />
            <input 
              placeholder="Absolute Path (e.g. C:/Vaults/MyBrain)" 
              value={newPath} 
              onChange={e => setNewPath(e.target.value)}
              style={{ background: '#121212', color: 'white', border: '1px solid #333', padding: '5px' }}
            />
            <div style={{ display: 'flex', gap: '10px' }}>
              <button type="submit" style={{ flex: 1, background: '#28a745', color: 'white', border: 'none', padding: '5px' }}>Save</button>
              <button type="button" onClick={() => setShowAdd(false)} style={{ flex: 1, background: '#6c757d', color: 'white', border: 'none', padding: '5px' }}>Cancel</button>
            </div>
          </form>
        </div>
      )}
    </div>
  )
}
