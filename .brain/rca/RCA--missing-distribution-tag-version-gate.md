# RCA: Missing Version Match Gate for Distribution Tags

## Status / Date

Root cause confirmed; preflight guard added; hosted CI pending / 2026-09-27.

## Symptom

The engine release workflows publish N-API packages, standalone server
archives, and a GHCR image when any `v*` tag is pushed. A tag whose version
does not match the checked-out package versions could publish artifacts under
a misleading release version.

## Evidence

- `.github/workflows/release.yml` derives the npm version from `package.json`
  and runs publication on `v*` tags, but had no step comparing `GITHUB_REF_NAME`
  to the package version.
- `.github/workflows/server-distribution.yml` used `github.ref_name` for the
  GitHub Release and container tags while building the version from
  `Cargo.toml`; it had no equality check.
- Both workflows support PR dry runs, but those run with `IS_RELEASE=false`
  and do not exercise a real version tag.
- The latest release `v0.2.6` predates the merged server distribution workflow;
  no mismatched release is evidenced.

## Root Cause

The release workflows validated that packaging and builds succeeded but treated
the Git tag as trusted metadata. They did not compare that tag with the
lock-step version contract before reaching their independent publish jobs.

## Why the Issue Escaped Detection

The pull-request release dry runs validate package payloads without a release
tag, and the server PR workflow skips release-asset and GHCR publishing.
Consequently, neither path checked the tag-to-manifest relationship.

## Fix

Add version-contract jobs that run `npm run version:check` and compare a
release or repair tag with the version read by `scripts/version.mjs`. Make
every mutating release job depend on that preflight. The tag push remains
blocked from publication when the version files disagree.

## Proposed Prevention

Keep the manifest consistency and tag equality checks as dependencies of all
tag-triggered publication jobs. Add the version files and version script to
the release workflow path filters so future changes exercise the same CI gate.
