---
id: AUDIT--MSP-PREPUSH-HOOK
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M5a pre-push hook acceptance audit
tags:
  - msp
  - m5
  - m5a
  - audit
  - hook
  - prepush
crosslinks: {"references":["FEAT--MSP-PREPUSH-HOOK","BLUEPRINT--MSP-PREPUSH-HOOK","ADR--MSP-PREPUSH-HOOK"]}
linked_symbols:
  - {"file":"packages/msp/examples/hooks/pre-push-verify.sh"}
  - {"file":"packages/msp/examples/hooks/install.sh"}
  - {"file":"packages/msp/examples/hooks/README.md"}
  - {"file":"packages/msp/test/hooks/pre-push.test.ts"}
created_at: 2026-05-03T18:01:44.663+07:00
---

# AUDIT — pre-push hook (M5a)

## Scope

Closes FEAT--MSP-PREPUSH-HOOK. Closes P1 #7 from M3 backlog.

## Acceptance criteria from FEAT

| # | Criterion | Result |
|---|---|---|
| 1 | bash with `set -euo pipefail` + marker comment | ✅ |
| 2 | parses git pre-push stdin protocol | ✅ test |
| 3 | new-branch fallback to merge-base origin/main | ✅ implementation |
| 4 | git diff filtered to `gks/feat/FEAT--*.md` | ✅ test (only FEAT-touch triggers) |
| 5 | runs `npx gks verify-flow` per touched FEAT | ✅ test |
| 6 | aggregates non-OK → exit 1 | ✅ test |
| 7 | zero touched FEATs → exit 0 silently | ✅ test |
| 8 | `git push --no-verify` bypasses | ✅ test |
| 9 | install.sh handles both hooks idempotently | ✅ manual + smoke |
| 10 | README updated | ✅ |
| 11 | smoke test (4 cases) | ✅ |

## Test summary

```
test/hooks/pre-push.test.ts: 4/4 passing
```

## Bug found during dogfood

`set -o pipefail` + `set -e` made the hook exit at the verify-flow re-run pipeline before printing the summary line. Root cause: `npx ... | sed` exits non-zero when npx exits 1 with pipefail; set -e then aborts. Fix: capture verify-flow output once with `&& rc=0 || rc=$?`, no re-run, no pipe.

Same precedent as M2 + M4a: bug recorded in audit, not as a separate INCIDENT/ISSUE.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 4/4 tests + dogfood (this audit + sibling M5 audits push through the hook)
- Date: 2026-05-03
