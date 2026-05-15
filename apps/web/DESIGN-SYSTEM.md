# Genesis UI — Design System
# Created At: 2026-05-14 13:00:00 +07:00 (v1.0.0)

> **PURPOSE — READ FIRST**
> This document is the machine-readable design standard for Genesis UI (`packages/ui/`).
> Every value is taken directly from `src/index.css` and `src/App.tsx`.
> Do NOT invent tokens. Genesis UI is a
> separate product with a completely different visual language.
>
> **Philosophy: "Dev Tool, not App"**
> Genesis UI looks and feels like VS Code / a developer tool — dark, dense, monochromatic,
> sharp corners. It is NOT a marketing site, NOT a dashboard for end users. Every design
> decision prioritizes information density and keyboard-first navigation over visual warmth.

---

## SECTION 1 — Design Tokens (CSS Variables)

All values are defined in `src/index.css`. Never hardcode raw hex values in TSX.

### 1.1 Background Layers

```css
--bg-0:     #1e1e1e;   /* page/canvas background — deepest layer */
--bg-1:     #252526;   /* sidebar, topbar, status bar */
--bg-2:     #2d2d2d;   /* panels, cards, inputs */
--bg-3:     #37373d;   /* active/selected items, overlays */
--bg-hover: #2a2d2e;   /* hover state for list rows */
```

Rule: every UI layer is `bg-N` where N increases from outer to inner. Never skip levels.

### 1.2 Borders

```css
--border:        #333333;   /* default — separates panels */
--border-strong: #444444;   /* active tabs, focused inputs, selected cards */
```

No "rounded" borders. Genesis UI uses sharp `--radius: 2px` throughout.

### 1.3 Text

```css
--text:      #cccccc;   /* primary — readable content */
--text-mute: #858585;   /* secondary — labels, metadata, captions */
--text-dim:  #666666;   /* tertiary — section headers, kbd hints, placeholders */
```

### 1.4 Accent (User-Selectable Theme)

The accent is overridden at runtime by the user's theme choice. Default:

```css
--accent:      #007acc;               /* VS Code Blue — default */
--accent-soft: rgba(0, 122, 204, 0.2); /* tinted bg for selected states */
```

**Available themes** (set by `useTweaks` → applied to `document.documentElement`):

| Theme key | `--accent` | `--accent-soft` |
|---|---|---|
| `nova` (default) | `#7c5cff` | `rgba(124,92,255,0.16)` |
| `citrus` | `#fbbf24` | `rgba(251,191,36,0.18)` |
| `bloom` | `#f472b6` | `rgba(244,114,182,0.20)` |
| `mono` | `#a4a9be` | `rgba(164,169,190,0.16)` |
| `cyber` | `#4dd6e8` | `rgba(77,214,232,0.18)` |

**Rule:** Always use `var(--accent)` and `var(--accent-soft)` — never hardcode the theme hex.
The user picks their accent; components must work with all five values.

### 1.5 Node Type Colors (Knowledge Graph)

Each GKS atom type has a fixed semantic color. Used for dots, graph nodes, legend swatches.

```css
--c-entity:  #4dd6e8;   /* ENTITY — cyan */
--c-feat:    #a78bfa;   /* FEAT / ADR — purple */
--c-flow:    #f472b6;   /* PROCESS / flow — pink */
--c-concept: #fbbf24;   /* CONCEPT — amber */
--c-moc:     #4ade80;   /* MOC — green (also used for "active/live" pulse) */
--c-tag:     #6b7390;   /* TAG — muted blue-grey */
--c-adr:     #ff7b72;   /* ADR — coral */
--c-master:  #ffa657;   /* MASTER / EPISODE — orange */
```

**Always access via `TYPE_META[type]?.raw`** in TypeScript — never hardcode these hex values in
component logic. The mapping lives in `src/types/gks.ts`.

### 1.6 Geometry

```css
--radius:    2px;    /* default border-radius — sharp VS Code feel */
--radius-sm: 1px;    /* tighter elements, rarely needed */
```

**Sharp corners are intentional.** Do not round corners to 8px+.
Exception: pill shapes (kbd, chips, badges) use `border-radius: 999px`.

### 1.7 Shadows

```css
--shadow-pop: 0 4px 12px rgba(0,0,0,0.5);  /* floating overlays, tooltips, cmd palette */
```

Shadows are dark and tight — matching a dense tool aesthetic. No soft `0 12px 40px` glow shadows.

---

## SECTION 2 — Typography

### 2.1 Fonts

```css
--font-sans: 'Inter', ui-sans-serif, system-ui, sans-serif;
--font-mono: 'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, monospace;
```

Inter is loaded from the system or CDN. JetBrains Mono is used extensively for code, IDs,
labels, metadata, and any structural UI text (section headers, kbd hints, status bar).

**Fonts are loaded in `index.html`:**
```html
<link rel="preconnect" href="https://fonts.googleapis.com"/>
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap"/>
```

### 2.2 Font Usage Rules

| Usage | Font | Size | Weight | Color |
|---|---|---|---|---|
| Body / readable text | Inter | 13px (base) | 400 | `--text` |
| Editor body | Inter | 13px | 400 | `--text`, line-height 1.7 |
| Editor h1 | Inter | 24px | 600 | `--text` |
| Editor h2 | Inter | 18px | 600 | `--text` |
| List item label | Inter | 12.5px | 400 | `--text-mute` → `--text` (hover) |
| Tab label | Inter | 12px | 400 | `--text-mute` → `--text` (active) |
| Section headers (uppercase) | JetBrains Mono | 10–10.5px | 400 | `--text-dim`, uppercase, `letter-spacing: 0.08em` |
| Metadata / counts | JetBrains Mono | 10–10.5px | 400 | `--text-dim` |
| Status bar text | JetBrains Mono | 10.5px | 400 | `--text-dim` |
| Keyboard shortcuts (kbd) | JetBrains Mono | 10px | 400 | `--text-mute` |
| Command palette input | Inter | 15px | 400 | `--text` |
| Model / version chips | JetBrains Mono | 10–10.5px | 400 | `--text-dim` |

**Rule:** Mono for structure (labels, IDs, meta, counts, kbd). Sans for content (body, titles, descriptions).

---

## SECTION 3 — Layout Shell

Genesis UI uses a **CSS Grid 4-zone layout** baked into `.app`:

```
┌───────────────────────────────────────┐  h=28px  chrome
├───────────────────────────────────────┤  h=36px  topbar
│ sidebar (240px) │ main (1fr) │ right (300px) │  h=1fr
├───────────────────────────────────────┤  h=22px  status
└───────────────────────────────────────┘
```

```css
grid-template-columns: 240px 1fr 300px;
grid-template-rows: 28px 36px 1fr 22px;
grid-template-areas:
  "chrome chrome chrome"
  "topbar topbar topbar"
  "left   main   right"
  "status status status";
```

This layout is **fixed** — do not add responsive breakpoints. Genesis UI is a desktop-only tool.

---

## SECTION 4 — Component Specifications

### 4.1 App Chrome (title bar)

```
height:     28px
bg:         linear-gradient(180deg, #0a0c12, #07080c)
border-bottom: 1px solid --border
-webkit-app-region: drag   ← entire bar is draggable
font:       --font-sans, 12px, --text-mute
title:      centered absolute, Inter 12px
right meta: --font-mono, 10.5px, --text-dim
```

Traffic light dots: `red #ff5f57` · `yellow #febc2e` · `green #28c840`, 12px circles.

### 4.2 Topbar (tab bar)

```
height:     36px
bg:         linear-gradient(180deg, --bg-1, --bg-0)
border-bottom: 1px solid --border
padding:    0 12px
```

#### Tab states
```
default:  color=--text-mute, bg=transparent, border=1px solid transparent
hover:    bg=--bg-2, color=--text
active:   bg=--bg-3, color=--text, border=1px solid --border-strong
close (x): opacity=0 → 1 on tab hover
```

#### Search trigger
```
height:     28px
bg:         --bg-2
border:     1px solid --border
border-radius: 8px   ← exception: search input is rounded
padding:    0 10px
color:      --text-mute
font:       Inter, 12px
min-width:  280px, max-width: 360px
hover:      bg=--bg-3, border=--border-strong, color=--text
```

#### Icon buttons (tb-iconbtn)
```
size:       28×28px
border-radius: 7px
default:    color=--text-mute
hover:      bg=--bg-2, color=--text
active:     bg=--accent-soft, color=--accent
```

### 4.3 Sidebar

```
width:      240px
bg:         --bg-1
border-right: 1px solid --border
```

#### Nav tabs (sb-nav)
```
height:     28px per button
border-radius: 6px
default:    color=--text-dim
hover:      bg=--bg-2, color=--text
active:     bg=--bg-3, color=--accent
```

#### Section headers
```
font:       --font-mono, 10.5px, uppercase, letter-spacing=0.08em
color:      --text-dim
padding:    10px 8px 6px
```

#### Tree items (file list)
```
height:     ~24px (4px top/bottom padding)
padding:    4px 8px
border-radius: 4px
font:       Inter, 12.5px
default:    color=--text-mute
hover:      bg=--bg-hover, color=--text
active:     bg=--bg-3, color=--accent, font-weight=500
folder:     color=--text, font-weight=500
dot:        6px circle, color=TYPE node color, opacity=0.8
transition: background 0.1s, color 0.1s
```

#### Tag items
```
height:     ~22px (3px top/bottom)
padding:    3px 8px
border-radius: 5px
default:    color=--text
hover:      bg=--bg-2
active:     bg=--accent-soft, color=--accent
hash prefix: --font-mono, color=--text-dim
count:      --font-mono, 10px, --text-dim, margin-left=auto
```

#### Sidebar search input
```
height:     28px
bg:         --bg-2
border:     1px solid --border
border-radius: 7px
padding:    0 10px
font:       Inter, 12px
focus:      border-color=--accent, box-shadow=0 0 0 3px --accent-soft, outline=none
```

### 4.4 Main Header

```
height:     44px
bg:         --bg-1
border-bottom: 1px solid --border
padding:    8px 16px
```

#### Breadcrumb (crumb)
```
font:       --font-mono, 11px, --text-dim
bold parts: color=--text, font-weight=600
```

#### View switch (segmented control)
```
container:  bg=--bg-2, border=1px solid --border, border-radius=8px, padding=2px
button:     height=24px, padding=0 10px, border-radius=6px, font=Inter 11.5px
default:    color=--text-mute
hover:      color=--text
active:     bg=--accent-soft, color=--accent
```

### 4.5 Status Bar

```
height:     22px
bg:         --bg-1
border-top: 1px solid --border
padding:    0 12px
font:       --font-mono, 10.5px, --text-dim
gap:        12px between items
sep:        · character, opacity=0.4
```

Live pulse: 6px circle, color=`--c-moc`, `box-shadow: 0 0 8px --c-moc`, animation: pulse 2s.

### 4.6 Reference Card (ref-card)

```
padding:    10px 12px
bg:         --bg-2
border:     1px solid --border
border-radius: 8px
margin-bottom: 8px
hover:      bg=--bg-3, border=--border-strong

title:      Inter, 12.5px, font-weight=600, flex+gap=7px
dot:        7px circle, node type color
meta:       --font-mono, 10px, --text-dim, gap=8px
snippet:    Inter, 11.5px, --text-mute, line-height=1.5, margin-top=6px
sim-bar:    3px height, bg=--bg-3, border-radius=999px
            fill: linear-gradient(90deg, --c-feat, --c-entity)
```

### 4.7 Graph Overlays (floating panels)

```
bg:         rgba(11,13,20,0.78)
backdrop-filter: blur(10px)
border:     1px solid --border
border-radius: 12px
padding:    10px 12px
font:       Inter, 11.5px
```

These are the ONLY elements with backdrop blur in Genesis UI. Everything else is solid bg.

### 4.8 Node Hover Tooltip

```
bg:         --bg-2
border:     1px solid --border-strong
border-radius: 8px
padding:    6px 10px
font:       Inter, 11.5px
shadow:     --shadow-pop
transform:  translate(-50%, calc(-100% - 14px))
pointer-events: none
white-space: nowrap
```

### 4.9 Command Palette

```
backdrop:   rgba(2,3,7,0.65), backdrop-filter=blur(6px), fixed inset=0, z-index=100
modal:      width=620px, max-width=92vw, bg=--bg-1, border=1px solid --border-strong
            border-radius=14px, shadow=--shadow-pop
input:      height=48px, padding=0 16px, font=Inter 15px, bg=transparent, border=none
            border-bottom=1px solid --border, outline=none
mode tabs:  --font-mono 10.5px, active=color:--accent bg:--accent-soft border:rgba(accent,0.3)
result row: grid 3-col (dot | title+sub | score)
            hover/selected: bg=--bg-2
            selected gets: border-left=2px solid --accent
score chip: --font-mono 10.5px, bg=--bg-3, border-radius=4px, padding=2px 6px
```

### 4.10 AI Chat Bubbles

```
user bubble:  bg=--bg-2, border=1px solid --border, border-radius=10px, padding=10px 12px
bot bubble:   bg=linear-gradient(180deg, rgba(124,92,255,0.08), transparent)
              border=1px solid rgba(124,92,255,0.25), border-radius=10px, padding=12px 14px
```

The bot bubble gradient is **hardcoded purple** (not `--accent`) to always distinguish AI from user.

Source chips:
```
--font-mono, 10.5px, padding=3px 8px, border-radius=999px (pill)
bg=--bg-2, border=1px solid --border, color=--text-mute
hover: bg=--bg-3, color=--text
```

### 4.11 Settings Panel

```
backdrop:   rgba(2,3,7,0.6), blur(4px), fixed, z-index=90
panel:      760px wide, max 86vh height
            bg=--bg-1, border=1px solid --border-strong, border-radius=14px
            grid: 200px sidebar | 1fr main
sidebar:    bg=--bg-2, border-right=1px solid --border, padding=14px 8px
            section header: --font-mono 10px uppercase
            button: active=bg:--bg-3 color:--text, default=color:--text-mute
main:       padding=20px 24px, overflow-y=auto
```

Model cards:
```
grid: auto | 1fr | auto, gap=14px, padding=12px 14px
bg=--bg-2, border=1px solid --border, border-radius=10px
selected: border=--accent, bg=--accent-soft
```

### 4.12 Form Fields (settings inputs)

```
label:      --font-mono, 10.5px, uppercase, letter-spacing=0.06em, color=--text-dim
input:      inherits from body (Inter 13px --text), bg=--bg-2, border=1px solid --border
            border-radius=2px (--radius), focus: border=--accent, shadow=0 0 0 3px --accent-soft
helper:     Inter, 11px, --text-mute, margin-top=6px
```

### 4.13 Frontmatter Chips (fm-chip)

```
--font-mono, 10.5px, padding=3px 8px, border-radius=999px (pill)
bg=--bg-2, border=1px solid --border, color=--text-mute
bold key:   color=--text, font-weight=600
```

### 4.14 Keyboard Shortcut (kbd)

```
--font-mono, 10px, height=18px, padding=1px 5px
border=1px solid --border-strong, border-radius=4px
bg=--bg-1, color=--text-mute
```

### 4.15 Toggle (boolean switch)

```
size:       28×16px, border-radius=999px
off:        bg=--bg-3, knob=--text-mute
on:         bg=--accent, knob=white
knob:       12×12px circle, transition=0.2s
```

### 4.16 Wikilink

```
color:      --accent
bg:         rgba(124,92,255,0.10)   ← hardcoded purple, not --accent-soft
padding:    1px 5px, border-radius=4px
border-bottom: 1px dashed rgba(124,92,255,0.35)
hover:      bg=rgba(124,92,255,0.22)
dead link:  color=#f87171, border-bottom=#f87171, bg=rgba(248,113,113,0.08)
```

---

## SECTION 5 — Interaction Patterns

### 5.1 Transitions

Genesis UI uses **fast, subtle transitions** — tool-feel, not animation-forward:

```css
transition: background 0.1s, color 0.1s;   /* list items, nav items */
transition: transform 0.2s;                  /* chevron rotation, toggle knob */
transition: 0.2s;                            /* toggle bg */
```

No `cubic-bezier` easing — plain linear or default ease. No scale animations on hover.
No translateY lift on hover. This is not a marketing component.

### 5.2 Focus States

All inputs use `box-shadow: 0 0 0 3px var(--accent-soft)` on focus — NOT outline.

```css
outline: none;
border-color: var(--accent);
box-shadow: 0 0 0 3px var(--accent-soft);
```

### 5.3 Spinner

```css
width: 12px; height: 12px;
border: 1.5px solid --border-strong;
border-top-color: --accent;
border-radius: 50%;
animation: spin 0.7s linear infinite;
```

### 5.4 Selection highlight

```css
::selection { background: rgba(124,92,255,0.35); }
```

### 5.5 Pulse animation (live indicator)

```css
@keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: .35; } }
animation: pulse 2s ease-in-out infinite;
```

Used on: status bar sync dot (green `--c-moc`), chrome pill.

### 5.6 Scrollbar (where applied)

```css
::-webkit-scrollbar { width: 8px; height: 8px; }
::-webkit-scrollbar-thumb { background: --border-strong; border-radius: 4px; }
::-webkit-scrollbar-track { background: transparent; }
```

---

## SECTION 6 — Canvas Views (Graph, Galaxy, Embedding)

All three canvas views share the same camera and rendering model.

### Camera model
```typescript
camTgt = useRef({ yaw: 0, pitch: 0.4, dist: 700 })  // spherical coords
```

Mouse drag → adjust `yaw` / `pitch`. Wheel → adjust `dist`.

### Perspective projection
```typescript
const scale = dist / (dist + z_projected);
const sx = cx + x_rotated * scale;
const sy = cy + y_projected * scale;
```

### RAF loop pattern (mandatory)
```typescript
const timeRef = useRef(0);  // NEVER let time = 0 inside effect (resets on rerun)
// in tick:
const now = performance.now();
const dt = Math.min((now - lastRef.current) / 1000, 0.05);
lastRef.current = now;
timeRef.current += dt;
```

### Cleanup (mandatory)
```typescript
useEffect(() => {
  let raf: number;
  const tick = () => { /* ... */; raf = requestAnimationFrame(tick); };
  raf = requestAnimationFrame(tick);
  return () => cancelAnimationFrame(raf);  // ← must cancel on unmount
}, [deps]);
```

### Node colors in canvas
Use `TYPE_META[n.type as keyof typeof TYPE_META]?.raw ?? '#888'` — never hardcode per-type hex in canvas draw code.

---

## SECTION 7 — Do's and Don'ts

### DO
1. **DO** use VS Code dark palette — `--bg-0` through `--bg-3` for layering.
2. **DO** use JetBrains Mono for all structural UI text (labels, counts, IDs, meta, status).
3. **DO** use `--radius: 2px` sharp corners everywhere except pills (`999px`) and explicit exceptions.
4. **DO** use `var(--accent)` and `var(--accent-soft)` for interactive states — never hardcode a theme color.
5. **DO** keep transitions fast (`0.1s–0.2s`, linear or default ease).
6. **DO** use `useRef` for animation time accumulation in canvas views.
7. **DO** cancel `requestAnimationFrame` in useEffect cleanup.
8. **DO** access node type colors via `TYPE_META[type]?.raw` — never use `--c-entity` etc. in canvas draw code directly.

### DON'T
1. **DON'T** use external branding tokens or soft curves (e.g. `cubic-bezier(0.25,0.8,0.25,1)`).
2. **DON'T** round corners beyond 8px on non-pill elements.
3. **DON'T** use soft/warm shadows — only `--shadow-pop: 0 4px 12px rgba(0,0,0,0.5)`.
4. **DON'T** apply backdrop-filter blur outside of graph overlays and modals.
5. **DON'T** scale elements on hover (`transform: scale(1.02)`) — flat tool aesthetic only.
6. **DON'T** use `let time = 0` inside a useEffect for canvas animation — use `useRef`.
7. **DON'T** hardcode `#7c5cff` or other theme accent hex in component logic — use `var(--accent)`.
8. **DON'T** import from `packages/gks` or `packages/msp` — read `gksData.json` via `GKS_SERVICE` only.

---

## SECTION 8 — Component Generation Prompt Templates

### 8.1 New panel / view
```
Create a [ViewName] component at packages/ui/src/components/views/[ViewName].tsx.

Theme: Genesis UI dark dev-tool theme.
CSS variables: use tokens from DESIGN-SYSTEM.md Section 1 (--bg-0 through --bg-3, --accent, etc.).
Font: structural labels → var(--font-mono); body content → var(--font-sans).
Corners: var(--radius) = 2px. Pills use border-radius: 999px.
Transitions: 0.1s background/color — no scale, no translateY.

Props: [LIST]
Rules:
- No external branding tokens
- No hardcoded hex except when explicitly mapping GKS type colors via TYPE_META
- Canvas views: use useRef for time, cancel RAF in cleanup
```

### 8.2 New sidebar section
```
Add a [SectionName] section to Sidebar.tsx.
Section header: --font-mono, 10.5px, uppercase, letter-spacing=0.08em, color=--text-dim.
Items: tree-item pattern — 4px padding, 4px border-radius, hover=--bg-hover, active=--bg-3+--accent.
```

### 8.3 New graph overlay (floating panel)
```
Add a [OverlayName] floating panel to [ViewName].tsx.
Style: position=absolute, bg=rgba(11,13,20,0.78), backdrop-filter=blur(10px),
       border=1px solid --border, border-radius=12px, padding=10px 12px.
This is one of the few places blur is allowed in Genesis UI.
```

---

*v1.0.0 — 2026-05-14. Source: packages/ui/src/index.css + App.tsx. Owner: Boss.*
