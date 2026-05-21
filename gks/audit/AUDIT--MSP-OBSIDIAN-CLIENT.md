---
id: AUDIT--MSP-OBSIDIAN-CLIENT
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M7a — Obsidian client wrapper (REST adapter delegate + filesystem fallback)
tags: &a1
  - msp
  - obsidian
  - m7a
  - audit
crosslinks: &a2
  references:
    - FEAT--MSP-OBSIDIAN-CLIENT
    - BLUEPRINT--MSP-OBSIDIAN-CLIENT
    - ADR--MSP-OBSIDIAN-INTEGRATION
    - FRAMEWORK--MSP-ARCHITECTURE-V2
linked_symbols: &a3
  - file: packages/msp/src/obsidian/client.ts
  - file: packages/msp/src/obsidian/rest.ts
  - file: packages/msp/src/obsidian/filesystem.ts
  - file: packages/msp/src/obsidian/env.ts
  - file: packages/msp/src/obsidian/types.ts
  - file: packages/msp/test/obsidian/client.test.ts
created_at: 2026-05-04T12:29:16.095+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--MSP-OBSIDIAN-CLIENT
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: M7a — Obsidian client wrapper (REST adapter delegate + filesystem fallback)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-04T12:29:16.095+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--MSP-OBSIDIAN-CLIENT
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: M7a — Obsidian client wrapper (REST adapter delegate + filesystem fallback)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-04T12:29:16.095+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      id: AUDIT--MSP-OBSIDIAN-CLIENT
      phase: 6
      type: audit
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: M7a — Obsidian client wrapper (REST adapter delegate + filesystem
        fallback)
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-04T12:29:16.095+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Test results / quality report
      attributes:
        domain: audit
      domain: audit
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# M7a — Obsidian client wrapper (REST adapter delegate + filesystem fallback)

## Scope

M7a deliverable: `createObsidianClient(opts)` factory that wraps GksV3's REST adapter and adds a filesystem fallback for headless / no-Obsidian scenarios. Implements `[[FEAT--MSP-OBSIDIAN-CLIENT]]` per `[[BLUEPRINT--MSP-OBSIDIAN-CLIENT]]`.

## What shipped

| File | Purpose |
|---|---|
| `src/obsidian/types.ts` | `ObsidianClient`, `ClientOpts`, `SearchHit` |
| `src/obsidian/env.ts` | `OBSIDIAN_URL` resolution, `OBSIDIAN_HOST` deprecation, `isLoopback` |
| `src/obsidian/rest.ts` | `makeRestClient` (delegates to GKS `createRestObsidianAdapter`), `probe`, `smartViewDeepLink` |
| `src/obsidian/filesystem.ts` | `makeFilesystemClient` — reads `gks/<type>/*.md` directly |
| `src/obsidian/client.ts` | `createObsidianClient` factory; mode selection |
| `test/obsidian/client.test.ts` | 15 tests covering both modes + deprecation + helpers |

## Boundaries respected

- **No new files in `src/memory/backlinks/`** — graph traversal is GKS scope.
- **No bundled embedder** — semantic search delegated to Smart Connections (M7c).
- **REST adapter not duplicated** — `makeRestClient` imports `createRestObsidianAdapter` from `@freshair129/gks/memory`.

## Atoms landed

| Atom | Phase | Type |
|---|---|---|
| `[[FEAT--MSP-OBSIDIAN-CLIENT]]` | 2 | feat |
| `[[BLUEPRINT--MSP-OBSIDIAN-CLIENT]]` | 3 | blueprint |
| `[[AUDIT--MSP-OBSIDIAN-CLIENT]]` | 6 | audit |

## Verification

```
npm test                            → 248 passed, 0 failed (was 233)
npx tsx src/validator/cli.ts --all  → 98 passed, 0 failed
npm run msp:check-links             → OK (98 atoms scanned)
npm run typecheck                   → clean
```

## Test coverage (15 new)

- filesystem mode: no `OBSIDIAN_URL` → fallback
- filesystem mode: missing `OBSIDIAN_API_KEY` → fallback even with URL set
- filesystem mode: substring search returns ranked hits
- filesystem mode: empty query returns `[]`
- filesystem mode: `readFile` reads gks/ paths
- rest mode: probe success → mode='rest', `smartViewDeepLink` + `activeFile` defined
- rest mode: probe network failure → fallback
- rest mode: probe 5xx → fallback
- rest probe treats 401 as healthy (server present, just unauthorized)
- deprecation: `OBSIDIAN_HOST` set without `OBSIDIAN_URL` → one-shot warning
- deprecation: both set → no warn (URL takes precedence)
- `resolveEnv` picks up `OBSIDIAN_HOST` as URL when only HOST is set
- `smartViewDeepLink` builds `obsidian://` URI with atom id
- `isLoopback` recognises 127.0.0.1, localhost, ::1; rejects external + bad-url
- filesystem client reports correct mode + lacks deep-link helper

## Out of scope (deferred)

- M7b — Consolidator
- M7c — Retrieval orchestration (will fuse this client with episodic + Smart Connections)
- M7d — Context compression
- M7e — Identity / soul
- Companion plugin `msp-bridge` (future)

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 248/248 tests + validator (98/98) + check-links OK + typecheck clean
- Branch: `claude/msp-m7a-obsidian-client-C4dBJ`

## Connections

- [[ADR--MSP-OBSIDIAN-INTEGRATION]]
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
