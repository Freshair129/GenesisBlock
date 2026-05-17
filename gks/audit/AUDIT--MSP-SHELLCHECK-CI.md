---
id: AUDIT--MSP-SHELLCHECK-CI
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M5f shellcheck CI step audit
tags: &a1
  - msp
  - m5
  - m5f
  - audit
  - ci
  - shellcheck
crosslinks: &a2
  references:
    - FEAT--MSP-PRECOMMIT-HOOK
    - FEAT--MSP-PREPUSH-HOOK
linked_symbols: &a3
  - file: .github/workflows/test.yml
phase_override: &a4
  skip_blueprint: true
  reason: M5f added a single shellcheck step to the CI workflow — a config change,
    not code; governed by FEAT--MSP-PRECOMMIT-HOOK / FEAT--MSP-PREPUSH-HOOK.
created_at: 2026-05-03T18:01:44.015+07:00
aliases: &a5
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--MSP-SHELLCHECK-CI
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: M5f shellcheck CI step audit
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  phase_override: *a4
  created_at: 2026-05-03T18:01:44.015+07:00
  aliases: *a5
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--MSP-SHELLCHECK-CI
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: M5f shellcheck CI step audit
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    phase_override: *a4
    created_at: 2026-05-03T18:01:44.015+07:00
    aliases: *a5
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      domain: audit
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
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

