---
id: AUDIT--MSP-ARCHITECTURE-V2
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M7-prep — architecture v2 supersede + Obsidian/Smart Connections decision atoms
tags:
  - msp
  - m7
  - m7-prep
  - audit
  - architecture
  - supersede
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--OBSIDIAN-AS-RUNTIME","CONCEPT--EMBEDDING-STRATEGY","ADR--MSP-OBSIDIAN-INTEGRATION","ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS"]}
linked_symbols: []
created_at: 2026-05-03T23:55:51.476+07:00
---

# AUDIT — M7-prep architecture v2

## Scope

Doc-only PR. No source code changes. Records the architectural shift discovered during M7 planning:

- **MSP** is a **passport-orchestrator** that travels with the agent (memory + soul + retrieval + identity).
- **GKS** is the canonical knowledge layer — atomic markdown + wikilinks/crosslinks/backlinks.
- **Obsidian** is the runtime that hosts GKS (file watching, search, graph, plugins, REST API).
- **Smart Connections** is the embedding source — local, GUI-resourced, plugin-managed. MSP never embeds.

## Atoms landed

| Atom | Phase | Type | Purpose |
|---|---|---|---|
| `[[CONCEPT--OBSIDIAN-AS-RUNTIME]]` | 1 | concept | What Obsidian provides for free; what MSP still owns |
| `[[CONCEPT--EMBEDDING-STRATEGY]]` | 1 | concept | Smart Connections is the embedder; MSP delegates |
| `[[ADR--MSP-OBSIDIAN-INTEGRATION]]` | 2 | adr | REST primary, file fallback, TLS scope, auth |
| `[[ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS]]` | 2 | adr | No MSP embedder ever; runtime dep on Obsidian for semantic |
| `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` | 0 | frame | Supersedes v1; new two-layer mental model |
| `[[AUDIT--MSP-ARCHITECTURE-V2]]` | 6 | audit | This file |

## V1 supersede

`[[FRAMEWORK--MSP-ARCHITECTURE]]` (v1):
- `status: stable` → `status: superseded`
- `crosslinks.superseded_by: [[[FRAMEWORK--MSP-ARCHITECTURE-V2]]]`
- Body preserved verbatim with a header note pointing at v2

V1 stays in `gks/frame/` for historical reference; readers see the supersede note immediately.

## Verification

```
npm run msp:index             → 94 indexed, 0 skipped, 0 duplicates
npm run msp:validate -- --all → Total: 94 passed, 0 failed
npx gks validate --links      → status: OK (94 atoms scanned)
```

## Why doc-only first

Per `[[CONCEPT--PROPOSAL-TYPES]]`, a `supersede` is a governance event that should be recorded before the implementation work it enables. Locking the v2 frame + ADRs in the atom graph **first** means M7 implementation PRs (consolidator, retrieval, etc.) can crosslink back to a stable, promoted authority.

## What this AUDIT does NOT do

- **No source code changes.** All `src/` modules unchanged.
- **No npm script changes.** No build / test / dependency changes.
- **No msp_spec.md changes.** Spec edit lands as a separate doc-PR after M7 ships (so we update spec in light of working code).
- **No M7 implementation.** That starts next branch (M7a — Obsidian client).

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: validator (94/94) + validate-links OK + manual review of v1↔v2 supersede chain
- Date: 2026-05-03
