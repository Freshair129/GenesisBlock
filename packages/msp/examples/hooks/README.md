# MSP git hooks

Optional, opt-in. Install once per worktree.

## What's here

| File | Purpose |
|---|---|
| `pre-commit-validator.sh` | Runs `npm run msp:validate` on staged atom files; blocks the commit on hard-rule violations. |
| `pre-push-verify.sh` | Runs `gks verify-flow` on every FEAT touched in the push range; blocks pushes that leave a chain broken. |
| `install.sh` | Idempotent installer for both hooks; refuses to overwrite a non-MSP hook. |

## Install

```sh
# Idempotent — re-run any time to refresh both hooks.
bash examples/hooks/install.sh
```

Or manually:

```sh
cp examples/hooks/pre-commit-validator.sh .git/hooks/pre-commit
cp examples/hooks/pre-push-verify.sh      .git/hooks/pre-push
chmod +x .git/hooks/pre-commit .git/hooks/pre-push
```

## Behaviour

### pre-commit

Validates `.md` files matching:

- `gks/**/*.md`
- `.brain/msp/projects/<ns>/inbound/**/*.md`

For everything else (READMEs, source code, blueprints already in place, etc.) the hook is a no-op.

```
$ git add gks/concept/CONCEPT--BAD.md
$ git commit -m "..."
  ✗ /…/gks/concept/CONCEPT--BAD.md [forbidden-fields] frontmatter contains forbidden field 'commit_hash'
✗ MSP validator: 1 of 1 file(s) failed. Fix and re-stage, or use --no-verify to skip.
```

Skip: `git commit --no-verify`.

### pre-push

For every push range (`<remote-sha>..<local-sha>`), gathers FEAT atoms whose own file was touched and runs `gks verify-flow` on each.

```
$ git push
✓ MSP pre-push: 2 FEAT(s) verified.
```

When a chain is broken:

```
$ git push
  verify-flow FEAT--RATE-LIMIT
    visited: 3 atom(s)
    edges:   3 crosslink(s)
    status:  CHAIN-BROKEN  CONCEPT--RATE-LIMIT not stable
✗ MSP pre-push: 1 of 1 FEAT(s) failed verify-flow. Fix and re-push, or use --no-verify.
error: failed to push some refs to ...
```

Skip: `git push --no-verify`.

**Note**: pre-push only walks FEATs whose own file was touched, not reverse-traversal of every dependent. CI catches the rest.

## Skip (both hooks)

Standard git escape — no custom flag:

```sh
git commit --no-verify -m "..."
git push --no-verify
```

This is intentional: we don't invent a magic flag because reviewers expect `--no-verify` to mean what it always means.

## Uninstall

```sh
rm .git/hooks/pre-commit .git/hooks/pre-push
```

If you want to reinstall later, the installer is idempotent.

## Coexisting with other hooks

`install.sh` refuses to overwrite a non-MSP hook (it looks for the marker comments `msp:hook-marker:pre-commit-validator` and `msp:hook-marker:pre-push-verify`). If you already have a pre-commit/pre-push hook from another tool, do one of:

1. **Merge manually** — paste the contents of `pre-commit-validator.sh` / `pre-push-verify.sh` into your existing hook.
2. **Chain via your hook manager** — most tools (husky, lefthook) support multiple commands per hook; add `bash $REPO_ROOT/examples/hooks/<name>.sh` to your config.

## Why bash, not husky

See `gks/adr/ADR--MSP-PRECOMMIT-HOOK.md` and `gks/adr/ADR--MSP-PREPUSH-HOOK.md` — short version: zero new dependencies, single grep-able file per hook, works on any platform with Git Bash.
