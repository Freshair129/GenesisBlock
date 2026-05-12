---
id: AUDIT--MSP-CONTRACT-LOADER
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M3b runtime contract loader acceptance audit
tags:
  - msp
  - m3
  - m3b
  - audit
  - contract
  - validator
crosslinks: {"references":["ADR--FORBIDDEN-FIELDS-LIST","FEAT--MSP-VALIDATOR"]}
linked_symbols:
  - {"file":".brain/msp/LLM_Contract/atomic_contract.yaml"}
  - {"file":"src/validator/contract.ts"}
  - {"file":"src/validator/cli.ts"}
  - {"file":"test/validator/contract.test.ts"}
created_at: 2026-05-03T16:43:36.814+07:00
---

# AUDIT — runtime contract loader

## Scope

Closes M3b. Removes the "forbidden-fields list is hardcoded" caveat in `ADR--FORBIDDEN-FIELDS-LIST`. The list now lives in `.brain/msp/LLM_Contract/atomic_contract.yaml` and is loaded at validator-CLI invocation time.

## What changed

- New file: `.brain/msp/LLM_Contract/atomic_contract.yaml` (v1) — declares all 17 forbidden fields grouped by reason.
- New module: `src/validator/contract.ts` — `loadContract(root, path?)` returns `{ version, forbiddenFields, source: 'yaml' | 'default', warnings }`.
- Modified: `src/validator/cli.ts` — calls `loadContract` once per invocation and threads `forbiddenFields` into `ValidationContext`.
- Hardcoded `FORBIDDEN_FIELDS` set retained in `src/validator/rules/forbidden-fields.ts` as the fallback when YAML is missing or invalid.

## Acceptance criteria

| # | Criterion | Result |
|---|---|---|
| 1 | Missing YAML → fallback to defaults + warning | ✅ test #1 |
| 2 | Valid YAML → list honoured | ✅ test #2 + manual dogfood (truncated list let `commit_hash` pass) |
| 3 | Invalid YAML → fallback + warning | ✅ test #3 |
| 4 | Wrong field type → field-level fallback + warning | ✅ test #4 |
| 5 | Missing version → fallback | ✅ test #5 |
| 6 | Validator continues to function with no contract YAML present | ✅ falls back gracefully (legacy compatibility) |

## Test summary

```
test/validator/contract.test.ts: 5/5 passing
```

## Dogfood

Replaced YAML's `forbidden_fields` with `[non_real_field]` and re-validated `test/fixtures/CONCEPT--TEST-FORBIDDEN.md` (which contains `commit_hash`):
- With truncated YAML → ✓ passed (because `commit_hash` no longer in list)
- After restoring full YAML → ✗ rejected with `[forbidden-fields]`

Confirms the YAML actually drives runtime behaviour.

## Residual

- Other contract sections (`required_fields`, `field_constraints`) are accepted by the YAML schema but not yet enforced. M4+ work.
- No file-watcher; CLI re-loads on every invocation. Acceptable cost (~1 ms YAML parse).

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 5/5 unit tests + manual dogfood
- Date: 2026-05-03
