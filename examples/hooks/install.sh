#!/usr/bin/env bash
# MSP pre-commit hook installer — idempotent.
# Refuses to overwrite a non-MSP pre-commit hook.

set -euo pipefail

MARKER='msp:hook-marker:pre-commit-validator'

# 1. Must run inside a git repo.
if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "✗ install.sh: not inside a git repository" >&2
  exit 1
fi

REPO_ROOT=$(git rev-parse --show-toplevel)
GIT_HOOK_DIR=$(git rev-parse --git-path hooks)
SOURCE="$REPO_ROOT/examples/hooks/pre-commit-validator.sh"
TARGET="$GIT_HOOK_DIR/pre-commit"

if [ ! -f "$SOURCE" ]; then
  echo "✗ install.sh: source not found at $SOURCE" >&2
  exit 1
fi

# 2. Decide what to do based on the existing hook (if any).
if [ -e "$TARGET" ]; then
  if grep -q "$MARKER" "$TARGET" 2>/dev/null; then
    echo "✓ MSP hook already installed; refreshing."
  else
    echo "✗ install.sh: $TARGET exists and is not an MSP hook." >&2
    echo "  Inspect it, then either:" >&2
    echo "    - delete it manually and re-run this installer, OR" >&2
    echo "    - merge our pre-commit-validator.sh contents into yours." >&2
    exit 1
  fi
fi

# 3. Install + chmod.
mkdir -p "$GIT_HOOK_DIR"
cp "$SOURCE" "$TARGET"
chmod +x "$TARGET"

echo "✓ MSP pre-commit hook installed at $TARGET"
echo "  Skip per-commit with: git commit --no-verify"
echo "  Uninstall with:       rm $TARGET"
