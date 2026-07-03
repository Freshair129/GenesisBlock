// GenesisBlockDB Android SDK (MARK XVI Phase B-2). Standalone Gradle root —
// deliberately NOT part of the Rust Cargo workspace or the root npm workspace;
// it only consumes build artifacts (the .so slices cargo-ndk produces) via
// jniLibs, it never invokes cargo itself. See docs/SPEC--MOBILE-SDK.md §B-2.
pluginManagement {
    // Plugin resolution is separate from dependency resolution
    // (dependencyResolutionManagement below) — `com.android.library` is
    // published to Google's Maven repo, not the Gradle Plugin Portal, so it
    // needs its own repositories block here or `plugins { id("com.android.library") }`
    // in build.gradle.kts fails with "Plugin ... was not found" even though
    // google()/mavenCentral() are declared elsewhere.
    repositories {
        gradlePluginPortal()
        google()
        mavenCentral()
    }
}

rootProject.name = "genesisdb-android"

include(":genesisdb")

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}
