---
id: ADR--ANTI-HALLUCINATION-RULES
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Anti-hallucination rules — six guards on atom writes
tags:
  - msp
  - validator
  - anti-hallucination
  - rules
crosslinks:
  references:
    - CONCEPT--ATOMIC-WRITE-CONTRACT
    - ADR--GRAPH-IS-GKS-DOMAIN
  implements:
    - FEAT--MSP-VALIDATOR
created_at: 2026-05-03T14:08:41.252+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — anti-hallucination rules

> **Updated 2026-05-04 (M7-prep follow-up)**: clarification on rule #1 (`dangling-wikilinks`) — this is a **shift-left** of `gks validate --links`, not a duplicate. MSP runs the check at inbound time (pre-promote); GKS runs the same conceptual check against the canonical store (post-promote). Both useful, different timing — see `[[ADR--GRAPH-IS-GKS-DOMAIN]]` for the boundary.

## Context

Forbidden fields stop one class of forgery. Six other patterns appear in agent output that are not caught by schema-shape checks alone:

1. Made-up wikilinks (`[[FEAT--ghost]]` that doesn't exist)
2. Invented ADR numbers (claiming `ADR-007` when 1..5 exist; the new one should be 6)
3. Invented version numbers (jumping from 0.1.0 to 1.5.2 without intermediate releases)
4. Decisions without evidence (ADR body has Decision but no Context or Consequences)
5. Code-path claims without citations (`src/foo.ts:42 does X` with no `linked_symbols`)
6. Future dates (`created_at: 2099-01-01`)

Each one is mechanically detectable and the validator must enforce them.

## Decision

Six rules, each as a small pure function returning `ValidationError[]`. Hard rules exit-1; soft rules exit-0 with a warning.

| # | Rule ID | Severity | What |
|---|---|---|---|
| 1 | `dangling-wikilinks` | error | every `[[X]]` in body and every value in `crosslinks.*` must resolve via `atomic_index.jsonl`; self-reference allowed. **Shift-left of `gks validate --links`** — MSP catches at inbound time, GKS catches drift in canonical store |
| 2 | `adr-monotonic` | error | new ADR-NNN must equal `max(existing) + 1` (or 1 if none); slug-form `[[ADR--NAME]]` skipped |
| 3 | `no-invented-versions` | error | semver only; first draft is `0.1.0`; bumps must be one of patch/minor/major from the previous version |
| 4 | `evidence-for-decisions` | error | `type: adr` requires headings `## Context`, `## Decision`, `## Consequences` (case-insensitive) |
| 5 | `cite-or-mark-inferred` | warning | body claims about code (file path, line, function name) require either a matching `linked_symbols` entry or `epistemic.source_type: inferred` with `confidence < 1.0` |
| 6 | `no-future-dates` | error | `created_at` ≤ `now()`; injectable clock for testing |

## Implementation status (M2)

| Rule | Status |
|---|---|
| `dangling-wikilinks` | ✅ implemented (`src/validator/rules/dangling-wikilinks.ts`) |
| `adr-monotonic` | ✅ implemented (`src/validator/rules/adr-monotonic.ts`) |
| `no-invented-versions` | ⏳ M3 — needs version history tracking |
| `evidence-for-decisions` | ⏳ M3 — needs body parser for headings |
| `cite-or-mark-inferred` | ⏳ M3 — needs `linked_symbols` cross-check |
| `no-future-dates` | ✅ implemented (`src/validator/rules/future-date.ts`) |

3 of 6 ship in M2; the rest are queued for M3 alongside the runtime-loaded contract.

## Consequences

**Positive**
- Each rule is independently unit-testable (pure function, injectable context).
- Severity per rule lets us soft-launch experimental rules without blocking PRs.
- Pattern-matched against `msp_spec.md` §4.5 verbatim — easy to audit.

**Negative**
- M2 ships 50% coverage (3 hard, 0 warning). Until M3, agents could still slip past with invented versions or evidence-free ADRs. Mitigated by human review (gate 3) catching the obvious cases.

## What this ADR does NOT decide

- Forbidden frontmatter fields → see `[[ADR--FORBIDDEN-FIELDS-LIST]]`.
- Field shape constraints (length, regex) → see future `[[ADR--FIELD-CONSTRAINTS]]` (M3).
- Phase-status compatibility table → see `[[FRAMEWORK--PHASE-GOVERNANCE]]`.
- Atomic graph traversal / canonical-store integrity → **GKS** (`gks validate --links`, `gks verify-flow`); see `[[ADR--GRAPH-IS-GKS-DOMAIN]]` for the layered ownership.

## Source

`msp_spec.md` §4.5 (Anti-Hallucination Rules).

## Connections
- [[CONCEPT--ATOMIC-WRITE-CONTRACT]]
- [[FEAT--MSP-VALIDATOR]]

