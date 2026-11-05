#!/usr/bin/env python3
"""Explicitly copy the closed 100k source DB into paired disposable cases.

This script performs filesystem copying and hashing only. It never opens SQLite
or launches the benchmark. Invoke it explicitly after reviewing the source and
free-space guards.
"""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import sys
from pathlib import Path
from typing import Any

RNEW_ROOT = Path(__file__).resolve().parent
SOURCE_DB = Path(r"C:\Users\suanr\genesis-vector-memory-followup-20261031\query-reopen-repeat-20261031T134858\new\gdb")
CASES_ROOT = RNEW_ROOT / "100k-cases"
MIN_FREE_BYTES = 50 * 1024**3
CHUNK_BYTES = 1024 * 1024


def under(path: Path, root: Path) -> bool:
    path_s = os.path.normcase(os.path.realpath(os.fspath(path)))
    root_s = os.path.normcase(os.path.realpath(os.fspath(root)))
    try:
        return os.path.commonpath((path_s, root_s)) == root_s
    except ValueError:
        return False


def require_under(path: Path, root: Path) -> Path:
    resolved = path.resolve(strict=False)
    if not under(resolved, root):
        raise RuntimeError(f"path escapes Rnew root: {path}")
    return resolved


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(CHUNK_BYTES):
            digest.update(chunk)
    return digest.hexdigest()


def tree_manifest(root: Path) -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix().casefold()):
        if path.is_symlink():
            raise RuntimeError(f"symlink is not allowed in database tree: {path.name}")
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        size = path.stat().st_size
        digest = sha256_file(path)
        entries.append({"path": relative, "bytes": size, "sha256": digest})
    aggregate = hashlib.sha256()
    for entry in entries:
        aggregate.update(
            f"{entry['path']}\0{entry['bytes']}\0{entry['sha256']}\n".encode("utf-8")
        )
    return {
        "file_count": len(entries),
        "byte_count": sum(entry["bytes"] for entry in entries),
        "tree_sha256": aggregate.hexdigest(),
        "components": entries,
    }


def source_wal_state() -> dict[str, Any]:
    wal = SOURCE_DB / "projection.sqlite-wal"
    exists = wal.exists()
    size = wal.stat().st_size if exists else 0
    if size != 0:
        raise RuntimeError(f"source is not closed: projection WAL is {size} bytes")
    return {"exists": exists, "bytes": size, "closed_zero": True}


def write_json(path: Path, value: Any) -> None:
    temporary = path.with_name(f"{path.name}.{os.getpid()}.tmp")
    with temporary.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    temporary.replace(path)


def main() -> int:
    root = RNEW_ROOT.resolve(strict=True)
    source = SOURCE_DB.resolve(strict=True)
    if not source.is_dir() or not (source / "projection.sqlite").is_file():
        raise RuntimeError("source DB directory or projection.sqlite is missing")

    cases = require_under(CASES_ROOT, root)
    cases.mkdir(parents=True, exist_ok=True)
    before_case = require_under(cases / "before", root)
    after_case = require_under(cases / "after", root)
    before_db = require_under(before_case / "gdb", root)
    after_db = require_under(after_case / "gdb", root)
    manifest_path = require_under(cases / "copy-manifest.json", root)
    for destination in (before_db, after_db, manifest_path):
        if destination.exists() or destination.is_symlink():
            raise RuntimeError(f"refusing existing destination: {destination}")

    free_bytes = shutil.disk_usage(root).free
    if free_bytes < MIN_FREE_BYTES:
        raise RuntimeError(f"free space below required 50 GiB: {free_bytes} bytes")
    wal_before = source_wal_state()
    source_before = tree_manifest(source)

    before_case.mkdir(parents=True, exist_ok=True)
    after_case.mkdir(parents=True, exist_ok=True)
    shutil.copytree(source, before_db, copy_function=shutil.copy2)
    require_under(before_db, root)
    shutil.copytree(source, after_db, copy_function=shutil.copy2)
    require_under(after_db, root)

    source_after = tree_manifest(source)
    before_manifest = tree_manifest(before_db)
    after_manifest = tree_manifest(after_db)
    wal_after = source_wal_state()
    if source_before != source_after:
        raise RuntimeError("source DB changed while copies were being prepared")
    if before_manifest != source_before or after_manifest != source_before:
        raise RuntimeError("copied component hashes do not match the closed source DB")
    if before_manifest != after_manifest:
        raise RuntimeError("before/after DB copies differ")

    manifest = {
        "schema_version": 1,
        "operation": "copy closed source DB to before/gdb and after/gdb",
        "sqlite_opened": False,
        "benchmark_launched": False,
        "source_db": str(source),
        "rnew_root": str(root),
        "wal_before": wal_before,
        "wal_after": wal_after,
        "minimum_free_bytes": MIN_FREE_BYTES,
        "free_bytes_before_copy": free_bytes,
        "source": source_before,
        "before": before_manifest,
        "after": after_manifest,
        "copies_equal": before_manifest == after_manifest,
        "source_matches_both_copies": before_manifest == source_before and after_manifest == source_before,
        "equal_file_count": before_manifest["file_count"] == after_manifest["file_count"],
        "equal_byte_count": before_manifest["byte_count"] == after_manifest["byte_count"],
    }
    write_json(manifest_path, manifest)
    print(json.dumps({
        "manifest": str(manifest_path),
        "copies_equal": manifest["copies_equal"],
        "source_matches_both_copies": manifest["source_matches_both_copies"],
        "file_count": before_manifest["file_count"],
        "byte_count_each": before_manifest["byte_count"],
        "tree_sha256": before_manifest["tree_sha256"],
        "source_wal_bytes": wal_after["bytes"],
        "sqlite_opened": False,
        "benchmark_launched": False,
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"prepare-100k-cases: {type(exc).__name__}: {exc}", file=sys.stderr)
        raise SystemExit(2)