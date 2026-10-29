"""Run cargo-audit without GitHub reporting or write permissions."""

import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
LOCKFILES = (
    "Cargo.lock",
    "studio/src-tauri/Cargo.lock",
    "genesisblock-mobile/src-tauri/Cargo.lock",
)


def main() -> int:
    policy = json.loads(
        (ROOT / ".github/security-advisory-exceptions.json").read_text(encoding="utf-8")
    )
    first_failure = 0
    for lockfile in LOCKFILES:
        command = ["cargo", "audit", "--file", lockfile]
        for advisory in sorted(policy):
            command.extend(["--ignore", advisory])
        result = subprocess.call(command, cwd=ROOT)
        if result and not first_failure:
            first_failure = result
    return first_failure


if __name__ == "__main__":
    sys.exit(main())
