---
id: CONCEPT--PROTO-PATTERN
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: PROTO-- atoms — turn FRAME-- governance docs into machine-checkable contracts
tags:
  - msp
  - proto
  - governance
  - protocol
  - contract
  - m8a
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","FRAMEWORK--PHASE-GOVERNANCE","ADR--HUMAN-REVIEW-GATES","FEAT--MSP-VALIDATOR"]}
created_at: 2026-05-05T16:18:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — PROTO atoms

## Problem

`FRAME--*` atoms describe governance ("authority levels", "phase order", "scaling levels", "crosslink vocabulary"). They're **descriptive** — humans read them, agents reference them, but no code automatically checks compliance.

Example: `[[FRAMEWORK--PHASE-GOVERNANCE]]` says "P3 BLUEPRINT must precede P5 CODE". A new PR landing code-without-blueprint is technically a violation, but nothing fails at CI today — only a human reviewer might catch it.

The result: governance docs decay into folklore. A new contributor can land a PR that violates the FRAME and nobody notices for months.

## What PROTO does

`PROTO--<NAME>` is a new atom type that turns a governance rule into:

1. **A schema** — what shape the rule expects to find (frontmatter fields, file paths, crosslinks, …)
2. **A predicate** — a pure function that returns `OK | violations` for a given input
3. **A trigger** — when CI / validator / pre-commit invokes the predicate

Each PROTO atom links back to the FRAME it enforces. The FRAME describes the rule in prose; the PROTO encodes it.

## Atom shape

```
PROTO--<RULE-NAME>
  frontmatter:
    id: PROTO--<RULE-NAME>
    phase: 2                      (decision-level — between FRAME and FEAT)
    type: proto
    status: stable | draft | superseded
    crosslinks:
      enforces: [FRAME--<which-frame-rule>]   ← required
      references: [...]
  body:
    ## Rule
    Plain-language statement of the rule.

    ## Schema
    What input shape the predicate operates on (atoms? PR diff? git blame? frontmatter?).

    ## Predicate
    Pseudocode or TS sketch of the check.

    ## Trigger
    Where the predicate runs:
      - pre-commit hook
      - validator (msp:validate)
      - CI workflow
      - on-demand (manual gks tool call)

    ## Severity
    error | warning | info

    ## Implementation
    Path to the actual TS implementation in src/validator/proto/<id>.ts (or similar).
```

## Why a new atom type

Existing types don't fit:

- **`ADR--`** is a one-shot decision record; PROTOs are living rules with implementations
- **`FEAT--`** is user-facing functionality; PROTOs are internal governance enforcement
- **`FRAME--`** is descriptive; PROTOs are machine-readable contracts

A separate type lets the validator distinguish them and apply specific rules (e.g. "every PROTO must have an `enforces:` crosslink to a FRAME").

## Relationship to existing atomic taxonomy

| Existing | What | PROTO relation |
|---|---|---|
| FRAME | Governance prose | PROTO `enforces:` it |
| ADR | One-shot decision | PROTO may cite related ADRs |
| FEAT | User-facing feature | PROTOs run alongside FEATs but don't replace them |
| BLUEPRINT | Impl plan for a FEAT | PROTO has its own embedded plan in `## Implementation` |
| AUDIT | What shipped | PROTO emits AUDIT entries when rules trigger |
| ALGO/PARAM | Tunable algo + its params | A PROTO MAY tune via PARAM--; out of M8a scope |

## Where they live

```
gks/proto/PROTO--<NAME>.md         ← atom file
src/validator/proto/<name>.ts      ← predicate implementation
test/validator/proto/<name>.test.ts ← unit tests
```

The validator's main loop (`src/validator/cli.ts --all`) runs all PROTO predicates after the existing rules. Per-PROTO `severity` decides whether to fail (error) or warn.

## What M8a delivers

M8a is the **pattern foundation** — not a specific PROTO yet:

1. **Atom-type registration** — `proto` becomes a valid `type:` in the contract
2. **Validator scaffold** — `src/validator/proto/loader.ts` finds PROTO atoms + their TS predicates and wires them into `src/validator/cli.ts`
3. **Required-fields contract** — every PROTO atom must have `crosslinks.enforces` to a FRAME
4. **Sample PROTO** — `[[PROTO--SAMPLE-RULE]]` (a trivial demo) to validate the loader works end-to-end
5. **Documentation** — this concept atom + ADR + FEAT + BLUEPRINT

M8b–f then implement specific PROTOs (PHASE-GATES, SCALING-LEVEL-GATE, ALGO-PARAM-COUPLING, AUTHORITY-ENFORCEMENT, plus auditing existing rules into PROTOs).

## Invariants

- **PROTOs are pure functions** — no I/O beyond reading atoms / git state
- **PROTOs return `{ ok, violations[] }`** — never throw
- **PROTOs are versioned via `status:` + `supersededBy:`** — same as other atoms
- **A PROTO MUST link to a FRAME via `crosslinks.enforces:`** — required-fields-enforced
- **A PROTO's TS impl path is recorded in `linked_symbols:`** — auditable

## Out of scope (deferred to M8b+)

- Actual phase-gate / scaling-level / algo-param / authority enforcement (each is M8b/c/d/e)
- PROTO discovery via Obsidian (file-system scan is enough for now)
- Cross-repo PROTO inheritance — M9c

## Source

`msp_spec.md` §10 (governance), `[[FRAMEWORK--PHASE-GOVERNANCE]]`, `[[FRAMEWORK--AUTHORITY-MATRIX]]`, `[[ADR--HUMAN-REVIEW-GATES]]`, user direction "วางแผนและทำให้จบ ทุก M" — M8a is the foundation enabling M8b–f.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[FEAT--MSP-VALIDATOR]]

