import datetime, hashlib, json, os, pathlib, subprocess, sys, time, traceback

def write_json(path, obj):
    pathlib.Path(path).write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()

def input_hashes(cfg):
    base = pathlib.Path(cfg["input_dir"])
    names = {"corpus": "corpus.f32", "queries": "queries.f32", "truth": "truth.json", "meta": "meta.json"}
    return {key: sha256(base / name) for key, name in names.items()}

def now_local():
    return datetime.datetime.now().astimezone().isoformat()

def main():
    cfg_path = pathlib.Path(sys.argv[1]).resolve()
    cfg = json.loads(cfg_path.read_text(encoding="utf-8-sig"))
    run_root = pathlib.Path(cfg["run_root"])
    output = pathlib.Path(cfg["extra_env"]["GB_MEMORY_OUT"])
    binary = pathlib.Path(cfg["binary"])
    if sha256(binary).lower() != cfg["binary_sha256"].lower():
        raise RuntimeError("frozen candidate diagnostic binary hash mismatch")
    if output.exists() or (output / "gdb").exists():
        raise RuntimeError("refusing non-fresh diagnostic output path")
    before = input_hashes(cfg)
    if before != cfg["expected_input_hashes"]:
        raise RuntimeError("real BGE input hash mismatch before launch")

    env = os.environ.copy()
    # Clear inherited benchmark switches, then set the complete diagnostic profile.
    for key in list(env):
        if key.startswith("GB_MEMORY_") or key in {
            "GB_LIMIT", "GB_EF_SWEEP", "GB_QUANT", "GB_RERANK", "GB_EF",
            "GB_VEC_Q", "GB_VBENCH", "RAYON_NUM_THREADS", "GB_VECTOR_QUERY_DIAGNOSTICS",
        }:
            env.pop(key, None)
    for key in cfg["clear_env"]:
        env.pop(key, None)
    for key, value in cfg["env_whitelist"].items():
        if value is None:
            env.pop(key, None)
        else:
            env[key] = str(value)
    for key, value in cfg["extra_env"].items():
        env[key] = str(value)

    effective = {key: env.get(key) for key in cfg["env_whitelist"]}
    extras = {key: env.get(key) for key in cfg["extra_env"]}
    for key, value in cfg["env_whitelist"].items():
        expected = None if value is None else str(value)
        if effective[key] != expected:
            raise RuntimeError(f"effective environment mismatch for {key}")
    cleared_absent = [key for key in cfg["clear_env"] if key not in env]
    if len(cfg["env_whitelist"]) != 16:
        raise RuntimeError("full 16-key environment whitelist is not present")
    context = {
        "prepared_at_local": now_local(),
        "driver_pid": os.getpid(),
        "binary": str(binary),
        "binary_sha256": cfg["binary_sha256"],
        "input_dir": cfg["input_dir"],
        "input_hashes_before": before,
        "input_hashes_expected": cfg["expected_input_hashes"],
        "env_whitelist_count": len(effective),
        "env_whitelist": effective,
        "extra_env": extras,
        "clear_env_names": cfg["clear_env"],
        "clear_env_names_absent": cleared_absent,
        "open_only_flags_absent": all(effective[k] is None for k in (
            "GB_MEMORY_OPEN_ONLY", "GB_MEMORY_OPEN_READ_WRITE", "GB_MEMORY_OPEN_OUT")),
        "query": {"deadline_ms": 5000, "initial_k": 10, "initial_k100": 100,
                  "reopened_queries": 24, "fresh_child_query_index": 0, "ef": 200, "alpha": 0.0},
    }
    write_json(run_root / "driver-context.json", context)

    stdout_path = run_root / "stdout.log"
    stderr_path = run_root / "stderr.log"
    started = time.monotonic()
    started_local = now_local()
    proc = None
    try:
        with open(stdout_path, "wb", buffering=0) as out, open(stderr_path, "wb", buffering=0) as err:
            proc = subprocess.Popen(
                [str(binary)], cwd=cfg["repo"], env=env,
                stdin=subprocess.DEVNULL, stdout=out, stderr=err,
                close_fds=True, shell=False,
            )
            write_json(run_root / "benchmark-start.json", {
                "driver_pid": os.getpid(), "benchmark_pid": proc.pid,
                "started_at_local": started_local, "binary_sha256": cfg["binary_sha256"],
                "output_dir": str(output), "direct_file_handles": True,
                "process_tree_root_pid": os.getpid(),
            })
            rc = proc.wait()
    except BaseException:
        if proc is not None and proc.poll() is None:
            try:
                proc.terminate()
                proc.wait(timeout=10)
            except Exception:
                try: proc.kill()
                except Exception: pass
        raise

    after = input_hashes(cfg)
    status = {
        "driver_pid": os.getpid(), "benchmark_pid": proc.pid, "exit_code": rc,
        "started_at_local": started_local, "finished_at_local": now_local(),
        "elapsed_seconds": round(time.monotonic() - started, 3),
        "binary_sha256": cfg["binary_sha256"],
        "input_hashes_before": before, "input_hashes_after": after,
        "input_unchanged": before == after == cfg["expected_input_hashes"],
        "direct_file_handles": True,
    }
    write_json(run_root / "driver-status.json", status)
    print(json.dumps(status, separators=(",", ":")))
    return rc

if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:
        run_root = None
        try:
            cfg = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8-sig"))
            run_root = pathlib.Path(cfg["run_root"])
        except Exception:
            pass
        if run_root is not None:
            write_json(run_root / "driver-error.json", {"at_local": now_local(), "error": repr(exc), "traceback": traceback.format_exc()})
        print(f"driver error: {exc!r}", file=sys.stderr)
        sys.exit(70)
