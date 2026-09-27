# RCA: Invalid PyPI Publish Workflow YAML

## Status / Date

Root cause confirmed; YAML scalar corrected; hosted validation pending / 2026-09-27.

## Symptom

GitHub marked the `python-publish.yml` workflow run failed before starting any
jobs. The run annotation reported: `Invalid workflow file:
.github/workflows/python-publish.yml#L54` and a YAML syntax error on line 54.

## Evidence

- Main push run [36302415375](https://github.com/Freshair129/GenesisBlock/actions/runs/36302415375)
  had conclusion `failure` and no jobs.
- The workflow error annotation points to line 54, the `run:` command for the
  manual-dispatch dry-run notice.
- That command had an unquoted YAML plain scalar containing `DRY RUN: wheel`.
- After the edit, PyYAML parsed the workflow and found the `publish` job; the
  local diff check passed.

## Root Cause

The plain YAML scalar after `run:` included a colon followed by a space in
`DRY RUN: wheel`. The quotes around the shell `echo` argument did not quote
the YAML scalar itself, so GitHub's workflow parser rejected the file.
Evidence is in `.github/workflows/python-publish.yml:52-55`.

## Why the Issue Escaped Detection

The workflow triggers only for `python-v*` tag pushes and manual dispatch;
the pull-request checks did not execute or parse this workflow. The invalid
syntax was reported after the workflow file reached `main`.

## Fix

Use a YAML block scalar for the dry-run shell command. Keep the publish step
conditional on a push to `refs/tags/python-v*`.

## Outcome (Measured)

Local PyYAML parsing succeeded and reported the `publish` job; `git diff
--check` passed. GitHub hosted validation for the corrected workflow is
pending.

## Proposed Prevention

Validate workflow YAML in pull requests, including workflows whose own
triggers do not include `pull_request`.
