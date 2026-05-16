---
id: AUDIT--M7-PREP-FOLLOWUP
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M7-prep follow-up — GKS audit alignment + 4 upstream proposals
tags:
  - msp
  - m7
  - m7-prep
  - audit
  - followup
  - gks-audit
crosslinks: {"references":["ADR--GRAPH-IS-GKS-DOMAIN","ADR--EMBEDDING-MODEL-PARITY","FRAMEWORK--MSP-ARCHITECTURE-V2","AUDIT--MSP-ARCHITECTURE-V2","CONCEPT--MSP-ROADMAP","CONCEPT--EMBEDDING-STRATEGY","ADR--MSP-OBSIDIAN-INTEGRATION","ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS","ADR--ANTI-HALLUCINATION-RULES","CONCEPT--MEMORY-VECTOR-BACKLINKS"]}
linked_symbols: []
created_at: 2026-05-04T09:18:00.000+07:00
---

# AUDIT — M7-prep follow-up (GKS audit alignment)

## Scope

Doc-only PR. Performs alignment between MSP's M7-prep architecture (PR #8, merged) and GksV3 3.6.0's actual capabilities. Discovered that PR #8 contained dogmatic claims that no longer hold:

- "MSP never embeds" → false (GKS ships `createNomicEmbedder()`; MSP wraps it)
- M7a builds an Obsidian client → wasteful (GKS already has `RestObsidianAdapter`)
- Env var `OBSIDIAN_HOST` → mismatch (GKS uses `OBSIDIAN_URL`)
- Atomic graph traversal is MSP responsibility → false (GKS `SCOPE.md` claims it)

## Audit findings (6)

| # | Finding | Source |
|---|---|---|
| F1 | GKS 3.6.0 `createNomicEmbedder()` makes "MSP never embeds" obsolete | GksV3 CHANGELOG 3.6.0 |
| F2 | M7a should wrap GKS adapter (`src/memory/obsidian-mcp.ts`), not build fresh | GksV3 source |
| F3 | Env var rename `OBSIDIAN_HOST` → `OBSIDIAN_URL` for parity | GksV3 `.env.example` |
| F4 | Atomic graph (wikilinks/backlinks) is GKS scope per `SCOPE.md` "In scope → Graph" | GksV3 `SCOPE.md` |
| F5 | `src/memory/backlinks/` (M3c-1) duplicates GKS-owned domain — keep as temporary, plan upstream | GksV3 `MSP_RELATIONSHIP.md` |
| F6 | GKS + Smart Connections must use **same model** to avoid double-embed | user direction + practical reasoning |

## Atoms landed

| Atom | Phase | Type | Purpose |
|---|---|---|---|
| `[[ADR--GRAPH-IS-GKS-DOMAIN]]` | 2 | adr | Records F4 + F5 — atomic graph is GKS; MSP only does shift-left validation + type-specific opinions |
| `[[ADR--EMBEDDING-MODEL-PARITY]]` | 2 | adr | Records F1 + F6 — locks `nomic-embed-text-v1.5` as canonical for both GKS and Smart Connections |
| `[[AUDIT--M7-PREP-FOLLOWUP]]` | 6 | audit | This file |

## Atoms updated (in-place; not superseded)

| Atom | Change |
|---|---|
| `[[CONCEPT--EMBEDDING-STRATEGY]]` | Reframed to two paths: GKS canonical (agent) + Smart Connections (in-Obsidian browse) |
| `[[ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS]]` | Reframed (GKS canonical, not "MSP never embeds"); preserved original alternatives |
| `[[ADR--MSP-OBSIDIAN-INTEGRATION]]` | Env var `OBSIDIAN_HOST` → `OBSIDIAN_URL`; M7a wraps GKS adapter |
| `[[CONCEPT--MEMORY-VECTOR-BACKLINKS]]` | Marked as planned upstream to GKS; added migration table |
| `[[ADR--ANTI-HALLUCINATION-RULES]]` | Clarified `dangling-wikilinks` = shift-left of `gks validate --links` |
| `[[CONCEPT--MSP-ROADMAP]]` | M7a effort medium → small; M8d scope reduced; added 4 upstream proposals |

## Files added (non-atomic)

- `upstream/gks-proposals/README.md` — explains the directory + workflow
- `upstream/gks-proposals/01-phase-6-acceptance.md` — accept `phase: 6` in `gks propose-inbound`
- `upstream/gks-proposals/02-verify-flow-through-superseded.md` — `--through-superseded` flag for verify-flow
- `upstream/gks-proposals/03-backlinks-api.md` — stable `gks backlinks --emit=jsonl` API
- `upstream/gks-proposals/04-smart-connections-parity.md` — documentation: model parity for browse plugins

These are informational drafts; MSP cannot push to `Freshair129/GksV3` directly.

## Files edited (public surface)

- `msp_spec.md` — bumped 2.0.0 → 2.0.1; updated §7a (env var) + §7b (embedding strategy reframed) + §7c (retrieval source list); changelog entry added
- `ROADMAP.md` — M7-prep marked merged; M7-prep follow-up added; M7a wording updated

## Counts

- Atoms in `gks/`: 95 → 98 (+3: 2 ADRs + this audit)
- Atoms updated in place: 6
- Tests: 233 (unchanged — doc-only PR)
- New non-atomic docs: 5 (`upstream/gks-proposals/`)

## Risk assessment

**Low**. Doc-only changes:

- No source code changes
- No new validator rules
- No CI gate changes
- All updated atoms keep `status: stable` (no supersede chain — this is clarification, not architectural change)
- Env var rename is forward-looking (M7a not yet implemented; no consumers to break)

## Verification

- `npm run msp:index` — atomic_index.jsonl regenerated
- `npm run msp:check-links` — all crosslinks resolve
- `npm test` — 233 tests pass (no functional change)
- `git status` — only doc files + `gks/` atoms + new `upstream/` directory

## Why an in-place update vs supersede

The 5 updated atoms remain semantically valid — the original framing was a design intent that GksV3 3.6.0's CHANGELOG (post-dating this project's M7-prep planning) made more concrete. Marking each as superseded would create a 5-atom supersede chain for what is essentially a clarification PR. In-place updates with explicit "Updated 2026-05-04 (M7-prep follow-up)" notes preserve readability and audit trail without inflating the atom count.

## Follow-up work

| Item | When | Owner |
|---|---|---|
| Ship the 4 upstream proposals to GKS maintainer (issue / PR / DM) | post-merge | human |
| When upstream `phase: 6` lands → remove `scripts/msp/propose.mjs` Phase-6 hack | post-upstream | next M-milestone |
| When upstream backlinks API lands → replace `src/memory/backlinks/` with thin caller; supersede `[[FEAT--MEMORY-BACKLINKS-INDEXER]]` | post-upstream | post-M7 |
| When upstream verify-flow flag lands → opt-in via `.mspconfig.json` | post-upstream | M8 |
| Smart Connections setup guide for users | M7a | M7 milestone |

## Source

GksV3 3.6.0 audit performed during M7-prep follow-up planning. User architectural correction on graph ownership + embedding model parity. See `gks/adr/[[ADR--GRAPH-IS-GKS-DOMAIN]]` and `gks/adr/[[ADR--EMBEDDING-MODEL-PARITY]]`.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[AUDIT--MSP-ARCHITECTURE-V2]]

