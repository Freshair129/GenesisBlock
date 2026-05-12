---
id: AUDIT--MSP-HOTFIX-WRAPPER
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M5b hotfix wrapper acceptance audit + id-format hex fix
tags:
  - msp
  - m5
  - m5b
  - audit
  - hotfix
crosslinks: {"references":["FEAT--MSP-HOTFIX-WRAPPER","BLUEPRINT--MSP-HOTFIX-WRAPPER","ADR--MSP-HOTFIX-WRAPPER","ADR--HOTFIX-ESCAPE-HATCH","FEAT--MSP-PRECOMMIT-HOOK"]}
linked_symbols:
  - {"file":"examples/hooks/pre-commit-validator.sh"}
  - {"file":"package.json"}
  - {"file":"src/validator/rules/id-format.ts"}
created_at: 2026-05-03T19:01:45.299+07:00
---

# AUDIT — hotfix wrapper (M5b)

## Scope

Closes FEAT--MSP-HOTFIX-WRAPPER. Closes P1 #6 from M3 backlog.

## Acceptance criteria from FEAT

| # | Criterion | Result |
|---|---|---|
| 1 | 4 npm scripts (open/list/close/check) — thin passthroughs | ✅ |
| 2 | `npm run msp:hotfix:list` exits 0 with empty/no-overdue | ✅ |
| 3 | pre-commit hook extended with hotfix gate after validator | ✅ |
| 4 | gather staged paths excluding gks/.brain/infra dirs | ✅ test |
| 5 | empty paths → no hotfix check (zero-cost path) | ✅ implementation |
| 6 | non-empty → single `gks hotfix check --file=<p>...` call | ✅ test |
| 7 | non-zero exit → human-readable output + summary + exit 1 | ✅ test |
| 8 | `git commit --no-verify` bypasses (standard) | ✅ |
| 9 | smoke test: overdue HOTFIX → blocked | ✅ test |
| 10 | smoke test: no HOTFIX → succeeds | ✅ test |

## Test summary

```
test/hooks/pre-commit.test.ts:                6/6 passing (4 prior + 2 new)
test/validator/rules/id-format.test.ts:      11/11 passing (9 prior + 2 new for HOTFIX hex)
```

## Bug found during dogfood

Validator's `ID_PATTERN` required UPPERCASE in the slug part, which rejected the lowercase-hex `HOTFIX--<sha>` form that `gks hotfix open` actually emits (e.g. `HOTFIX--abc1234` from a real git SHA). Extended the regex with `HOTFIX--[a-f0-9]+` as a special case while keeping the general `[A-Z]...` pattern for everything else.

Same precedent as M2 + M4a: bug recorded in audit + new test cases added.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 6/6 hook tests + 11/11 id-format tests + manual gks hotfix end-to-end
- Date: 2026-05-03
