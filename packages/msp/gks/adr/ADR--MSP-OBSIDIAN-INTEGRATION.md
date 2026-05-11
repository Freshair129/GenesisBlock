---
id: ADR--MSP-OBSIDIAN-INTEGRATION
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: MSP↔Obsidian integration — REST primary, file fallback, plugin-aware
tags:
  - msp
  - obsidian
  - integration
  - decision
crosslinks: {"references":["CONCEPT--OBSIDIAN-AS-RUNTIME","CONCEPT--EMBEDDING-STRATEGY","ADR--GRAPH-IS-GKS-DOMAIN"]}
created_at: 2026-05-03T16:55:06.326Z
---

# ADR — MSP↔Obsidian integration

> **Updated 2026-05-04 (M7-prep follow-up)**: env var renamed `OBSIDIAN_HOST` → `OBSIDIAN_URL` to match GksV3 3.6.0's `RestObsidianAdapter` (in `src/memory/obsidian-mcp.ts`). M7a now wraps GKS's existing adapter rather than building a fresh client.

## Context

Per `CONCEPT--OBSIDIAN-AS-RUNTIME`, MSP delegates search/graph/file-watching to Obsidian. The integration shape needs to be specific enough to implement, with clear fallbacks for headless scenarios (CI, no-GUI, server boot).

GksV3 3.6.0 already ships a working Obsidian adapter (`RestObsidianAdapter` in `src/memory/obsidian-mcp.ts`) reading `OBSIDIAN_URL` from env. M7a's job is to wrap it (with fallback + probe), not reinvent it — see `ADR--GRAPH-IS-GKS-DOMAIN`.

Three modes Obsidian can be in for any given MSP invocation:

1. **Live + Local REST API plugin enabled** — best case; HTTPS endpoint at `https://127.0.0.1:27124` (default).
2. **Live but no REST plugin** — Obsidian is open but we cannot query it from outside.
3. **Not running** — only the vault's files exist on disk.

## Decision

### Primary path: Obsidian Local REST API (via GKS adapter)

If `OBSIDIAN_URL` (default `https://127.0.0.1:27124`) responds with a `/` GET returning the plugin's manifest signature, MSP uses it through GKS's `RestObsidianAdapter` for:

- **Vault search** (text + tags) via `/search/simple` (or equivalent endpoints depending on plugin version).
- **File read** by path.
- **Active-file pointer** (what the human is looking at — useful for context priming).
- **Optional: Smart Connections endpoint** if the plugin exposes one through REST.

### Fallback path: filesystem

If REST is unreachable, MSP falls back to:

- Reading atoms directly from `gks/<type>/*.md`.
- Using `gks/00_index/atomic_index.jsonl` for ID lookup.
- Using `.brain/.../vector/backlinks.jsonl` for crosslink traversal (M3c-1 builds this; planned to move upstream — see `ADR--GRAPH-IS-GKS-DOMAIN`).
- **Text search** = grep-on-disk OR delegating to gks-mcp-server's `gks_recall`.
- **Semantic search** = still works via GKS's local `createNomicEmbedder()` + vector store — Obsidian is **not** required for agent-facing recall (see `ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS` updated framing). Only Smart Connections's human-browse pane is unavailable.

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

M7a wraps GKS's `createObsidianAdapter()` in a thin `createObsidianClient(opts)` that adds a `mode` property — one of `'rest' | 'filesystem'`. Callers check `client.mode === 'rest'` before requesting Obsidian-only features (active-file pointer, Smart View deep links). Semantic recall does **not** depend on `mode`.

### Env vars

| Var | Default | Source |
|---|---|---|
| `OBSIDIAN_URL` | `https://127.0.0.1:27124` | matches GksV3 3.6.0 `RestObsidianAdapter` |
| `OBSIDIAN_API_KEY` | (none) | required for REST path |
| `OBSIDIAN_INSECURE` | `false` | accept self-signed TLS on 127.0.0.1 |

Old `OBSIDIAN_HOST` from M7-prep draft is **deprecated** — projects should rename. Adapter wrapper will read `OBSIDIAN_HOST` as a fallback for one minor release with a deprecation warning, then drop it.

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
