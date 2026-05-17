---
id: FEAT--MSP-OBSIDIAN-CLIENT
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: MSP Obsidian client wrapper — REST primary, filesystem fallback
tags:
  - msp
  - obsidian
  - client
  - m7a
  - user-facing
crosslinks: {"implements":["ADR--MSP-OBSIDIAN-INTEGRATION"],"references":["CONCEPT--OBSIDIAN-AS-RUNTIME","FRAMEWORK--MSP-ARCHITECTURE-V2"]}
linked_symbols:
  - {"file":"packages/msp/src/obsidian/client.ts"}
  - {"file":"packages/msp/src/obsidian/types.ts"}
  - {"file":"packages/msp/src/obsidian/filesystem.ts"}
created_at: 2026-05-04T12:24:39.673+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: "Feature spec"
---

# MSP Obsidian client wrapper — REST primary, filesystem fallback

## User-facing behaviour

```ts
import { createObsidianClient } from '@/obsidian/client'

const client = await createObsidianClient({ root: process.cwd() })
if (client.mode === 'rest') {
  const hits = await client.search('passport', { limit: 5 })
}
const note = await client.readFile('gks/frame/FRAMEWORK--MSP-ARCHITECTURE-V2.md')
const link = client.smartViewDeepLink?.('FRAMEWORK--MSP-ARCHITECTURE-V2')
```

## Acceptance criteria

- [ ] `createObsidianClient(opts)` returns `{ mode, search, readFile, activeFile?, smartViewDeepLink? }`
- [ ] mode='rest' when probe to OBSIDIAN_URL succeeds AND OBSIDIAN_API_KEY set
- [ ] mode='filesystem' when probe fails OR key missing
- [ ] REST path delegates to GksV3's createRestObsidianAdapter (no duplicate adapter)
- [ ] Filesystem path reads gks/<type>/*.md directly
- [ ] Deprecation warning emitted when OBSIDIAN_HOST set instead of OBSIDIAN_URL
- [ ] TLS bypass scoped to 127.0.0.1/localhost (and `OBSIDIAN_INSECURE=true` override)
- [ ] No new files in src/memory/backlinks (graph is GKS scope)
- [ ] No bundled embedder (Smart Connections owns embedding)

## Surfaces

| Surface | Form |
|---|---|
| TS API | `createObsidianClient(opts?: ClientOpts): Promise<ObsidianClient>` |
| Env | `OBSIDIAN_URL`, `OBSIDIAN_API_KEY`, `OBSIDIAN_INSECURE`, `OBSIDIAN_HOST` (deprecated) |

## Out of scope

- Smart Connections semantic-search bridge (M7c)
- Companion plugin msp-bridge (future)
- Any bundled embedder

## Connections
- [[ADR--MSP-OBSIDIAN-INTEGRATION]]
- [[CONCEPT--OBSIDIAN-AS-RUNTIME]]
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

