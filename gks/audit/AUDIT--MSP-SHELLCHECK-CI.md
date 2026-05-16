---
id: AUDIT--MSP-SHELLCHECK-CI
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M5f shellcheck CI step audit
tags:
  - msp
  - m5
  - m5f
  - audit
  - ci
  - shellcheck
crosslinks: {"references":["FEAT--MSP-PRECOMMIT-HOOK","FEAT--MSP-PREPUSH-HOOK"]}
linked_symbols:
  - {"file":".github/workflows/test.yml"}
phase_override:
  skip_blueprint: true
  reason: "M5f added a single shellcheck step to the CI workflow — a config change, not code; governed by FEAT--MSP-PRECOMMIT-HOOK / FEAT--MSP-PREPUSH-HOOK."
created_at: 2026-05-03T18:01:44.015+07:00
---

# AUDIT — M5f

## Scope

L1 infrastructure — added `shellcheck examples/hooks/*.sh` step to CI right after `npm ci`. Ubuntu runners ship shellcheck so no `apt-get install` step needed.

## Verification

- shellcheck 0.9.0 locally — clean against both `pre-commit-validator.sh` and `install.sh` (and the new `pre-push-verify.sh` from M5a after one SC2001 disable was applied).
- CI will exercise on next push.

## Why care

Pre-commit hooks shipping with bash bugs (unquoted expansions, `set -e` interactions) silently lose protection. shellcheck catches these statically. Cheap to run, big upside.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: local shellcheck run; CI step lands with this PR
- Date: 2026-05-03

## Connections
- [[FEAT--MSP-PRECOMMIT-HOOK]]
- [[FEAT--MSP-PREPUSH-HOOK]]

