---
id: AUDIT--INBOUND-TO-CANDIDATES-MIGRATION-COMPLETE
phase: 6
type: audit
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — inbound→candidates migration complete (4 phases shipped, atom
  supersession recorded)
tags:
  - msp
  - inbound
  - candidates
  - migration
  - audit
  - supersession
crosslinks:
  references:
    - BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION
    - CONCEPT--KNOWLEDGE-LAYERS-V2
    - CONCEPT--INBOUND-QUEUE
    - ADR--AGENT-WRITE-BOUNDARIES
    - ADR--PROMOTION-WORKFLOW
    - ADR--PROMOTION-LEVELS
    - FRAMEWORK--KNOWLEDGE-3-TIER
created_at: 2026-05-09T14:45:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  domain: audit
---

# AUDIT — inbound→candidates migration complete

## Summary

`[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]` shipped in 4 phases over PRs #46, #49, #50, and this PR. The inbound queue is removed from runtime + canonical docs; atoms reflect the new model.

## Phases shipped

| Phase | PR | Outcome |
|---|---|---|
| 1 — additive | #46 | `msp_candidate` MCP tool + `CandidateWriter` + Knowledge Browser candidates tab. Writes to `.brain/.../candidates/`. |
| 2 — deprecate | #49 | `msp_propose` description prefixed `[deprecated]`; handler delegates to `CandidateWriter` (writes to `candidates/`, not `inbound/`). CLI wrapper kept for one-cycle back-compat. |
| 3 — delete | #50 | `msp_propose` MCP tool + `scripts/msp/propose.mjs` + `test/scripts/propose.test.ts` deleted. Validator `--all` walks `gks/` only; web UI's Inbound surface removed (`/api/inbound`, `getInbound()`, `inboundCount`). `package.json` drops `msp:propose`/`msp:list`/`msp:promote`. [[PROTO--AUTHORITY-ENFORCEMENT]] predicate renamed substring `inbound` → `candidates`. |
| 4 — atom supersession + 3-tier intro | this PR | atoms below flipped; new FRAME atom introduced. |

## Atom supersession (this PR)

| Atom | Before | After | Rationale |
|---|---|---|---|
| `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]` | `status: draft` | `status: stable` + `tier: genesis` + `source_type: axiomatic` | Promoted; supersedes the inbound concept. |
| `[[CONCEPT--INBOUND-QUEUE]]` | `status: stable` | `status: superseded` + `superseded_by: [[[CONCEPT--KNOWLEDGE-LAYERS-V2]]]` + body banner | The 4-layer model replaces the queue. |
| `[[ADR--AGENT-WRITE-BOUNDARIES]]` | `status: draft` | `status: stable` + `tier: genesis` + `source_type: axiomatic` + `supersedes: [[[ADR--PROMOTION-WORKFLOW]], [[ADR--PROMOTION-LEVELS]]]` | Promoted; supersedes both old promotion ADRs. |
| `[[ADR--PROMOTION-WORKFLOW]]` | `status: stable` | `status: superseded` + `superseded_by: [[[ADR--AGENT-WRITE-BOUNDARIES]]]` + body banner | Three-gate model retired. |
| `[[ADR--PROMOTION-LEVELS]]` | `status: stable` | `status: superseded` + `superseded_by: [[[ADR--AGENT-WRITE-BOUNDARIES]]]` + body banner | L0/L1/L2 model replaced by candidate→stable two-state model. |
| `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]` | `status: draft` | `status: stable` + `tier: process` + `source_type: axiomatic` | Promoted; the migration it specifies is now complete. |

## 3-tier model introduced (this PR — Stream B)

- New atom: `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` (`status: draft`, `tier: genesis`, `source_type: axiomatic`) — defines Safety / Master / Genesis classification + provenance schema.
- Six new permitted optional frontmatter fields recognised across the codebase: `tier`, `source_type`, `learned_from`, `promoted_from`, `promoted_at`, `promotion_adr`. None are forbidden; none are required for existing types.
- The 6 atoms touched in Stream A are tagged with `tier:` + `source_type:` while we're editing them anyway. Bulk-tagging the remaining ~175 atoms is deferred to PR-4 of the rollout plan.

## What did NOT change

- `[[FRAMEWORK--MSP-ARCHITECTURE-V2]].md` — no inbound references found; left untouched.
- All `AUDIT--*` atoms — historical record, immutable; their body mentions of inbound remain accurate as historical context.
- `[[SPEC--ARCHITECTURE-V2]].md` — left as draft per the parent rollout plan.
- `CLAUDE.md` — already updated in PR-2 (Phase 3); no further edits in this PR.

## Verification

- `npx tsx src/validator/cli.ts --all` exit 0 (PROTO failures pre-existing)
- `npm run msp:check-links` OK; all crosslinks (including new `superseded_by` pointers) resolve
- `npm run msp:index` regenerates `atomic_index.jsonl` with new statuses
- `grep -rn "[[CONCEPT--INBOUND-QUEUE]]" gks/ src/ test/` → only crosslinks of superseding atoms + the atom's own self-reference; no live references in code paths
- `npm test` — green; tool count (11) unchanged

## What's next

| Phase | What |
|---|---|
| PR-4 | Bulk-tag remaining ~175 atoms with `tier:` + `source_type: axiomatic`. New validator rules `tier-enum` (warn) + `master-requires-promotion` (error). |
| PR-5 | First two Master promotions: `[[MASTER--MSP-DOC-TO-CODE]]` + `[[MASTER--ATOM-CONTRADICTION-POLICY]]` via `[[ADR--MASTER-PROMOTION]]-*` evidence ADRs. New PROTOs: `[[PROTO--MASTER-BODY-SCHEMA]]` + `[[PROTO--MASTER-TOKEN-CAP]]`. |
| PR-6 | CLI loader `npm run msp:master compose <ID1> <ID2> ...` + tests + `CORE_FRAMEWORK_MASTER_SPEC.md` §3.6 Master Block section. |

## Source

- `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]` — implementation plan executed
- PRs #46, #49, #50, and this PR — phase-by-phase shipping
- `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` — knowledge-class model introduced concurrently
- 2026-05-09 session — user-driven 3-tier model design
