---
id: BLUEPRINT--MSP-PRECOMMIT-HOOK
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — pre-commit hook implementation plan
tags:
  - msp
  - precommit
  - hook
  - blueprint
  - implementation
crosslinks: {"implements":["FEAT--MSP-PRECOMMIT-HOOK"],"references":["ADR--MSP-PRECOMMIT-HOOK"]}
linked_symbols:
  - {"file":"examples/hooks/pre-commit-validator.sh"}
  - {"file":"examples/hooks/install.sh"}
  - {"file":"examples/hooks/README.md"}
  - {"file":"test/hooks/pre-commit.test.ts"}
created_at: 2026-05-03T14:39:05.783+07:00
---

# BLUEPRINT — pre-commit hook

```yaml
metadata:
  title: "MSP pre-commit hook"
  parent_feat: FEAT--MSP-PRECOMMIT-HOOK

architectural_pattern: |
  Plain bash script. Single file, ~50 LOC. No node code, no shell helpers
  imported. Companion installer is similarly small.

data_logic: |
  pre-commit-validator.sh:
    1. List staged files: git diff --cached --name-only --diff-filter=ACMR
    2. Filter: keep paths matching ^(gks/.*\.md|\.brain/msp/projects/[^/]+/inbound/.*\.md)$
    3. If empty → exit 0 silently
    4. For each path, invoke: npm run msp:validate --silent -- "$path"
    5. Track failures; on each failure re-run without --silent for human-readable error
    6. Exit 0 if all passed; 1 if any failed

  install.sh:
    1. Fail early if run outside a git repo
    2. Check for existing .git/hooks/pre-commit
       - if present and contains MARKER → idempotent reinstall, OK
       - if present and lacks MARKER → refuse with diff suggestion + exit 1
    3. Copy examples/hooks/pre-commit-validator.sh → .git/hooks/pre-commit
    4. chmod +x
    5. Print success + uninstall hint

geography:
  - "examples/hooks/pre-commit-validator.sh"   # the hook itself
  - "examples/hooks/install.sh"                 # idempotent installer
  - "examples/hooks/README.md"                  # docs (install + uninstall + escape)
  - "test/hooks/pre-commit.test.ts"             # vitest spawning real bash

api_contracts:
  - name: pre-commit-validator.sh
    contract: |
      stdin:  none
      stdout: ✓ MSP validator: N file(s) passed.   (on success)
              ✗ <path> [rule-id] <message>          (per failed file, indented)
              ✗ MSP validator: N file(s) failed.   (summary line on failure)
      stderr: passes through validator stderr verbatim
      exit:   0 = all staged atom files passed (or none staged)
              1 = one or more staged atom files failed
              2 = internal error (validator infrastructure broken)

  - name: install.sh
    contract: |
      args:   none (installs into the current repo's .git/hooks/)
      stdout: install confirmation + uninstall hint
      stderr: error message if .git missing or non-MSP hook present
      exit:   0 = installed (fresh or idempotent re-install)
              1 = aborted (non-MSP hook present, or not a git repo)

verification_plan:
  - bash -n examples/hooks/pre-commit-validator.sh   # syntax check
  - shellcheck examples/hooks/pre-commit-validator.sh examples/hooks/install.sh (if available)
  - vitest test that:
      - spawns a temp git repo
      - copies the hook in
      - stages a known-good fixture → commit succeeds
      - stages a known-bad fixture → commit fails with code 1
      - verifies --no-verify still bypasses
  - manual smoke: install.sh on this repo's worktree, then make a known-bad atom and try to commit
```

## Implementation order

T1 DETECT-STAGED-MD (the regex + git diff --cached invocation)
T2 RUN-VALIDATOR (the loop calling `npm run msp:validate`)
T3 SUMMARISE-FAILURES (re-run without --silent + count + exit code)
T4 INSTALL-HELPER (idempotent installer with marker comment)
