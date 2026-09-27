# RCA: Scoop Shim Path Is Not Available to the Consumer Step

## Status / Date

Root cause confirmed; workflow correction pending hosted rerun / 2026-09-27.

## Symptom

The Scoop clean-consumer job failed immediately after the install step with:

```text
The term 'scoop' is not recognized as a name of a cmdlet
```

The failure occurred at `scoop install <manifest URL>`; manifest parsing and
binary installation did not run.

## Evidence

- Hosted run [36308304499](https://github.com/Freshair129/GenesisBlock/actions/runs/36308304499)
  completed the `Install Scoop` step, then failed in
  `Install from the public manifest URL and smoke-test the server`.
- The later step's PowerShell process could not resolve the `scoop` command.
- The workflow installed Scoop in one GitHub Actions step but did not add its
  per-user `scoop\shims` directory to `GITHUB_PATH`.

## Root Cause

Environment-variable changes made by the Scoop installer apply to its current
PowerShell process. GitHub Actions starts a fresh process for each later step;
without persisting the shim path through `GITHUB_PATH`, the subsequent step
cannot locate `scoop`.

## Why the Issue Escaped Detection

The manifest JSON was validated locally, but the first workflow that installed
Scoop and invoked it from a separate hosted step was this clean-consumer job.

## Fix (Decided)

Add the installed Scoop shims directory to the workflow's `GITHUB_PATH` during
installation so subsequent steps can resolve the command.

## Outcome (Measured)

Pending: rerun the Windows consumer job; it must install the manifest, launch
the published server binary, and pass the persistence smoke.

## Proposed Prevention

Keep package-manager setup and consumption in separate GitHub Actions steps
and persist any required command paths through the workflow's supported path
environment file.
