# MSP git hooks

Optional, opt-in. Install once per worktree.

## What's here

| File | Purpose |
|---|---|
| `pre-commit-validator.sh` | Runs `npm run msp:validate` on staged atom files; blocks the commit on hard-rule violations. |
| `install.sh` | Idempotent installer; refuses to overwrite a non-MSP hook. |

## Install

```sh
# Idempotent — re-run any time to refresh.
bash examples/hooks/install.sh
```

Or manually:

```sh
cp examples/hooks/pre-commit-validator.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Behaviour

The hook only validates `.md` files matching:

- `gks/**/*.md` (canonical atoms)
- `.brain/msp/projects/<ns>/inbound/**/*.md` (candidates awaiting review)

For everything else (READMEs, source code, blueprints already in place, etc.) the hook is a no-op.

### Pass

```
$ git add gks/concept/CONCEPT--FOO.md
$ git commit -m "..."
✓ MSP validator: 1 file(s) passed.
[main abc123] ...
```

### Fail

```
$ git add gks/concept/CONCEPT--BAD.md
$ git commit -m "..."
  ✗ /…/gks/concept/CONCEPT--BAD.md [forbidden-fields] frontmatter contains forbidden field 'commit_hash'
✗ MSP validator: 1 of 1 file(s) failed. Fix and re-stage, or use --no-verify to skip.
```

Fix the file, re-stage, commit again.

## Skip

Standard git escape — no custom flag:

```sh
git commit --no-verify -m "..."
```

This is intentional: we don't invent a magic flag for the MSP hook because reviewers expect `--no-verify` to mean what it always means.

## Uninstall

```sh
rm .git/hooks/pre-commit
```

If you want to reinstall later, the installer is idempotent.

## Coexisting with other hooks

`install.sh` refuses to overwrite a non-MSP hook (it looks for the marker comment `msp:hook-marker:pre-commit-validator`). If you already have a pre-commit hook from another tool (husky, lefthook, custom), do one of:

1. **Merge manually** — paste the contents of `pre-commit-validator.sh` into your existing hook.
2. **Chain via your hook manager** — most tools (husky, lefthook) support multiple commands per hook; add `bash $REPO_ROOT/examples/hooks/pre-commit-validator.sh` to your config.

## Why bash, not husky

See `gks/adr/ADR--MSP-PRECOMMIT-HOOK.md` — short version: zero new dependencies, single grep-able file, works on any platform with Git Bash.
