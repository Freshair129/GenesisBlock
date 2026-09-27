# RCA: Version Script Skips Lockfiles

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
- On PR #184, the top-level version and root package version in
  `package-lock.json` were both `0.2.0` while `package.json` declared `0.2.7`.
- The Server Distribution build and Docker smoke jobs, plus Python and Go
  live-server consumer jobs, reported that Cargo could not update the lockfile
  because `--locked` was passed.
- `scripts/version.mjs` read and wrote `Cargo.toml`, `package.json`, and
  `modules.json`; it did not inspect `Cargo.lock` or `package-lock.json`.
- [npm's package-lock documentation](https://docs.npmjs.com/files/package-lock.json/)
  states that the root package version in `package-lock.json` matches
  `package.json`.

## Root Cause

The version synchronization script omitted lockfiles. The bump command
therefore left stale root package versions in `Cargo.lock` and
`package-lock.json`, and the version consistency gate could not detect them.

## Why the Issue Escaped Detection

The version gate checked only the files that the version script already
managed. Version bumps had not regenerated or validated lockfile root
metadata. Server builds using `--locked` exposed the Cargo mismatch; no gate
checked either lockfile against its package manifest.

## Fix

Teach `scripts/version.mjs` to read, write, and validate the unique
`genesis-block-native` version in `Cargo.lock` and both root version fields in
`package-lock.json`. Update the canonical version record and run the `0.2.7`
set operation so all version files agree.

## Proposed Prevention

Keep both lockfiles in the version synchronization contract and run
`npm run version:check` before release builds. Continue using `cargo build
--locked` in distribution CI so Cargo lockfile drift fails closed.
