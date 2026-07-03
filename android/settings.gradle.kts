// GenesisBlockDB Android SDK (MARK XVI Phase B-2). Standalone Gradle root —
// deliberately NOT part of the Rust Cargo workspace or the root npm workspace;
// it only consumes build artifacts (the .so slices cargo-ndk produces) via
// jniLibs, it never invokes cargo itself. See docs/SPEC--MOBILE-SDK.md §B-2.
rootProject.name = "genesisdb-android"

include(":genesisdb")

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}
