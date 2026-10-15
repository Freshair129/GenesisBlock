#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_dir="${RUNNER_TEMP:-/tmp}/studio-packaged-evidence"
mkdir -p "$evidence_dir"
deb_dir="$repo_root/studio/src-tauri/target/debug/bundle/deb"
deb="$(find "$deb_dir" -maxdepth 1 -type f -name '*.deb' -print -quit)"
if [[ -z "$deb" ]]; then
  echo "No Studio DEB found in $deb_dir" >&2
  exit 1
fi

sudo apt-get install -y "$deb"
package="$(dpkg-deb -f "$deb" Package)"
binary="$(dpkg -L "$package" | awk '/^\/usr\/bin\/[^/]+$/ { print; exit }')"
if [[ -z "$binary" || ! -x "$binary" ]]; then
  echo "Installed Studio DEB has no executable in /usr/bin (package $package)" >&2
  dpkg -L "$package"
  exit 1
fi

log_file="$evidence_dir/studio-linux-startup.log"
status=0
dbus-run-session -- xvfb-run -a timeout --signal=TERM --kill-after=5s 12s "$binary" >"$log_file" 2>&1 || status=$?
if [[ "$status" -ne 124 ]]; then
  cat "$log_file"
  echo "Installed Studio did not remain alive for the 12-second startup window (exit $status)" >&2
  exit 1
fi

echo "Installed DEB app remained alive for the 12-second virtual-display startup window: $binary"
cat >"$evidence_dir/studio-linux-startup.json" <<EOF
{
  "package": "deb",
  "installed_path": "$binary",
  "startup_window_seconds": 12,
  "process_alive": true,
  "fixture_parity": false
}
EOF
