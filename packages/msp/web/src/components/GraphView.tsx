import { useEffect, useState, useRef } from 'react'
import CytoscapeComponent from 'react-cytoscapejs'
import type { GraphData } from '../api'
import cytoscape from 'cytoscape'

interface Props {
  data: GraphData
  selectedId: string | null
  onSelect: (id: string) => void
}

// Map Atom types to colors
const getColor = (type: string) => {
  switch(type) {
    case 'CONCEPT': return '#007acc' // blue
    case 'ADR': return '#9b59b6' // purple
    case 'BLUEPRINT': return '#e67e22' // orange
    case 'FEAT': return '#2ecc71' // green
    case 'HOTFIX': return '#e74c3c' // red
    case 'AUDIT': return '#95a5a6' // gray
    default: return '#ecf0f1' // white
  }
}

export default function GraphView({ data, selectedId, onSelect }: Props) {
  const [elements, setElements] = useState<any[]>([])
  const cyRef = useRef<cytoscape.Core | null>(null)

  useEffect(() => {
    const els = [
      ...data.nodes.map(n => ({
        data: { id: n.id, label: n.id, type: n.type }
      })),
      ...data.edges.map(e => ({
        data: { source: e.source, target: e.target, id: `${e.source}-${e.target}` }
      }))
    ]
    setElements(els)
  }, [data])

  useEffect(() => {
    if (!cyRef.current) return
    const cy = cyRef.current
    
    // Highlight selection
    cy.elements().removeClass('selected').removeClass('neighbor')
    if (selectedId) {
      const selected = cy.getElementById(selectedId)
      if (selected.length > 0) {
        selected.addClass('selected')
        selected.neighborhood().addClass('neighbor')
        cy.center(selected)
      }
    }
  }, [selectedId, elements])

  const style: cytoscape.StylesheetStyle[] = [
    {
      selector: 'node',
      style: {
        'background-color': (ele: any) => getColor(ele.data('type')),
        'label': 'data(label)',
        'color': '#fff',
        'text-valign': 'bottom',
        'text-halign': 'center',
        'font-size': '10px',
        'text-outline-width': 2,
        'text-outline-color': '#121212',
        'width': 20,
        'height': 20
      }
    },
    {
      selector: 'edge',
      style: {
        'width': 2,
        'line-color': '#444',
        'target-arrow-color': '#444',
        'target-arrow-shape': 'triangle',
        'curve-style': 'bezier',
        'opacity': 0.5
      }
    },
    {
      selector: 'node.selected',
      style: {
        'border-width': 4,
        'border-color': '#fff',
        'width': 30,
        'height': 30
      }
    },
    {
      selector: 'node.neighbor',
      style: {
        'border-width': 2,
        'border-color': '#aaa'
      }
    },
    {
      selector: 'edge.neighbor',
      style: {
        'line-color': '#888',
        'target-arrow-color': '#888',
        'opacity': 1
      }
    }
  ]

  return (
    <div style={{ width: '100%', height: '100%' }}>
      <CytoscapeComponent 
        elements={elements} 
        style={{ width: '100%', height: '100%' }}
        stylesheet={style}
        layout={{ name: 'cose', animate: false, randomize: true }}
        cy={cy => {
          cyRef.current = cy
          cy.on('tap', 'node', (evt) => {
            onSelect(evt.target.id())
          })
        }}
      />
    </div>
  )
}
