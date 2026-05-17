---
id: BLUEPRINT--MSP-HOTFIX-WRAPPER
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — hotfix wrapper implementation plan
tags:
  - msp
  - hotfix
  - blueprint
  - implementation
crosslinks:
  implements:
    - FEAT--MSP-HOTFIX-WRAPPER
  references:
    - ADR--MSP-HOTFIX-WRAPPER
linked_symbols:
  - file: examples/hooks/pre-commit-validator.sh
  - file: package.json
  - file: packages/msp/test/hooks/pre-commit.test.ts
created_at: 2026-05-03T17:45:50.637+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  domain: blueprint
---

# BLUEPRINT — hotfix wrapper

```yaml
metadata:
  title: "MSP hotfix wrapper"
  parent_feat: FEAT--MSP-HOTFIX-WRAPPER

architectural_pattern: |
  Two surfaces, no new code modules:
    1. package.json scripts: 4 thin passthroughs to `gks hotfix ...`
    2. examples/hooks/pre-commit-validator.sh: bash extension after the
       existing validator section

data_logic: |
  package.json scripts (no logic):
    "msp:hotfix:open":  "gks hotfix open",
    "msp:hotfix:list":  "gks hotfix list",
    "msp:hotfix:close": "gks hotfix close",
    "msp:hotfix:check": "gks hotfix check"

  pre-commit-validator.sh extension (after the existing STAGED loop):
    hotfix_paths=$(git diff --cached --name-only --diff-filter=ACMR \
      | grep -v -E '^(gks/|\.brain/|\.github/|examples/|scripts/|test/|node_modules/|dist/)' || true)
    if [ -n "$hotfix_paths" ]; then
      args=()
      while IFS= read -r p; do args+=(--file="$p"); done <<< "$hotfix_paths"
      output=$(npx gks hotfix check "${args[@]}" 2>&1) && rc=0 || rc=$?
      if [ "$rc" -ne 0 ]; then
        echo "$output" | sed 's/^/  /'
        echo "✗ MSP hotfix check failed. Backfill the HOTFIX-- atom or use --no-verify."
        exit 1
      fi
    fi

geography:
  - "examples/hooks/pre-commit-validator.sh"
  - "package.json"
  - "packages/msp/test/hooks/pre-commit.test.ts"

api_contracts:
  - name: "npm run msp:hotfix:open|list|close|check"
    contract: |
      Identical to `gks hotfix <subcommand>`. All flags pass through.
      Exit code mirrors gks hotfix.

  - name: pre-commit hotfix gate
    contract: |
      runs after the validator section in pre-commit-validator.sh
      input: staged paths from `git diff --cached --name-only`
      filter: skip paths starting with gks/ .brain/ .github/ examples/
              scripts/ test/ node_modules/ dist/
      action: single `npx gks hotfix check --file=<p>...` call
      exit:   0 if no remaining paths OR `gks hotfix check` passes
              1 if `gks hotfix check` exits non-zero (overdue HOTFIX)

verification_plan:
  - packages/msp/test/hooks/pre-commit.test.ts extended:
    - "no overdue HOTFIX → commit succeeds even with src/ change"
    - "fake overdue HOTFIX atom + staged src/foo.ts → commit blocked"
    - both fixtures use a tmp git repo + symlinked node_modules same
      as existing pre-commit smoke tests
  - shellcheck examples/hooks/pre-commit-validator.sh
```

## Implementation order

T1 NPM-SCRIPTS (4 lines in package.json)
T2 HOOK-INTEGRATION (extend pre-commit-validator.sh)
T3 TESTS (extend pre-commit.test.ts with 2 new cases)

## Connections
- [[FEAT--MSP-HOTFIX-WRAPPER]]
- [[ADR--MSP-HOTFIX-WRAPPER]]

