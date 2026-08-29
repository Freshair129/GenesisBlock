#!/usr/bin/env bash
#
# Validate a genesisdb-android publication against what Maven Central requires,
# WITHOUT needing any Central credential or signing key.
#
# Why this exists: Central validates a bundle only after it has been uploaded,
# and rejects it for things that are entirely knowable beforehand - a missing
# <description>, no <scm>, no javadoc jar. Finding that out during a release is
# the pattern this repo has already paid for repeatedly (see
# .github/workflows/release.yml's dry-run rationale). This turns those into a
# PR-time failure instead.
#
#   usage: verify-maven-central-pom.sh [repo-dir]
#          default repo-dir: ~/.m2/repository
#
# Signatures are checked only if signing was enabled, since the key is a CI
# secret; everything else is checked unconditionally.

set -euo pipefail

REPO="${1:-$HOME/.m2/repository}"
GROUP_PATH="io/github/freshair129"
ARTIFACT="genesisdb-android"

DIR=$(find "$REPO/$GROUP_PATH/$ARTIFACT" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | head -1)
if [ -z "$DIR" ]; then
  echo "::error::no publication found under $REPO/$GROUP_PATH/$ARTIFACT - did publishToMavenLocal run?"
  exit 1
fi
VERSION=$(basename "$DIR")
echo "validating $ARTIFACT $VERSION in $DIR"
echo

POM=$(ls "$DIR"/*.pom 2>/dev/null | head -1)
[ -n "$POM" ] || { echo "::error::no .pom in $DIR"; exit 1; }

fail=0
note() { printf '  %-14s %s\n' "$1" "$2"; }

# Central's required POM elements. Checked by element name rather than by
# XPath so this needs no XML tooling on the runner.
for tag in groupId artifactId version name description url; do
  if grep -q "<$tag>" "$POM"; then
    note "OK" "<$tag>"
  else
    note "MISSING" "<$tag>"; fail=1
  fi
done

for block in licenses developers scm; do
  if grep -q "<$block>" "$POM"; then
    note "OK" "<$block>"
  else
    note "MISSING" "<$block>"; fail=1
  fi
done

# scm needs all three; a bare <scm><url> passes the block check above but is
# still rejected.
for tag in connection developerConnection; do
  grep -q "<$tag>" "$POM" || { note "MISSING" "<scm><$tag>"; fail=1; }
done

echo
# Required companion artifacts.
for suffix in "-sources.jar" "-javadoc.jar" ".aar"; do
  if ls "$DIR"/*"$suffix" >/dev/null 2>&1; then
    note "OK" "*$suffix"
  else
    note "MISSING" "*$suffix"; fail=1
  fi
done

echo
# Signatures: only meaningful when a key was available.
if ls "$DIR"/*.asc >/dev/null 2>&1; then
  note "OK" "PGP signatures present ($(ls "$DIR"/*.asc | wc -l | tr -d ' ') files)"
else
  note "SKIP" "no .asc signatures - expected without GPG_SIGNING_KEY; Central WILL reject an unsigned bundle"
fi

echo
if [ "$fail" -ne 0 ]; then
  echo "::error::the publication does not meet Maven Central's requirements - it would be rejected after upload"
  echo "--- generated POM ---"
  cat "$POM"
  exit 1
fi
echo "publication satisfies Maven Central's metadata and artifact requirements"
