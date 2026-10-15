#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dmg_dir="$repo_root/studio/src-tauri/target/debug/bundle/dmg"
dmg="$(find "$dmg_dir" -maxdepth 1 -type f -name '*.dmg' -print -quit)"
if [[ -z "$dmg" ]]; then
  echo "No Studio DMG found in $dmg_dir" >&2
  exit 1
fi

mount_point="$(mktemp -d "${RUNNER_TEMP:-/tmp}/studio-dmg.XXXXXX")"
install_path="${RUNNER_TEMP:-/tmp}/Applications/Genesis Studio.app"
evidence_dir="${RUNNER_TEMP:-/tmp}/studio-packaged-evidence"
startup_log="$evidence_dir/studio-macos-startup.log"
launcher_pid=""
mkdir -p "$evidence_dir"
cleanup() {
  if [[ -n "$launcher_pid" ]] && kill -0 "$launcher_pid" 2>/dev/null; then
    pkill -TERM -f "$install_path/Contents/MacOS/" 2>/dev/null || true
    kill "$launcher_pid" 2>/dev/null || true
    wait "$launcher_pid" 2>/dev/null || true
  fi
  hdiutil detach "$mount_point" -quiet -force 2>/dev/null || true
  case "$mount_point" in
    "${RUNNER_TEMP:-/tmp}"/*|/tmp/*) rm -rf "$mount_point" ;;
    *) echo "Refusing to remove unexpected DMG mount path: $mount_point" >&2 ;;
  esac
}
trap cleanup EXIT

hdiutil attach "$dmg" -readonly -nobrowse -mountpoint "$mount_point"
source_app="$(find "$mount_point" -maxdepth 1 -type d -name '*.app' -print -quit)"
if [[ -z "$source_app" ]]; then
  echo "DMG did not contain a Studio app bundle" >&2
  exit 1
fi
mkdir -p "$(dirname "$install_path")"
if [[ -e "$install_path" ]]; then
  case "$install_path" in
    "${RUNNER_TEMP:-/tmp}"/*|/tmp/*) rm -rf "$install_path" ;;
    *) echo "Refusing to replace unexpected app path: $install_path" >&2; exit 1 ;;
  esac
fi
ditto "$source_app" "$install_path"
hdiutil detach "$mount_point" -quiet

open -W -n "$install_path" >"$startup_log" 2>&1 &
launcher_pid=$!
sleep 10
if ! kill -0 "$launcher_pid" 2>/dev/null; then
  status=0
  wait "$launcher_pid" || status=$?
  cat "$startup_log"
  echo "Installed Studio exited during the 10-second startup window (exit $status)" >&2
  exit 1
fi
if ! pgrep -f "$install_path/Contents/MacOS/" >/dev/null; then
  cat "$startup_log"
  echo "Installed Studio app process was not present after launch" >&2
  exit 1
fi

echo "Installed DMG app started and remained alive for 10 seconds: $install_path"
cat >"$evidence_dir/studio-macos-startup.json" <<EOF
{
  "package": "dmg",
  "installed_path": "$install_path",
  "startup_window_seconds": 10,
  "process_alive": true,
  "fixture_parity": false
}
EOF
