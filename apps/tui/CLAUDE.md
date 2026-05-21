# GKS TUI — CLAUDE.md

# Created At: 2026-05-14 14:00:00 +07:00 (v0.1.0-skeleton)

## What This Is

`@freshair129/gks-tui` — terminal UI for the Genesis Knowledge System.
Interactive knowledge graph explorer that runs in any terminal.

**Consumer: human developers** who prefer terminal-first workflows.

## Status

**SKELETON** — entry point exists, no UI implemented yet.
Next step: scaffold Ink app with a basic atom browser.

## Planned Views

```
┌─ GKS ──────────────────────────────────────────┐
│ [Search]  [Browse]  [Graph]  [Validate]         │  ← tab nav
├──────────────────────┬─────────────────────────┤
│ CONCEPT-- (12)       │ CONCEPT--TAXONOMY-V2-3  │
│ FEAT--    (21)       │ ─────────────────────── │
│ ADR--     (19)  ←   │ title: Atom Taxonomy     │
│ BLUEPRINT-(8)   tree │ tags: #taxonomy #v2.3   │
│ AUDIT--   (7)        │                         │
└──────────────────────┴─────────────────────────┘
```

## Stack

- TypeScript + Node.js (ESM)
- `ink` — React for CLIs (renders React components to terminal)
- `react` 18 (Ink requires React 18, not 19)
- Imports `@freshair129/gks` directly

## Dev

```bash
npm run dev       # run via tsx (live reload)
npm run build     # tsc → dist/
npm run typecheck
```

## Key Ink Patterns

```tsx
import { render, Box, Text, useInput } from 'ink';

// Entry point
render(<App />);

// Keyboard navigation
useInput((input, key) => {
  if (key.upArrow) /* move selection up */;
  if (key.return) /* open selected atom */;
  if (input === '/') /* focus search */;
});
```
