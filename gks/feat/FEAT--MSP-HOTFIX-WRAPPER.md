---
id: FEAT--MSP-HOTFIX-WRAPPER
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: msp:hotfix:* scripts + pre-commit gate via gks hotfix check
tags:
  - msp
  - hotfix
  - user-facing
  - hook
crosslinks: {"implements":["ADR--MSP-HOTFIX-WRAPPER"],"references":["CONCEPT--MSP-HOTFIX-WRAPPER","FEAT--MSP-PRECOMMIT-HOOK","ADR--HOTFIX-ESCAPE-HATCH"]}
linked_symbols:
  - {"file":"examples/hooks/pre-commit-validator.sh"}
  - {"file":"package.json"}
  - {"file":"packages/msp/test/hooks/pre-commit.test.ts"}
created_at: 2026-05-03T17:45:50.058+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: "Feature spec"
---

# FEAT — hotfix wrapper

## User-facing behaviour

```sh
# Open a hotfix on the current HEAD
npm run msp:hotfix:open -- $(git rev-parse HEAD) \
  --title="prod down: rate limiter overflow" \
  --file=src/api/rate-limit.ts \
  --reason="customer escalation"

# List open hotfixes
npm run msp:hotfix:list
npm run msp:hotfix:list -- --overdue

# Close after backfill atoms exist + are stable
npm run msp:hotfix:close -- HOTFIX--abc1234 \
  --resolved-by=ADR--RATE-LIMIT \
  --resolved-by=BLUEPRINT--RATE-LIMIT
```

After install, the pre-commit hook automatically blocks commits to files with overdue hotfix windows:

```
$ git add src/api/rate-limit.ts
$ git commit -m "tweak"
✓ MSP validator: 0 file(s) passed.
  ✗ src/api/rate-limit.ts: HOTFIX--abc1234 expired 12h ago — backfill required
✗ MSP hotfix check failed. Backfill the HOTFIX-- atom or use --no-verify.
```

## Acceptance criteria

- [ ] `package.json` has 4 new scripts: `msp:hotfix:open`, `msp:hotfix:list`, `msp:hotfix:close`, `msp:hotfix:check` — all thin passthroughs to `gks hotfix ...`
- [ ] `npm run msp:hotfix:list` exits 0 with no overdue (or empty list)
- [ ] `examples/hooks/pre-commit-validator.sh` extended with a hotfix-check section after the validator pass
- [ ] Hook gathers staged paths excluding `gks/`, `.brain/`, `.github/`, `examples/`, `scripts/`, `test/`
- [ ] If gathered paths empty → no hotfix check (zero-cost path)
- [ ] If gathered paths non-empty → `gks hotfix check --file=<p>...` once
- [ ] On non-zero exit → re-run for human-readable output + summary line + exit 1
- [ ] `git commit --no-verify` bypasses (already standard)
- [ ] Smoke test extended: a fixture with a fake overdue HOTFIX atom + staged source file → commit blocked
- [ ] Smoke test: same fixture without hotfix atom → commit succeeds

## Surfaces

| Surface | Form |
|---|---|
| npm scripts | `msp:hotfix:open`, `msp:hotfix:list`, `msp:hotfix:close`, `msp:hotfix:check` |
| Hook | `pre-commit-validator.sh` extended (no separate file) |
| Tests | `test/hooks/pre-commit.test.ts` extended (existing file) |

## Out of scope

- Auto-SHA detection in `msp:hotfix:open` — user passes `$(git rev-parse HEAD)`.
- Reviewer enforcement on close per `[[ADR--HUMAN-REVIEW-GATES]]` — defer.
- Distributed enforcement across machines — orchestrator's job.

## Connections
- [[ADR--MSP-HOTFIX-WRAPPER]]
- [[CONCEPT--MSP-HOTFIX-WRAPPER]]
- [[FEAT--MSP-PRECOMMIT-HOOK]]
- [[ADR--HOTFIX-ESCAPE-HATCH]]

