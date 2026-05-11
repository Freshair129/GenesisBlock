import { useEffect, useMemo, useRef, useState } from 'react'
import CytoscapeComponent from 'react-cytoscapejs'
import cytoscape from 'cytoscape'
import {
  api,
  type SymbolCommunity,
  type SymbolEdgeType,
  type SymbolKind,
  type SymbolNode,
} from '../api'

interface Props {
  symbols: SymbolNode[]
  communities: Map<number, SymbolCommunity>
  selectedId: string | null
  onSelect: (id: string) => void
}

/**
 * Stable HSL color from a community id.
 * Hash inspired by the CLI's cytoscape JSON exporter.
 */
function communityColor(id: number | null): string {
  if (id === null) return '#888'
  const hue = (id * 137.508) % 360
  return `hsl(${hue.toFixed(0)}, 60%, 55%)`
}

const KIND_SHAPES: Record<SymbolKind, string> = {
  function: 'ellipse',
  method: 'ellipse',
  class: 'rectangle',
  interface: 'triangle',
  type: 'diamond',
  enum: 'hexagon',
  const: 'pentagon',
  module: 'octagon',
}

const EDGE_LINE_STYLE: Record<SymbolEdgeType, string> = {
  defines: 'solid',
  imports: 'dashed',
  calls: 'solid',
  extends: 'dotted',
  implements: 'dotted',
  references: 'dashed',
}

interface Element {
  data: Record<string, unknown>
}

export default function SymbolGraphView({ symbols, communities, selectedId, onSelect }: Props) {
  const cyRef = useRef<cytoscape.Core | null>(null)
  const [neighborEdges, setNeighborEdges] = useState<
    { src_id: string; dst_id: string; type: string }[]
  >([])
  const [neighborError, setNeighborError] = useState<string | null>(null)

  // Build node + initial edge elements from local symbols list (no edges yet — neighbor
  // edges are loaded lazily on selection).
  const elements = useMemo<Element[]>(() => {
    const nodes: Element[] = symbols.map((s) => ({
      data: {
        id: s.id,
        label: s.name,
        kind: s.kind,
        community: s.community_id,
        exported: s.exported ? 1 : 0,
      },
    }))
    const seenEdges = new Set<string>()
    const edges: Element[] = []
    for (const e of neighborEdges) {
      const key = `${e.src_id}|${e.dst_id}|${e.type}`
      if (seenEdges.has(key)) continue
      seenEdges.add(key)
      edges.push({
        data: {
          id: key,
          source: e.src_id,
          target: e.dst_id,
          edgeType: e.type,
        },
      })
    }
    return [...nodes, ...edges]
  }, [symbols, neighborEdges])

  // Load neighbors when a symbol is selected
  useEffect(() => {
    if (!selectedId) {
      setNeighborEdges([])
      setNeighborError(null)
      return
    }
    let cancelled = false
    api
      .getSymbolNeighbors(selectedId, 1)
      .then((res) => {
        if (cancelled) return
        setNeighborEdges(res.edges)
        setNeighborError(null)
      })
      .catch((err: Error) => {
        if (cancelled) return
        setNeighborEdges([])
        setNeighborError(err.message)
      })
    return () => {
      cancelled = true
    }
  }, [selectedId])

  // Highlight selected + neighbors
  useEffect(() => {
    const cy = cyRef.current
    if (!cy) return
    cy.elements().removeClass('selected').removeClass('neighbor')
    if (selectedId) {
      const node = cy.getElementById(selectedId)
      if (node.length > 0) {
        node.addClass('selected')
        node.neighborhood().addClass('neighbor')
        cy.center(node)
      }
    }
  }, [selectedId, elements])

  const stylesheet: cytoscape.StylesheetStyle[] = [
    {
      selector: 'node',
      style: {
        'background-color': (ele: cytoscape.NodeSingular) =>
          communityColor((ele.data('community') as number | null) ?? null),
        'shape': (ele: cytoscape.NodeSingular) => {
          const kind = ele.data('kind') as SymbolKind
          return (KIND_SHAPES[kind] ?? 'ellipse') as cytoscape.Css.NodeShape
        },
        label: 'data(label)',
        color: '#fff',
        'text-valign': 'bottom',
        'text-halign': 'center',
        'font-size': '9px',
        'text-outline-width': 2,
        'text-outline-color': '#121212',
        width: 18,
        height: 18,
      },
    },
    {
      selector: 'node[exported = 1]',
      style: {
        'border-width': 1,
        'border-color': '#fff',
      },
    },
    {
      selector: 'edge',
      style: {
        width: 1.5,
        'line-color': '#666',
        'line-style': (ele: cytoscape.EdgeSingular) => {
          const t = ele.data('edgeType') as SymbolEdgeType
          return (EDGE_LINE_STYLE[t] ?? 'solid') as cytoscape.Css.LineStyle
        },
        'target-arrow-color': '#666',
        'target-arrow-shape': 'triangle',
        'curve-style': 'bezier',
        opacity: 0.5,
      },
    },
    {
      selector: 'node.selected',
      style: {
        'border-width': 4,
        'border-color': '#fff',
        width: 30,
        height: 30,
      },
    },
    {
      selector: 'node.neighbor',
      style: {
        'border-width': 2,
        'border-color': '#aaa',
      },
    },
    {
      selector: 'edge.neighbor',
      style: {
        'line-color': '#aaa',
        'target-arrow-color': '#aaa',
        opacity: 1,
      },
    },
  ]

  // Compute community count for footer hint.
  const communityCount = communities.size

  return (
    <div style={{ width: '100%', height: '100%', position: 'relative' }}>
      <CytoscapeComponent
        elements={elements}
        style={{ width: '100%', height: '100%' }}
        stylesheet={stylesheet}
        layout={{ name: 'cose', animate: false, randomize: true }}
        cy={(cy) => {
          cyRef.current = cy
          cy.on('tap', 'node', (evt) => {
            onSelect(evt.target.id())
          })
        }}
      />
      <div
        style={{
          position: 'absolute',
          bottom: 8,
          left: 8,
          fontSize: '0.75em',
          opacity: 0.7,
          background: 'rgba(255,255,255,0.7)',
          padding: '2px 6px',
          borderRadius: 4,
          pointerEvents: 'none',
        }}
      >
        {symbols.length} symbols · {communityCount} communities
        {neighborError ? ` · neighbor load: ${neighborError}` : ''}
      </div>
    </div>
  )
}
