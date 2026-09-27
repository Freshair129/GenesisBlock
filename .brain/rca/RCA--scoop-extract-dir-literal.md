# RCA: Scoop Uses a Literal Version in `extract_dir`

## Status / Date

Root cause confirmed; manifest correction pending hosted consumer rerun / 2026-09-27.

## Symptom

The Scoop clean-consumer job downloaded and hash-verified the v0.2.7 Windows
archive, then failed during extraction:

```text
Could not find 'genesisblockdb-server-v$version-x86_64-pc-windows-msvc'!
```

## Evidence

- Hosted run [36308660672](https://github.com/Freshair129/GenesisBlock/actions/runs/36308660672),
  job [108590184058](https://github.com/Freshair129/GenesisBlock/actions/runs/36308660672/job/108590184058),
  resolved `scoop`, downloaded the release ZIP, and verified its SHA-256 before
  extraction failed.
- The manifest set `extract_dir` to
  `genesisblockdb-server-v$version-x86_64-pc-windows-msvc`.
- The actual ZIP contains the top-level directory
  `genesisblockdb-server-v0.2.7-x86_64-pc-windows-msvc`.
- Scoop's manifest documentation defines `extract_dir` as the directory name
  to extract; `$version` expansion is documented for install scripts, not this
  property: https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests

## Root Cause

The manifest treated `extract_dir` as a version-templated property. Scoop uses
the field as a literal archive directory name, so it searched for a path that
is not present in the ZIP.

## Why the Issue Escaped Detection

The manifest had only been JSON-validated. The first clean Scoop install job
initially failed earlier because the Scoop shim path was absent. After that
separate issue was fixed, the same job reached archive extraction and exposed
the incorrect directory name.

## Proposed Prevention

Keep the clean Scoop installation and persistence smoke as a distribution CI
gate. Derive the manifest's `extract_dir` from the actual release archive path
when updating package metadata.
