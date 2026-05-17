---
id: ADR--REVERSE-CITATION-LOOKUP
phase: 2
type: adr
status: stable
created_at: 2026-05-13T12:00:00+07:00
vault_id: GKS-CORE
tier: genesis
title: Bidirectional traceability via reverse citation lookup
tags: &a1
  - traceability
  - drift-detection
  - doc-to-code
  - lookup
crosslinks: &a2
  references:
    - CONCEPT--MEMORY-STORE
    - FEAT--LOOKUP-BY-SYMBOL
  superseded_by: []
  resolves: []
linked_symbols: &a3
  - file: packages/gks/src/memory/gks.ts
    fn: searchBySymbol
  - file: packages/gks/src/memory/index.ts
    fn: lookupBySymbol
aliases: &a4
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--REVERSE-CITATION-LOOKUP
  phase: 2
  type: adr
  status: stable
  created_at: 2026-05-13T12:00:00+07:00
  vault_id: GKS-CORE
  tier: genesis
  title: Bidirectional traceability via reverse citation lookup
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  aliases: *a4
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--REVERSE-CITATION-LOOKUP
    phase: 2
    type: adr
    status: stable
    created_at: 2026-05-13T12:00:00+07:00
    vault_id: GKS-CORE
    tier: genesis
    title: Bidirectional traceability via reverse citation lookup
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    aliases: *a4
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      domain: adr
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# ADR — Reverse citation lookup

## Context

`linked_symbols` (3.5.1) and `geography` (P3 blueprint field) let atoms
cite specific code symbols. This is **forward** traceability — given an
atom, find the code it governs.

The gap: no **reverse** traceability. When a developer edits
`src/x.ts:foo`, GitNexus can answer "which code is affected" but
nothing answers "which atoms govern this code?". The result was the
exact failure mode `BLUEPRINT--memory §write_rules` forbids: code
drifts ahead of doc, the SSOT becomes a lie.

## Decision

Add `MemoryStore.lookupBySymbol(symbolPath)` returning every atom whose
`linked_symbols` or `geography` cites the path. Match semantics:
file-only query matches any fn in that file; atom missing fn covers
the whole file; line-level only enforced when both sides specify.

Surfaces:
- TS API: `lookupBySymbol`
- CLI: `gks lookup-by-symbol src/x.ts:foo[:line]`
- MCP: `gks_lookup_by_symbol`
- Audit: new `lookup_by_symbol` op

The atomic index loader extends `AtomicEntry` with optional
`linked_symbols` and `geography` so reverse queries don't re-parse
markdown frontmatter.

## Consequences

**Positive** — closes the bidirectional drift loop. Combined with
GitNexus's `detect_changes`, an MSP pre-commit hook can catch "this
code has citations from N atoms; review them before push."

**Negative** — linear scan over the index (O(N) where N = atoms).
Acceptable up to ~10k atoms; an inverted index would help beyond that
but is deferred until measured.

## Alternatives considered

1. Maintain inverse index in MSP — wastes work; GKS already has the
   data and the index abstraction.
2. Push reverse lookup into GitNexus — breaks ADR-009.
3. Defer until pre-commit demands it — the primitive is small (~80
   LOC); adding it now closes the architecture.

## References

- ADR 008 — query primitive scope (in scope)
- ADR 009 — peer subsystem boundary (orchestrator combines)
- 3.5.2 release — shipped this primitive

## Connections
- [[CONCEPT--MEMORY-STORE]]
- [[FEAT--LOOKUP-BY-SYMBOL]]

