---
id: ADR--MSP-PRECOMMIT-HOOK
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Pre-commit hook is a portable bash script under examples/hooks/, opt-in install
tags: &a1
  - msp
  - precommit
  - hook
  - decision
  - bash
crosslinks: &a2
  references:
    - CONCEPT--MSP-PRECOMMIT-HOOK
    - FEAT--MSP-VALIDATOR
created_at: 2026-05-03T14:39:04.838+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--MSP-PRECOMMIT-HOOK
  phase: 2
  type: adr
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Pre-commit hook is a portable bash script under examples/hooks/, opt-in
    install
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T14:39:04.838+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--MSP-PRECOMMIT-HOOK
    phase: 2
    type: adr
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Pre-commit hook is a portable bash script under examples/hooks/, opt-in
      install
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T14:39:04.838+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--MSP-PRECOMMIT-HOOK
      phase: 2
      type: adr
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Pre-commit hook is a portable bash script under examples/hooks/, opt-in
        install
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-03T14:39:04.838+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Architecture decision record
      attributes:
        domain: adr
      domain: adr
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — pre-commit hook implementation shape

## Context

A pre-commit hook can be implemented as:

1. **Bash script** copied into `.git/hooks/pre-commit` by the user.
2. **husky / simple-git-hooks** managed by `package.json` (`"prepare": "husky install"`).
3. **A node script** invoked by a thin bash shim.
4. **Lefthook / pre-commit (the Python tool)** managed by a YAML config.

Each has trade-offs around portability, install friction, and dep weight.

## Decision

**Plain bash script under `examples/hooks/pre-commit-validator.sh`**, opt-in install via a one-line copy or via `examples/hooks/install.sh`. No new runtime dependencies on `package.json`.

### Why bash, not husky

- **Zero new dependencies.** husky pulls in itself (~5 KB) but more importantly creates an opinion about hook management (the `husky/_/` shim, lifecycle scripts in `package.json`). For a single hook on a small repo, that's overkill.
- **Portability.** Bash hooks work identically on Linux, macOS, and Git Bash on Windows. node-based hook would need `node` on PATH at hook time which is usually true but adds a startup latency we can avoid for the no-files-staged early-exit path.
- **Discoverability.** A bash script under `examples/hooks/` is grep-able and reviewable in a single `cat`. husky obscures the actual hook behind framework layers.
- **Skip path uses standard `--no-verify`** — the ADR-level decision that we never invent a custom skip flag.

### Algorithm

```bash
#!/usr/bin/env bash
set -euo pipefail

# 1. List staged .md files matching the patterns we care about.
staged=$(git diff --cached --name-only --diff-filter=ACMR | \
         grep -E '^(gks/.*\.md|\.brain/msp/projects/[^/]+/inbound/.*\.md)$' || true)

# 2. Early exit if none staged.
[ -z "$staged" ] && exit 0

# 3. Run validator on each. Accumulate failures.
fail=0
while IFS= read -r f; do
  if ! npm run msp:validate --silent -- "$f" >/dev/null 2>&1; then
    fail=$((fail + 1))
    npm run msp:validate --silent -- "$f" 2>&1 | sed "s/^/  /"
  fi
done <<< "$staged"

# 4. Exit accordingly.
if [ "$fail" -gt 0 ]; then
  echo "✗ MSP validator: $fail file(s) failed. Fix and re-stage, or use --no-verify to skip."
  exit 1
fi

echo "✓ MSP validator: $(echo "$staged" | wc -l) file(s) passed."
```

### Install paths

Two options documented in `examples/hooks/README.md`:

```sh
# Option A — manual (most explicit)
cp examples/hooks/pre-commit-validator.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit

# Option B — install script (idempotent; refuses if a non-MSP hook already exists)
bash examples/hooks/install.sh
```

`install.sh` writes a small marker comment so re-runs are idempotent and so it can detect hooks installed by other tools without overwriting them.

## Consequences

**Positive**

- Zero new dependencies on `package.json`.
- Works identically across platforms supporting Git Bash.
- The script is the documentation — anyone can `cat` it and understand the contract.
- Validator's existing exit-code contract (0 pass / 1 hard / 2 internal) translates 1:1.

**Negative**

- Each developer must install the hook manually. Mitigated by a one-line install helper + a README badge.
- No automatic enforcement on Windows users without Git Bash. Acceptable — they get CI as a fallback.

## Alternatives considered

1. **husky.** Rejected per dep weight + opinion overhead.
2. **simple-git-hooks.** Lighter than husky but still adds `package.json` config. Bash + opt-in is even simpler.
3. **Pre-push hook instead of pre-commit.** Rejected — pre-commit blocks bad work earlier; pre-push lets the commit land then surprises you at push time.
4. **Run the validator from inside the hook via `node` directly (skipping `npm`).** Considered. Faster startup (~50ms) but couples the hook to an absolute path. `npm run msp:validate` lets `package.json` indirect the runner.

## Source

`[[CONCEPT--MSP-PRECOMMIT-HOOK]]` + `[[FEAT--MSP-VALIDATOR]]` exit-code contract.
