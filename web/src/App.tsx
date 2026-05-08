import { useState, useEffect } from 'react'
import { api } from './api'
import type { Atom, GraphData } from './api'
import AtomList from './components/AtomList'
import CandidatesList from './components/CandidatesList'
import GraphView from './components/GraphView'
import AtomDetail from './components/AtomDetail'
import SearchBar from './components/SearchBar'
import StatusBar from './components/StatusBar'
import BrainSwitcher from './components/BrainSwitcher'

type LeftTab = 'atoms' | 'candidates'

export default function App() {
  const [atoms, setAtoms] = useState<Atom[]>([])
  const [graphData, setGraphData] = useState<GraphData>({ nodes: [], edges: [] })
  const [selectedAtomId, setSelectedAtomId] = useState<string | null>(null)
  const [inboundCount, setInboundCount] = useState(0)
  const [hotfixCount, setHotfixCount] = useState(0)
  const [candidatesCount, setCandidatesCount] = useState(0)
  const [leftTab, setLeftTab] = useState<LeftTab>('atoms')
  const [reloadKey, setReloadKey] = useState(0)

  useEffect(() => {
    // Initial fetch
    Promise.all([
      api.getAtoms(),
      api.getGraph(),
      api.getInbound(),
      api.getHotfixes(),
      api.listCandidates()
    ]).then(([atomsRes, graphRes, inboundRes, hotfixRes, candidatesRes]) => {
      setAtoms(atomsRes)
      setGraphData(graphRes)
      setInboundCount(inboundRes.length)
      setHotfixCount(hotfixRes.length)
      setCandidatesCount(candidatesRes.length)
    }).catch(console.error)
  }, [reloadKey])

  return (
    <>
      <div className="top-bar" style={{ justifyContent: 'space-between' }}>
        <BrainSwitcher onChanged={() => setReloadKey(k => k + 1)} />
        <SearchBar onSelectAtom={setSelectedAtomId} />
      </div>

      <div className="main-content">
        <div className="panel-left">
          <div style={{ display: 'flex', borderBottom: '1px solid #ddd' }}>
            <button
              onClick={() => setLeftTab('atoms')}
              style={{
                flex: 1,
                padding: 8,
                border: 'none',
                background: leftTab === 'atoms' ? '#eef' : 'transparent',
                cursor: 'pointer',
              }}
            >
              Atoms ({atoms.length})
            </button>
            <button
              onClick={() => setLeftTab('candidates')}
              style={{
                flex: 1,
                padding: 8,
                border: 'none',
                background: leftTab === 'candidates' ? '#eef' : 'transparent',
                cursor: 'pointer',
              }}
            >
              Candidates ({candidatesCount})
            </button>
          </div>
          {leftTab === 'atoms' ? (
            <AtomList
              atoms={atoms}
              selectedId={selectedAtomId}
              onSelect={setSelectedAtomId}
            />
          ) : (
            <CandidatesList onChanged={() => setReloadKey((k) => k + 1)} />
          )}
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
        candidatesCount={candidatesCount}
      />
    </>
  )
}
