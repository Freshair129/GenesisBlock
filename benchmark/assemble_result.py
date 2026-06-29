#!/usr/bin/env python3
"""Assemble a public, schema-conformant result.json for one benchmark run.

The engine-side harnesses (soak/graph/vector) emit only what they can *observe*
(cycles, nodes, disk, latency percentiles, recall, reopen timing, timestamps) to
a partial "metrics" JSON. This tool wraps that with the things the benchmark must
NOT self-report — the git commit, dirty-tree status, package version, host
environment (from collect_env.py), and an externally measured peak-RAM figure —
into the final result.json defined by result_schema.json.

It never invents numbers: every value comes from the metrics file, env.json, git,
or explicit CLI input. Stdlib only.

Usage:
    python benchmark/assemble_result.py \
        --metrics run/metrics.json --env run/env.json \
        --out run/result.json \
        --raw-log run/raw.log --stderr-log run/stderr.log --summary run/summary.md \
        --peak-ram-mb 5123
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys


def _git(args: list[str], repo: str) -> str | None:
    try:
        out = subprocess.run(
            ["git", "-C", repo, *args], capture_output=True, text=True, timeout=30
        )
        if out.returncode == 0:
            return out.stdout.strip()
    except (OSError, subprocess.SubprocessError):
        pass
    return None


def detect_version(repo: str) -> str:
    # Prefer Cargo.toml [package] version (engine SSOT), fall back to package.json.
    cargo = os.path.join(repo, "Cargo.toml")
    try:
        in_pkg = False
        with open(cargo, "r", encoding="utf-8") as f:
            for line in f:
                s = line.strip()
                if s.startswith("["):
                    in_pkg = s == "[package]"
                elif in_pkg and s.startswith("version"):
                    return s.split("=", 1)[1].strip().strip('"')
    except OSError:
        pass
    pkg = os.path.join(repo, "package.json")
    try:
        with open(pkg, "r", encoding="utf-8") as f:
            return json.load(f).get("version", "unknown")
    except (OSError, ValueError):
        return "unknown"


def load_json(path: str) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _fmt(v) -> str:
    if v is None:
        return "n/a"
    if isinstance(v, bool):
        return "yes" if v else "no"
    return str(v)


def render_summary(result: dict, template_path: str, out_path: str) -> None:
    """Fill report_template.md placeholders from the assembled result."""
    try:
        with open(template_path, "r", encoding="utf-8") as f:
            tmpl = f.read()
    except OSError as e:
        print(f"warning: cannot read template {template_path}: {e}", file=sys.stderr)
        return
    env = result.get("environment", {})
    res = result.get("results", {})
    mapping = {
        "benchmark_id": result.get("benchmark_id"),
        "project": result.get("project"),
        "pass": "PASS" if res.get("pass") else "FAIL",
        "commit": result.get("commit"),
        "repo_dirty": _fmt(result.get("repo_dirty")),
        "version": result.get("version"),
        "interrupted": _fmt(result.get("interrupted")),
        "timestamp_start": result.get("timestamp_start"),
        "timestamp_end": result.get("timestamp_end"),
        "duration_sec": result.get("duration_sec"),
        "os": _fmt(env.get("os")),
        "cpu": _fmt(env.get("cpu")),
        "ram_gb": _fmt(env.get("ram_gb")),
        "disk": _fmt(env.get("disk")),
        "rustc": _fmt(env.get("rustc")),
        "cargo": _fmt(env.get("cargo")),
        "total_nodes": _fmt(res.get("total_nodes")),
        "cycles": _fmt(res.get("cycles")),
        "peak_ram_mb": _fmt(res.get("peak_ram_mb")),
        "final_disk_mb": _fmt(res.get("final_disk_mb")),
        "recall_miss_rate": _fmt(res.get("recall_miss_rate")),
        "query_p50": _fmt(res.get("query_latency_p50_ms")),
        "query_p95": _fmt(res.get("query_latency_p95_ms")),
        "query_p99": _fmt(res.get("query_latency_p99_ms")),
        "ingest_p50": _fmt(res.get("ingest_latency_p50_ms")),
        "ingest_p95": _fmt(res.get("ingest_latency_p95_ms")),
        "reopen_ok": _fmt(res.get("reopen_ok")),
        "reopen_load_sec": _fmt(res.get("reopen_load_sec")),
        "config_json": json.dumps(result.get("config", {}), indent=2),
        "result_json": result.get("artifacts", {}).get("result_json", "result.json"),
    }
    for key, val in mapping.items():
        tmpl = tmpl.replace("{{" + key + "}}", str(val))
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(tmpl)
    print(f"summary rendered -> {out_path}")


def main() -> int:
    ap = argparse.ArgumentParser(description="Assemble result.json from metrics + env + git.")
    ap.add_argument("--metrics", required=True, help="partial metrics JSON from the benchmark")
    ap.add_argument("--env", required=True, help="env.json from collect_env.py")
    ap.add_argument("--out", required=True, help="output result.json path")
    ap.add_argument("--benchmark-id", default=None, help="override benchmark_id")
    ap.add_argument("--repo-root", default=".", help="git repo root")
    ap.add_argument("--peak-ram-mb", default=None, help="externally measured peak RSS in MB (or 'null')")
    ap.add_argument("--raw-log", default=None)
    ap.add_argument("--stderr-log", default=None)
    ap.add_argument("--summary", default=None, help="render a human-readable summary.md here")
    ap.add_argument(
        "--template",
        default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "report_template.md"),
        help="summary template (default: benchmark/report_template.md)",
    )
    args = ap.parse_args()

    metrics = load_json(args.metrics)
    env = load_json(args.env)
    repo = args.repo_root

    commit = _git(["rev-parse", "HEAD"], repo) or ""
    dirty_out = _git(["status", "--porcelain"], repo)
    repo_dirty = bool(dirty_out) if dirty_out is not None else False

    results = dict(metrics.get("results", {}))
    # Merge externally measured peak RAM only when the harness left it null.
    if args.peak_ram_mb is not None and args.peak_ram_mb.lower() != "null":
        try:
            ram = float(args.peak_ram_mb)
            if results.get("peak_ram_mb") in (None, 0):
                results["peak_ram_mb"] = ram
        except ValueError:
            print(f"warning: ignoring non-numeric --peak-ram-mb={args.peak_ram_mb}", file=sys.stderr)

    # environment block in the schema's canonical key order (extra keys kept).
    environment = {
        "os": env.get("os"),
        "cpu": env.get("cpu"),
        "ram_gb": env.get("ram_gb"),
        "disk": env.get("disk"),
        "rustc": env.get("rustc"),
        "cargo": env.get("cargo"),
    }
    for k, v in env.items():
        if k not in environment and not k.startswith("_"):
            environment[k] = v

    result = {
        "benchmark_id": args.benchmark_id or metrics.get("benchmark_id", "unknown"),
        "project": metrics.get("project", "GenesisBlockDB"),
        "commit": commit,
        "repo_dirty": repo_dirty,
        "interrupted": bool(metrics.get("interrupted", False)),
        "version": detect_version(repo),
        "timestamp_start": metrics.get("timestamp_start"),
        "timestamp_end": metrics.get("timestamp_end"),
        "duration_sec": metrics.get("duration_sec", 0),
        "environment": environment,
        "config": metrics.get("config", {}),
        "results": results,
        "artifacts": {
            "raw_log": args.raw_log,
            "stderr_log": args.stderr_log,
            "summary_markdown": args.summary,
            "result_json": args.out,
            "env_json": args.env,
        },
    }

    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2)
    print(f"result assembled -> {args.out}")
    print(f"  commit={commit[:12]} dirty={repo_dirty} version={result['version']}")
    print(f"  benchmark_id={result['benchmark_id']} pass={results.get('pass')}")

    if args.summary:
        render_summary(result, args.template, args.summary)
    return 0


if __name__ == "__main__":
    sys.exit(main())
