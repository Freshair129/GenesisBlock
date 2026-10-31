#!/usr/bin/env python3
"""Run exactly one configured 100k case with the frozen benchmark binary.

This runner does not build the binary, copy a database, reset the OS file cache,
or impose its own timeout/resource-admission policy. The parent guard owns the
1,200-second, 2-GiB, and CPU-admission limits. A before-case query failure is an
expected observation, never a PASS or retrieval-quality qualification.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

RNEW_ROOT = Path(__file__).resolve().parent
CHUNK_BYTES = 1024 * 1024
EXPECTED_BY_CASE = {
    "before": ("error", "error"),
    "after": ("ok", "ok"),
}


def under(path: Path, root: Path) -> bool:
    path_s = os.path.normcase(os.path.realpath(os.fspath(path)))
    root_s = os.path.normcase(os.path.realpath(os.fspath(root)))
    try:
        return os.path.commonpath((path_s, root_s)) == root_s
    except ValueError:
        return False


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(CHUNK_BYTES):
            digest.update(chunk)
    return digest.hexdigest()


def tree_summary(root: Path) -> dict[str, Any]:
    entries: list[tuple[str, int, str]] = []
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix().casefold()):
        if path.is_symlink():
            raise RuntimeError(f"input tree contains a symlink: {path.name}")
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        size = path.stat().st_size
        entries.append((relative, size, sha256_file(path)))
    aggregate = hashlib.sha256()
    for relative, size, digest in entries:
        aggregate.update(f"{relative}\0{size}\0{digest}\n".encode("utf-8"))
    return {
        "file_count": len(entries),
        "byte_count": sum(size for _, size, _ in entries),
        "tree_sha256": aggregate.hexdigest(),
    }


def write_json_atomic(path: Path, value: Any) -> None:
    temporary = path.with_name(f"{path.name}.{os.getpid()}.{time.time_ns()}.tmp")
    with temporary.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    temporary.replace(path)


def read_query_state(result: Any) -> tuple[str | None, dict[str, Any]]:
    if not isinstance(result, dict):
        return None, {}
    query0 = result.get("query0")
    if not isinstance(query0, dict):
        return None, {}
    state = "error" if "error" in query0 else "ok"
    selected = {
        key: query0.get(key)
        for key in ("result_count_min", "result_count_max", "recall", "p50_us", "p95_us", "p99_us")
        if key in query0
    }
    selected["error_present"] = "error" in query0
    return state, selected


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True, type=Path, help="JSON config for one before/after case")
    args_ns = parser.parse_args()
    config_path = args_ns.config.resolve(strict=True)
    config = json.loads(config_path.read_text(encoding="utf-8-sig"))

    binary = Path(config["binary_path"]).resolve(strict=True)
    expected_binary_hash = str(config["binary_sha256"]).lower()
    if len(expected_binary_hash) != 64 or any(ch not in "0123456789abcdef" for ch in expected_binary_hash):
        raise RuntimeError("config binary_sha256 must be a 64-character SHA-256 hex digest")
    actual_binary_hash = sha256_file(binary)
    if actual_binary_hash != expected_binary_hash:
        raise RuntimeError("frozen binary SHA-256 does not match config")

    case_path = Path(config["case_path"]).resolve(strict=True)
    cases_root = (RNEW_ROOT / "100k-cases").resolve(strict=True)
    if not under(case_path, cases_root):
        raise RuntimeError("case path must remain under Rnew\\100k-cases")
    case_kind = case_path.name.lower()
    if case_kind not in EXPECTED_BY_CASE:
        raise RuntimeError("case directory must be named before or after")
    expected_engine_state, expected_query0_state = EXPECTED_BY_CASE[case_kind]
    if config.get("expected_engine_state") != expected_engine_state:
        raise RuntimeError(f"{case_kind} config expected_engine_state must be {expected_engine_state!r}")
    if config.get("expected_query0_state") != expected_query0_state:
        raise RuntimeError(f"{case_kind} config expected_query0_state must be {expected_query0_state!r}")

    db_path = (case_path / "gdb").resolve(strict=True)
    if not under(db_path, cases_root):
        raise RuntimeError("case gdb must remain under Rnew\\100k-cases")
    if not db_path.is_dir() or not (db_path / "projection.sqlite").is_file():
        raise RuntimeError("case gdb or projection.sqlite is missing")
    input_path = Path(config["input_path"]).resolve(strict=True)
    if not input_path.is_dir():
        raise RuntimeError("input_path must be a directory")
    for required in ("meta.json", "corpus.f32", "queries.f32", "truth.json"):
        if not (input_path / required).is_file():
            raise RuntimeError(f"input fixture is missing {required}")
    input_meta = json.loads((input_path / "meta.json").read_text(encoding="utf-8-sig"))
    if input_meta.get("n") != 100000 or input_meta.get("dim") != 1024:
        raise RuntimeError("input fixture metadata must be n=100000 and dim=1024")

    extra_args = config.get("args", [])
    if not isinstance(extra_args, list) or not all(isinstance(item, str) for item in extra_args):
        raise RuntimeError("config args must be a list of strings")
    result_path = case_path / "result.json"
    stdout_path = case_path / "stdout.log"
    stderr_path = case_path / "stderr.log"
    receipt_path = case_path / "run-receipt.json"
    for artifact in (result_path, stdout_path, stderr_path, receipt_path):
        if artifact.exists() or artifact.is_symlink():
            raise RuntimeError(f"refusing existing run artifact: {artifact.name}")

    profile = {
        "GB_VBENCH": str(input_path),
        "GB_MEMORY_AUDIT": "1",
        "GB_MEMORY_INPUT": str(input_path),
        "GB_MEMORY_OPEN_ONLY": str(db_path),
        "GB_MEMORY_OPEN_OUT": str(result_path),
        "GB_MEMORY_OPEN_READ_WRITE": "1",
        "GB_MEMORY_OPEN_REPEAT": "2",
        "GB_MEMORY_OPEN_QUERY_SAMPLES": "24",
        "GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN": "1",
        "GB_VECTOR_QUERY_DIAGNOSTICS": "1",
        "RAYON_NUM_THREADS": "4",
        "GB_EF": "200",
        "GB_QUANT": "bq",
        "GB_RERANK": "0",
        "RETENTION": "full",
        "GB_MEMORY_RSS_LIMIT_BYTES": str(2 * 1024**3),
        "GB_VEC_Q": "24",
    }
    child_env = os.environ.copy()
    for name in tuple(child_env):
        if name.upper().startswith("GB_") or name.upper() in {"RAYON_NUM_THREADS", "RETENTION"}:
            child_env.pop(name, None)
    child_env.update(profile)
    cwd = Path(config.get("working_directory", str(case_path))).resolve(strict=True)
    if not cwd.is_dir():
        raise RuntimeError("working_directory must be an existing directory")

    input_before = tree_summary(input_path)
    args_digest = hashlib.sha256("\0".join(extra_args).encode("utf-8")).hexdigest()
    started = datetime.now(timezone.utc).isoformat()
    receipt: dict[str, Any] = {
        "schema_version": 1,
        "host": platform.node(),
        "case_kind": case_kind,
        "case_path": str(case_path),
        "database_path": str(db_path),
        "binary_path": str(binary),
        "binary_sha256": actual_binary_hash,
        "argument_count": len(extra_args),
        "arguments_sha256": args_digest,
        "working_directory": str(cwd),
        "input_path": str(input_path),
        "source_input_before": input_before,
        "source_input_after": None,
        "input_unchanged": None,
        "expected_engine_state": expected_engine_state,
        "expected_query0_state": expected_query0_state,
        "benchmark_environment_profile": profile,
        "started_utc": started,
        "finished_utc": None,
        "stdout_path": str(stdout_path),
        "stderr_path": str(stderr_path),
        "result_path": str(result_path),
        "process": {
            "state": "launching",
            "engine_pid": None,
            "launcher_pid": os.getpid(),
            "exit_code": None,
        },
        "engine_state": None,
        "query0_state": None,
        "query0_metrics": {},
        "expected_observation": False,
        "case_disposition": "not_qualification",
        "qualification_claimed": False,
    }
    write_json_atomic(receipt_path, receipt)

    exit_code: int | None = None
    process = None
    command = [str(binary), *extra_args]
    try:
        with stdout_path.open("xb") as stdout_file, stderr_path.open("xb") as stderr_file:
            process = subprocess.Popen(
                command,
                cwd=str(cwd),
                env=child_env,
                stdin=subprocess.DEVNULL,
                stdout=stdout_file,
                stderr=stderr_file,
                shell=False,
                close_fds=True,
            )
            receipt["process"].update({"state": "running", "engine_pid": process.pid})
            receipt["spawned_utc"] = datetime.now(timezone.utc).isoformat()
            write_json_atomic(receipt_path, receipt)
            exit_code = process.wait()
            receipt["process"].update({"state": "exited", "exit_code": exit_code})
    except Exception as exc:
        receipt["process"].update({"state": "launch_or_wait_error"})
        receipt["launch_or_wait_error_type"] = type(exc).__name__
        if process is not None and exit_code is None:
            try:
                exit_code = process.wait()
                receipt["process"].update({"state": "exited", "exit_code": exit_code})
            except Exception as wait_exc:
                receipt["wait_error_type"] = type(wait_exc).__name__

    input_after = tree_summary(input_path)
    receipt["source_input_after"] = input_after
    receipt["input_unchanged"] = input_before == input_after
    receipt["finished_utc"] = datetime.now(timezone.utc).isoformat()
    receipt["process_exit_code"] = exit_code
    if result_path.is_file():
        receipt["result_sha256"] = sha256_file(result_path)
        try:
            result = json.loads(result_path.read_text(encoding="utf-8-sig"))
        except Exception as exc:
            receipt["result_parse_error"] = type(exc).__name__
        else:
            if not isinstance(result, dict):
                receipt["result_shape_error"] = "top_level_not_object"
                result = {}
            engine_state = result.get("state")
            query0_state, query0_metrics = read_query_state(result)
            receipt["engine_state"] = engine_state
            receipt["query0_state"] = query0_state
            receipt["query0_metrics"] = query0_metrics
            receipt["engine_result_counts"] = {
                "k10_min": result.get("query_samples_k10", {}).get("result_count_min")
                    if isinstance(result.get("query_samples_k10"), dict) else None,
                "k10_max": result.get("query_samples_k10", {}).get("result_count_max")
                    if isinstance(result.get("query_samples_k10"), dict) else None,
                "k100_min": result.get("query_samples_k100", {}).get("result_count_min")
                    if isinstance(result.get("query_samples_k100"), dict) else None,
                "k100_max": result.get("query_samples_k100", {}).get("result_count_max")
                    if isinstance(result.get("query_samples_k100"), dict) else None,
            }
            receipt["coverage_bench"] = result.get("coverage_bench")
            receipt["metadata_counts"] = {
                "node_count": result.get("metadata_node_count"),
                "metadata_count": result.get("metadata_count"),
                "marker_recovered": result.get("marker_recovered"),
            }
            receipt["expected_observation"] = (
                engine_state == expected_engine_state and query0_state == expected_query0_state
            )
    else:
        receipt["result_missing"] = True

    if exit_code is None:
        receipt["case_disposition"] = "benchmark_process_not_exited"
    elif exit_code != 0:
        receipt["case_disposition"] = "benchmark_process_nonzero"
    elif not receipt.get("result_sha256"):
        receipt["case_disposition"] = "result_missing_or_unparsed"
    elif not receipt["expected_observation"]:
        receipt["case_disposition"] = "expected_state_mismatch"
    elif not receipt["input_unchanged"]:
        receipt["case_disposition"] = "input_changed_during_run"
    elif case_kind == "before":
        receipt["case_disposition"] = "expected_query_failure_observed_not_qualified"
    else:
        receipt["case_disposition"] = "after_case_state_observed_not_qualified"
    write_json_atomic(receipt_path, receipt)
    print(json.dumps({
        "receipt": str(receipt_path),
        "case_kind": case_kind,
        "process_exit_code": exit_code,
        "engine_state": receipt.get("engine_state"),
        "query0_state": receipt.get("query0_state"),
        "expected_observation": receipt.get("expected_observation"),
        "case_disposition": receipt["case_disposition"],
        "qualification_claimed": False,
    }, sort_keys=True))
    return 0 if (
        exit_code == 0
        and receipt.get("result_sha256")
        and receipt.get("expected_observation")
        and receipt.get("input_unchanged")
    ) else 3


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"run-100k-case: {type(exc).__name__}: {exc}", file=sys.stderr)
        raise SystemExit(2)