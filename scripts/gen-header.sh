#!/usr/bin/env bash
# Regenerate the mobile C ABI header (include/genesisdb.h) from src/ffi.rs.
#
# The header is the contract the iOS xcframework (and any C/Swift caller) links
# against. It is generated, committed, and verified fresh by the CI step
# "C header freshness" in .github/workflows/mobile-build.yml — that job runs this
# script and fails if the committed header differs, so regenerate and commit
# whenever you change a `genesisdb_*` signature in src/ffi.rs.
#
# Requires cbindgen:  cargo install cbindgen --locked
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cbindgen >/dev/null 2>&1; then
  echo "error: cbindgen not found. Install with: cargo install cbindgen --locked" >&2
  exit 1
fi

mkdir -p include
cbindgen --config cbindgen.toml \
         --crate genesis-block-native \
         --output include/genesisdb.h

echo "Wrote include/genesisdb.h"
