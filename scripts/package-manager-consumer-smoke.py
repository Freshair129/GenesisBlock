#!/usr/bin/env python3
"""Smoke-test a cleanly installed GenesisBlockDB server binary."""

from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path


def request_json(url: str, body: dict | None = None) -> dict:
    data = None if body is None else json.dumps(body).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=data,
        headers={"Content-Type": "application/json"} if data else {},
    )
    with urllib.request.urlopen(request, timeout=5) as response:
        return json.load(response)


def wait_ready(process: subprocess.Popen[bytes], base_url: str) -> dict:
    for _ in range(60):
        if process.poll() is not None:
            raise RuntimeError(f"server exited early with code {process.returncode}")
        try:
            return request_json(f"{base_url}/v1/status")
        except Exception:
            time.sleep(1)
    raise TimeoutError("installed server did not become ready")


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: package-manager-consumer-smoke.py SERVER_BINARY")

    binary = Path(sys.argv[1]).resolve()
    if not binary.is_file():
        raise FileNotFoundError(binary)

    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]

    data_dir = Path(tempfile.mkdtemp(prefix="genesisdb-package-consumer-"))
    env = os.environ.copy()
    env.update(
        {
            "GENESIS_DATA_DIR": str(data_dir),
            "GENESIS_HOST": "127.0.0.1",
            "GENESIS_PORT": str(port),
        }
    )
    base_url = f"http://127.0.0.1:{port}"

    def start() -> subprocess.Popen[bytes]:
        return subprocess.Popen(
            [str(binary)],
            cwd=data_dir,
            env=env,
            creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0,
        )

    process = start()
    try:
        before = wait_ready(process, base_url)["node_count"]
        request_json(
            f"{base_url}/v1/node/add",
            {"id": "package-manager-consumer-smoke", "labels": ["Smoke"]},
        )
        after_write = request_json(f"{base_url}/v1/status")["node_count"]
        if after_write <= before:
            raise AssertionError(f"node_count did not increase: {before} -> {after_write}")

        if os.name == "nt":
            process.terminate()
        else:
            process.terminate()
        process.wait(timeout=20)
        process = start()
        after_restart = wait_ready(process, base_url)["node_count"]
        if after_restart < after_write:
            raise AssertionError(
                f"persisted node count decreased after restart: {after_write} -> {after_restart}"
            )
        print(f"installed server persistence smoke passed: {after_write} -> {after_restart}")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)


if __name__ == "__main__":
    main()
