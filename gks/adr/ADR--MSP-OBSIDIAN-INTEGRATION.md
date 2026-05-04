---
id: ADR--MSP-OBSIDIAN-INTEGRATION
phase: 2
type: adr
status: stable
vault_id: default
title: MSP↔Obsidian integration — REST primary, file fallback, plugin-aware
tags:
  - msp
  - obsidian
  - integration
  - decision
crosslinks: {"references":["CONCEPT--OBSIDIAN-AS-RUNTIME","CONCEPT--EMBEDDING-STRATEGY"]}
created_at: 2026-05-03T16:55:06.326Z
---

# ADR — MSP↔Obsidian integration

## Context

Per `CONCEPT--OBSIDIAN-AS-RUNTIME`, MSP delegates search/graph/file-watching to Obsidian. The integration shape needs to be specific enough to implement, with clear fallbacks for headless scenarios (CI, no-GUI, server boot).

Three modes Obsidian can be in for any given MSP invocation:

1. **Live + Local REST API plugin enabled** — best case; HTTPS endpoint at `https://127.0.0.1:27124` (default).
2. **Live but no REST plugin** — Obsidian is open but we cannot query it from outside.
3. **Not running** — only the vault's files exist on disk.

## Decision

### Primary path: Obsidian Local REST API

If `OBSIDIAN_HOST` (default `https://127.0.0.1:27124`) responds with a `/` GET returning the plugin's manifest signature, MSP uses it for:

- **Vault search** (text + tags) via `/search/simple` (or equivalent endpoints depending on plugin version).
- **File read** by path.
- **Active-file pointer** (what the human is looking at — useful for context priming).
- **Optional: Smart Connections endpoint** if the plugin exposes one through REST.

### Fallback path: filesystem

If REST is unreachable, MSP falls back to:

- Reading atoms directly from `gks/<type>/*.md`.
- Using `gks/00_index/atomic_index.jsonl` for ID lookup.
- Using `.brain/.../vector/backlinks.jsonl` for crosslink traversal (M3c-1 already builds this).
- **Text search** = grep-on-disk OR delegating to gks-mcp-server's `gks_recall`.
- **Semantic search** = unavailable; MSP returns a clear "semantic recall requires Obsidian + Smart Connections" message in the result envelope.

### Authentication

The Obsidian Local REST API plugin issues an API key on enable. MSP reads it from:

```
OBSIDIAN_API_KEY     env var
~/.config/msp/obsidian.key   if env empty
```

If neither is set, MSP skips the REST attempt and goes to filesystem fallback (no error spam).

### TLS

The plugin uses self-signed HTTPS by default. MSP's HTTP client must accept this **only for `127.0.0.1` / `localhost`** (no remote-host TLS bypass). Configurable via `OBSIDIAN_INSECURE=true` for local-dev override.

### Detection

`createObsidianClient(opts)` returns a client object whose `mode` property is one of `'rest' | 'filesystem'`. Callers check `client.mode === 'rest'` before requesting semantic features.

## Consequences

**Positive**
- One client interface for both modes; callers pick capability based on `mode`.
- No surprise crashes — semantic recall fails gracefully when Obsidian is offline.
- Local-only TLS posture; no accidental remote calls.

**Negative**
- Two code paths for the same logical operation (`searchText` via REST vs grep). Mitigated by hiding behind the client interface.
- Smart Connections's REST endpoint shape (if exposed) is plugin-version-specific. Wrap in a probe + version check.

## Alternatives considered

1. **Require Obsidian always running.** Rejected — cuts out CI / headless / first-time-setup scenarios.
2. **Build our own search service.** Rejected — duplicates Obsidian; defeats `CONCEPT--OBSIDIAN-AS-RUNTIME`.
3. **Use only filesystem; ignore REST.** Rejected — loses live invalidation, user's open-file context, and the easy semantic-search bridge.

## Source

`CONCEPT--OBSIDIAN-AS-RUNTIME` + Obsidian Local REST API plugin docs.
