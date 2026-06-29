#!/usr/bin/env python3
"""Validate an Independent Benchmark result.json.

This is the trust gate. It must NOT silently pass an incomplete or overstated
report. It runs three layers of checks:

  1. Structural   — required envelope fields present and well-typed
                    (driven by benchmark/result_schema.json, stdlib validator).
  2. Common       — commit recorded, environment metadata present, repo clean
                    (unless --allow-dirty), latency metrics recorded, pass==true.
  3. Profile      — soak: reopen verified, total_nodes>0, disk recorded; and for
                    the 12h profile, duration_sec >= 43,200 s unless the run is
                    explicitly marked interrupted (in which case it is a FAIL,
                    never a silent pass).

Exit code: 0 only when the report is complete AND represents a successful run.
Any missing field, type error, dirty tree, failed/interrupted run, or short 12h
soak exits non-zero.

Usage:
    python benchmark/verify_report.py path/to/result.json
    python benchmark/verify_report.py result.json --allow-dirty
"""
from __future__ import annotations

import argparse
import json
import os
import sys

TWELVE_HOURS_SEC = 43_200
SCHEMA_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "result_schema.json")


# --------------------------------------------------------------------------- #
# Minimal JSON-Schema (draft-07 subset) validator — stdlib only, no pip deps.
# Supports: type (incl. unions + null), required, properties, minLength,
# minimum, items. Unknown keywords are ignored. Good enough for our schema.
# --------------------------------------------------------------------------- #
_TYPE_CHECKS = {
    "object": lambda v: isinstance(v, dict),
    "array": lambda v: isinstance(v, list),
    "string": lambda v: isinstance(v, str),
    "number": lambda v: isinstance(v, (int, float)) and not isinstance(v, bool),
    "integer": lambda v: isinstance(v, int) and not isinstance(v, bool),
    "boolean": lambda v: isinstance(v, bool),
    "null": lambda v: v is None,
}


def _type_ok(value, types) -> bool:
    if isinstance(types, str):
        types = [types]
    return any(_TYPE_CHECKS.get(t, lambda v: True)(value) for t in types)


def validate_schema(value, schema, path: str, errors: list[str]) -> None:
    if "type" in schema and not _type_ok(value, schema["type"]):
        errors.append(f"{path}: expected type {schema['type']}, got {type(value).__name__}")
        return
    if isinstance(value, str) and "minLength" in schema and len(value) < schema["minLength"]:
        errors.append(f"{path}: string shorter than minLength {schema['minLength']}")
    if isinstance(value, (int, float)) and not isinstance(value, bool) and "minimum" in schema:
        if value < schema["minimum"]:
            errors.append(f"{path}: {value} below minimum {schema['minimum']}")
    if isinstance(value, dict):
        for req in schema.get("required", []):
            if req not in value:
                errors.append(f"{path}: missing required field '{req}'")
        for key, subschema in schema.get("properties", {}).items():
            if key in value:
                validate_schema(value[key], subschema, f"{path}.{key}", errors)
    if isinstance(value, list) and "items" in schema:
        for i, item in enumerate(value):
            validate_schema(item, schema["items"], f"{path}[{i}]", errors)


def load_schema() -> dict:
    with open(SCHEMA_PATH, "r", encoding="utf-8") as f:
        return json.load(f)


# --------------------------------------------------------------------------- #
# Verification
# --------------------------------------------------------------------------- #
PLACEHOLDER_COMMITS = {"", "...", "none", "unknown"}


def is_soak(benchmark_id: str) -> bool:
    return benchmark_id.startswith("soak")


def is_twelve_hour(report: dict) -> bool:
    bid = report.get("benchmark_id", "")
    if bid == "soak_heavy_12h":
        return True
    target = (report.get("config") or {}).get("duration_target_sec")
    return isinstance(target, (int, float)) and target >= TWELVE_HOURS_SEC


def verify(report: dict, allow_dirty: bool = False) -> list[str]:
    """Return a list of failure messages. Empty list == PASS."""
    errors: list[str] = []

    # 1. Structural (schema) — only the soak schema is strict; graph/vector reuse
    #    the same envelope and are checked structurally for the common parts.
    schema = load_schema()
    bid = report.get("benchmark_id", "")
    if is_soak(bid):
        validate_schema(report, schema, "$", errors)
    else:
        # Common envelope: take the schema but drop soak-only required results.
        relaxed = json.loads(json.dumps(schema))
        relaxed["properties"]["results"]["required"] = ["pass"]
        validate_schema(report, relaxed, "$", errors)
    # If structure is broken, deeper semantic checks would be noise.
    if errors:
        return errors

    results = report.get("results", {})

    # 2. Common semantic checks.
    commit = str(report.get("commit", "")).strip().lower()
    if commit in PLACEHOLDER_COMMITS or len(commit) < 7:
        errors.append(f"commit not recorded or placeholder: {report.get('commit')!r}")

    env = report.get("environment", {})
    if not env.get("os"):
        errors.append("environment.os missing — no OS metadata")
    if not env.get("rustc"):
        errors.append("environment.rustc missing — toolchain not captured")
    if env.get("cpu") is None and env.get("ram_gb") is None:
        errors.append("environment has neither cpu nor ram_gb — insufficient hardware metadata")

    if report.get("repo_dirty") and not allow_dirty:
        errors.append("repo_dirty=true — run on a clean tree, or pass --allow-dirty to accept")

    if results.get("pass") is not True:
        errors.append("results.pass is not true — benchmark did not pass")

    if report.get("interrupted") is True:
        errors.append("run marked interrupted — not a complete benchmark")

    # latency metrics recorded (present and numeric)
    for key in ("query_latency_p50_ms", "query_latency_p95_ms", "query_latency_p99_ms"):
        v = results.get(key)
        if not isinstance(v, (int, float)) or isinstance(v, bool):
            errors.append(f"results.{key} not recorded")

    tn = results.get("total_nodes")
    if not isinstance(tn, int) or tn <= 0:
        errors.append(f"results.total_nodes must be > 0 (got {tn!r})")

    # 3. Profile-specific (soak).
    if is_soak(bid):
        if results.get("reopen_ok") is not True:
            errors.append("results.reopen_ok is not true — reopen/load verification did not pass")
        if results.get("final_disk_mb") is None:
            errors.append("results.final_disk_mb not recorded — disk-growth evidence missing")
        if is_twelve_hour(report):
            dur = report.get("duration_sec", 0)
            if not isinstance(dur, (int, float)) or dur < TWELVE_HOURS_SEC:
                errors.append(
                    f"12h profile ran only {dur}s (< {TWELVE_HOURS_SEC}s); "
                    "mark interrupted/fail rather than presenting as a complete 12h soak"
                )

    return errors


def main() -> int:
    ap = argparse.ArgumentParser(description="Verify a benchmark result.json.")
    ap.add_argument("report", help="path to result.json")
    ap.add_argument("--allow-dirty", action="store_true", help="accept a dirty working tree")
    ap.add_argument("--quiet", action="store_true", help="only print PASS/FAIL line")
    args = ap.parse_args()

    try:
        with open(args.report, "r", encoding="utf-8") as f:
            report = json.load(f)
    except (OSError, ValueError) as e:
        print(f"FAIL: cannot read/parse {args.report}: {e}")
        return 2

    errors = verify(report, allow_dirty=args.allow_dirty)
    bid = report.get("benchmark_id", "?")
    if errors:
        print(f"FAIL: {args.report} ({bid}) — {len(errors)} problem(s):")
        for e in errors:
            print(f"  - {e}")
        return 1
    if not args.quiet:
        print(f"PASS: {args.report} ({bid}) is a complete, clean, successful benchmark report.")
    else:
        print(f"PASS: {bid}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
