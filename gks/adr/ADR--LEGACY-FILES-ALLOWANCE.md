---
id: ADR--LEGACY-FILES-ALLOWANCE
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Legacy files allowance — exempt from strict validation with frontmatter flag
tags:
  - msp
  - legacy
  - exemption
  - validator
crosslinks: {"references":["ADR--ANTI-HALLUCINATION-RULES","ADR--FORBIDDEN-FIELDS-LIST","CONCEPT--MSP-VALIDATOR"]}
created_at: 2026-05-03T14:08:43.977+07:00
---

# ADR — legacy files allowance

## Context

When MSP is adopted into an existing project, there are atoms (or atom-shaped markdown) authored before the contract existed. Forcing them through the full validator either rejects huge swaths of historical knowledge or forces a one-time mass rewrite. Neither is acceptable.

## Decision

```yaml
legacy_files:
  allowed: true
  requires_frontmatter: "legacy: true"
```

### Behaviour

- An atom with `legacy: true` in frontmatter is **exempt from**:
  - `forbidden-fields` (legacy atoms may carry derived fields with their original meaning)
  - `summary-min` (legacy summaries may be too short or contain TBD)
  - `evidence-for-decisions` (older ADRs predate the Context/Decision/Consequences template)
  - `cite-or-mark-inferred` (legacy atoms didn't have `linked_symbols`)

- An atom with `legacy: true` is **still subject to**:
  - `id-format` and `id-filename-match` (must rename if non-conforming)
  - `dangling-wikilinks` (broken links must be fixed even in legacy)
  - `future-date` (always wrong)

### Reporting

`npm run msp:validate -- --all --report-legacy` (M3) emits a separate count of legacy-flagged atoms so the team has visibility into the migration backlog.

### Migration path

To "legacify-then-clean":
1. Add `legacy: true` to the atom; validator now passes.
2. Open a P3 BLUEPRINT for refactoring the atom to current contract (one PR per type or batch).
3. After refactor, remove `legacy: true`. Validator must pass without it.

## Consequences

**Positive**
- Adoption is incremental — no one-shot mass rewrite.
- Legacy atoms are visible in reports, not silently broken.
- The exemption list is narrow: structural rules (ID format, wikilinks, dates) still apply.

**Negative**
- `legacy: true` could be abused to bypass new rules ("just mark it legacy"). Mitigated by code review on every addition of the flag and the report-legacy count showing if it's growing.
- The exemption set is hardcoded in the validator. M3 plan loads it from `atomic_contract.yaml`.

## Alternatives considered

1. **No legacy flag — rewrite everything.** Rejected per scale concerns above.
2. **Two parallel validators (strict / legacy).** Rejected — code duplication risk.
3. **`legacy: true` exempts EVERYTHING.** Rejected — leaves dangling links unaddressed.

## Source

`msp_spec.md` §10.2 (Legacy Files).
