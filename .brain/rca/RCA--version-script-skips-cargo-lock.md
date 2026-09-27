# RCA: Version Script Skips Cargo.lock

## Status / Date

Root cause confirmed; version reader/writer and CI gate corrected / 2026-09-27.

## Symptom

Release CI jobs that run `cargo build --locked` fail after the engine version
is bumped because the root package's lockfile version remains at the prior
release.

## Evidence

- On PR #184 head `4e5cf340a39e7bdfa80dfa6aa2e0d4aef6b0c2dd`, `Cargo.toml`,
  `package.json`, and `modules.json` declare `0.2.7`, while the
  `genesis-block-native` entry in `Cargo.lock` declares `0.2.6`.
- The Server Distribution build and Docker smoke jobs, plus Python and Go
  live-server consumer jobs, reported that Cargo could not update the lockfile
  because `--locked` was passed.
- `scripts/version.mjs` read and wrote `Cargo.toml`, `package.json`, and
  `modules.json`; `collectVersions()` and `cmdCheck()` did not inspect
  `Cargo.lock`.

## Root Cause

The version synchronization script omitted the root package version in
`Cargo.lock`. The bump command therefore left a stale lockfile entry, and the
version consistency gate could not detect it.

## Why the Issue Escaped Detection

The version gate checked only the files that the version script already
managed. The test matrix had versioned server builds using `--locked`, but no
gate established that the lockfile's root package version matched the
manifest before those builds ran.

## Fix

Teach `scripts/version.mjs` to read, write, and validate the unique
`genesis-block-native` package version in `Cargo.lock`. Update the canonical
version record and run the `0.2.7` set operation so all version files agree.

## Proposed Prevention

Keep `Cargo.lock` in the version synchronization contract and run
`npm run version:check` before release builds. Continue using `cargo build
--locked` in distribution CI so lockfile drift fails closed.
