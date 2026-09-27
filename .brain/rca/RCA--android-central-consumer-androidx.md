# RCA: Android Central Consumer Fixture Omits AndroidX Configuration

## Status / Date

Root cause confirmed; fixture correction pending hosted consumer rerun / 2026-09-27.

## Symptom

The Android Maven Central consumer job failed while configuring its instrumented
test runtime, before either consumer test ran:

```text
Configuration `:app:debugAndroidTestRuntimeClasspath` contains AndroidX dependencies,
but the `android.useAndroidX` property is not enabled.
```

## Evidence

- Hosted run [36308660627](https://github.com/Freshair129/GenesisBlock/actions/runs/36308660627),
  job [108590184161](https://github.com/Freshair129/GenesisBlock/actions/runs/36308660627/job/108590184161),
  completed Android SDK setup and emulator boot, then failed at
  `:app:checkDebugAndroidTestAarMetadata`.
- The error lists `androidx.test.ext:junit:1.1.5` and
  `androidx.test:runner:1.5.2` from the fixture's `androidTest` dependencies.
- The blank app fixture at `mobile-acceptance/android-central/` had no
  `gradle.properties`. The repository's Android SDK project enables
  `android.useAndroidX=true` in `android/gradle.properties`.
- The parent mobile SDK spec defines a blank consumer app resolving the
  published artifact as its distribution acceptance boundary.

## Root Cause

The new standalone consumer fixture uses AndroidX test dependencies but omitted
the Gradle project property required to enable AndroidX. Unlike the main
`android/` project, the fixture is an independent Gradle build and does not
inherit that project's `gradle.properties`.

## Why the Issue Escaped Detection

The fixture compiled its app configuration but had not completed a connected
instrumented-test build. Earlier package checks validated Maven coordinates and
publication metadata without running this blank-app AndroidX test classpath.

## Proposed Prevention

Keep this blank-project emulator job as a required distribution consumer gate,
and configure Gradle properties in the fixture itself instead of relying on
settings from the repository's Android SDK project.
