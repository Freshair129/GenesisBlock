#!/usr/bin/env python3
"""Fail CI when a RustSec ignore is undocumented, stale, or past its removal gate."""

from __future__ import annotations

import datetime as dt
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
POLICY = ROOT / ".github" / "security-advisory-exceptions.json"
WORKFLOW = ROOT / ".github" / "workflows" / "security.yml"
CARGO = ROOT / "Cargo.toml"


def parse_version(text: str) -> tuple[int, int, int]:
    match = re.search(r'^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"', text, re.MULTILINE)
    if not match:
        raise ValueError("package version not found in Cargo.toml")
    return tuple(int(part) for part in match.groups())


def parse_ignore_ids(text: str) -> set[str]:
    match = re.search(r"^\s*ignore:\s*([^#\n]+)", text, re.MULTILINE)
    if not match:
        return set()
    return {item.strip() for item in match.group(1).split(",") if item.strip()}


def main() -> int:
    today = dt.date.today()
    policy = json.loads(POLICY.read_text(encoding="utf-8"))
    workflow_ids = parse_ignore_ids(WORKFLOW.read_text(encoding="utf-8"))
    policy_ids = set(policy)
    errors: list[str] = []

    if workflow_ids != policy_ids:
        errors.append(
            "security.yml ignore IDs do not match exception registry: "
            f"workflow_only={sorted(workflow_ids - policy_ids)} "
            f"registry_only={sorted(policy_ids - workflow_ids)}"
        )

    package_version = parse_version(CARGO.read_text(encoding="utf-8"))

    for advisory_id, entry in sorted(policy.items()):
        for key in ("package", "kind", "reason", "owner", "review_by", "tracking"):
            if not entry.get(key):
                errors.append(f"{advisory_id}: missing required field {key}")

        try:
            review_by = dt.date.fromisoformat(entry["review_by"])
        except (KeyError, ValueError):
            errors.append(f"{advisory_id}: review_by must be YYYY-MM-DD")
            continue

        if review_by < today:
            errors.append(
                f"{advisory_id}: exception expired on {review_by}; remove it or renew with evidence"
            )

        remove_at = entry.get("remove_at_version")
        if remove_at:
            try:
                threshold = tuple(int(part) for part in remove_at.split("."))
                if len(threshold) != 3:
                    raise ValueError
            except ValueError:
                errors.append(f"{advisory_id}: remove_at_version must be semver X.Y.Z")
                continue
            if package_version >= threshold:
                errors.append(
                    f"{advisory_id}: package version {package_version} reached removal gate {threshold}"
                )

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1

    print(
        f"security exception policy valid: {len(policy_ids)} exceptions, "
        f"package version {'.'.join(map(str, package_version))}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
