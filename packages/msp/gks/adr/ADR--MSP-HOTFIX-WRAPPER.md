---
id: ADR--MSP-HOTFIX-WRAPPER
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Thin passthrough scripts + pre-commit gate that calls gks hotfix check
tags:
  - msp
  - hotfix
  - decision
  - hook
crosslinks: {"references":["CONCEPT--MSP-HOTFIX-WRAPPER","ADR--HOTFIX-ESCAPE-HATCH","FEAT--MSP-PRECOMMIT-HOOK"]}
created_at: 2026-05-03T10:45:49.473Z
---

# ADR — hotfix wrapper shape

## Context

Two surfaces to design:

1. **CLI surface**: how do users invoke `gks hotfix` from MSP-namespace scripts?
2. **Hook integration**: when does the pre-commit hook invoke `gks hotfix check`?

For (1) the trade-off is between adding logic on top of `gks hotfix` (e.g. auto-derive SHA, format output, integrate with reviewer per `ADR--HUMAN-REVIEW-GATES`) versus thin passthrough.

For (2) the trade-off is between checking every staged file (cheap but noisy) versus only files known to have associated HOTFIX-- atoms (cleaner but requires lookup).

## Decision

### (1) Thin passthrough npm scripts

```json
"msp:hotfix:open":  "gks hotfix open",
"msp:hotfix:list":  "gks hotfix list",
"msp:hotfix:close": "gks hotfix close",
"msp:hotfix:check": "gks hotfix check"
```

No wrapper logic. Discoverable via `npm run`. All flags pass through. If we later need auto-SHA detection or close-time reviewer enforcement, we can replace a single script with a `node scripts/msp/hotfix-X.mjs` shim — no atom-graph churn required.

### (2) Pre-commit hook calls `gks hotfix check` once with all staged paths

After the existing validator pass, the pre-commit hook:

```bash
hotfix_paths=$(git diff --cached --name-only --diff-filter=ACMR \
  | grep -v -E '^(gks/|\.brain/|\.github/|examples/|scripts/|test/)' || true)

if [ -n "$hotfix_paths" ]; then
  args=()
  while IFS= read -r p; do args+=(--file="$p"); done <<< "$hotfix_paths"
  if ! npx gks hotfix check "${args[@]}" >/dev/null 2>&1; then
    npx gks hotfix check "${args[@]}" 2>&1 | sed 's/^/  /'
    echo "✗ MSP hotfix check failed. Backfill the HOTFIX-- atom or use --no-verify."
    exit 1
  fi
fi
```

Why these path exclusions:

- `gks/` — atoms; not subject to hotfix windows
- `.brain/` — runtime state
- `.github/`, `examples/`, `scripts/`, `test/` — infra; rarely under hotfix gate

Everything else (`src/`, root config, etc.) is candidate for hotfix gating.

### Bypass

Standard `git commit --no-verify`. No new flag.

## Consequences

**Positive**
- Wrapper is one-line-per-command; trivial to maintain.
- Pre-commit gate makes the 48 h timer a real wall, not paperwork.
- Hot path skipped (no hotfix atoms → no extra fork/exec).
- Single `gks hotfix check` invocation per commit (cheap — under 100 ms even with many files).

**Negative**
- The path-exclusion list is hardcoded. If a project keeps source under a non-`src/` root (e.g. `lib/`, `app/`), we still gate it. Acceptable — that's the desired default.
- Thin passthrough means a `gks hotfix` interface change ripples through. Mitigation: `gks hotfix` is in GksV3's stable surface.

## Alternatives considered

1. **Wrap with auto-SHA / reviewer enforcement.** Considered. Adds value but couples MSP to git internals (auto-SHA) and to `ADR--HUMAN-REVIEW-GATES` (reviewer). Defer to M6 if needed.
2. **Hook walks atom index for HOTFIX-- atoms first; only checks files those atoms reference.** Considered. More targeted but doubles the hook latency (parse + filter atoms) for marginal gain. The bulk path-exclusion is good enough.
3. **No hook integration; just expose CLI.** Rejected — the whole point is closing the contract decay loop locally.

## Source

`CONCEPT--MSP-HOTFIX-WRAPPER` + GksV3 `gks hotfix` CLI surface.
