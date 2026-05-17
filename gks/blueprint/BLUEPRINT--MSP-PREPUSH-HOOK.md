---
id: BLUEPRINT--MSP-PREPUSH-HOOK
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — pre-push hook implementation plan
tags:
  - msp
  - prepush
  - hook
  - blueprint
  - implementation
crosslinks: {"implements":["FEAT--MSP-PREPUSH-HOOK"],"references":["ADR--MSP-PREPUSH-HOOK"]}
linked_symbols:
  - {"file":"examples/hooks/pre-push-verify.sh"}
  - {"file":"examples/hooks/install.sh"}
  - {"file":"examples/hooks/README.md"}
  - {"file":"packages/msp/test/hooks/pre-push.test.ts"}
created_at: 2026-05-03T17:39:29.189+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: "Implementation plan"
---

# BLUEPRINT — pre-push hook

```yaml
metadata:
  title: "MSP pre-push hook"
  parent_feat: FEAT--MSP-PREPUSH-HOOK

architectural_pattern: |
  Single bash script ~60 LOC. Reuses git's pre-push stdin protocol.
  Companion installer extended to handle two hooks (pre-commit + pre-push)
  with one marker comment per script.

data_logic: |
  pre-push-verify.sh:
    REPO_ROOT=$(git rev-parse --show-toplevel)
    cd "$REPO_ROOT"
    FAIL=0
    COUNT=0
    while read local_ref local_sha remote_ref remote_sha; do
      [ -z "$local_sha" ] && continue
      if [ "$remote_sha" = "0000000000000000000000000000000000000000" ]; then
        base=$(git merge-base "$local_sha" origin/main 2>/dev/null || echo "$local_sha~1")
      else
        base="$remote_sha"
      fi
      changed=$(git diff --name-only "$base" "$local_sha" -- 'gks/feat/FEAT--*.md' 2>/dev/null || true)
      [ -z "$changed" ] && continue
      while IFS= read -r path; do
        [ -z "$path" ] && continue
        feat=$(basename "$path" .md)
        COUNT=$((COUNT + 1))
        if ! npx gks verify-flow "$feat" --root="$REPO_ROOT" >/dev/null 2>&1; then
          FAIL=$((FAIL + 1))
          npx gks verify-flow "$feat" --root="$REPO_ROOT" 2>&1 | sed 's/^/  /'
        fi
      done <<< "$changed"
    done
    if [ "$FAIL" -gt 0 ]; then
      echo "✗ MSP pre-push: $FAIL of $COUNT FEAT(s) failed verify-flow. Fix and re-push, or use --no-verify."
      exit 1
    fi
    [ "$COUNT" -gt 0 ] && echo "✓ MSP pre-push: $COUNT FEAT(s) verified."
    exit 0

  install.sh (extended):
    Now installs BOTH pre-commit and pre-push hooks.
    Per-hook marker: msp:hook-marker:pre-commit-validator-v1
                     msp:hook-marker:pre-push-verify-v1
    Refusal logic per-hook: skip + warn if existing non-MSP hook present.

geography:
  - "examples/hooks/pre-push-verify.sh"
  - "examples/hooks/install.sh"
  - "examples/hooks/README.md"
  - "packages/msp/test/hooks/pre-push.test.ts"

api_contracts:
  - name: pre-push-verify.sh
    contract: |
      stdin:  git pre-push protocol — one or more lines of
              "<local-ref> <local-sha> <remote-ref> <remote-sha>"
      stdout: ✓ summary or ✗ per-FEAT failure lines
      stderr: passes through gks verify-flow stderr
      exit:   0 = no FEATs touched OR all verified OK
              1 = at least one verify-flow returned non-zero

verification_plan:
  - shellcheck examples/hooks/pre-push-verify.sh
  - vitest test/hooks/pre-push.test.ts:
      - spawn temp repo with bare-repo origin
      - commit a known-OK FEAT chain
      - install hook + try push → succeeds
      - introduce a chain break (delete a CONCEPT the FEAT references)
      - try push → exits 1, output mentions the failed FEAT
      - --no-verify bypass works
```

## Implementation order

T1 READ-STDIN (parse git pre-push protocol)
T2 DETECT-TOUCHED-FEATS (git diff filter)
T3 RUN-VERIFY-FLOW (loop + aggregate)
+ extend install.sh + README + smoke test

## Connections
- [[FEAT--MSP-PREPUSH-HOOK]]
- [[ADR--MSP-PREPUSH-HOOK]]

