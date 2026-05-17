---
id: FEAT--MSP-PREPUSH-HOOK
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: pre-push-verify.sh — verify-flow per touched FEAT before push
tags:
  - msp
  - prepush
  - hook
  - user-facing
crosslinks:
  implements:
    - ADR--MSP-PREPUSH-HOOK
  references:
    - CONCEPT--MSP-PREPUSH-HOOK
    - FEAT--MSP-PRECOMMIT-HOOK
linked_symbols:
  - file: examples/hooks/pre-push-verify.sh
  - file: examples/hooks/install.sh
  - file: examples/hooks/README.md
created_at: 2026-05-03T17:39:28.605+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  domain: feat
---

# FEAT — pre-push hook

## User-facing behaviour

```sh
bash examples/hooks/install.sh
# installs both pre-commit AND pre-push hooks (idempotent)
```

After install:

```sh
$ git push
✓ MSP pre-push: 2 FEAT(s) verified — FEAT--MSP-VALIDATOR, FEAT--MSP-PRECOMMIT-HOOK
Total ...
```

When a chain is broken:

```sh
$ git push
✗ FEAT--RATE-LIMIT: chain integrity broken (CONCEPT--RATE-LIMIT not stable)
✗ MSP pre-push: 1 FEAT(s) failed verify-flow. Fix and re-push, or use --no-verify.
error: failed to push some refs to ...
```

If no FEAT files were touched in the push range, the hook exits 0 silently (zero-cost happy path).

## Acceptance criteria

- [ ] Hook script is bash with `set -euo pipefail` and the same marker-comment scheme as pre-commit
- [ ] Reads stdin lines `<local-ref> <local-sha> <remote-ref> <remote-sha>` per git pre-push protocol
- [ ] For each line, computes `git diff --name-only <base> <local-sha> -- 'gks/feat/FEAT--*.md'`
- [ ] Handles new-branch case (`remote-sha == 0000...`) by falling back to merge-base with `origin/main` (or `<sha>~1` if that fails)
- [ ] For each touched FEAT path, runs `npx gks verify-flow <basename> --root=<repoRoot>`
- [ ] Aggregates non-OK exits → exit 1 at end
- [ ] Zero touched FEATs → exit 0 silently
- [ ] `git push --no-verify` bypasses (standard git behaviour)
- [ ] `install.sh` updated to install both hooks; idempotent on re-run; refuses non-MSP existing pre-push (same as pre-commit)
- [ ] `README.md` documents the new hook + behaviour
- [ ] Smoke test (`test/hooks/pre-push.test.ts`) spawns a temp repo with origin, stages a chain that breaks verify-flow, asserts the hook exits 1

## Surfaces

| Surface | Form |
|---|---|
| File | `examples/hooks/pre-push-verify.sh` |
| Installer | `examples/hooks/install.sh` (extended) |
| Docs | `examples/hooks/README.md` (extended) |
| Test | `test/hooks/pre-push.test.ts` |

## Out of scope

- Reverse traversal (find FEATs that depend on a touched ADR/CONCEPT) — `--all` flag in M6.
- Running the validator on touched files — pre-commit already does that.
- Pushing to multiple remotes — git calls the hook once per push regardless.

## Connections
- [[ADR--MSP-PREPUSH-HOOK]]
- [[CONCEPT--MSP-PREPUSH-HOOK]]
- [[FEAT--MSP-PRECOMMIT-HOOK]]

