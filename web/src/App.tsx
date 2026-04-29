import { useState, useEffect } from 'react'
import { api } from './api'
import type { Atom, GraphData } from './api'
import AtomList from './components/AtomList'
import GraphView from './components/GraphView'
import AtomDetail from './components/AtomDetail'
import SearchBar from './components/SearchBar'
import StatusBar from './components/StatusBar'

export default function App() {
  const [atoms, setAtoms] = useState<Atom[]>([])
  const [graphData, setGraphData] = useState<GraphData>({ nodes: [], edges: [] })
  const [selectedAtomId, setSelectedAtomId] = useState<string | null>(null)
  const [inboundCount, setInboundCount] = useState(0)
  const [hotfixCount, setHotfixCount] = useState(0)
  
  useEffect(() => {
    // Initial fetch
    Promise.all([
      api.getAtoms(),
      api.getGraph(),
      api.getInbound(),
      api.getHotfixes()
    ]).then(([atomsRes, graphRes, inboundRes, hotfixRes]) => {
      setAtoms(atomsRes)
      setGraphData(graphRes)
      setInboundCount(inboundRes.length)
      setHotfixCount(hotfixRes.length)
    }).catch(console.error)
  }, [])

  return (
    <>
      <div className="top-bar">
        <SearchBar onSelectAtom={setSelectedAtomId} />
      </div>
      
      <div className="main-content">
        <div className="panel-left">
          <AtomList 
            atoms={atoms} 
            selectedId={selectedAtomId} 
            onSelect={setSelectedAtomId} 
          />
        </div>
        
        <div className="panel-center">
          <GraphView 
            data={graphData} 
            selectedId={selectedAtomId} 
            onSelect={setSelectedAtomId} 
          />
        </div>
        
        <div className="panel-right">
          <AtomDetail atomId={selectedAtomId} />
        </div>
      </div>
      
      <StatusBar 
        totalAtoms={atoms.length} 
        inboundCount={inboundCount} 
        hotfixCount={hotfixCount} 
      />
    </>
  )
}
