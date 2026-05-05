---
id: ADR--PROTO-ATOM-TYPE
phase: 2
type: adr
status: stable
vault_id: default
title: PROTO is a new atom type — separate from ADR / FEAT / FRAME
tags:
  - msp
  - proto
  - atom-type
  - decision
  - m8a
crosslinks: {"references":["CONCEPT--PROTO-PATTERN","CONCEPT--ATOMIC-WRITE-CONTRACT","ADR--FORBIDDEN-FIELDS-LIST"]}
created_at: 2026-05-05T09:18:00.000Z
---

# ADR — PROTO atom type

## Context

We need a way to encode governance rules as machine-checkable contracts (per `CONCEPT--PROTO-PATTERN`). Three placement options:

1. **Inside FRAME atoms** — extend FRAME with `## Predicate` section + linked TS file. FRAMEs become both descriptive and prescriptive.
2. **As ADRs with a special tag** — `ADR-- + tag: governance-rule` plus a convention for the implementation file.
3. **A new `PROTO--` atom type** — first-class type, validator-aware.

Option 3 is cleanest. Records here.

## Decision

Add `proto` as a new top-level atom type. New directory `gks/proto/`, new prefix `PROTO--<NAME>`.

### Why not extend FRAME

- FRAMEs describe; PROTOs prescribe. Same atom doing both makes the file harder to evolve (FRAMEs change rarely; PROTOs may iterate on impl)
- FRAMEs don't have impl paths today; adding one is an envelope change for every existing FRAME
- Symbol matching: `FRAME-- = governance-doctrine`, `PROTO-- = governance-mechanism` — distinct concerns, distinct prefixes

### Why not just an ADR

- ADRs are one-shot historical decisions; PROTOs are living rules with implementations that mutate
- ADR conventions don't naturally express "this atom MUST link to a TS predicate"
- Forbidden-fields rule already strict on ADR shape; reusing it for PROTOs would dilute

### What changes in the contract

`atomic_contract.yaml` (or its embedded equivalent in `src/validator/contract.ts`) gets:

```yaml
types:
  proto:
    required_frontmatter:
      - id
      - phase
      - type
      - status
      - crosslinks.enforces        # ← new, PROTO-only requirement
    forbidden_frontmatter:
      # same general list as other types
    id_pattern: '^PROTO--[A-Z][A-Z0-9-]*$'
    valid_phases: [2]              # PROTOs are decision-level
    requires_implementation: true  # ← new: linked_symbols must include a src/ TS file
```

### File layout

```
gks/proto/PROTO--<NAME>.md         atom file
src/validator/proto/<name>.ts      predicate
test/validator/proto/<name>.test.ts tests
```

The validator's loader (`src/validator/proto/loader.ts`) finds atoms in `gks/proto/`, derives the impl path from the atom slug (or reads from `linked_symbols[0].file`), imports the predicate, and runs it.

### `crosslinks.enforces` is required

A PROTO without an `enforces:` link is a rule without governance backing. Required-fields enforcement (M5d infrastructure) gets a new entry: `proto.crosslinks.enforces` is required and must point to an existing `FRAME--*` atom.

### `linked_symbols` is required

A PROTO without an impl is documentation, not a contract. Required-fields gets a new entry: `proto.linked_symbols` must contain at least one entry pointing into `src/validator/proto/`.

### Status field semantics

Same as other atoms: `draft | stable | superseded`. A draft PROTO does NOT block CI even if it would fail — only `stable` PROTOs are hard gates. (Validator already has this distinction in some rules; M8a generalises it.)

## Consequences

**Positive**
- Clean type boundary; FRAMEs stay descriptive, PROTOs are mechanisms
- Extending the contract is a small change — adds one type entry, one id pattern
- Validator loader is a small generic component; specific PROTOs are independent
- Future `PROTO--` atoms can be drafted, tested, then promoted to `stable` (gradual rollout)

**Negative**
- Atom-type taxonomy growth — but justified; the prior 6 types didn't cover this space
- Two-place change required when adding any PROTO (atom + TS file); accepted (impl is the point)
- Contract changes need careful versioning so existing atoms still validate

## Alternatives considered

1. **Fold into FRAME** — see "Why not extend FRAME"
2. **Tag-based ADRs** — see "Why not just an ADR"
3. **Pure code (no atom)** — predicates as TS-only without atomic doc. Rejected: defeats the doc-to-code workflow + makes governance rules invisible to atom search
4. **External governance config** — JSON file outside `gks/`. Rejected: same visibility argument

## What this ADR does NOT decide

- **Specific PROTO rules** — M8b–f
- **Predicate runtime / sandboxing** — predicates are pure functions in the same Node process; no sandboxing
- **PROTO weights / priorities** — out of scope
- **Cross-namespace PROTO inheritance** — M9c

## Source

`CONCEPT--PROTO-PATTERN`, `msp_spec.md` §10, user direction during all-M planning (M8 governance protocol layer).
