---
id: AUDIT--REQUIRED-FIELDS-CONTRACT
phase: 6
type: audit
status: stable
vault_id: default
title: M5d required_fields runtime contract audit
tags:
  - msp
  - m5
  - m5d
  - audit
  - validator
  - contract
crosslinks: {"references":["ADR--FORBIDDEN-FIELDS-LIST","FEAT--MSP-VALIDATOR"]}
linked_symbols:
  - {"file":".brain/msp/LLM_Contract/atomic_contract.yaml"}
  - {"file":"src/validator/contract.ts"}
  - {"file":"src/validator/rules/required-fields.ts"}
  - {"file":"src/validator/types.ts"}
created_at: 2026-05-03T10:58:58.818Z
---

# AUDIT — required_fields contract loader

## Scope

Closes P2 #11 from M3 backlog: enforce `required_fields` from `atomic_contract.yaml` at runtime (no code release needed to widen/narrow per-type required field lists).

## What changed

- `atomic_contract.yaml` extended with `required_fields.{default,by_type}` section.
- `src/validator/contract.ts` parses the new section into `RequiredFieldsConfig`. Skips gracefully (warning + undefined) on malformed input.
- `src/validator/types.ts` exports `RequiredFieldsConfig` + adds `requiredFields?` to `ValidationContext`.
- `src/validator/rules/required-fields.ts` new hard rule. No-op when context lacks config. Per-type lookup with fallback to default.
- `src/validator/index.ts` adds the rule to `HARD_RULES` (runs first so structural problems surface before content shape).
- `src/validator/cli.ts` threads contract.requiredFields into the context.

## Acceptance criteria

| # | Criterion | Result |
|---|---|---|
| 1 | YAML section parsed; default + by_type honoured | ✅ contract.test.ts |
| 2 | Missing `default` → skip with warning | ✅ test |
| 3 | Section absent → requiredFields undefined | ✅ test |
| 4 | Rule no-op without config | ✅ required-fields.test.ts |
| 5 | by-type lookup falls back to default | ✅ test |
| 6 | Empty string and empty array treated as missing | ✅ test |
| 7 | All 78 existing atoms still validate | ✅ dogfood (Total: 78 passed, 0 failed) |

## Test summary

```
test/validator/rules/required-fields.test.ts: 7/7 passing
test/validator/contract.test.ts:               8/8 passing (5 prior + 3 new)
```

## Conservative defaults

The shipped YAML uses fields that ALL existing atoms already have:
- `default`: id, phase, type, status, title, created_at
- `adr`: + tags
- `blueprint`: + linked_symbols

Tightening these (e.g. adding `summary` per spec) is a separate atom edit; would surface migration work for existing atoms that lack it.

## Residual

- `field_constraints` section in YAML is reserved as `{}` for now. Summary length + placeholder checks remain hardcoded in `summary-min.ts`. Migrating to YAML is M6.
- `epistemic.confidence` requirement for ADR/protocol per spec §4.2 is not enforced yet (would need an `epistemic` validator). M6.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 15 new tests + 78/78 atoms validate after enabling
- Date: 2026-05-03
