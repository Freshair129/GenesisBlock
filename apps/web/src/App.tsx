import { useState, useEffect, useMemo } from 'react';
import { GKS_SERVICE } from './services/gksService';
import { useTweaks } from './hooks/useTweaks';
import { Icon } from './components/shared/Icon';
import { TypeDot } from './components/shared/TypeDot';
import { Sidebar } from './components/layout/Sidebar';
import { RightRail } from './components/layout/RightRail';
import { TopbarSearch } from './components/layout/TopbarSearch';
import { Editor } from './components/views/Editor';
import { Graph3DView } from './components/views/Graph3DView';
import { Graph2DView } from './components/views/Graph2DView';
import { GalaxyView } from './components/views/GalaxyView';
import { EmbeddingView } from './components/views/EmbeddingView';
import { Daily } from './components/views/Daily';
import { Chat } from './components/views/Chat';
import { CommandPalette } from './components/modals/CommandPalette';
import { Settings } from './components/modals/Settings';

const TWEAK_DEFAULTS = {
  theme: "nova",
  density: "comfy",
  showStatus: true,
  accent: "#7c5cff"
};

const THEMES: Record<string, { accent: string, soft: string }> = {
  nova:   { accent: "#7c5cff", soft: "rgba(124,92,255,0.16)" },
  citrus: { accent: "#fbbf24", soft: "rgba(251,191,36,0.18)" },
  bloom:  { accent: "#f472b6", soft: "rgba(244,114,182,0.20)" },
  mono:   { accent: "#a4a9be", soft: "rgba(164,169,190,0.16)" },
  cyber:  { accent: "#4dd6e8", soft: "rgba(77,214,232,0.18)" },
};

function App() {
  const [tweaks] = useTweaks(TWEAK_DEFAULTS);

  useEffect(() => {
    const t = THEMES[tweaks.theme] || THEMES.nova;
    document.documentElement.style.setProperty("--accent", tweaks.accent || t.accent);
    document.documentElement.style.setProperty("--accent-soft", t.soft);
  }, [tweaks.theme, tweaks.accent]);

  // Tabs
  const [tabs, setTabs] = useState<any[]>([
    { kind: "graph",  id: "__graph__",         title: "Knowledge Graph" },
    { kind: "note",   id: "MOC--product",      title: "MOC — Product" },
  ]);
  const [activeTabIdx, setActiveTabIdx] = useState(1);
  const active = tabs[activeTabIdx];

  const [sbMode, setSbMode] = useState<'files' | 'tags' | 'daily'>("files");
  const [graphMode, setGraphMode] = useState<'2d' | '3d' | 'galaxy'>('2d');
  const [activeTags, setActiveTags] = useState<string[]>([]);
  const [mobileNav, setMobileNav] = useState(false);
  const [mobileRail, setMobileRail] = useState(false);
  const toggleTag = (t: string) => setActiveTags(arr => arr.includes(t) ? arr.filter(x => x !== t) : [...arr, t]);

  const filteredNotes = useMemo(() => {
    if (activeTags.length === 0) return GKS_SERVICE.D.notes;
    return GKS_SERVICE.D.notes.filter(n => activeTags.every(t => n.tags.includes(t)));
  }, [activeTags]);

  const openId = (id: string) => {
    const isView = id?.startsWith("__");
    if (isView) {
      const titles: any = { 
        "__graph__": "Knowledge Graph", 
        "__embed__": "Embedding Map", 
        "__daily__": "Daily Notes", 
        "__chat__": "Ask Genesis", 
        "__settings__": "Settings", 
        "__mcp__": "Agents & MCP" 
      };
      if (id === "__settings__") { setShowSettings(true); return; }
      const existing = tabs.findIndex(t => t.id === id);
      if (existing >= 0) { setActiveTabIdx(existing); return; }
      const kind = id.slice(2, -2);
      setTabs(ts => [...ts, { kind, id, title: titles[id] }]);
      setActiveTabIdx(tabs.length);
      return;
    }
    const note = GKS_SERVICE.NOTE_BY_ID[id];
    if (!note) return;
    const existing = tabs.findIndex(t => t.id === id);
    if (existing >= 0) { setActiveTabIdx(existing); return; }
    setTabs(ts => [...ts, { kind: "note", id, title: note.title }]);
    setActiveTabIdx(tabs.length);
  };

  const closeTab = (i: number) => {
    setTabs(ts => {
      const next = ts.filter((_, idx) => idx !== i);
      if (next.length === 0) return [{ kind: "graph", id: "__graph__", title: "Knowledge Graph" }];
      return next;
    });
    setActiveTabIdx(idx => Math.max(0, Math.min(idx, tabs.length - 2)));
  };

  const [paletteOpen, setPaletteOpen] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [semantic, setSemantic] = useState({ query: "", hits: [], state: "idle" as any });
  const [model, setModel] = useState("haiku-4-5");
  const [embedModel, setEmbedModel] = useState("voyage-3-large");

  const runSemanticSearch = (q: string) => {
    setSemantic({ query: q, hits: [], state: "loading" });
    setTimeout(() => {
      const hits = GKS_SERVICE.D.notes
        .map(n => ({ id: n.id, score: GKS_SERVICE.searchScore(q, n), why: "semantic recall" }))
        .filter(r => r.score > 0.1)
        .sort((a,b) => b.score - a.score)
        .slice(0, 8);
      setSemantic({ query: q, hits: hits as any, state: "done" });
    }, 1000);
  };

  const focusNoteId = active?.kind === "note" ? active.id : null;

  return (
    <div className={`app${mobileNav ? ' nav-open' : ''}${mobileRail ? ' rail-open' : ''}`}>
      <div className="topbar">
        <button className="tb-hamburger" onClick={() => setMobileNav(v => !v)}>
          <Icon name="menu"/>
        </button>
        <div className="tb-brand">
          <span className="tb-logo"/>
          Genesis <small>v0.4.2</small>
        </div>
        <div className="tb-tabs">
          {tabs.map((t, i) => (
            <span key={t.id} className={"tb-tab" + (i === activeTabIdx ? " active" : "")}
                  onClick={() => setActiveTabIdx(i)}>
              {t.kind === "note" && <TypeDot type={GKS_SERVICE.NOTE_BY_ID[t.id]?.type} />}
              {t.kind === "graph" && <Icon name="graph"/>}
              {t.kind === "embed" && <Icon name="embed"/>}
              {t.kind === "daily" && <Icon name="calendar"/>}
              {t.kind === "chat" && <Icon name="chat"/>}
              <span style={{ overflow:"hidden", textOverflow:"ellipsis" }}>{t.title}</span>
              <span className="x" onClick={(e) => { e.stopPropagation(); closeTab(i); }}><Icon name="x"/></span>
            </span>
          ))}
        </div>
        <TopbarSearch onSubmit={runSemanticSearch}/>
        <div className="tb-right">
          <button className="tb-iconbtn" title="Open graph (⌘G)" onClick={() => openId("__graph__")}><Icon name="graph"/></button>
          <button className="tb-iconbtn" title="Open embedding map" onClick={() => openId("__embed__")}><Icon name="embed"/></button>
          <button className="tb-iconbtn" title="Ask Genesis" onClick={() => openId("__chat__")}><Icon name="chat"/></button>
          <button className="tb-iconbtn" title="Command Palette (⌘K)" onClick={() => setPaletteOpen(true)}><Icon name="search"/></button>
          <button className="tb-iconbtn tb-rail-toggle" title="Toggle info panel" onClick={() => setMobileRail(v => !v)}><Icon name="info"/></button>
        </div>
      </div>

      {mobileNav && <div className="mobile-backdrop" onClick={() => setMobileNav(false)} />}
      <Sidebar 
        activeId={focusNoteId}
        onOpen={(id) => { openId(id); setMobileNav(false); }}
        activeTags={activeTags}
        toggleTag={toggleTag}
        mode={sbMode}
        setMode={setSbMode}
        onOpenSettings={() => setShowSettings(true)}
      />

      <main className="main">
        <div className="main-header">
          <div className="crumb">
            {active?.kind === "note" && (
              <>
                <span>{GKS_SERVICE.TYPE_META[GKS_SERVICE.NOTE_BY_ID[active.id]?.type]?.label}</span>
                <span style={{ margin: "0 6px" }}>/</span>
                <b>{active.title}</b>
              </>
            )}
            {active?.kind === "graph" && (
              <>
                <span>VIEW</span><span style={{ margin:"0 6px" }}>/</span><b>Knowledge Graph</b>
                <div style={{ marginLeft: 14, display:"flex", gap: 2, background:"var(--bg-2)", border:"1px solid var(--border)", borderRadius: 7, padding: 2 }}>
                  <button style={{ height: 22, padding: "0 9px", borderRadius: 5, fontSize: 11, color: graphMode==="2d" ? "var(--accent)" : "var(--text-mute)", background: graphMode==="2d" ? "var(--accent-soft)" : "transparent", fontFamily: "var(--font-mono)", cursor: "pointer" }}
                          onClick={() => setGraphMode("2d")}>2D · classic</button>
                  <button style={{ height: 22, padding: "0 9px", borderRadius: 5, fontSize: 11, color: graphMode==="3d" ? "var(--accent)" : "var(--text-mute)", background: graphMode==="3d" ? "var(--accent-soft)" : "transparent", fontFamily: "var(--font-mono)", cursor: "pointer" }}
                          onClick={() => setGraphMode("3d")}>3D · neural</button>
                  <button style={{ height: 22, padding: "0 9px", borderRadius: 5, fontSize: 11, color: graphMode==="galaxy" ? "var(--accent)" : "var(--text-mute)", background: graphMode==="galaxy" ? "var(--accent-soft)" : "transparent", fontFamily: "var(--font-mono)", cursor: "pointer" }}
                          onClick={() => setGraphMode("galaxy")}>🌌 galaxy</button>
                </div>
              </>
            )}
            {active?.kind === "embed" && (
              <><span>VIEW</span><span style={{ margin:"0 6px" }}>/</span><b>Embedding Map</b></>
            )}
            {active?.kind === "daily" && (
              <><span>VIEW</span><span style={{ margin:"0 6px" }}>/</span><b>Daily Notes</b></>
            )}
            {active?.kind === "chat" && (
              <><span>VIEW</span><span style={{ margin:"0 6px" }}>/</span><b>Ask Genesis · {model}</b></>
            )}
          </div>
          <div className="view-switch">
            <button className={active?.kind === "note" ? "active" : ""} onClick={() => openId(focusNoteId || "MOC--product")}>
              <Icon name="note"/><span style={{ marginLeft: 6 }}>Editor</span>
            </button>
            <button className={active?.kind === "graph" ? "active" : ""} onClick={() => openId("__graph__")}>
              <Icon name="graph"/><span style={{ marginLeft: 6 }}>Graph</span>
            </button>
            <button className={active?.kind === "embed" ? "active" : ""} onClick={() => openId("__embed__")}>
              <Icon name="embed"/><span style={{ marginLeft: 6 }}>Embeddings</span>
            </button>
          </div>
        </div>
        <div className="main-body">
          {active?.kind === "note"  && <Editor note={GKS_SERVICE.NOTE_BY_ID[active.id]} onOpen={openId}/>}
          {active?.kind === "graph" && graphMode === "2d"     && <Graph2DView notes={filteredNotes} edges={GKS_SERVICE.D.edges} focusId={null} onOpen={openId}/>}
          {active?.kind === "graph" && graphMode === "3d"     && <Graph3DView notes={filteredNotes} edges={GKS_SERVICE.D.edges} focusId={null} onOpen={openId}/>}
          {active?.kind === "graph" && graphMode === "galaxy" && <GalaxyView   notes={filteredNotes} edges={GKS_SERVICE.D.edges} focusId={null} onOpen={openId}/>}
          {active?.kind === "embed" && <EmbeddingView notes={filteredNotes} focusId={focusNoteId} onOpen={openId}/>}
          {active?.kind === "daily" && <Daily onOpen={openId}/>}
          {active?.kind === "chat"  && <Chat activeId={focusNoteId} onOpen={openId} model={model}/>}
        </div>
      </main>

      {mobileRail && <div className="mobile-backdrop" onClick={() => setMobileRail(false)} />}
      <RightRail 
        activeId={focusNoteId} 
        onOpen={(id) => { openId(id); setMobileRail(false); }}
        semanticHits={semantic.hits} 
        semanticState={semantic.state} 
        semanticQuery={semantic.query}
      />

      {tweaks.showStatus && (
        <div className="status">
          <span className="pulse"/>
          <span>graph synced · {GKS_SERVICE.D.notes.length} notes · {GKS_SERVICE.D.edges.length} edges</span>
          <span className="sep">·</span>
          <span>vector store: {embedModel}</span>
          <span className="sep">·</span>
          <span>llm: {model}</span>
          <span className="right">
            <span>UTC+07</span>
          </span>
        </div>
      )}

      <CommandPalette 
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onOpen={openId}
        model={model}
      />

      <Settings 
        open={showSettings} 
        onClose={() => setShowSettings(false)}
        model={model} 
        setModel={setModel}
        embedModel={embedModel} 
        setEmbedModel={setEmbedModel}
      />
    </div>
  );
}

export default App;
