---
id: ADR--MSP-PREPUSH-HOOK
phase: 2
type: adr
status: stable
vault_id: default
title: Pre-push hook walks touched FEAT files only (no reverse traversal)
tags:
  - msp
  - prepush
  - hook
  - decision
crosslinks: {"references":["CONCEPT--MSP-PREPUSH-HOOK","FEAT--MSP-VALIDATOR","ADR--MSP-PRECOMMIT-HOOK"]}
created_at: 2026-05-03T10:39:27.995Z
---

# ADR — pre-push hook scope

## Context

Three reasonable scopes for a pre-push verify gate:

1. **Touched FEATs only** — only FEATs whose own `gks/feat/FEAT--*.md` file was modified in the push range. Cheap, partial coverage.
2. **Reverse traversal** — for every touched ADR/CONCEPT/BLUEPRINT, find every FEAT that references it (via crosslinks or backlinks.jsonl), and run verify-flow on all. Complete coverage, more expensive.
3. **All FEATs** — run verify-flow on every FEAT in `gks/feat/` regardless of touch. Trivially complete, slowest.

Cost matters: pre-push runs on every `git push`. For a project with 50 FEATs and a 200-ms verify-flow per FEAT, option 3 = 10 s on every push. Option 2 amortises into option 3 in the worst case (touching a foundational FRAME).

## Decision

**Option 1 — touched FEATs only**, plus an explicit `--all` opt-in flag for the ambitious.

### Stdin format (git pre-push protocol)

```
<local-ref> <local-sha> <remote-ref> <remote-sha>
```

Multiple lines for multi-ref pushes. We process each.

### Algorithm

```bash
while read local_ref local_sha remote_ref remote_sha; do
  if [ "$remote_sha" = "0000000000000000000000000000000000000000" ]; then
    # New branch: diff against the merge base with the default branch.
    base=$(git merge-base "$local_sha" origin/main 2>/dev/null || echo "$local_sha~1")
  else
    base="$remote_sha"
  fi
  changed=$(git diff --name-only "$base" "$local_sha" -- 'gks/feat/FEAT--*.md')
  for path in $changed; do
    feat=$(basename "$path" .md)
    npx gks verify-flow "$feat" --root="$REPO_ROOT" || FAIL=1
  done
done
exit $FAIL
```

### Bypass

Standard `git push --no-verify`. We do not invent a custom flag — same precedent as `ADR--MSP-PRECOMMIT-HOOK`.

### Install

Same shape as `pre-commit-validator.sh`: a script under `examples/hooks/pre-push-verify.sh` + an updated `install.sh` that installs both hooks (idempotent, marker-comment-protected).

## Consequences

**Positive**
- Fast: typical push touches 0–2 FEATs → 0–400 ms.
- Cheap to reason about: each push range is checked once.
- Composable: orchestrator can run option 2 (reverse traversal) in CI for completeness.

**Negative**
- A push that only touches an ADR (not its dependent FEAT file) won't catch the chain break. Mitigation: dependent FEATs usually get touched together in a single PR; CI catches the rest.
- Stdin parsing is fragile if a hook gets called outside `git push` (e.g. a tool that mimics the protocol). Acceptable — non-git callers don't matter.

## Alternatives considered

1. **Reverse traversal.** Rejected for default; offered as `--all` opt-in (planned for M6 — not in M5a).
2. **Run validator (re-validate atoms).** Rejected — pre-commit already does that; pre-push is the chain gate.
3. **Use `git rev-parse @{upstream}` to find the base.** Considered. The git pre-push stdin already gives us `remote_sha`; using it is more correct (handles non-tracking branches).

## Source

`CONCEPT--MSP-PREPUSH-HOOK` + git pre-push hook docs.
