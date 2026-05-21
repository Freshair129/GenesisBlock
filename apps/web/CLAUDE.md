# Genesis UI — Frontend CLAUDE.md

# Created At: 2026-05-14 12:00:00 +07:00 (v1.1.0)

# Last Updated: 2026-05-14 14:30:00 +07:00 (v1.1.0) — moved packages/ui → apps/web

> Read this before doing anything in `apps/web/`.
> This is the web frontend for Genesis Knowledge System.

---

## What This Is

**Genesis UI** — a Vite + React 19 + TypeScript knowledge graph explorer.
Deployed to: **<https://genesis-ui-eight.vercel.app/>**
Vercel project ID: `prj_aELlyAqrhvnGBAPxl70HpE24DLrR`

This is a **read-only visualization layer** over GKS data. It does not write to GKS.

---

## Commands

Run from `apps/web/` (NOT from repo root):

```bash
npm run dev       # dev server → http://localhost:5173
npm run build     # tsc -b && vite build → dist/
npm run lint      # eslint check
vercel deploy --prod   # deploy to genesis-ui-eight.vercel.app
```

From repo root:

```bash
npm run dev --workspace=apps/web
npm run build --workspace=apps/web
```

---

## File Structure

```
apps/web/
  src/
    App.tsx                    # root — tabs, routing, global state
    index.css                  # all CSS variables + global styles
    main.tsx                   # React entry point
    services/
      gksService.tsx           # SSOT for data — reads gksData.json, exposes NOTE_BY_ID, TYPE_META
    data/
      gksData.json             # GKS snapshot (regenerate with sync-gks.mjs)
      mockData.ts              # fallback mock data
    types/
      gks.ts                   # shared TypeScript interfaces
    hooks/
      useTweaks.ts             # persists user settings (theme, density, etc.)
    components/
      views/
        Editor.tsx             # markdown editor / note viewer
        Graph3DView.tsx        # Canvas 2D — 3D force graph with perspective projection
        GalaxyView.tsx         # Canvas 2D — spiral galaxy particle visualization
        EmbeddingView.tsx      # 2D scatter plot of note embeddings
        Daily.tsx              # daily notes list
        Chat.tsx               # LLM chat interface
      layout/
        Sidebar.tsx            # left nav — files / tags / daily modes
        RightRail.tsx          # semantic search results panel
        TopbarSearch.tsx       # search input in topbar
      modals/
        CommandPalette.tsx     # ⌘K palette
        Settings.tsx           # settings modal
      shared/
        Icon.tsx               # icon component (svg sprite)
        TypeDot.tsx            # colored dot for atom type
        RefCard.tsx            # reference card
  scripts/
    sync-gks.mjs               # syncs GKS atomic_index.jsonl → src/data/gksData.json
  public/
    favicon.svg
    icons.svg                  # SVG sprite for Icon component
  DESIGN-SYSTEM.md             # Genesis UI design tokens (VS Code dark theme)
```

---

## Data Flow

```
GKS atomic_index.jsonl
  → node apps/web/scripts/sync-gks.mjs
  → src/data/gksData.json
  → gksService.tsx (GKS_SERVICE singleton)
  → components (via props: notes[], edges[], focusId, onOpen)
```

Run sync: `node apps/web/scripts/sync-gks.mjs`

GKS index path (hardcoded in sync-gks.mjs):
`C:/Users/freshair/cognitive_system/gks/00_index/atomic_index.jsonl`

---

## Design System

Full spec: **`apps/web/DESIGN-SYSTEM.md`** — read before touching CSS or components.

Genesis UI uses a **VS Code dark dev-tool theme**.
Do NOT use branding tokens or soft-app aesthetics (rounded corners, warm gradients).

---

## Canvas Views — Key Patterns

`Graph3DView.tsx` and `GalaxyView.tsx` both use the same camera model:

- `camTgt` ref: `{ yaw, pitch, dist }` — spherical coordinates
- Perspective projection: `z_proj = node.z * cos(pitch) + ...`, scale = `dist / (dist + z)`
- Mouse: drag = rotate (yaw/pitch), wheel = zoom (dist)
- Animation: `requestAnimationFrame` loop inside `useEffect`, cleanup with `cancelAnimationFrame(raf)`
- Time accumulation: always use `useRef` for `timeRef` — NEVER local `let time = 0` (resets on effect rerun)

```typescript
const timeRef = useRef(0);
// in RAF loop:
timeRef.current += dt;
```

---

## Deployment

Full guide: **`apps/web/DEPLOY.md`**

Vercel project: `genesis-ui` (ID: `prj_aELlyAqrhvnGBAPxl70HpE24DLrR`)
Deploy from `apps/web/` directory:

```bash
cd apps/web && vercel deploy --prod
```

Build output: `dist/` — Vite SPA build. No SSR. Pure static deployment.

---

## Rules

1. **Never import from `packages/gks` or `packages/msp`** — UI reads the JSON snapshot only.
2. **Always use `GKS_SERVICE.*`** for data access — never read `gksData.json` directly in components.
3. **Canvas components must clean up RAF** — `return () => cancelAnimationFrame(raf)` in useEffect.
4. **Use `useRef` for animation state** (time, positions) — not `useState` (causes re-renders + effect resets).
5. **TypeScript**: Use `n.type as keyof typeof GKS_SERVICE.TYPE_META` when indexing TYPE_META.
6. **Theme tokens**: Always use `var(--accent)`, `var(--bg)` etc. — never hardcode hex in TSX.
7. **No backend calls** from this app — all data is local JSON.
