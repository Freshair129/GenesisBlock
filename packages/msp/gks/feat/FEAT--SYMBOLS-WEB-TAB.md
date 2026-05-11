---
id: FEAT--SYMBOLS-WEB-TAB
phase: 2
type: feat
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Knowledge Browser Symbols tab — Cytoscape graph + community filter + click-through
tags:
  - msp
  - symbol-graph
  - web-ui
  - knowledge-browser
  - cytoscape
  - feat
crosslinks: {"implements":["ADR--SYMBOL-GRAPH-PERSISTENCE"],"references":["FRAME--SYMBOL-GRAPH","FEAT--MSP-SYMBOL-MCP"]}
linked_symbols:
  - {"file":"web/src/components/SymbolsTab.tsx"}
  - {"file":"web/src/components/SymbolList.tsx"}
  - {"file":"web/src/components/SymbolGraphView.tsx"}
  - {"file":"web/src/components/SymbolDetail.tsx"}
  - {"file":"web/src/api.ts"}
  - {"file":"src/index.ts"}
created_at: 2026-05-09T09:54:00.000Z
---

# FEAT — Knowledge Browser Symbols tab

## User-facing behaviour

A new `Symbols` tab in the Knowledge Browser web UI (alongside existing `Atoms` and `Candidates` tabs). When the symbol graph is built, the tab shows:

- **Left pane** (`SymbolList.tsx`) — tree grouped by `community → file → symbol`. Filter chips: kind (function / class / interface / etc.), exported-only, community.
- **Center pane** (`SymbolGraphView.tsx`) — Cytoscape.js graph (cose layout). Color = community (HSL hash on `community_id`); shape = kind; edge style = type (dashed for `imports`, solid for `calls`, etc.). Click a node to highlight its neighbors.
- **Right pane** (`SymbolDetail.tsx`) — selected symbol's `file:line`, signature, exported flag, parent (if method/member), neighbor list (collapsible by edge type), `vscode://file/<abs>:<line>` link button.

If the graph isn't built yet, the tab shows an empty-state with a `Run msp:graph build` hint and a one-click button that surfaces the right CLI command (it doesn't execute it; copy-to-clipboard).

## API endpoints (added to `src/index.ts` near existing `/api/graph`)

| Method | Path | Returns |
|---|---|---|
| GET | `/api/symbols` | `{ symbols: Symbol[], communities: Community[] }` (capped at 5000; `?offset&limit` paginate) |
| GET | `/api/symbols/:id` | `Symbol` + neighbor preview |
| GET | `/api/symbols/:id/neighbors?depth=1` | `{ nodes, edges }` for Cytoscape |
| GET | `/api/symbols/community/:id` | `{ community, members, edges }` |
| GET | `/api/symbols/search?q=...&limit=20` | ranked hit list |
| GET | `/api/symbols/stats` | counts + last-built |

All endpoints return `404` with `{ ok: false, error: "graph not built" }` when SQLite is missing. No auth — same trust model as existing `/api/atoms` (local dev server only).

## Component architecture

`SymbolsTab.tsx` is the parent; mounts `SymbolList` + `SymbolGraphView` + `SymbolDetail` and threads `selectedSymbolId` state. Reuses the Cytoscape pattern from `web/src/components/GraphView.tsx` (cose layout, neighbor highlight on click, drag-to-pan).

`web/src/api.ts` extends with:
```typescript
getSymbols(): Promise<{ symbols: Symbol[]; communities: Community[] }>
getSymbol(id: string): Promise<SymbolDetailDTO>
getSymbolNeighbors(id: string, depth?: number): Promise<{ nodes; edges }>
getSymbolCommunity(id: number): Promise<CommunityDetailDTO>
searchSymbols(q: string, limit?: number): Promise<SymbolSearchHit[]>
getSymbolStats(): Promise<SymbolStats>
```

Plus TypeScript types `Symbol`, `Edge`, `Community`, etc. mirroring the SQLite schema.

## Tab visibility (feature flag in PR-5)

To avoid shipping an empty tab to users who haven't built the graph yet, the tab is gated behind `MSP_SYMBOL_GRAPH=1` env var (or `?symbols=1` URL param) in PR-5. Once `meta.last_built_at` exists for the active project, the tab auto-enables on the next page load. Flag-removal happens in PR-6 alongside the AUDIT.

## Verification

- Snapshot/render tests for the 4 new components in PR-5 (vitest + jsdom)
- Manual: `npm run dev`, build a graph, navigate to Symbols tab, click through ~5 symbols
- Empty-state path: same flow but without graph → see hint card

## Out of scope

- Editing / refactoring from the UI (read-only)
- Cross-vault Brain Switcher integration for symbols (Phase 2 — current `BrainSwitcher` only swaps GKS atom roots)
- 3D graph view — Phase 2 if 5000+ nodes get unwieldy in 2D

## Source

- `FRAME--SYMBOL-GRAPH`, `ADR--SYMBOL-GRAPH-PERSISTENCE`, `FEAT--MSP-SYMBOL-MCP`
- Existing Cytoscape pattern: `web/src/components/GraphView.tsx`
- Existing tab pattern: `web/src/App.tsx` `Atoms | Candidates` toggle
