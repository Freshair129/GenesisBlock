plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val centralArtifactVersion = providers.gradleProperty("centralArtifactVersion").orElse("0.1.1").get()

android {
    namespace = "dev.genesisblock.centralconsumer"
    compileSdk = 34

    defaultConfig {
        applicationId = "dev.genesisblock.centralconsumer"
        minSdk = 24
        targetSdk = 34
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    implementation("io.github.freshair129:genesisdb-android:$centralArtifactVersion")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test:runner:1.5.2")
    // 0.1.1 published JsonElement APIs but kept serialization JSON runtime-only.
    // 0.1.2 exports it as an API dependency; the workflow tests that without this workaround.
    if (centralArtifactVersion == "0.1.1") {
        androidTestImplementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
    }
    androidTestImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.8.1")
}
