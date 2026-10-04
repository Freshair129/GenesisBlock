# RCA: Python Query Test Expects Obsolete Request Arguments

## Status / Date

Confirmed / 2026-09-27

## Symptom

The hosted Python Distribution workflow at PR head b8bbeb517a1737eb58c0e321cf9378d9abb26456 fails `test_query_posts_hql` on Python 3.10, 3.11, 3.12, and 3.13.

The test expects `requests.post(url, json=payload)`. The observed call also includes `headers={'Content-Type': 'application/json'}` and `timeout=10.0`.

## Evidence

- GitHub Actions run 36280213962: package build and clean wheel installation pass; the `Run unit tests against installed wheel` step fails on all four Python versions.
- The failure is in `genesisdb-python/tests/test_client.py:18-21`, where `test_query_posts_hql` asserts the call without headers or timeout.
- `genesisdb-python/genesisdb/client.py:10` sets the default timeout to 10.0 seconds.
- `genesisdb-python/genesisdb/client.py:29-37` builds the JSON content-type header and passes headers plus timeout to each request.
- The new contract tests for timeout and API-key behavior pass in the same hosted workflow.

## Root Cause

The query test's expected mock call was not updated when the shared request helper began adding the JSON content-type header and finite timeout. The production request behavior matches the client contract; the test expectation is stale.

## Why the Issue Escaped Detection

The Python Distribution run on the earlier PR head e524c323 passed before the mainline transport changes were merged. The combined branch was then evaluated by hosted CI, which exposed the stale assertion. Local tests were not run for this correction per task instructions.

## Fix (Decided)

Update only the `test_query_posts_hql` expected call to include `headers={"Content-Type": "application/json"}` and `timeout=10.0`, preserving the existing URL and JSON payload.

## Proposed Prevention

When shared request defaults change, update request-mock contract assertions alongside the client behavior. Keep the clean-wheel Python version matrix as the release gate.

## Outcome (Measured)

Pending hosted CI for the correction commit. No local tests were run.
