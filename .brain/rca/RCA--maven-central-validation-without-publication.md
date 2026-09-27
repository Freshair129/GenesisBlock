# RCA: Maven Central Validation Was Mistaken for Publication

## Status / Date

Root cause confirmed; release-workflow correction pending / 2026-09-27.

## Symptom

The Maven Central publishing workflow reported success for Android SDK
`0.1.2`, but a clean consumer could not resolve the artifact from Maven
Central.

## Evidence

- Workflow run [36313190467](https://github.com/Freshair129/GenesisBlock/actions/runs/36313190467)
  uploaded deployment `6147063c-4cbb-4b3e-aa94-ec442dcdf71e`; the status
  response listed `genesisdb-android@0.1.2` and
  `deploymentState: VALIDATED`.
- The same run uploaded with `publishingType=USER_MANAGED`. The script
  `scripts/central-await-validation.sh` classified `VALIDATED` as `OK` and
  returned without issuing a publish request.
- Clean Maven Central consumer run
  [36313815882](https://github.com/Freshair129/GenesisBlock/actions/runs/36313815882)
  failed its `Android Maven Central consumer (x86_64 emulator)` job while
  requesting version `0.1.2`.
- Sonatype's [Publisher API documentation](https://central.sonatype.org/publish/publish-portal-api/)
  defines `VALIDATED` as awaiting manual publication and `PUBLISHED` as
  successfully available on Maven Central. A validated user-managed deployment
  must be published with `POST /api/v1/publisher/deployment/{deploymentId}`.

## Root Cause

The workflow uploaded a user-managed deployment but treated validation as the
release's terminal success state. It never called Sonatype's publish endpoint,
so `0.1.2` remained validated in the Portal and absent from the public Maven
repository.

## Why the Issue Escaped Detection

The status guard's self-test explicitly expected `VALIDATED` to pass. The
publishing workflow therefore went green at the validation boundary. The
consumer test was subsequently dispatched against `0.1.2`; the regular PR
consumer test defaulted to the already-published `0.1.1` artifact.

## Proposed Prevention

- Upload new releases with `publishingType=AUTOMATIC` and require the status
  guard to wait for `PUBLISHED` before succeeding.
- Add a guarded manual recovery workflow for an existing deployment: verify its
  Maven coordinate, publish only from `VALIDATED`, then wait for `PUBLISHED`.
- Run the state-guard self-test in pull-request CI so the publishing boundary
  is exercised before any release dispatch.
- Keep the clean public Central consumer check as evidence separate from
  upload and validation.

## Version Diff

| From | To | Change |
|---|---|---|
| Current Maven release guard | Published-state guard | Require `PUBLISHED` instead of accepting `VALIDATED`; add a coordinate-checked recovery path for deployment `6147063c-4cbb-4b3e-aa94-ec442dcdf71e`. |
