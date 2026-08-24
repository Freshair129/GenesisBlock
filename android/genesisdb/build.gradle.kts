plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.serialization")
    id("maven-publish")
}

// Single source of truth for the published coordinate's version — keep in
// sync with modules.json's `genesisdb-android` surface entry by hand
// (per-surface versions are intentionally not SSOT-gated, see
// docs/SPEC--MOBILE-SDK.md "Versioning model").
val genesisdbAndroidVersion = "0.1.0"

android {
    namespace = "dev.genesisblock"
    compileSdk = 34

    defaultConfig {
        minSdk = 24
        // arm64-v8a + armeabi-v7a match the two targets built by
        // .github/workflows/mobile-build.yml (aarch64-linux-android,
        // armv7-linux-androideabi) — see docs/SPEC--MOBILE-SDK.md §0-B.
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    // The .so slices are NOT built here — this module never invokes cargo.
    // CI (mobile-build.yml) copies the cargo-ndk output into
    // src/main/jniLibs/<abi>/libgenesis_block_native.so before assembling.
    // Local dev: run `scripts/gen-android-jnilibs.sh` (or copy manually) first.
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    // Publishing the "release" variant as a Maven component — AGP's
    // singleVariant API (needed for `publications { from(components["release"]) }`
    // below; a bare `com.android.library` module has no software component
    // to publish without this).
    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.8.1")
}

// Publishes to GitHub Packages (not Maven Central — see issue #125): zero new
// accounts/secrets, reuses the repo's own GITHUB_TOKEN in CI. Tradeoff
// consumers must know about: unlike Maven Central, GitHub Packages requires
// authentication for EVERY read of a Maven artifact, even on a public repo —
// a consumer needs a GitHub PAT with `read:packages` scope in their own
// ~/.gradle/gradle.properties or settings.gradle repository credentials, not
// just the dependency coordinate. Documented in android/README.md.
publishing {
    publications {
        register<MavenPublication>("release") {
            groupId = "dev.genesisblock"
            artifactId = "genesisdb-android"
            version = genesisdbAndroidVersion

            // AGP creates the "release" component asynchronously — reading
            // components["release"] before project evaluation finishes
            // throws "Software component 'release' not found".
            afterEvaluate {
                from(components["release"])
            }
        }
    }

    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/Freshair129/GenesisBlock")
            credentials {
                username = System.getenv("GITHUB_ACTOR")
                password = System.getenv("GITHUB_TOKEN")
            }
        }
    }
}
