# RCA: Android Central Consumer Test Omits Coroutines Compile Dependency

## Status / Date

Root cause confirmed; test-fixture dependency correction pending hosted rerun / 2026-09-27.

## Symptom

The Android Maven Central consumer job reached Kotlin test source compilation,
then failed on its coroutine helper import and calls:

```text
Unresolved reference: kotlinx
Unresolved reference: runBlocking
Suspend function 'open' should be called only from a coroutine
```

## Evidence

- Hosted run [36309590832](https://github.com/Freshair129/GenesisBlock/actions/runs/36309590832),
  job [108592833082](https://github.com/Freshair129/GenesisBlock/actions/runs/36309590832/job/108592833082),
  passed Gradle configuration and failed compiling
  `MavenCentralConsumerTest.kt` on the unresolved coroutine references.
- The test imports `kotlinx.coroutines.runBlocking` and invokes suspend SDK
  methods, while the fixture had no direct test dependency on coroutines.
- The published artifact POM at
  `https://repo.maven.apache.org/maven2/io/github/freshair129/genesisdb-android/0.1.1/genesisdb-android-0.1.1.pom`
  declares `kotlinx-coroutines-android:1.8.1` with Maven scope `runtime`.
  That runtime dependency does not provide the test source's compile-time
  `runBlocking` symbol.
- The library's own Gradle tests declare coroutines test support explicitly.

## Root Cause

The standalone consumer test used `runBlocking` without declaring a test
compile dependency that provides it. The artifact's internal coroutine runtime
is published as runtime-only and is not a substitute for the fixture's test
source dependency.

## Why the Issue Escaped Detection

The new fixture had not reached Kotlin source compilation in prior runs: the
first run stopped at AndroidX metadata checks and the next run stopped at JVM
target validation. Each earlier failure prevented this missing test dependency
from appearing.

## Proposed Prevention

Declare coroutine helpers explicitly in the fixture's `androidTestImplementation`
configuration and retain the clean Maven Central compile-and-run acceptance job.
