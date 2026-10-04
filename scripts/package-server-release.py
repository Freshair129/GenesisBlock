#!/usr/bin/env python3
"""Package a built GenesisBlockDB standalone server binary reproducibly enough for release CI.

Usage:
    python scripts/package-server-release.py --target x86_64-unknown-linux-gnu

The script reads the canonical engine version from Cargo.toml, locates the
binary under target/<target>/release/, emits a platform archive into dist/, and
writes a matching .sha256 file next to it.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import shutil
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / "dist"


def cargo_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    package = text.split("[package]", 1)[1] if "[package]" in text else text
    match = re.search(r'^version\s*=\s*"([^"]+)"', package, re.MULTILINE)
    if not match:
        raise SystemExit("could not read [package] version from Cargo.toml")
    return match.group(1)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    args = parser.parse_args()

    target = args.target
    windows = "windows" in target or "pc-windows" in target
    exe = "genesis-db-server.exe" if windows else "genesis-db-server"
    binary = ROOT / "target" / target / "release" / exe
    if not binary.exists():
        raise SystemExit(f"built server binary not found: {binary}")

    version = cargo_version()
    stem = f"genesisblockdb-server-v{version}-{target}"
    staging = DIST / stem
    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, staging / exe)
    shutil.copy2(ROOT / "LICENSE", staging / "LICENSE")

    DIST.mkdir(exist_ok=True)
    if windows:
        archive = DIST / f"{stem}.zip"
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            for path in sorted(staging.iterdir()):
                zf.write(path, arcname=f"{stem}/{path.name}")
    else:
        archive = DIST / f"{stem}.tar.gz"
        with tarfile.open(archive, "w:gz") as tf:
            tf.add(staging, arcname=stem)

    digest = sha256(archive)
    checksum = archive.with_name(archive.name + ".sha256")
    checksum.write_text(f"{digest}  {archive.name}\n", encoding="utf-8")
    print(archive.relative_to(ROOT))
    print(checksum.relative_to(ROOT))


if __name__ == "__main__":
    main()
