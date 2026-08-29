plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.serialization")
    id("maven-publish")
    id("signing")
}

// Single source of truth for the published coordinate's version — keep in
// sync with modules.json's `genesisdb-android` surface entry by hand
// (per-surface versions are intentionally not SSOT-gated, see
// docs/SPEC--MOBILE-SDK.md "Versioning model").
val genesisdbAndroidVersion = "0.1.1"

// Maven COORDINATE group, overridable with -PgenesisdbGroup.
//
// DEFAULTS to the existing `dev.genesisblock` so the GitHub Packages publish in
// release.yml is completely unchanged. Hard-coding the Central group here
// instead would have been an invisible breaking change: the next release would
// publish to GitHub Packages under a new coordinate while
// react-native-genesisdb still asks for `dev.genesisblock:genesisdb-android`,
// and every Android consumer's resolve would fail. Central adoption is additive
// until the artifact actually exists there and consumers have been migrated -
// you cannot point a consumer at an artifact that is not published yet.
//
// Maven Central needs a namespace you can prove you own. `dev.genesisblock` is
// not obtainable (genesisblock.dev belongs to an unrelated business), so the
// Central workflow passes -PgenesisdbGroup=io.github.freshair129, which IS
// verifiable from the GitHub account owning this repo.
//
// None of this touches the Kotlin package, and it must not: JNI symbol names
// derive from a class's fully-qualified name, so the native entry points are
// literally `Java_dev_genesisblock_GenesisDB_native*`. A Maven groupId and a
// JVM package are independent identifiers; renaming the package to match a
// groupId would break every native binding at load time.
val genesisdbAndroidGroup: String =
    (findProperty("genesisdbGroup") as String?) ?: "dev.genesisblock"

android {
    namespace = "dev.genesisblock"
    compileSdk = 34

    defaultConfig {
        minSdk = 24
        // These three ABI names match the three Rust targets built by
        // .github/workflows/mobile-build.yml and release.yml
        // (aarch64-linux-android, armv7-linux-androideabi,
        // x86_64-linux-android) — see docs/SPEC--MOBILE-SDK.md §0-B. Note the
        // spellings differ: abiFilters and jniLibs/ take the Android ABI name,
        // cargo-ndk takes the Rust triple.
        //
        // x86_64 is the emulator ABI. The default Android Studio AVD on a
        // Windows or Linux dev machine is x86_64, so an .aar carrying only
        // the two ARM slices cannot be run in an emulator at all —
        // System.loadLibrary finds no matching slice and the consumer is
        // limited to physical hardware. Worth roughly +7-10 MiB uncompressed.
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }

        // Required for the src/androidTest instrumented suite. Until it was
        // added, the only Kotlin tests here were pure-JVM and loaded no native
        // library at all, so System.loadLibrary and every JNI entry point were
        // unexercised on an Android runtime.
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
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
            // Maven Central REQUIRES both a sources and a javadoc jar; a
            // publication missing either is rejected at validation, after
            // upload. Only the sources jar was produced before.
            withSourcesJar()
            withJavadocJar()
        }
    }
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.8.1")

    // Instrumented (on-device / emulator) suite - src/androidTest.
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test:runner:1.5.2")
    androidTestImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.8.1")
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
            groupId = genesisdbAndroidGroup
            artifactId = "genesisdb-android"
            version = genesisdbAndroidVersion

            // Maven Central rejects a publication missing any of these. They
            // are cheap to add and impossible to add retroactively to a
            // version that is already released, so they go in before the
            // first Central publish rather than after the first rejection.
            pom {
                name.set("GenesisDB Android")
                description.set(
                    "Embedded GenesisBlockDB for Android - a local-first hybrid " +
                        "semantic-graph and vector engine that runs in-process, with no server.",
                )
                url.set("https://github.com/Freshair129/GenesisBlock")
                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://github.com/Freshair129/GenesisBlock/blob/main/LICENSE")
                        distribution.set("repo")
                    }
                }
                developers {
                    developer {
                        id.set("Freshair129")
                        name.set("Freshair129")
                        url.set("https://github.com/Freshair129")
                    }
                }
                scm {
                    url.set("https://github.com/Freshair129/GenesisBlock")
                    connection.set("scm:git:https://github.com/Freshair129/GenesisBlock.git")
                    developerConnection.set("scm:git:ssh://git@github.com/Freshair129/GenesisBlock.git")
                }
            }

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

// Central requires every artifact to carry a detached PGP signature. Signing is
// CONDITIONAL on the key being present so that an ordinary build, a
// publishToMavenLocal, or CI without secrets all still work - the alternative
// is a module that cannot be built at all without a private key on the machine.
//
// The key lives only in CI, as the GPG_SIGNING_KEY / GPG_SIGNING_PASSWORD
// secrets, exactly like NPM_TOKEN. `useInMemoryPgpKeys` takes an ASCII-armored
// private key so nothing has to be written to a keyring on the runner.
//
// The Central publish workflow asserts the key is present before it starts, so
// a missing secret fails loudly there rather than silently producing an
// unsigned publication that Central rejects after upload.
signing {
    val signingKey: String? = System.getenv("GPG_SIGNING_KEY")
    val signingPassword: String? = System.getenv("GPG_SIGNING_PASSWORD")
    if (!signingKey.isNullOrBlank()) {
        useInMemoryPgpKeys(signingKey, signingPassword)
        sign(publishing.publications)
    } else {
        logger.lifecycle("signing: GPG_SIGNING_KEY not set - publications will be UNSIGNED (fine locally, rejected by Maven Central)")
    }
}
