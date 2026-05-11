#!/usr/bin/env bash
# MSP git hooks installer — idempotent.
# Installs both pre-commit and pre-push hooks.
# Refuses to overwrite a non-MSP hook of either kind.

set -euo pipefail

# 1. Must run inside a git repo.
if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "✗ install.sh: not inside a git repository" >&2
  exit 1
fi

REPO_ROOT=$(git rev-parse --show-toplevel)
GIT_HOOK_DIR=$(git rev-parse --git-path hooks)
mkdir -p "$GIT_HOOK_DIR"

# Per-hook table: <hook-name> <source-relative-to-repo> <marker>
HOOKS=(
  "pre-commit examples/hooks/pre-commit-validator.sh msp:hook-marker:pre-commit-validator"
  "pre-push examples/hooks/pre-push-verify.sh        msp:hook-marker:pre-push-verify"
)

INSTALLED=0
SKIPPED=0

for entry in "${HOOKS[@]}"; do
  read -r hook_name rel_source marker <<< "$entry"
  source="$REPO_ROOT/$rel_source"
  target="$GIT_HOOK_DIR/$hook_name"

  if [ ! -f "$source" ]; then
    echo "✗ install.sh: source not found at $source" >&2
    exit 1
  fi

  if [ -e "$target" ]; then
    if grep -q "$marker" "$target" 2>/dev/null; then
      cp "$source" "$target"
      chmod +x "$target"
      echo "✓ refreshed $hook_name hook"
      INSTALLED=$((INSTALLED + 1))
      continue
    else
      echo "⚠ $hook_name: $target exists and is not an MSP hook — skipping" >&2
      echo "  Inspect it; either delete + re-run this installer, or merge our $rel_source into yours." >&2
      SKIPPED=$((SKIPPED + 1))
      continue
    fi
  fi

  cp "$source" "$target"
  chmod +x "$target"
  echo "✓ installed $hook_name → $target"
  INSTALLED=$((INSTALLED + 1))
done

echo
echo "Summary: $INSTALLED installed/refreshed, $SKIPPED skipped."
echo "Skip per-commit / per-push with: --no-verify"
echo "Uninstall: rm $GIT_HOOK_DIR/{pre-commit,pre-push}"

# Exit 1 if anything was skipped due to existing non-MSP hook
if [ "$SKIPPED" -gt 0 ]; then
  exit 1
fi
