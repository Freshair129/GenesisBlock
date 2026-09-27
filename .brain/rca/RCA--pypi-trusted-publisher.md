# RCA: PyPI Trusted Publishing Rejected the First Release

## Status / Date

Suspected external Trusted Publisher configuration mismatch; publication is
blocked pending account-side inspection / 2026-09-27.

## Symptom

The `python-v0.1.0` workflow [36305930749](https://github.com/Freshair129/GenesisBlock/actions/runs/36305930749)
passed tag/version validation and built a wheel and sdist, then failed during
PyPI upload. The `genesisblockdb-client` project remains unavailable from the
public PyPI JSON API.

## Evidence

- The version-tag validation passed for package version `0.1.0`.
- Wheel and sdist build plus Twine metadata validation succeeded.
- `pypa/gh-action-pypi-publish` reports that the failure generally indicates a
  Trusted Publisher configuration error, while noting an external GitHub/PyPI
  service error is also possible.
- The workflow OIDC claims are `sub=repo:Freshair129/GenesisBlock:environment:pypi`,
  `repository=Freshair129/GenesisBlock`,
  `workflow_ref=Freshair129/GenesisBlock/.github/workflows/python-publish.yml@refs/tags/python-v0.1.0`,
  and `environment=pypi`.
- The project was not publicly discoverable before or after the upload attempt.

## Root Cause

Suspected: PyPI has no Trusted Publisher registration matching the workflow's
OIDC claims, or the external PyPI/GitHub publisher handshake failed. The
repository cannot inspect the account-side publisher configuration, so this
cause is not confirmed. No package was published.

## Why the Issue Escaped Detection

The manual publisher workflow is intentionally dry-run only. It builds and
checks metadata without contacting PyPI's upload endpoint, so it could not
validate the external account binding. The first tag-triggered upload was the
first end-to-end publisher handshake.

## Fix (Decided)

Inspect or create the PyPI Trusted Publisher for project
`genesisblockdb-client`, repository `Freshair129/GenesisBlock`, workflow
`python-publish.yml`, and GitHub Environment `pypi`. After account-side
configuration is corrected, rerun the existing `python-v0.1.0` tag workflow.
The workflow now includes a clean PyPI registry install check after a
successful upload.

## Outcome (Measured)

Blocked: package artifacts were built and validated, but upload was rejected
and the public project endpoint still returns not found. Registry installation
has not run.

## Proposed Prevention

Keep Trusted Publisher identity claims in the release evidence and exercise the
actual upload only on an explicit version tag. Retain an immediate clean-registry
install check after upload so publish success is not confused with consumer
availability.
