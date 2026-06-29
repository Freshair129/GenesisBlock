#!/usr/bin/env python3
"""Capture host environment + toolchain for an Independent Benchmark run.

Writes a JSON file (default: env.json) with OS, CPU, RAM, disk free space, and
the rustc/cargo versions. Uses only the Python standard library so an external
reproducer needs nothing beyond a stock Python 3.8+ interpreter.

Any field that genuinely cannot be collected on this OS is recorded as `null`
and the reason is appended to `_notes` — per the suite rule "if a metric cannot
be collected on an OS, record null and explain why in env.json".

Usage:
    python benchmark/collect_env.py --out env.json
"""
from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys


def _run(cmd: list[str]) -> str | None:
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if out.returncode == 0:
            return out.stdout.strip() or None
    except (OSError, subprocess.SubprocessError):
        pass
    return None


def detect_cpu(notes: list[str]) -> str | None:
    system = platform.system()
    # platform.processor() is often empty on Linux and a bare family string on
    # Windows; try the richer OS-specific source first.
    if system == "Linux":
        try:
            with open("/proc/cpuinfo", "r", encoding="utf-8", errors="replace") as f:
                for line in f:
                    if line.lower().startswith("model name"):
                        return line.split(":", 1)[1].strip()
        except OSError:
            notes.append("cpu: /proc/cpuinfo unreadable")
    elif system == "Darwin":
        v = _run(["sysctl", "-n", "machdep.cpu.brand_string"])
        if v:
            return v
    elif system == "Windows":
        v = os.environ.get("PROCESSOR_IDENTIFIER")
        if v:
            return v.strip()
    fallback = platform.processor() or None
    if not fallback:
        notes.append("cpu: no model string available on this OS")
    return fallback


def detect_ram_gb(notes: list[str]) -> float | None:
    system = platform.system()
    try:
        if system == "Linux":
            with open("/proc/meminfo", "r", encoding="utf-8") as f:
                for line in f:
                    if line.startswith("MemTotal:"):
                        kb = int(line.split()[1])
                        return round(kb / (1024 * 1024), 1)
        elif system == "Darwin":
            v = _run(["sysctl", "-n", "hw.memsize"])
            if v:
                return round(int(v) / (1024 ** 3), 1)
        elif system == "Windows":
            import ctypes

            class MEMORYSTATUSEX(ctypes.Structure):
                _fields_ = [
                    ("dwLength", ctypes.c_ulong),
                    ("dwMemoryLoad", ctypes.c_ulong),
                    ("ullTotalPhys", ctypes.c_ulonglong),
                    ("ullAvailPhys", ctypes.c_ulonglong),
                    ("ullTotalPageFile", ctypes.c_ulonglong),
                    ("ullAvailPageFile", ctypes.c_ulonglong),
                    ("ullTotalVirtual", ctypes.c_ulonglong),
                    ("ullAvailVirtual", ctypes.c_ulonglong),
                    ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
                ]

            stat = MEMORYSTATUSEX()
            stat.dwLength = ctypes.sizeof(MEMORYSTATUSEX)
            if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(stat)):
                return round(stat.ullTotalPhys / (1024 ** 3), 1)
    except (OSError, ValueError, ImportError) as e:
        notes.append(f"ram_gb: detection failed ({e})")
        return None
    notes.append("ram_gb: no detection path for this OS")
    return None


def detect_disk(target: str, notes: list[str]) -> tuple[str | None, float | None]:
    """Returns (human description, free GB). Disk *model* is not portably
    available without elevated/3rd-party tooling, so the description records the
    mount + free space rather than a hardware model."""
    try:
        usage = shutil.disk_usage(target)
        free_gb = round(usage.free / (1024 ** 3), 1)
        total_gb = round(usage.total / (1024 ** 3), 1)
        desc = f"{os.path.abspath(target)} ({free_gb} GB free / {total_gb} GB total)"
        notes.append("disk: hardware model not collected (requires elevated/3rd-party tools); reporting mount + capacity")
        return desc, free_gb
    except OSError as e:
        notes.append(f"disk: usage probe failed ({e})")
        return None, None


def main() -> int:
    ap = argparse.ArgumentParser(description="Capture host env for a benchmark run.")
    ap.add_argument("--out", default="env.json", help="output JSON path")
    ap.add_argument("--disk-target", default=".", help="path to probe for free disk space")
    args = ap.parse_args()

    notes: list[str] = []
    disk_desc, disk_free_gb = detect_disk(args.disk_target, notes)

    env = {
        "os": platform.platform() or None,
        "cpu": detect_cpu(notes),
        "cpu_cores_logical": os.cpu_count(),
        "ram_gb": detect_ram_gb(notes),
        "disk": disk_desc,
        "disk_free_gb": disk_free_gb,
        "rustc": _run(["rustc", "--version"]),
        "cargo": _run(["cargo", "--version"]),
        "python": platform.python_version(),
        "arch": platform.machine() or None,
        "hostname": platform.node() or None,
        "_notes": notes,
    }
    if env["rustc"] is None:
        notes.append("rustc: not found on PATH — benchmark cannot build")
    if env["cargo"] is None:
        notes.append("cargo: not found on PATH — benchmark cannot build")

    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(env, f, indent=2)
    print(f"env captured -> {args.out}")
    for k in ("os", "cpu", "ram_gb", "rustc", "cargo"):
        print(f"  {k}: {env[k]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
