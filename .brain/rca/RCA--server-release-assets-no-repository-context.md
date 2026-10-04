# RCA: Server Release Asset Upload Lacks Repository Context

## Status / Date

Root cause confirmed; workflow fix pending hosted validation / 2026-09-27.

## Symptom

The `Attach standalone server binaries to GitHub Release` job in server
distribution run [36304122190](https://github.com/Freshair129/GenesisBlock/actions/runs/36304122190)
failed at `Create/update release and upload server assets`. The four target
builds, Docker persistence smoke, and GHCR publication succeeded, but the
release contained no standalone server archives or SHA-256 sidecars.

## Evidence

- The failed job downloaded its server artifacts successfully before running
  the release upload step.
- Hosted logs report `failed to run git: fatal: not a git repository (or any
  of the parent directories): .git` when `gh release view` and
  `gh release create` attempt to determine the repository.
- `.github/workflows/server-distribution.yml` does not check out the repository
  in `release-assets` and calls `gh release view`, `gh release create`, and
  `gh release upload` without `--repo`.
- The server binaries, Docker persistence smoke, and GHCR image jobs succeeded
  in the same run; their outputs are separate from this release attachment
  failure.

## Root Cause

The release-assets job runs without a Git working tree. GitHub CLI therefore
cannot infer `Freshair129/GenesisBlock` for release operations. Because the
workflow commands omit `--repo`, asset attachment fails even though the
release tag, token, and downloaded archive files are valid.

## Why the Issue Escaped Detection

The release-asset step runs only after a version-tag push. Pull-request and
manual distribution checks build and package each target but do not execute
this tag-only GitHub Release upload command. The preceding build and container
checks therefore passed without exercising repository discovery in `gh`.

## Fix (Decided)

Pass `${GITHUB_REPOSITORY}` explicitly to each `gh release` command in the
release-assets job. This gives GitHub CLI the repository context it needs and
keeps the job independent of a checked-out worktree.

## Outcome (Measured)

Manually attached all four CI-built archives and their `.sha256` sidecars to
v0.2.7 using `gh release upload --repo Freshair129/GenesisBlock`; GitHub
shows nine total release assets, and the uploaded archive digests match the
verified checksums. The workflow's tag-only upload path still awaits the next
engine tag because replaying v0.2.7 could overwrite already-published binaries.

## Proposed Prevention

Keep repository identity explicit in release automation that operates from
downloaded artifacts without a checkout. Verify release asset names and
checksums from the resulting public release before advertising installer
commands.
