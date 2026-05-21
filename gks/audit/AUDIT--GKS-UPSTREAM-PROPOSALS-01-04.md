---
id: AUDIT--GKS-UPSTREAM-PROPOSALS-01-04
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: GKS upstream proposals 01–04 merged in v3.7.0
tags: &a1
  - msp
  - upstream
  - gks
  - phase-6
  - backlinks
  - verify-flow
  - embedder
created_at: 2026-05-11T07:00:00.000+07:00
crosslinks: &a2
  references:
    - AUDIT--GKS-UPSTREAM-PROPOSALS-FILED
aliases: &a3
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--GKS-UPSTREAM-PROPOSALS-01-04
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: GKS upstream proposals 01–04 merged in v3.7.0
  tags: *a1
  created_at: 2026-05-11T07:00:00.000+07:00
  crosslinks: *a2
  aliases: *a3
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--GKS-UPSTREAM-PROPOSALS-01-04
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: GKS upstream proposals 01–04 merged in v3.7.0
    tags: *a1
    created_at: 2026-05-11T07:00:00.000+07:00
    crosslinks: *a2
    aliases: *a3
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      id: AUDIT--GKS-UPSTREAM-PROPOSALS-01-04
      phase: 6
      type: audit
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: GKS upstream proposals 01–04 merged in v3.7.0
      tags: *a1
      created_at: 2026-05-11T07:00:00.000+07:00
      crosslinks: *a2
      aliases: *a3
      cluster: implementation_flow
      role: Test results / quality report
      attributes:
        domain: audit
      domain: audit
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: aws_secret
      leak_risk: high
      encryption_level: none
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: aws_secret
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: aws_secret
  leak_risk: high
  encryption_level: none
---

# AUDIT — GKS upstream proposals 01–04 merged (v3.7.0)

## Scope

Records the landing of all four pending upstream proposals filed against
`Freshair129/GksV3` in M7-prep. Corresponding changes are in
`packages/gks` at version 3.7.0 and in the standalone GksV3 repo at
commit `dd076db`.

## Proposals landed

| # | Topic | GksV3 issue | Status |
|---|---|---|---|
| 01 | Accept `phase: 6` in `gks propose-inbound` | GksV3#32 | ✅ merged |
| 02 | `gks verify-flow --through-superseded` flag | GksV3#31 | ✅ merged |
| 03 | Stable backlinks derivation API (`gks backlinks`) | GksV3#30 | ✅ merged |
| 04 | Smart Connections + nomic-embed-text-v1.5 docs | GksV3#29 | ✅ merged |

## Changes in GKS 3.7.0

### Proposal 01 — Phase 6 acceptance

- `Phase` type extended: `0 | 1 | 2 | 3 | 4 | 5 | 6`
- `Status` type extended with `'superseded'` (was already used by MSP atoms but not in the type)
- `validatePhase()` now allows phase 6
- MSP can now use `gks_propose_inbound` / `InboundQueue.propose()` for AUDIT-- atoms without workarounds

### Proposal 02 — `--through-superseded`

- New `VerifyFlowOptions.throughSuperseded: boolean` on the `verifyFlow()` function
- CLI flag: `gks verify-flow <ID> --through-superseded`
- Walker follows `crosslinks.superseded_by` transparently on superseded atoms
- Cycle guard prevents infinite loop
- Default off — existing CI pipelines unaffected
- MSP pre-push hook can now pass `--through-superseded` when projects use supersede chains (e.g. [[FRAMEWORK--MSP-ARCHITECTURE]] → V2)

### Proposal 03 — Backlinks derivation API

- New `deriveBacklinksFromEntries()` — pure derivation from an `AtomicEntry[]`
- New `emitBacklinksJsonl()` — write to JSONL with optional filtering
- CLI: `gks backlinks [--emit=jsonl|json] [--out=PATH] [--filter-type=...]`
- MCP tool: `gks_backlinks({ filter_types? })`
- Exported from `@freshair129/gks` public entry point
- MSP's `src/orchestrator/retrieval/sources/backlinks.ts` can now call `deriveBacklinksFromEntries` directly instead of maintaining its own derivation logic

### Proposal 04 — Embedder compatibility docs

- New `docs/embedder-compatibility.md` in GKS
- Covers Smart Connections model parity, re-embed workflow, and why GKS doesn't enforce model choice

## MSP-side follow-up items

- [ ] Simplify `src/orchestrator/retrieval/sources/backlinks.ts` to use `deriveBacklinksFromEntries` from GKS (removes ~150 LoC of duplicated crosslinks parsing)
- [ ] `msp_backlinks_rebuild` MCP tool can delegate to `gks_backlinks` for the derivation step
- [ ] Pre-push hook: consider adding `--through-superseded` to the `gks verify-flow` call (requires MSP config flag per project)
- [ ] Phase 6 workaround in `scripts/msp/propose.mjs` — **already removed** in Phase 3 of `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]`; `msp_candidate` MCP handles audit atoms directly

## GKS workspace sync note

`packages/gks` in this monorepo is the workspace copy of GKS. Changes from
the standalone GksV3 repo (`dd076db`) were applied here manually. Future
sync: update `packages/gks` from GksV3 releases when new upstream features land.

## Sign-off

- Implemented in GksV3: commit `dd076db` on branch `claude/build-gks-v3-W8a7V`
- Propagated to `packages/gks`: this PR
- Date: 2026-05-11

## Connections

- [[AUDIT--GKS-UPSTREAM-PROPOSALS-FILED]]
