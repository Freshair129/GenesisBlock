---
id: FEAT--MSP-PRECOMMIT-HOOK
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: pre-commit-validator.sh — bash hook that runs MSP validator on staged atoms
tags: &a1
  - msp
  - precommit
  - hook
  - user-facing
crosslinks: &a2
  implements:
    - ADR--MSP-PRECOMMIT-HOOK
  references:
    - CONCEPT--MSP-PRECOMMIT-HOOK
    - FEAT--MSP-VALIDATOR
linked_symbols: &a3
  - file: examples/hooks/pre-commit-validator.sh
  - file: examples/hooks/install.sh
  - file: examples/hooks/README.md
created_at: 2026-05-03T14:39:05.322+07:00
aliases: &a4
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  id: FEAT--MSP-PRECOMMIT-HOOK
  phase: 2
  type: feat
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: pre-commit-validator.sh — bash hook that runs MSP validator on staged atoms
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-03T14:39:05.322+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Feature spec
  attributes:
    id: FEAT--MSP-PRECOMMIT-HOOK
    phase: 2
    type: feat
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: pre-commit-validator.sh — bash hook that runs MSP validator on staged atoms
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-03T14:39:05.322+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Feature spec
    attributes:
      domain: feat
    domain: feat
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: feat
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# FEAT — pre-commit hook

## User-facing behaviour

Install once:

```sh
bash examples/hooks/install.sh
# OR
cp examples/hooks/pre-commit-validator.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

After install, every `git commit` runs the validator on staged `.md` files under `gks/` and `.brain/msp/projects/<ns>/inbound/`.

```sh
$ git add gks/concept/CONCEPT--BAD.md
$ git commit -m "..."
✗ /…/CONCEPT--BAD.md [forbidden-fields] frontmatter contains forbidden field 'commit_hash'
✗ MSP validator: 1 file(s) failed. Fix and re-stage, or use --no-verify to skip.
```

Fixing + re-staging makes the next commit pass:

```sh
$ git add gks/concept/CONCEPT--BAD.md
$ git commit -m "..."
✓ MSP validator: 1 file(s) passed.
[main abc123] ...
```

If no relevant `.md` files are staged, the hook exits silently with code 0 (zero-cost happy path).

## Acceptance criteria

- [ ] Hook script is executable bash with `#!/usr/bin/env bash` and `set -euo pipefail`
- [ ] When no `.md` files matching the patterns are staged → exits 0 without printing
- [ ] When a staged file under `gks/` violates a hard rule → exits 1 with the validator's `[rule-id]` line
- [ ] When a staged file under `.brain/msp/projects/*/inbound/` violates → exits 1
- [ ] When a staged `.md` file outside both patterns (e.g. `README.md`) → not validated, hook still exits 0
- [ ] When all staged files pass → exits 0 with a `✓ MSP validator: N file(s) passed.` line
- [ ] `install.sh` copies the hook to `.git/hooks/pre-commit` + sets executable bit, idempotent on re-run
- [ ] `install.sh` refuses to overwrite an existing non-MSP pre-commit hook (detects via marker comment) and exits 1 with a clear message
- [ ] `git commit --no-verify` bypasses the hook (standard git behaviour, must not be broken by us)
- [ ] `examples/hooks/README.md` documents both install paths + the `--no-verify` escape
- [ ] Smoke test (`test/hooks/pre-commit.test.ts`) spawns a temp git repo, stages a known-bad fixture, asserts hook exit 1

## Surfaces

| Surface | Form |
|---|---|
| File | `examples/hooks/pre-commit-validator.sh` (the hook itself) |
| File | `examples/hooks/install.sh` (idempotent installer) |
| Doc | `examples/hooks/README.md` (install + uninstall + escape) |
| Test | `test/hooks/pre-commit.test.ts` (vitest spawning real bash) |

## Out of scope

- Pre-push hook for chain integrity (`gks verify-flow`). Separate FEAT in M3+.
- Hotfix gate (48h backfill check). Belongs to `[[ADR--HOTFIX-ESCAPE-HATCH]]` implementation.
- Cross-platform Windows install without Git Bash. Defer.

## Connections
- [[ADR--MSP-PRECOMMIT-HOOK]]
- [[CONCEPT--MSP-PRECOMMIT-HOOK]]
- [[FEAT--MSP-VALIDATOR]]

