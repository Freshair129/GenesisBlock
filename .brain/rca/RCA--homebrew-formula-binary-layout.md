# RCA: Homebrew Formula Binary Lookup Misses the Extracted Archive

## Status / Date

Root cause confirmed; formula correction verified in hosted consumers / 2026-09-27.

## Symptom

The Homebrew clean-consumer job failed on macOS and Ubuntu during
`brew install` with:

```text
TypeError: no implicit conversion of nil into String
```

The install command had downloaded the v0.2.7 release archive and selected the
formula; the server persistence smoke did not run.

## Evidence

- Hosted run [36308304499](https://github.com/Freshair129/GenesisBlock/actions/runs/36308304499)
  failed at `Install from the tap and smoke-test the server` on macOS and Ubuntu.
- Homebrew logs show the formula was selected and the release asset was fetched
  before the TypeError.
- The formula uses
  `Dir["genesisblockdb-server-v#{version}-*/genesis-db-server"].first` and
  passes that result directly to `bin.install`.
- The published tar archive contains a top-level release directory plus
  `genesis-db-server` and `LICENSE`; Homebrew's build working directory can
  already be rooted at the extracted release directory, making that
  version-prefixed glob return nil.

## Root Cause

The formula assumes one fixed working-directory layout after Homebrew extracts
the archive. When Homebrew has already changed into the archive's root
directory, the version-prefixed glob does not match. Its `.first` result is
nil, which reaches `bin.install` and raises the TypeError.

## Why the Issue Escaped Detection

The formula was added after the binaries had been published, and no clean
Homebrew install fixture exercised the extracted archive layout before this
PR.

## Fix (Decided)

Search recursively for the single `genesis-db-server` file in the extracted
archive and fail with an explicit message if it is absent before calling
`bin.install`.

## Outcome (Measured)

Hosted run [36308660672](https://github.com/Freshair129/GenesisBlock/actions/runs/36308660672)
passed the clean consumer and persistence smoke on macos-15 (Apple Silicon),
macos-15-intel, and ubuntu-latest.

## Proposed Prevention

Keep the package-manager consumer jobs as required distribution checks and
validate the packaged executable from the archive layout the package manager
actually supplies.
