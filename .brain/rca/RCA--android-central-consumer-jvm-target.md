# RCA: Android Central Consumer Has Mismatched JVM Targets

## Status / Date

Root cause confirmed; target alignment verified; separate coroutine test dependency defect found / 2026-09-27.

## Symptom

After the AndroidX dependency check passed, the Android Maven Central consumer
job failed compiling the instrumented Kotlin tests:

```text
Inconsistent JVM-target compatibility detected for tasks
'compileDebugAndroidTestJavaWithJavac' (1.8) and
'compileDebugAndroidTestKotlin' (17).
```

## Evidence

- Hosted run [36309301502](https://github.com/Freshair129/GenesisBlock/actions/runs/36309301502),
  job [108592035305](https://github.com/Freshair129/GenesisBlock/actions/runs/36309301502/job/108592035305),
  passed AndroidX/AAR metadata checks and failed at
  `:app:compileDebugAndroidTestKotlin` with the JVM target mismatch above.
- The published Android library's build configuration in
  `android/genesisdb/build.gradle.kts` sets Java source/target compatibility
  and Kotlin `jvmTarget` to 17.
- The independent fixture app in `mobile-acceptance/android-central/` did not
  configure either compile target, leaving Java at 1.8 while Kotlin compiled
  at 17.

## Root Cause

The standalone consumer app's Java and Kotlin compile tasks use different JVM
targets. The consumer fixture omitted the target configuration that the
published Android module declares, so Gradle's compatibility validation
stopped compilation before instrumentation ran.

## Why the Issue Escaped Detection

The initial fixture run stopped at AndroidX metadata validation, before Kotlin
compilation. The next hosted run reached the test compiler only after the
AndroidX property was added. Existing Android SDK tests compile the library's
own module, whose Java and Kotlin targets are already aligned.

## Proposed Prevention

Keep the clean consumer fixture's Java and Kotlin targets aligned with the
published Android artifact, and retain the emulator test as a distribution
acceptance gate.

## Outcome (Measured)

Hosted run [36309590832](https://github.com/Freshair129/GenesisBlock/actions/runs/36309590832)
passed JVM target validation and reached Kotlin source compilation. The
compiler then reported an unresolved coroutine test dependency, recorded in
`RCA--android-central-consumer-test-coroutines.md`.
