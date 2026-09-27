# RCA: Central Consumer Places the Android SDK Only in the Test APK

## Status / Date

Root cause confirmed; fixture dependency-scope correction verified / 2026-09-27.

## Symptom

The Maven Central consumer compiled but both emulator round-trip tests failed
when the Android SDK called `System.loadLibrary`:

```text
UnsatisfiedLinkError: dlopen failed: library "libgenesis_block_native.so" not found
```

## Evidence

- Hosted Mobile Build run [36311090685](https://github.com/Freshair129/GenesisBlock/actions/runs/36311090685),
  job [108597033514](https://github.com/Freshair129/GenesisBlock/actions/runs/36311090685/job/108597033514),
  compiled the tests and failed both at runtime on the missing JNI library.
- Downloaded the public Maven Central artifact
  `genesisdb-android:0.1.1`; its AAR contains `jni/arm64-v8a/`,
  `jni/armeabi-v7a/`, and `jni/x86_64/libgenesis_block_native.so`.
- The consumer fixture declares the SDK as `androidTestImplementation`, while
  `MavenCentralConsumerTest` calls `GenesisDB.open` with the instrumented app's
  `targetContext`. The app itself therefore has no SDK dependency to package
  the JNI libraries into its APK.

## Root Cause

The fixture attached the published SDK to the instrumentation-test source set
instead of the application source set. Its test code could compile against the
wrapper classes, but the target application APK did not receive the AAR's JNI
libraries, so loading the SDK failed at runtime.

## Why the Issue Escaped Detection

Earlier jobs validated the coordinate, POM metadata, compilation, and the
repository-built AAR in a different consumer fixture. None executed a runtime
round trip from a clean app whose dependency came from Maven Central.

## Proposed Prevention

Declare the published SDK with the app's `implementation` configuration so the
consumer app packages the AAR and native slices as a real application would.
Keep the clean Maven Central emulator test as the runtime distribution gate.

## Outcome (Measured)

Hosted run [36311724941](https://github.com/Freshair129/GenesisBlock/actions/runs/36311724941),
job [108598834274](https://github.com/Freshair129/GenesisBlock/actions/runs/36311724941/job/108598834274),
passed both `publishedArtifactOpensAndWrites` and
`publishedArtifactPersistsAcrossHandles` on the x86_64 emulator using the
published Maven Central `0.1.1` AAR.
