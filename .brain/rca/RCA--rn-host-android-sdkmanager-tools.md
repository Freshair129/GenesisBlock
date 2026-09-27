# RCA: Android Consumer Workflow Requests Removed SDK Tools Package

## Status / Date

Root cause confirmed; upgrade to the upstream action fix pending hosted validation / 2026-09-27.

## Symptom

Published React Native host run [35597783607](https://github.com/Freshair129/GenesisBlock/actions/runs/35597783607)
failed before it staged the Android host. The failing step was
`android-actions/setup-android@v3`; the later diagnostics also failed because
`rn-host/android` had not been created.

## Evidence

- The hosted `sdkmanager` log shows the action invoked `sdkmanager tools`,
  followed by `Warning: Failed to find package 'tools'` and exit code 1.
- The fixture-generation step was skipped after SDK setup failed.
- The `Show what resolved` diagnostic then failed with `No such file or
  directory` for its configured `rn-host/android` working directory. This is a
  cascade from the missing fixture, not the initial cause.
- The current workflow uses `android-actions/setup-android@v3` in
  `.github/workflows/rn-host-acceptance.yml` and
  `.github/workflows/mobile-build.yml`.
- Upstream `android-actions/setup-android` documents that the legacy `tools`
  SDK package is no longer served and that v4 no longer requests it.

## Root Cause

The Android SDK setup action version in both consumer and mobile acceptance
workflows requests the obsolete SDK package `tools`. Google no longer serves
that package, so SDK setup exits before either Android fixture or emulator job
can run. The absent working directory is a later diagnostic failure caused by
the fixture step being skipped.

## Why the Issue Escaped Detection

The published React Native host job last ran on 2026-09-21 with action v3 and
failed before dependency resolution. Earlier green host runs predate removal of
the SDK package. The regular root distribution and server workflows do not
execute this Android setup path.

## Fix (Decided)

Upgrade affected workflows to `android-actions/setup-android@v4`, which no
longer installs the removed `tools` package. Make the Android dependency
diagnostic skip cleanly when setup or fixture staging fails, so it does not
create a second failure that hides the first. Keep the published Central
consumer path and dependency assertions.

## Outcome (Measured)

Pending: rerun the published React Native Android host and Android emulator
acceptance jobs and confirm SDK setup, package resolution, and native tests
succeed.

## Proposed Prevention

Run the published-package Android host workflow after SDK action or Android
runner-image changes, and retain the first failing SDK-manager step as the
primary diagnosis when later fixture paths are missing.
