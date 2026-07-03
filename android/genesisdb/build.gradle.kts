plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.serialization")
}

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
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.8.1")
}
