#!/usr/bin/env python3
"""Exercise standalone REST recovery after hard and graceful restarts.

The test uses only the Python standard library so the release CI matrix can run
the exact server binary it just built on Linux, Windows, and macOS.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
HOST = "127.0.0.1"
FIXTURE_COUNT = 3
NODE_COUNT = 4
VECTOR_DIM = 1536
READY_TIMEOUT_SECONDS = 90
SEARCH_TIMEOUT_SECONDS = 45


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def request_json(
    base_url: str, method: str, path: str, payload: dict[str, Any] | None = None
) -> Any:
    body = None if payload is None else json.dumps(payload).encode("utf-8")
    headers = {} if body is None else {"Content-Type": "application/json"}
    request = urllib.request.Request(
        f"{base_url}{path}", data=body, headers=headers, method=method
    )
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            raw = response.read()
            status = response.status
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {path} returned HTTP {error.code}: {detail}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"{method} {path} failed: {error}") from error
    require(200 <= status < 300, f"{method} {path} returned HTTP {status}")
    try:
        return json.loads(raw)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"{method} {path} returned invalid JSON: {raw[:300]!r}") from error


def choose_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind((HOST, 0))
        return int(listener.getsockname()[1])


def read_log(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def start_server(binary: Path, database: Path, port: int, log_path: Path) -> subprocess.Popen[bytes]:
    environment = os.environ.copy()
    environment.pop("GENESIS_API_KEY", None)
    # The test intentionally exercises the server's default retention profile.
    environment.pop("GENESIS_RETENTION", None)
    environment.update(
        GENESIS_DATA_DIR=str(database),
        GENESIS_HOST=HOST,
        GENESIS_PORT=str(port),
        RUST_LOG="genesis_db_server=info",
    )
    options: dict[str, Any] = {}
    if os.name == "nt":
        options["creationflags"] = (
            subprocess.CREATE_NEW_CONSOLE | subprocess.CREATE_NEW_PROCESS_GROUP
        )
        startup_info = subprocess.STARTUPINFO()
        startup_info.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup_info.wShowWindow = subprocess.SW_HIDE
        options["startupinfo"] = startup_info
    else:
        options["start_new_session"] = True

    with log_path.open("wb") as output:
        return subprocess.Popen(
            [str(binary)],
            cwd=ROOT,
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=output,
            stderr=subprocess.STDOUT,
            **options,
        )


def wait_ready(process: subprocess.Popen[bytes], base_url: str, log_path: Path) -> None:
    deadline = time.monotonic() + READY_TIMEOUT_SECONDS
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(
                f"server exited with code {process.returncode} before readiness:\n"
                f"{read_log(log_path)}"
            )
        try:
            request_json(base_url, "GET", "/v1/status")
            return
        except (OSError, RuntimeError, TimeoutError) as error:
            last_error = error
            time.sleep(0.25)
    raise RuntimeError(
        f"server did not become ready within {READY_TIMEOUT_SECONDS}s; "
        f"last error: {last_error}\n{read_log(log_path)}"
    )


def stop_hard(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is None:
        process.kill()
    process.wait(timeout=30)


def emit_windows_ctrl_c(process_id: int) -> None:
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.FreeConsole.restype = ctypes.c_int
    kernel32.AttachConsole.argtypes = [ctypes.c_uint32]
    kernel32.AttachConsole.restype = ctypes.c_int
    kernel32.GenerateConsoleCtrlEvent.argtypes = [ctypes.c_uint32, ctypes.c_uint32]
    kernel32.GenerateConsoleCtrlEvent.restype = ctypes.c_int
    kernel32.SetConsoleCtrlHandler.argtypes = [ctypes.c_void_p, ctypes.c_int]
    kernel32.SetConsoleCtrlHandler.restype = ctypes.c_int

    kernel32.FreeConsole()
    if not kernel32.AttachConsole(process_id):
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        # CTRL_C_EVENT is broadcast to the attached console; keep the test
        # runner alive while the Tokio server handles its normal Ctrl-C path.
        if not kernel32.SetConsoleCtrlHandler(None, 1):
            raise ctypes.WinError(ctypes.get_last_error())
        if not kernel32.GenerateConsoleCtrlEvent(signal.CTRL_C_EVENT, 0):
            raise ctypes.WinError(ctypes.get_last_error())
        # Console events are delivered asynchronously. Keep this helper's
        # ignore-Ctrl-C attribute active until the broadcast has been handled.
        time.sleep(0.5)
    finally:
        kernel32.FreeConsole()
        kernel32.SetConsoleCtrlHandler(None, 0)


def send_windows_ctrl_c(process: subprocess.Popen[bytes]) -> None:
    helper = subprocess.run(
        [
            sys.executable,
            str(Path(__file__).resolve()),
            "--send-windows-ctrl-c",
            str(process.pid),
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        creationflags=subprocess.CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP,
        check=False,
    )
    require(
        helper.returncode == 0,
        f"could not send graceful Ctrl-C to server (helper exit {helper.returncode}): "
        f"{helper.stderr.strip()}",
    )


def stop_gracefully(process: subprocess.Popen[bytes], log_path: Path) -> None:
    if process.poll() is None:
        if os.name == "nt":
            send_windows_ctrl_c(process)
        else:
            process.send_signal(signal.SIGTERM)
    try:
        exit_code = process.wait(timeout=60)
    except subprocess.TimeoutExpired as error:
        stop_hard(process)
        raise RuntimeError(f"server did not stop gracefully:\n{read_log(log_path)}") from error
    log = read_log(log_path)
    require(exit_code == 0, f"graceful server exit was {exit_code}:\n{log}")
    require("checkpoint complete" in log, f"server did not report checkpoint completion:\n{log}")


def make_records(fixture: int) -> list[dict[str, Any]]:
    records = []
    for index in range(NODE_COUNT):
        tag = f"fixture-{fixture}-node-{index}"
        vector = [0.0] * VECTOR_DIM
        vector[index] = 1.0
        records.append(
            {
                "id": f"rest-recovery-{tag}",
                "props": {"recovery_tag": tag, "fixture": fixture, "index": index},
                "embedding": vector,
            }
        )
    return records


def write_records(base_url: str, records: list[dict[str, Any]]) -> None:
    for record in records:
        response = request_json(
            base_url,
            "POST",
            "/v1/node/add",
            {
                "id": record["id"],
                "labels": ["RestartRecovery"],
                "props": record["props"],
                "embedding": record["embedding"],
            },
        )
        require(response.get("id") == record["id"], f"write acknowledgement mismatch: {response}")


def verify_records(base_url: str, records: list[dict[str, Any]]) -> None:
    for record in records:
        encoded_id = urllib.parse.quote(record["id"], safe="")
        inspection = request_json(base_url, "GET", f"/v1/studio/entity/{encoded_id}")
        require(inspection.get("entity_id") == record["id"], f"entity ID mismatch: {inspection}")
        require(
            inspection.get("properties", {}).get("recovery_tag")
            == record["props"]["recovery_tag"],
            f"properties attached to the wrong entity for {record['id']}: {inspection}",
        )
        require(inspection.get("vector_present") is True, f"vector missing for {record['id']}")

    for record in records:
        deadline = time.monotonic() + SEARCH_TIMEOUT_SECONDS
        last_ids: list[str] = []
        while time.monotonic() < deadline:
            hits = request_json(
                base_url,
                "POST",
                "/v1/search/hybrid",
                {"query_vector": record["embedding"], "k": NODE_COUNT, "alpha": 0.0},
            )
            require(isinstance(hits, list), f"hybrid search did not return an array: {hits}")
            last_ids = [hit.get("node", {}).get("id") for hit in hits]
            matching = next(
                (hit for hit in hits if hit.get("node", {}).get("id") == record["id"]),
                None,
            )
            if matching is not None:
                require(
                    last_ids[0] == record["id"],
                    f"exact vector query ranked the wrong node first: {last_ids}",
                )
                require(
                    matching.get("node", {}).get("props", {}).get("recovery_tag")
                    == record["props"]["recovery_tag"],
                    f"vector hit has the wrong properties for {record['id']}: {matching}",
                )
                break
            time.sleep(0.25)
        else:
            raise AssertionError(
                f"vector hit for {record['id']} was not searchable; returned IDs: {last_ids}"
            )


def run_fixture(binary: Path, root: Path, fixture: int) -> None:
    database = root / f"fixture-{fixture}"
    port = choose_port()
    base_url = f"http://{HOST}:{port}"
    records = make_records(fixture)
    process: subprocess.Popen[bytes] | None = None
    try:
        process = start_server(binary, database, port, root / f"fixture-{fixture}-write.log")
        wait_ready(process, base_url, root / f"fixture-{fixture}-write.log")
        write_records(base_url, records)

        # Simulate a process crash after every node write has been acknowledged.
        stop_hard(process)
        process = None

        forced_log = root / f"fixture-{fixture}-forced-restart.log"
        process = start_server(binary, database, port, forced_log)
        wait_ready(process, base_url, forced_log)
        verify_records(base_url, records)

        # Checkpoint normally, reopen the same database, and verify a second time.
        stop_gracefully(process, forced_log)
        process = None
        graceful_log = root / f"fixture-{fixture}-graceful-reopen.log"
        process = start_server(binary, database, port, graceful_log)
        wait_ready(process, base_url, graceful_log)
        verify_records(base_url, records)
        stop_gracefully(process, graceful_log)
        process = None
    finally:
        if process is not None and process.poll() is None:
            stop_hard(process)


def main() -> int:
    if len(sys.argv) == 3 and sys.argv[1] == "--send-windows-ctrl-c":
        emit_windows_ctrl_c(int(sys.argv[2]))
        return 0

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--server",
        required=True,
        type=Path,
        help="path to the standalone server executable just built",
    )
    arguments = parser.parse_args()
    binary = arguments.server
    if not binary.is_absolute():
        binary = ROOT / binary
    if not binary.is_file():
        print(f"built server binary not found: {binary}", file=sys.stderr)
        return 2

    with tempfile.TemporaryDirectory(prefix="genesis-rest-recovery-") as temporary:
        root = Path(temporary)
        for fixture in range(1, FIXTURE_COUNT + 1):
            run_fixture(binary.resolve(), root, fixture)
            print(f"REST restart recovery fixture {fixture}/{FIXTURE_COUNT} passed", flush=True)
    print("REST restart recovery passed: forced restart and graceful reopen", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
