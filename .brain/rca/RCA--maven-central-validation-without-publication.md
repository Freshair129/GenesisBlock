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
- After recovery run [36315684077](https://github.com/Freshair129/GenesisBlock/actions/runs/36315684077)
  reached `PUBLISHED`, Sonatype's status response had `purls: []` and
  `Deployment components info not found`. Direct Maven Central `HEAD` requests
  for both the `0.1.2` POM and AAR returned HTTP 200.

## Root Cause

The workflow uploaded a user-managed deployment but treated validation as the
release's terminal success state. It never called Sonatype's publish endpoint,
so `0.1.2` remained validated in the Portal and absent from the public Maven
repository. During recovery, the status API also omitted PURLs while the
deployment was publishing and after publication. Coordinate validation belongs
before the publish request, when Central reports the validated components; a
resume check must allow `PUBLISHING`/`PUBLISHED` by deployment ID without
sending another publish request.

## Why the Issue Escaped Detection

The status guard's self-test explicitly expected `VALIDATED` to pass. The
publishing workflow therefore went green at the validation boundary. The
consumer test was subsequently dispatched against `0.1.2`; the regular PR
consumer test defaulted to the already-published `0.1.1` artifact. Recovery
tests also assumed Central continued to return PURLs during publishing.

## Proposed Prevention

- Upload new releases with `publishingType=AUTOMATIC` and require the status
  guard to wait for `PUBLISHED` before succeeding.
- Add a guarded manual recovery workflow for an existing deployment: verify its
  Maven coordinate, publish only from `VALIDATED`, then wait for `PUBLISHED`.
- Resume `PUBLISHING`/`PUBLISHED` by deployment ID when the status API omits
  PURLs; verify availability through the clean public consumer and direct
  artifact checks.
- Run the state-guard self-test in pull-request CI so the publishing boundary
  is exercised before any release dispatch.
- Keep the clean public Central consumer check as evidence separate from
  upload and validation.

## Outcome (Measured)

- The local self-test passes all publisher-state, wrong-coordinate, missing-PURL,
  and no-duplicate-publish cases.
- The published `0.1.2` POM and AAR each return HTTP 200 from Maven Central.
- Clean Android Maven Central consumer run
  [36316221124](https://github.com/Freshair129/GenesisBlock/actions/runs/36316221124)
  passed, including its x86_64 emulator job.
- Hosted validation of this recovery regression change: pending PR CI.

## Version Diff

| From | To | Change |
|---|---|---|
| Current Maven release guard | Published-state guard | Require `PUBLISHED` instead of accepting `VALIDATED`; check coordinates before publish and allow read-only recovery/resume states when the API omits PURLs. |
