---
id: AUDIT--MSP-PRECOMMIT-HOOK
phase: 5
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: MSP pre-commit hook M3a acceptance audit
tags:
  - msp
  - precommit
  - hook
  - audit
  - m3a
  - dogfood
crosslinks: {"references":["FEAT--MSP-PRECOMMIT-HOOK","BLUEPRINT--MSP-PRECOMMIT-HOOK","ADR--MSP-PRECOMMIT-HOOK","FEAT--MSP-VALIDATOR"]}
linked_symbols:
  - {"file":"packages/msp/examples/hooks/pre-commit-validator.sh"}
  - {"file":"packages/msp/examples/hooks/install.sh"}
  - {"file":"packages/msp/examples/hooks/README.md"}
  - {"file":"packages/msp/test/hooks/pre-commit.test.ts"}
created_at: 2026-05-03T14:42:57.238+07:00
---

# AUDIT — MSP pre-commit hook M3a acceptance

## Scope

Closes the M3a item from `AUDIT--MSP-VALIDATOR` and `AUDIT--KNOWLEDGE-BASE`. Walks `FEAT--MSP-PRECOMMIT-HOOK` end to end via the doc-to-code loop:

1. Scaffolded 4 atoms via `gks new-feature msp-precommit-hook` (CONCEPT/ADR/FEAT/BLUEPRINT)
2. Filled and promoted them through inbound queue
3. Implemented the script + installer + README under `examples/hooks/`
4. Wrote vitest spawning real bash to assert behaviour
5. Installed the hook on this very repo and dogfooded it (this commit's own validator runs)

## Acceptance criteria from FEAT--MSP-PRECOMMIT-HOOK

| # | Criterion | Result |
|---|---|---|
| 1 | Hook is executable bash with `set -euo pipefail` | ✅ `pre-commit-validator.sh` |
| 2 | No-files-staged → exits 0 silently | ✅ test #1 |
| 3 | Bad atom under `gks/` → exits 1 with `[rule-id]` | ✅ test #3 |
| 4 | Bad atom under `.brain/msp/projects/*/inbound/` → exits 1 | ✅ regex covers both patterns; tested manually |
| 5 | Other staged `.md` (e.g. `README.md`) → not validated, exit 0 | ✅ regex anchors to the two specific path prefixes |
| 6 | Pass → `✓ MSP validator: N file(s) passed.` line | ✅ test #2 |
| 7 | `install.sh` idempotent on re-run | ✅ verified manually with two consecutive runs |
| 8 | `install.sh` refuses non-MSP hook | ✅ verified manually with planted fake hook |
| 9 | `--no-verify` bypasses | ✅ test #4 |
| 10 | README documents both install paths + escape | ✅ `examples/hooks/README.md` |
| 11 | Smoke test spawns real git + asserts exit codes | ✅ `test/hooks/pre-commit.test.ts` (4/4 passing) |

## Test summary

```
10 test files, 53 tests, 53 passed (4.81s)
```

New files in M3a:
- `test/hooks/pre-commit.test.ts` (4 tests) — spawns a fixture git repo, copies the hook in, stages valid/invalid atoms, asserts hook behaviour incl. `--no-verify` bypass.

## Validator dogfood (after M3a)

```
npm run msp:index            → 52 indexed (was 47 + 4 hook atoms + this audit)
npm run msp:validate -- --all → Total: 52 passed, 0 failed
npx gks verify-flow FEAT--MSP-PRECOMMIT-HOOK → 12 atoms visited, status OK
npx gks validate --links     → status OK (52 atoms scanned)
```

## What this audit does NOT certify

- **Cross-platform on Windows without Git Bash** — defer (acceptable per `ADR--MSP-PRECOMMIT-HOOK`).
- **shellcheck clean** — shellcheck not available in the dev env at audit time. Manual review of the 50-LOC script gave no obvious issues; CI integration in M3+ should add a shellcheck step.
- **Hook auto-install on `npm install`** — out of scope; this is opt-in by design (one-line `bash examples/hooks/install.sh`).

## Bug found and fixed during M3a

None. The implementation was small enough to land first-try; the smoke test passed on the first run.

## Residual M3+ backlog

- M3b: Refactor forbidden-fields list to load from `atomic_contract.yaml` at runtime (per `ADR--FORBIDDEN-FIELDS-LIST` "Negative" note).
- M3c: Implement the 4 FEAT scaffolds (codegen runner + 3 memory writers).
- M3d: GKS upstream patch for `phase: 6` so AUDIT atoms land at canonical phase.
- Doc-PR: align `msp_spec.md` §12 with `ADR--PATH-ENCODING`.
- CI: add shellcheck on the hook script + `npm test` + `npm run msp:validate -- --all` matrix.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 53/53 tests, hook installed + manually tested on this repo, validator dogfood OK
- Date: 2026-05-03

## References

- `FEAT--MSP-PRECOMMIT-HOOK` — acceptance criteria source
- `BLUEPRINT--MSP-PRECOMMIT-HOOK` — geography + algorithm
- `ADR--MSP-PRECOMMIT-HOOK` — bash-vs-husky decision
- `FEAT--MSP-VALIDATOR` — what the hook invokes
