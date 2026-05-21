# GKS iOS — CLAUDE.md

# Created At: 2026-05-14 14:00:00 +07:00 (v0.1.0-skeleton)

## What This Is

GKS iOS app — mobile knowledge graph explorer for iPhone/iPad.

**Consumer: human users** on mobile.

## Status

**PLACEHOLDER** — directory reserved. Not scaffolded yet.

## Planned Stack

- **Swift + SwiftUI** (native) — preferred for performance and platform integration
- Communicates with GKS via `server/api/` REST endpoint (GKS runs on host/cloud, not on device)

## Data access pattern

```
iPhone app  →  HTTP/HTTPS  →  server/api/  →  @freshair129/gks
```

iOS cannot run Node.js — must go through API server.

## When to scaffold

When `server/api/` exists and has stable endpoints.
iOS development requires macOS + Xcode — not buildable on Windows.
