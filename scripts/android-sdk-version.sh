#!/usr/bin/env bash
#
# Print the genesisdb-android surface version. One source for it, because
# release.yml needs it in two places and a duplicate of this lookup is a
# duplicate that can drift - and the version lookup is where #139's defect
# actually lived.
#
#   usage: android-sdk-version.sh [repo-root]     (default: this script's ..)
#          android-sdk-version.sh --self-test
#
# modules.json first, NOT build.gradle.kts. Repair mode checks out an OLD tag's
# source, which can predate the packaging machinery doing the repair: at v0.2.0
# `android/genesisdb/build.gradle.kts` has neither the `genesisdbAndroidVersion`
# val nor a `publishing {}` block at all - both arrived later with the issue
# #125 publish work - so the gradle-file lookup failed there and killed the
# first repair run. modules.json has carried the version since well before that
# tag and is the declared SSOT for surface versions, so it is the stable source
# across the whole tag range. The gradle file remains a fallback for any future
# tree that drops the modules.json entry.
#
# Prints nothing and exits 1 when it cannot tell. Guessing a version here would
# publish or attach an artifact under the wrong coordinate, so this fails
# closed and lets the caller stop.

set -uo pipefail

# `python3` is not universally the name: the Windows dev host has only
# `python`. The first draft hardcoded python3, and the self-test caught it -
# the primary lookup failed silently and the gradle fallback answered instead,
# which is precisely the degradation this must not do quietly.
python_bin() {
  command -v python3 2>/dev/null || command -v python 2>/dev/null
}

resolve() { # <repo-root>
  local root="$1" version="" py
  py=$(python_bin)

  if [ -n "$py" ]; then
    version=$("$py" -c "import json,sys;d=json.load(open(sys.argv[1]));print(next(s['version'] for s in d['surfaces'] if s['name']=='genesisdb-android'))"                 "$root/modules.json" 2>/dev/null || true)
  fi

  if [ -n "$version" ]; then
    printf '%s
' "$version"
    return 0
  fi

  # Falling back is legitimate on an old tag, and a silent failure of the
  # primary source otherwise. Say which happened - a release that quietly
  # switched sources is a release nobody can explain later.
  if [ -z "$py" ]; then
    echo "  no python on PATH; reading the version from build.gradle.kts instead" >&2
  else
    echo "  $root/modules.json did not yield genesisdb-android; falling back to build.gradle.kts" >&2
  fi

  # grep + cut, not a sed backreference: no backslashes to lose in any layer
  # that touches this file, and the intent reads without decoding a regex.
  version=$(grep -o 'genesisdbAndroidVersion = "[^"]*"' "$root/android/genesisdb/build.gradle.kts" 2>/dev/null | head -1 | cut -d'"' -f2)

  [ -n "$version" ] || return 1
  printf '%s
' "$version"
}

self_test() {
  local failures=0 tmp out rc
  check() { # <label> <expected-rc> <expected-out> <actual-rc> <actual-out>
    if [ "$4" = "$2" ] && [ "$5" = "$3" ]; then
      printf '  ok    %-38s -> rc=%s out=%s\n' "$1" "$4" "${5:-<none>}"
    else
      printf '  FAIL  %-38s -> rc=%s out=%s (expected rc=%s out=%s)\n' "$1" "$4" "${5:-<none>}" "$2" "${3:-<none>}"
      failures=$((failures + 1))
    fi
  }

  tmp=$(mktemp -d)
  mkdir -p "$tmp/android/genesisdb"

  # 1. modules.json wins.
  printf '{"surfaces":[{"name":"other","version":"9.9.9"},{"name":"genesisdb-android","version":"1.2.3"}]}\n' > "$tmp/modules.json"
  printf 'val genesisdbAndroidVersion = "0.0.1"\n' > "$tmp/android/genesisdb/build.gradle.kts"
  out=$(resolve "$tmp"); rc=$?
  check "modules.json wins over gradle" 0 "1.2.3" "$rc" "$out"

  # 2. No modules.json at all - the repair-mode-on-an-old-tag case.
  rm "$tmp/modules.json"
  out=$(resolve "$tmp"); rc=$?
  check "falls back to build.gradle.kts" 0 "0.0.1" "$rc" "$out"

  # 3. modules.json present but without the surface entry.
  printf '{"surfaces":[{"name":"other","version":"9.9.9"}]}\n' > "$tmp/modules.json"
  out=$(resolve "$tmp"); rc=$?
  check "surface entry missing -> fallback" 0 "0.0.1" "$rc" "$out"

  # 4. Malformed modules.json must not crash the lookup.
  printf 'not json at all\n' > "$tmp/modules.json"
  out=$(resolve "$tmp"); rc=$?
  check "malformed modules.json -> fallback" 0 "0.0.1" "$rc" "$out"

  # 5. Neither source. Must FAIL, not print an empty or invented version.
  rm "$tmp/modules.json" "$tmp/android/genesisdb/build.gradle.kts"
  out=$(resolve "$tmp"); rc=$?
  check "neither source -> fails closed" 1 "" "$rc" "$out"

  # 6. A gradle file that exists but declares nothing.
  printf 'plugins { id("com.android.library") }\n' > "$tmp/android/genesisdb/build.gradle.kts"
  out=$(resolve "$tmp"); rc=$?
  check "gradle file without the val -> fails" 1 "" "$rc" "$out"

  # 7. The real repository, so a rename of modules.json's keys is caught here
  #    rather than at release time.
  out=$(resolve "$(cd "$(dirname "$0")/.." && pwd)"); rc=$?
  if [ "$rc" -eq 0 ] && [ -n "$out" ]; then
    printf '  ok    %-38s -> rc=0 out=%s\n' "this repository resolves" "$out"
  else
    printf '  FAIL  %-38s -> rc=%s out=%s\n' "this repository resolves" "$rc" "${out:-<none>}"
    failures=$((failures + 1))
  fi

  echo
  if [ "$failures" -ne 0 ]; then
    echo "::error::android-sdk-version self-test: $failures failure(s)"
    return 1
  fi
  echo "self-test passed"
}

case "${1:-}" in
  --self-test) self_test ;;
  *)
    ROOT="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
    if ! resolve "$ROOT"; then
      echo "::error::could not determine the genesisdb-android version from $ROOT/modules.json or $ROOT/android/genesisdb/build.gradle.kts" >&2
      exit 1
    fi ;;
esac
