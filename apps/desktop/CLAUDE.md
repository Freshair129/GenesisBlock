# GKS Desktop — CLAUDE.md

# Created At: 2026-05-14 14:00:00 +07:00 (v0.1.0-skeleton)

## What This Is

GKS Desktop app — native desktop wrapper for the Genesis Knowledge System.

**Consumer: human developers / power users** who want a native desktop experience.

## Status

**PLACEHOLDER** — directory reserved. Not scaffolded yet.

## Planned Stack: Tauri

- **Rust** shell (Tauri v2)
- **Web frontend** — reuse `apps/web/` React UI via Tauri webview
- Single binary, no Electron overhead

## When to scaffold

When `apps/web/` is stable and there's a reason to ship a native binary
(offline mode, file system access beyond browser sandbox, system tray, notifications).

## Scaffold command (when ready)

```bash
npm create tauri-app@latest -- --template react-ts
# Then link to apps/web/ as the frontend source
```

## Key differences from `apps/web/`

- File system access: Tauri `fs` plugin (read gksData.json from local path, no sync script needed)
- System tray: background agent status indicator
- Auto-update: Tauri updater plugin
- IPC: Tauri commands replace the JSON snapshot pattern
