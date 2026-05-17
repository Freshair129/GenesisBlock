---
id: ADR--GRAPH-IS-GKS-DOMAIN
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Atomic graph traversal is GKS domain; MSP only does shift-left +
  type-specific opinions
tags: &a1
  - msp
  - gks
  - scope
  - decision
  - governance
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - FRAMEWORK--CROSSLINKS-VOCABULARY
    - ADR--ANTI-HALLUCINATION-RULES
    - CONCEPT--GENESIS-GRAPH-BACKEND
created_at: 2026-05-04T09:02:48.270+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--GRAPH-IS-GKS-DOMAIN
  phase: 2
  type: adr
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Atomic graph traversal is GKS domain; MSP only does shift-left +
    type-specific opinions
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-04T09:02:48.270+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--GRAPH-IS-GKS-DOMAIN
    phase: 2
    type: adr
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Atomic graph traversal is GKS domain; MSP only does shift-left +
      type-specific opinions
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-04T09:02:48.270+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      domain: adr
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — atomic graph is GKS domain

## Context

GksV3 `SCOPE.md` ("In scope" → "Storage layers" → "Graph") declares atomic relationship traversal (note ↔ note backlinks) as GKS scope. GKS already ships:

- `gks/00_index/atomic_index.jsonl` — derived index of every atom's frontmatter (incl. `crosslinks`)
- `gks validate --links` — checks every crosslink value resolves to an indexed atom
- `gks verify-flow` — walks the chain from a FEAT through `crosslinks.*`
- `ObsidianAdapter.resolveWikilink` / `backlinksOf` — runtime wikilink + body-link resolution via Obsidian Local REST API

During M3c-1 we built `src/memory/backlinks/` (and `npm run msp:backlinks`) inside MSP — duplicating part of GKS's domain because GKS hadn't exposed a stable backlinks API yet at the time. Per `MSP_RELATIONSHIP.md`, this is something MSP "should not do".

This ADR records the boundary so we don't drift further and so future reviewers know the MSP-side backlinks indexer is **temporary**, not authoritative.

## Decision

### Boundary (refined from `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`)

| Concern | Owner |
|---|---|
| Atomic graph storage (`gks/<type>/*.md`, `atomic_index.jsonl`) | **GKS** |
| Crosslink existence check (does `[[X]]` / `crosslinks.references: [X]` resolve?) | **GKS** (`gks validate --links`) |
| Chain traversal (FEAT → ADR → CONCEPT) | **GKS** (`gks verify-flow`) |
| Body wikilink resolution | **GKS** (Obsidian adapter) |
| Backlinks reverse-index derivation | **GKS** (logically — currently in MSP `src/memory/backlinks/` as M3c-1; planned upstream) |
| Pre-promote shift-left validation | **MSP** (`src/validator/rules/dangling-wikilinks.ts` runs at inbound time, before atom hits the store) |
| Type-specific semantic relationship rules | **MSP** (e.g. ALGO ↔ PARAM reciprocal coupling, ADR-monotonic, evidence-for-decisions) |
| Anti-hallucination rules over crosslink content | **MSP** (per `[[ADR--ANTI-HALLUCINATION-RULES]]`) |

### Why MSP keeps the dangling-wikilinks rule even though GKS has `gks validate --links`

**Different timing**:

- `gks validate --links` runs against the **canonical store** (after promote). Catches drift after it lands.
- MSP's `dangling-wikilinks` rule runs at **inbound time** (before promote). Blocks broken atoms from entering the store.

This is "shift left" — same check, earlier point. Both useful. **Not duplicate logic — duplicate-by-intent**.

### Why MSP keeps `src/memory/backlinks/` as temporary

GKS could expose `gks backlinks` as a CLI command + corresponding TS API (proposed upstream patch in `upstream/gks-proposals/`). Until then:

- MSP's `npm run msp:backlinks` derives `.brain/.../vector/backlinks.jsonl`
- CI uses it for drift detection
- M7c retrieval orchestration consumes it for graph-hop expansion

When GKS ships native backlinks API, MSP code becomes a thin caller; the indexer logic moves upstream. The atom (`[[FEAT--MEMORY-BACKLINKS-INDEXER]]`) gets `superseded_by: GKS-native` and the source migrates.

## Consequences

**Positive**
- Clear boundary: any future graph feature (e.g. `crosslinks.partially_supersedes`, multi-hop queries, transitive closure) goes upstream to GKS, not into MSP.
- Reduces MSP scope; smaller surface, easier maintenance.
- Aligns with `SCOPE.md` decision rule (1) "manipulates stored data, indexes, or queries → GKS".

**Negative**
- M3c-1 work technically lives in the wrong layer until upstream lands. Trade-off accepted: shipping faster vs perfect layering.
- Some upstream PRs we may need (backlinks API, phase 6, verify-flow through superseded) are out of MSP's direct push access — proposed as drafts under `upstream/gks-proposals/` for manual application by the GKS maintainer.

## Alternatives considered

1. **Move `src/memory/backlinks/` into GKS upstream right now (this PR).** Rejected — MSP's PR can't push to `Freshair129/GksV3`. We draft the patch + leave the temporary code in place until upstream lands.

2. **Delete MSP's dangling-wikilinks rule; rely solely on `gks validate --links`.** Rejected — gives up shift-left semantics. CI would only catch drift after promote, and pre-commit wouldn't block bad atoms.

3. **Build a "graph layer" module in MSP that wraps GKS calls.** Rejected — adds an indirection without value. Callers can use `gks` CLI / MCP directly.

## What this ADR does NOT change

- Existing M3c-1 implementation continues running.
- Existing `gks/concept/[[CONCEPT--MEMORY-VECTOR-BACKLINKS]]` content stays valid; gets a "planned upstream" note.
- GKS upstream proposals live under `upstream/gks-proposals/` — not enforced by MSP CI; informational.

## Source

GksV3 `SCOPE.md` "Graph" + `MSP_RELATIONSHIP.md` compatibility checklist + audit performed during M7-prep follow-up.

## Connections
- [[FRAMEWORK--CROSSLINKS-VOCABULARY]]
- [[CONCEPT--GENESIS-GRAPH-BACKEND]]

