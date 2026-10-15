# RCA: Android SDK Serialization Types Are Missing From the Public Compile Classpath

## Status / Date

Root cause confirmed; dependency-scope correction and genesisdb-android 0.1.2 publication pending / 2026-09-27.

## Symptom

The clean Maven Central consumer failed compiling SDK calls because it could not
load `kotlinx.serialization.json.JsonElement` from the Android SDK API.

## Evidence

- Hosted run [36309923402](https://github.com/Freshair129/GenesisBlock/actions/runs/36309923402),
  job [108593755849](https://github.com/Freshair129/GenesisBlock/actions/runs/36309923402/job/108593755849),
  failed compiling `MavenCentralConsumerTest.kt` with `Cannot access class
  'kotlinx.serialization.json.JsonElement'` at `addNode` and
  `retrieveContext` calls.
- Public SDK types in `android/genesisdb/src/main/kotlin/dev/genesisblock/Types.kt`
  and methods in `GenesisDB.kt` expose `JsonElement`.
- The published `genesisdb-android:0.1.1` Gradle metadata places
  `kotlinx-serialization-json` in `releaseVariantReleaseRuntimePublication`,
  while `releaseVariantReleaseApiPublication` exposes only Kotlin stdlib. Its
  POM also marks serialization JSON with Maven scope `runtime`.
- Gradle's API dependency guidance says dependencies whose types appear in a
  library's public signatures belong in the consumer compile API:
  https://docs.gradle.org/current/userguide/java_library_plugin.html

## Root Cause

`kotlinx-serialization-json` was declared with Gradle `implementation`, even
though public Android SDK signatures expose `JsonElement`. The publication
therefore omits serialization JSON from consumers' compile classpath.

## Why the Issue Escaped Detection

The Central metadata gate checked required POM fields and companion artifacts,
but not whether dependencies exposed by public signatures were published in
compile scope. Earlier consumer runs failed at SDK setup, AndroidX metadata,
JVM target, and coroutine helper compilation before reaching this API type.

## Proposed Prevention

Declare serialization JSON as `api`, assert its compile scope in the generated
Maven publication metadata, publish a new immutable Android SDK patch version,
and run the clean emulator consumer against that published version without a
manual serialization dependency.
