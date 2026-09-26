"""Run cargo-audit without GitHub reporting or write permissions."""

import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    policy = json.loads(
        (ROOT / ".github/security-advisory-exceptions.json").read_text(encoding="utf-8")
    )
    command = ["cargo", "audit", "--file", "Cargo.lock"]
    for advisory in sorted(policy):
        command.extend(["--ignore", advisory])
    return subprocess.call(command, cwd=ROOT)


if __name__ == "__main__":
    sys.exit(main())
