import { useEffect, useState } from 'react';
import {
  Activity,
  ArrowLeft,
  Boxes,
  Braces,
  ChevronRight,
  CircleDot,
  Database,
  GitBranch,
  Network,
  Play,
  Search,
  ShieldCheck,
  Sparkles,
  Table2,
  Unplug,
} from 'lucide-react';
import type {
  EntityInspection,
  GraphScene,
  GraphSceneNode,
  RelationalSchemaSummary,
  StudioCapabilities,
  StudioCollection,
  StudioStatus,
  StudioTransport,
} from './domain/contracts';
import { GraphCanvas } from './features/graph/GraphCanvas';
import { createLocalTransport, createRemoteTransport } from './transports/backend';
import { createMockTransport } from './transports/mock';

const navigation = [
  { label: 'Overview', icon: Activity },
  { label: 'Data', icon: Table2 },
  { label: 'Graph', icon: Network },
  { label: 'Vectors', icon: CircleDot },
  { label: 'Query', icon: Braces },
  { label: 'Operations', icon: Boxes },
];

function formatNumber(value: number): string {
  return new Intl.NumberFormat('en-US').format(value);
}

function ConnectionScreen({ onConnect }: { onConnect: (transport: StudioTransport) => void }) {
  const [mode, setMode] = useState<'local' | 'remote'>('local');
  const [path, setPath] = useState('');
  const [baseUrl, setBaseUrl] = useState('http://127.0.0.1:3000');
  const [token, setToken] = useState('');
  const [error, setError] = useState('');
  const [connecting, setConnecting] = useState(false);

  const connect = async () => {
    setConnecting(true);
    setError('');
    try {
      const transport = mode === 'local'
        ? await createLocalTransport(path)
        : await createRemoteTransport(baseUrl, token);
      onConnect(transport);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setConnecting(false);
    }
  };

  return (
    <main className="connection-screen">
      <div className="brand-lockup">
        <span className="brand-mark"><GitBranch size={20} /></span>
        <span>GENESIS / STUDIO</span>
      </div>
      <section className="connection-card">
        <div className="eyebrow"><Sparkles size={14} /> READ-ONLY EXPLORER</div>
        <h1>One place to see<br /><em>what your data means.</em></h1>
        <p className="connection-lede">
          Relational evidence, graph structure, vector proximity, and time in one bounded scene.
        </p>
        <div className="mode-switch" role="group" aria-label="Connection mode">
          <button className={mode === 'local' ? 'active' : ''} onClick={() => setMode('local')}>Local embedded</button>
          <button className={mode === 'remote' ? 'active' : ''} onClick={() => setMode('remote')}>Remote server</button>
        </div>
        {mode === 'local' ? (
          <label className="connection-field">
            <span>GENESIS DATA ROOT</span>
            <input value={path} onChange={(event) => setPath(event.target.value)} placeholder="C:\\data\\my-genesis-db" />
          </label>
        ) : (
          <div className="connection-fields">
            <label className="connection-field"><span>SERVER URL</span><input value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} /></label>
            <label className="connection-field"><span>BEARER TOKEN</span><input type="password" value={token} onChange={(event) => setToken(event.target.value)} placeholder="Optional for local-only server" /></label>
          </div>
        )}
        {error && <p className="connection-error" role="alert">{error}</p>}
        <div className="connection-actions">
          <button className="primary-action" onClick={() => void connect()} disabled={connecting || (mode === 'local' && !path.trim())}>
            {connecting ? 'Negotiating capabilities...' : `Connect ${mode}`} <ChevronRight size={18} />
          </button>
          <button className="fixture-action" onClick={() => onConnect(createMockTransport())}>Open fixture workspace</button>
        </div>
        <div className="connection-options">
          <div><Database size={18} /><span><strong>Local embedded</strong>Read-only core + exclusive data-root lock</span></div>
          <div><Unplug size={18} /><span><strong>Remote self-hosted</strong>REST capability negotiation + bearer auth</span></div>
        </div>
      </section>
      <aside className="truth-card">
        <ShieldCheck size={22} />
        <div><strong>Backend-enforced read only</strong><span>Studio enables only negotiated capabilities.</span></div>
      </aside>
      <div className="connection-orbit orbit-a" />
      <div className="connection-orbit orbit-b" />
    </main>
  );
}

function EntityInspector({ inspection }: { inspection: EntityInspection | null }) {
  if (!inspection) {
    return (
      <aside className="inspector empty-inspector">
        <CircleDot size={25} />
        <h2>Select an entity</h2>
        <p>Its relational, graph, vector, and temporal evidence will meet here.</p>
      </aside>
    );
  }
  return (
    <aside className="inspector">
      <div className="inspector-heading">
        <div><span className="eyebrow">ENTITY INSPECTOR</span><h2>{inspection.label}</h2></div>
        <span className="status-dot">READ</span>
      </div>
      <p className="entity-id">{inspection.entityId}</p>
      <div className="evidence-stack">
        <section><span>01 / RELATIONAL</span><strong>{String(inspection.relational.kind ?? inspection.availability.relational)}</strong><small>{Object.keys(inspection.relational).length} visible properties</small></section>
        <section><span>02 / GRAPH</span><strong>{inspection.graph.incoming + inspection.graph.outgoing} links</strong><small>{inspection.graph.incoming} in / {inspection.graph.outgoing} out</small></section>
        <section><span>03 / VECTOR</span><strong>{inspection.vector.collection ?? 'No vector'}</strong><small>{inspection.availability.vector}</small></section>
        <section><span>04 / TIME + CAUSE</span><strong>{new Date(inspection.temporal.validFrom).toLocaleDateString()}</strong><small>{inspection.temporal.causedBy ?? 'root event'}</small></section>
      </div>
    </aside>
  );
}

interface WorkspaceProps {
  transport: StudioTransport;
  onDisconnect: () => void;
}

function Workspace({ transport, onDisconnect }: WorkspaceProps) {
  const [active, setActive] = useState('Graph');
  const [filter, setFilter] = useState('');
  const [scene, setScene] = useState<GraphScene | null>(null);
  const [status, setStatus] = useState<StudioStatus | null>(null);
  const [capabilities, setCapabilities] = useState<StudioCapabilities | null>(null);
  const [collections, setCollections] = useState<StudioCollection[]>([]);
  const [schemas, setSchemas] = useState<RelationalSchemaSummary[]>([]);
  const [selected, setSelected] = useState<GraphSceneNode | null>(null);
  const [inspection, setInspection] = useState<EntityInspection | null>(null);
  const [query, setQuery] = useState('TRAVERSE FROM "entity-1" DEPTH 1 REL ANY');
  const [queryResult, setQueryResult] = useState<unknown>(null);
  const [namedQueryKey, setNamedQueryKey] = useState('');
  const [namedQueryParameters, setNamedQueryParameters] = useState('{}');
  const [namedQueryResult, setNamedQueryResult] = useState<unknown>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    let current = true;
    void Promise.all([
      transport.getCapabilities(),
      transport.getStatus(),
      transport.loadGraphScene({ limit: 240 }),
      transport.listCollections(),
      transport.listRelationalSchemas(),
    ]).then(([nextCapabilities, nextStatus, nextScene, nextCollections, nextSchemas]) => {
      if (!current) return;
      setCapabilities(nextCapabilities);
      setStatus(nextStatus.data);
      setScene(nextScene.data);
      setCollections(nextCollections.data);
      setSchemas(nextSchemas.data);
    }).catch((reason) => current && setError(reason instanceof Error ? reason.message : String(reason)));
    return () => { current = false; };
  }, [transport]);

  const selectNode = (node: GraphSceneNode) => {
    setSelected(node);
    void transport.inspectEntity(node.id)
      .then((result) => setInspection(result.data))
      .catch((reason) => setError(reason instanceof Error ? reason.message : String(reason)));
  };

  const disconnect = async () => {
    await transport.close().catch(() => undefined);
    onDisconnect();
  };

  const runQuery = async () => {
    setError('');
    try {
      setQueryResult((await transport.executeReadOnlyHql(query)).data);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const runNamedQuery = async () => {
    setError('');
    try {
      const [namespace, queryName] = namedQueryKey.split(':', 2);
      const schema = schemas.find((candidate) => candidate.namespace === namespace);
      const definition = schema?.namedQueries.find((candidate) => candidate.name === queryName);
      if (!schema || !definition) throw new Error('Select a registered named query first.');
      const parameters = JSON.parse(namedQueryParameters) as Record<string, unknown>;
      setNamedQueryResult((await transport.executeNamedQuery({
        namespace,
        schemaVersion: schema.version,
        queryName,
        parameters,
        limit: definition.defaultLimit,
      })).data);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const renderWorkspace = () => {
    if (active === 'Graph') {
      return (
        <>
          <section className="metric-strip">
            <div><span>SCENE</span><strong>{scene ? formatNumber(scene.nodes.length) : '---'}</strong><small>of {capabilities?.limits.sceneNodeCeiling ?? '---'} nodes</small></div>
            <div><span>DATABASE</span><strong>{status ? formatNumber(status.nodeCount) : '---'}</strong><small>visible entities</small></div>
            <div><span>INDEX LAG</span><strong>{status?.indexLag ?? '---'}</strong><small>{status?.indexLag ? 'results may be stale' : 'caught up'}</small></div>
            <div><span>FRONTIER</span><strong>{capabilities?.consistency ?? '---'}</strong><small>{capabilities?.engineVersion ?? 'negotiating'}</small></div>
          </section>
          <section className="graph-workbench">
            <div className="graph-toolbar">
              <label><Search size={16} /><input value={filter} onChange={(event) => setFilter(event.target.value)} placeholder="Filter this bounded scene" /></label>
              <div className="legend">{scene?.groups.slice(0, 4).map((group) => <span key={group}><i /> {group}</span>)}</div>
            </div>
            <div className="graph-frame">
              {scene ? <GraphCanvas scene={scene} filter={filter} selectedId={selected?.id ?? null} onSelect={selectNode} /> : <div className="graph-loading">Composing bounded scene...</div>}
              <div className="scene-stamp"><Network size={15} /><span>{transport.kind.toUpperCase()} SCENE<br /><b>{scene?.nodes.length ?? 0} / {capabilities?.limits.sceneNodeCeiling ?? '---'} node budget</b></span></div>
            </div>
          </section>
        </>
      );
    }
    if (active === 'Overview' || active === 'Operations') {
      return (
        <section className="card-grid">
          {[
            ['Nodes', status?.nodeCount], ['Edges', status?.edgeCount], ['Collections', status?.collectionCount],
            ['Index lag', status?.indexLag], ['Logical clock', status?.logicalClock], ['Mode', capabilities?.mode],
          ].map(([label, value]) => <article key={label}><span>{label}</span><strong>{typeof value === 'number' ? formatNumber(value) : value ?? '---'}</strong></article>)}
          {active === 'Operations' && <p className="read-only-notice"><ShieldCheck size={18} /> Lifecycle actions remain disabled until scoped admin authorization is negotiated.</p>}
        </section>
      );
    }
    if (active === 'Data') {
      if (!schemas.length) return <p className="read-only-notice">No application relational schemas are registered.</p>;
      return <>
        <section className="card-grid">{schemas.map((schema) => <article key={schema.namespace}><span>SCHEMA v{schema.version}</span><strong>{schema.namespace}</strong><small>{schema.tables} tables / {schema.namedQueries.length} named queries</small></article>)}</section>
        <section className="query-workbench relational-query">
          <span className="eyebrow">REGISTERED READ CONTRACT</span>
          <select value={namedQueryKey} onChange={(event) => setNamedQueryKey(event.target.value)}>
            <option value="">Select a named query</option>
            {schemas.flatMap((schema) => schema.namedQueries.map((definition) => <option key={`${schema.namespace}:${definition.name}`} value={`${schema.namespace}:${definition.name}`}>{schema.namespace} / {definition.name} ({definition.parameters.join(', ') || 'no parameters'})</option>))}
          </select>
          <textarea value={namedQueryParameters} onChange={(event) => setNamedQueryParameters(event.target.value)} spellCheck={false} aria-label="Named query JSON parameters" />
          <button className="primary-action" onClick={() => void runNamedQuery()}><Play size={15} /> Run named query</button>
          <pre>{namedQueryResult ? JSON.stringify(namedQueryResult, null, 2) : 'Only queries registered in the active relational schema can run here.'}</pre>
        </section>
      </>;
    }
    if (active === 'Vectors') {
      return <section className="card-grid">{collections.map((collection) => <article key={collection.name}><span>{collection.metric} / {collection.dimension}D</span><strong>{collection.name}</strong><small>{formatNumber(collection.vectorCount)} vectors / lag {collection.indexLag}</small></article>)}</section>;
    }
    return (
      <section className="query-workbench">
        <span className="eyebrow">READ-ONLY HQL</span>
        <textarea value={query} onChange={(event) => setQuery(event.target.value)} spellCheck={false} />
        <button className="primary-action" onClick={() => void runQuery()}><Play size={15} /> Run bounded query</button>
        <pre>{queryResult ? JSON.stringify(queryResult, null, 2) : 'Results appear here with no write fallback.'}</pre>
      </section>
    );
  };

  return (
    <div className="studio-shell">
      <aside className="sidebar">
        <div className="sidebar-brand"><span className="brand-mark"><GitBranch size={18} /></span><span>GENESIS<br /><b>STUDIO</b></span></div>
        <div className="connection-chip"><i /><span><b>{transport.kind} / {capabilities?.mode ?? 'connecting'}</b>{capabilities?.protocolVersion ?? 'negotiating'}</span></div>
        <nav>{navigation.map((item) => <button key={item.label} className={active === item.label ? 'active' : ''} aria-label={`${item.label} workspace`} title={`${item.label} workspace`} onClick={() => setActive(item.label)}><item.icon size={17} /><span>{item.label}</span></button>)}</nav>
        <button className="disconnect" onClick={() => void disconnect()}><ArrowLeft size={16} /> Connections</button>
      </aside>
      <main className="workspace">
        <header className="workspace-header"><div><span className="breadcrumb">WORKSPACES /</span><h1>{active}</h1></div><div className="truth-badge"><ShieldCheck size={14} /> {transport.kind.toUpperCase()} / READ ONLY</div></header>
        {error && <p className="workspace-error" role="alert">{error}</p>}
        {renderWorkspace()}
      </main>
      <EntityInspector inspection={inspection} />
    </div>
  );
}

export default function App() {
  const [transport, setTransport] = useState<StudioTransport | null>(null);
  return transport
    ? <Workspace transport={transport} onDisconnect={() => setTransport(null)} />
    : <ConnectionScreen onConnect={setTransport} />;
}
