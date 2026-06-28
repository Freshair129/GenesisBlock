import { useState } from "react";
import GraphView from "./components/GraphView";
import RetrieverPanel from "./components/RetrieverPanel";
import type { ContextPackage, NodeOutput, EdgeOutput } from "./lib/api";

export default function App() {
  const [nodes, setNodes] = useState<NodeOutput[]>([]);
  const [edges, setEdges] = useState<EdgeOutput[]>([]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  function handleResult(pkg: ContextPackage) {
    setNodes(pkg.nodes);
    setEdges(pkg.edges);
  }

  function handleNodeTap(id: string) {
    // Feeds back into RetrieverPanel, which re-runs retrieve_context for this
    // node — the Obsidian "tap a node to expand" loop.
    setSelectedNodeId(id);
  }

  const empty = nodes.length === 0;

  return (
    <div className="app">
      <header className="app-header">
        <h1 className="app-title">GenesisBlock</h1>
      </header>

      <RetrieverPanel onResult={handleResult} selectedNodeId={selectedNodeId} />

      <main className="app-graph">
        {empty ? (
          <div className="empty-state">
            <p className="empty-title">No context loaded</p>
            <p className="empty-hint">
              Enter a node id or query above and pick a tier (H0–H5) to retrieve
              a context graph. Tap any node to expand its neighborhood.
            </p>
          </div>
        ) : (
          <GraphView nodes={nodes} edges={edges} onNodeTap={handleNodeTap} />
        )}
      </main>
    </div>
  );
}
